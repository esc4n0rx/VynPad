//! Firewall do Windows (T-306): a causa nº 1 de "não conecta" é o firewall bloqueando UDP e mDNS.
//!
//! - [`check`] lê as regras de entrada (COM, `INetFwPolicy2`, sem precisar de admin) e diz se as
//!   portas do `PadGaming` estão liberadas para este executável no perfil de rede atual.
//! - [`fix`] cria as regras com um único pedido de elevação (UAC).
//! - [`self_test`] confere, pela interface de LAN, se os serviços do próprio server respondem.
//!
//! A avaliação ([`evaluate`]) e a montagem do comando ([`fix_command`]) são código puro e rodam
//! (e são testadas) em qualquer sistema; só a leitura das regras e a elevação são do Windows.
//! Em outros sistemas [`check`] devolve um status "não se aplica" (o Linux é a T-802).

use std::io;
use std::path::{Path, PathBuf};

use padgaming_protocol::discovery::BROADCAST_PORT;

#[cfg(windows)]
mod windows;

/// Porta mDNS (multicast DNS).
pub const MDNS_PORT: u16 = 5353;
/// Página de rede do Windows, onde o usuário troca o perfil "Pública" por "Privada".
pub const NETWORK_SETTINGS_URI: &str = "ms-settings:network-status";
/// Prefixo do nome das regras criadas pelo app (o instalador, T-703, usa o mesmo).
pub const RULE_PREFIX: &str = "PadGaming";

const PROFILE_DOMAIN: i32 = 1;
const PROFILE_PRIVATE: i32 = 2;
const PROFILE_PUBLIC: i32 = 4;
const PROTOCOL_TCP: i32 = 6;
const PROTOCOL_UDP: i32 = 17;
const PROTOCOL_ANY: i32 = 256;

/// Portas do server (podem mudar nas configurações).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirewallPorts {
    /// Canal de controle (TCP).
    pub control_tcp: u16,
    /// Input (UDP).
    pub input_udp: u16,
    /// Descoberta por broadcast (UDP).
    pub discovery_udp: u16,
}

impl Default for FirewallPorts {
    fn default() -> Self {
        Self {
            control_tcp: crate::control::DEFAULT_PORT,
            input_udp: crate::input::DEFAULT_PORT,
            discovery_udp: BROADCAST_PORT,
        }
    }
}

/// Protocolo de transporte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// TCP.
    Tcp,
    /// UDP.
    Udp,
}

impl Transport {
    const fn number(self) -> i32 {
        match self {
            Self::Tcp => PROTOCOL_TCP,
            Self::Udp => PROTOCOL_UDP,
        }
    }

    const fn netsh(self) -> &'static str {
        match self {
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
        }
    }
}

/// Uma porta de entrada de que o server precisa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requirement {
    /// Protocolo.
    pub transport: Transport,
    /// Porta local.
    pub port: u16,
    /// Para que serve (texto para a UI).
    pub purpose: &'static str,
}

impl Requirement {
    /// Nome da regra criada para esta porta.
    pub fn rule_name(&self) -> String {
        format!(
            "{RULE_PREFIX} {} {} ({})",
            self.transport.netsh(),
            self.port,
            self.purpose
        )
    }
}

/// As portas de entrada de que o server precisa.
pub fn requirements(ports: FirewallPorts) -> [Requirement; 4] {
    [
        Requirement {
            transport: Transport::Tcp,
            port: ports.control_tcp,
            purpose: "controle",
        },
        Requirement {
            transport: Transport::Udp,
            port: ports.input_udp,
            purpose: "input",
        },
        Requirement {
            transport: Transport::Udp,
            port: ports.discovery_udp,
            purpose: "descoberta",
        },
        Requirement {
            transport: Transport::Udp,
            port: MDNS_PORT,
            purpose: "mDNS",
        },
    ]
}

/// Uma regra do firewall, como o COM a devolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleInfo {
    /// Nome.
    pub name: String,
    /// Habilitada.
    pub enabled: bool,
    /// Regra de entrada.
    pub inbound: bool,
    /// Ação "permitir".
    pub allow: bool,
    /// Executável a que a regra se restringe (`None` = qualquer programa).
    pub application: Option<String>,
    /// Protocolo IP (6 TCP, 17 UDP, 256 qualquer).
    pub protocol: i32,
    /// Portas locais (`"47800"`, `"47801,47802"`, `"5000-6000"`, `"*"`; vazio = todas).
    pub local_ports: String,
    /// Máscara de perfis (1 domínio, 2 privado, 4 público).
    pub profiles: i32,
}

/// Perfis de rede ativos agora.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActiveProfiles {
    /// Rede de domínio.
    pub domain: bool,
    /// Rede privada.
    pub private: bool,
    /// Rede pública.
    pub public: bool,
}

impl ActiveProfiles {
    /// A partir da máscara de `CurrentProfileTypes`.
    pub const fn from_mask(mask: i32) -> Self {
        Self {
            domain: mask & PROFILE_DOMAIN != 0,
            private: mask & PROFILE_PRIVATE != 0,
            public: mask & PROFILE_PUBLIC != 0,
        }
    }

    const fn mask(self) -> i32 {
        (if self.domain { PROFILE_DOMAIN } else { 0 })
            | (if self.private { PROFILE_PRIVATE } else { 0 })
            | (if self.public { PROFILE_PUBLIC } else { 0 })
    }

    /// Alguma rede ativa é pública: as regras do `PadGaming` (só Privado/Domínio) não valem nela e
    /// o celular provavelmente não conecta. A UI avisa e aponta [`NETWORK_SETTINGS_URI`].
    pub const fn has_public(self) -> bool {
        self.public
    }
}

/// Uma porta e se ela está liberada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortStatus {
    /// O que é preciso.
    pub requirement: Requirement,
    /// Há regra de entrada que a libera para este executável em todos os perfis ativos.
    pub allowed: bool,
}

/// O estado do firewall para o `PadGaming`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirewallStatus {
    /// O sistema tem firewall gerenciável (Windows).
    pub applicable: bool,
    /// O firewall está ligado em algum perfil ativo. Desligado, nada é bloqueado.
    pub firewall_on: bool,
    /// Perfis de rede ativos.
    pub profiles: ActiveProfiles,
    /// Situação de cada porta.
    pub ports: Vec<PortStatus>,
}

impl FirewallStatus {
    /// Status para sistemas sem o firewall do Windows.
    pub fn not_applicable() -> Self {
        Self {
            applicable: false,
            firewall_on: false,
            profiles: ActiveProfiles::default(),
            ports: Vec::new(),
        }
    }

    /// Nada bloqueia o `PadGaming`: o firewall está desligado ou todas as portas estão liberadas.
    pub fn ok(&self) -> bool {
        !self.applicable || !self.firewall_on || self.ports.iter().all(|p| p.allowed)
    }

    /// As portas que ainda estão bloqueadas.
    pub fn missing(&self) -> impl Iterator<Item = &Requirement> {
        self.ports
            .iter()
            .filter(|p| !p.allowed)
            .map(|p| &p.requirement)
    }

    /// Precisa mostrar o aviso de rede pública.
    pub const fn public_network_warning(&self) -> bool {
        self.applicable && self.profiles.has_public()
    }
}

/// `port` está na lista de portas de uma regra? Aceita `*`, vazio, números, intervalos
/// (`5000-6000`) e ignora palavras-chave (`RPC`, `IPHTTPSIn`...).
fn ports_include(list: &str, port: u16) -> bool {
    let list = list.trim();
    if list.is_empty() || list == "*" {
        return true;
    }
    list.split(',').map(str::trim).any(|item| {
        if let Some((low, high)) = item.split_once('-') {
            matches!(
                (low.trim().parse::<u16>(), high.trim().parse::<u16>()),
                (Ok(low), Ok(high)) if (low..=high).contains(&port)
            )
        } else {
            item.parse::<u16>() == Ok(port)
        }
    })
}

/// Mesmo arquivo? Caminhos do Windows não diferenciam maiúsculas e aceitam `/` ou `\`.
fn same_path(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.trim().replace('/', "\\").to_lowercase();
    norm(a) == norm(b)
}

/// A regra libera `requirement` para `exe`? Devolve os perfis que ela cobre.
fn rule_covers(rule: &RuleInfo, requirement: &Requirement, exe: &str) -> Option<i32> {
    let applies = rule.enabled
        && rule.inbound
        && rule.allow
        && (rule.protocol == requirement.transport.number() || rule.protocol == PROTOCOL_ANY)
        && ports_include(&rule.local_ports, requirement.port)
        && rule
            .application
            .as_deref()
            .is_none_or(|app| app.trim().is_empty() || app == "*" || same_path(app, exe));
    applies.then_some(rule.profiles)
}

/// Decide, a partir das regras lidas, se as portas estão liberadas.
///
/// Uma porta está liberada se a união dos perfis das regras que a permitem cobre todos os perfis
/// ativos (uma regra só para "Privado" não vale numa rede "Pública").
pub fn evaluate(
    rules: &[RuleInfo],
    exe: &Path,
    ports: FirewallPorts,
    profiles: ActiveProfiles,
    firewall_on: bool,
) -> FirewallStatus {
    let exe = exe.to_string_lossy();
    let active = profiles.mask();
    let port_status = requirements(ports)
        .into_iter()
        .map(|requirement| {
            let covered = rules
                .iter()
                .filter_map(|rule| rule_covers(rule, &requirement, &exe))
                .fold(0, |all, mask| all | mask);
            PortStatus {
                requirement,
                allowed: active != 0 && covered & active == active,
            }
        })
        .collect();
    FirewallStatus {
        applicable: true,
        firewall_on,
        profiles,
        ports: port_status,
    }
}

/// Erro ao ler ou corrigir o firewall.
#[derive(Debug, thiserror::Error)]
pub enum FirewallError {
    /// O usuário recusou o pedido de elevação (UAC).
    #[error("o pedido de permissão de administrador foi recusado")]
    Cancelled,
    /// O caminho do executável tem caracteres que não dá para passar com segurança ao `netsh`.
    #[error("caminho do executável não suportado: {0}")]
    BadPath(String),
    /// O comando elevado terminou com erro.
    #[error("o netsh terminou com código {0}")]
    CommandFailed(u32),
    /// Falha do sistema (COM, elevação).
    #[error("{0}")]
    System(String),
    /// Não é Windows.
    #[error("o firewall do Windows só existe no Windows")]
    Unsupported,
}

impl From<io::Error> for FirewallError {
    fn from(error: io::Error) -> Self {
        Self::System(error.to_string())
    }
}

/// Monta a linha de comando (para `cmd /c`) que recria as regras do `PadGaming`: apaga as antigas
/// pelo nome e cria uma regra de entrada por porta, restrita ao executável e aos perfis
/// Privado e Domínio.
///
/// O caminho vai entre aspas; `"` não existe em nomes de arquivo do Windows e `%` seria
/// expandido pelo `cmd`, então este é recusado.
pub fn fix_command(exe: &Path, ports: FirewallPorts) -> Result<String, FirewallError> {
    let exe = exe.to_string_lossy();
    if exe.contains(['"', '%', '\r', '\n', '\0']) || exe.is_empty() {
        return Err(FirewallError::BadPath(exe.into_owned()));
    }
    let reqs = requirements(ports);
    let mut commands: Vec<String> = reqs
        .iter()
        .map(|r| {
            format!(
                "netsh advfirewall firewall delete rule name=\"{}\" >nul 2>&1",
                r.rule_name()
            )
        })
        .collect();
    // `&` ignora o resultado dos deletes (não havia regra); `&&` encadeia os adds, então o
    // código de saída final é o do primeiro add que falhar.
    let adds: Vec<String> = reqs
        .iter()
        .map(|r| {
            format!(
                "netsh advfirewall firewall add rule name=\"{}\" dir=in action=allow \
                 program=\"{exe}\" enable=yes profile=private,domain protocol={} localport={}",
                r.rule_name(),
                r.transport.netsh(),
                r.port
            )
        })
        .collect();
    commands.push(adds.join(" && "));
    Ok(commands.join(" & "))
}

/// Lê o estado do firewall para `exe`. No Windows usa o COM; em outros sistemas devolve
/// [`FirewallStatus::not_applicable`].
#[cfg_attr(not(windows), allow(unused_variables))]
pub fn check(exe: &Path, ports: FirewallPorts) -> Result<FirewallStatus, FirewallError> {
    #[cfg(windows)]
    {
        windows::check(exe, ports)
    }
    #[cfg(not(windows))]
    {
        Ok(FirewallStatus::not_applicable())
    }
}

/// Cria as regras do `PadGaming` para `exe`, pedindo elevação (UAC) uma única vez, e devolve o
/// estado depois da correção.
#[cfg_attr(not(windows), allow(unused_variables))]
pub fn fix(exe: &Path, ports: FirewallPorts) -> Result<FirewallStatus, FirewallError> {
    #[cfg(windows)]
    {
        let command = fix_command(exe, ports)?;
        windows::run_elevated(&command)?;
        windows::check(exe, ports)
    }
    #[cfg(not(windows))]
    {
        Err(FirewallError::Unsupported)
    }
}

/// O executável do processo atual (para as regras).
pub fn current_exe() -> io::Result<PathBuf> {
    std::env::current_exe()
}

/// Resultado de uma sonda do autodiagnóstico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// O que foi testado.
    pub what: &'static str,
    /// Respondeu.
    pub ok: bool,
}

/// Autodiagnóstico ("Testar conexão"): pela interface de LAN `lan_ip`, confere se o canal de
/// controle aceita conexão TCP e se o respondedor de broadcast responde ao `PGDISC1`.
///
/// Limite honesto: o tráfego de um processo para o próprio IP não passa pelo filtro de entrada do
/// firewall, então isto só prova que os serviços estão de pé e ligados à LAN, não que o firewall
/// deixaria um celular entrar. Quem prova isso é um aparelho externo (o `padgaming-sim` em
/// outro PC).
pub fn self_test(lan_ip: std::net::Ipv4Addr, ports: FirewallPorts) -> Vec<Probe> {
    use std::net::{SocketAddr, TcpStream, UdpSocket};
    use std::time::Duration;

    let timeout = Duration::from_millis(800);
    let control =
        TcpStream::connect_timeout(&SocketAddr::from((lan_ip, ports.control_tcp)), timeout).is_ok();
    let discovery = (|| {
        let socket = UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).ok()?;
        socket.set_read_timeout(Some(timeout)).ok()?;
        socket
            .send_to(
                padgaming_protocol::discovery::BROADCAST_REQUEST,
                SocketAddr::from((lan_ip, ports.discovery_udp)),
            )
            .ok()?;
        let mut buf = [0u8; 512];
        let (n, _) = socket.recv_from(&mut buf).ok()?;
        padgaming_protocol::discovery::DiscoveryResponse::from_json(&buf[..n]).ok()
    })()
    .is_some();
    vec![
        Probe {
            what: "canal de controle (TCP)",
            ok: control,
        },
        Probe {
            what: "descoberta (UDP)",
            ok: discovery,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Program Files\PadGaming\padgaming.exe";

    fn rule(name: &str, protocol: i32, ports: &str, profiles: i32) -> RuleInfo {
        RuleInfo {
            name: name.into(),
            enabled: true,
            inbound: true,
            allow: true,
            application: Some(EXE.into()),
            protocol,
            local_ports: ports.into(),
            profiles,
        }
    }

    fn private() -> ActiveProfiles {
        ActiveProfiles::from_mask(PROFILE_PRIVATE)
    }

    fn status(rules: &[RuleInfo], profiles: ActiveProfiles) -> FirewallStatus {
        evaluate(
            rules,
            Path::new(EXE),
            FirewallPorts::default(),
            profiles,
            true,
        )
    }

    /// As 4 regras que o `fix` cria.
    fn ours(profiles: i32) -> Vec<RuleInfo> {
        vec![
            rule("c", PROTOCOL_TCP, "47800", profiles),
            rule("i", PROTOCOL_UDP, "47801", profiles),
            rule("d", PROTOCOL_UDP, "47802", profiles),
            rule("m", PROTOCOL_UDP, "5353", profiles),
        ]
    }

    #[test]
    fn without_rules_every_port_is_blocked_and_the_status_is_not_ok() {
        let s = status(&[], private());
        assert!(!s.ok());
        assert_eq!(s.missing().count(), 4);
    }

    #[test]
    fn our_rules_for_private_and_domain_cover_a_private_network() {
        let s = status(&ours(PROFILE_PRIVATE | PROFILE_DOMAIN), private());
        assert!(s.ok(), "{s:?}");
        assert_eq!(s.missing().count(), 0);
    }

    #[test]
    fn a_public_network_is_not_covered_and_raises_the_warning() {
        let public = ActiveProfiles::from_mask(PROFILE_PUBLIC);
        let s = status(&ours(PROFILE_PRIVATE | PROFILE_DOMAIN), public);
        assert!(!s.ok());
        assert!(s.public_network_warning());
        assert!(!status(&ours(PROFILE_PRIVATE), private()).public_network_warning());
    }

    #[test]
    fn several_active_profiles_must_all_be_covered() {
        let both = ActiveProfiles::from_mask(PROFILE_PRIVATE | PROFILE_PUBLIC);
        assert!(!status(&ours(PROFILE_PRIVATE), both).ok());
        assert!(status(&ours(PROFILE_PRIVATE | PROFILE_PUBLIC), both).ok());
        // Duas regras que juntas cobrem os perfis também valem.
        let mut split = ours(PROFILE_PRIVATE);
        split.extend(ours(PROFILE_PUBLIC));
        assert!(status(&split, both).ok());
    }

    #[test]
    fn a_rule_for_another_program_or_that_is_disabled_or_blocks_does_not_count() {
        let mut other_app = ours(PROFILE_PRIVATE);
        for r in &mut other_app {
            r.application = Some(r"C:\Outro\app.exe".into());
        }
        assert!(!status(&other_app, private()).ok());
        let mut disabled = ours(PROFILE_PRIVATE);
        for r in &mut disabled {
            r.enabled = false;
        }
        assert!(!status(&disabled, private()).ok());
        let mut blocking = ours(PROFILE_PRIVATE);
        for r in &mut blocking {
            r.allow = false;
        }
        assert!(!status(&blocking, private()).ok());
        let mut outbound = ours(PROFILE_PRIVATE);
        for r in &mut outbound {
            r.inbound = false;
        }
        assert!(!status(&outbound, private()).ok());
    }

    #[test]
    fn program_paths_compare_ignoring_case_and_slashes() {
        let mut rules = ours(PROFILE_PRIVATE);
        for r in &mut rules {
            r.application = Some(EXE.to_uppercase().replace('\\', "/"));
        }
        assert!(status(&rules, private()).ok());
    }

    #[test]
    fn a_rule_for_any_program_or_any_protocol_and_port_lists_count() {
        let any = RuleInfo {
            application: None,
            protocol: PROTOCOL_ANY,
            local_ports: "47800-47802, 5353".into(),
            ..rule("port-based", 0, "", PROFILE_PRIVATE)
        };
        assert!(status(&[any], private()).ok());
        let star = RuleInfo {
            application: Some("*".into()),
            protocol: PROTOCOL_UDP,
            local_ports: "*".into(),
            ..rule("all-udp", 0, "", PROFILE_PRIVATE)
        };
        let s = status(&[star], private());
        // Libera todo o UDP, mas o controle é TCP.
        assert_eq!(s.missing().count(), 1);
        assert_eq!(s.missing().next().unwrap().transport, Transport::Tcp);
    }

    #[test]
    fn the_wrong_protocol_or_port_does_not_count() {
        let mut rules = ours(PROFILE_PRIVATE);
        rules[0].protocol = PROTOCOL_UDP; // controle liberado só em UDP
        rules[1].local_ports = "47811".into();
        let s = status(&rules, private());
        assert_eq!(s.missing().count(), 2);
    }

    #[test]
    fn a_disabled_firewall_blocks_nothing() {
        let s = evaluate(
            &[],
            Path::new(EXE),
            FirewallPorts::default(),
            private(),
            false,
        );
        assert!(s.ok());
        assert!(!FirewallStatus::not_applicable().public_network_warning());
        assert!(FirewallStatus::not_applicable().ok());
    }

    #[test]
    fn no_active_profile_is_never_reported_as_covered() {
        let s = status(&ours(PROFILE_PRIVATE), ActiveProfiles::default());
        assert!(s.missing().count() > 0);
    }

    #[test]
    fn port_lists_accept_ranges_stars_and_ignore_keywords() {
        assert!(ports_include("47800", 47800));
        assert!(ports_include("1,2, 47800 ,9", 47800));
        assert!(ports_include("47000-48000", 47801));
        assert!(!ports_include("47000-47100", 47801));
        assert!(ports_include("*", 1));
        assert!(ports_include("", 1));
        assert!(!ports_include("RPC,IPHTTPSIn", 47800));
        assert!(!ports_include("abc-def", 47800));
    }

    #[test]
    fn configured_ports_drive_the_requirements() {
        let ports = FirewallPorts {
            control_tcp: 50000,
            input_udp: 50001,
            discovery_udp: 50002,
        };
        let reqs = requirements(ports);
        assert_eq!(
            reqs.map(|r| (r.transport, r.port)),
            [
                (Transport::Tcp, 50000),
                (Transport::Udp, 50001),
                (Transport::Udp, 50002),
                (Transport::Udp, 5353)
            ]
        );
    }

    #[test]
    fn the_fix_command_deletes_then_adds_one_scoped_rule_per_port() {
        let command = fix_command(Path::new(EXE), FirewallPorts::default()).unwrap();
        assert_eq!(command.matches("delete rule").count(), 4);
        assert_eq!(command.matches("add rule").count(), 4);
        for fragment in [
            r#"name="PadGaming TCP 47800 (controle)""#,
            r#"name="PadGaming UDP 47801 (input)""#,
            r#"name="PadGaming UDP 47802 (descoberta)""#,
            r#"name="PadGaming UDP 5353 (mDNS)""#,
            "protocol=TCP localport=47800",
            "protocol=UDP localport=5353",
            "profile=private,domain",
            "dir=in action=allow",
            r#"program="C:\Program Files\PadGaming\padgaming.exe""#,
        ] {
            assert!(command.contains(fragment), "faltou {fragment}\n{command}");
        }
        // Nunca libera o perfil público.
        assert!(!command.contains("public"));
        // Os deletes são independentes (`&`); os adds encadeiam (`&&`) para propagar a falha.
        let adds_start = command.find("add rule").unwrap();
        assert_eq!(command[adds_start..].matches(" && ").count(), 3);
        assert!(!command[..adds_start].contains("&&"));
    }

    #[test]
    fn dangerous_paths_are_refused() {
        for bad in [r#"C:\a"b.exe"#, r"C:\%TEMP%\a.exe", "C:\\a\nb.exe", ""] {
            assert!(
                matches!(
                    fix_command(Path::new(bad), FirewallPorts::default()),
                    Err(FirewallError::BadPath(_))
                ),
                "{bad:?}"
            );
        }
        // `&`, `^` e espaços são seguros entre aspas.
        assert!(fix_command(Path::new(r"C:\R&D ^x\a.exe"), FirewallPorts::default()).is_ok());
    }

    #[test]
    fn non_windows_reports_not_applicable_and_windows_reads_the_rules() {
        let status = check(Path::new(EXE), FirewallPorts::default());
        #[cfg(not(windows))]
        assert!(!status.unwrap().applicable);
        #[cfg(windows)]
        {
            // Ler as regras não exige admin; o resultado depende da máquina.
            let status = status.expect("o COM do firewall deve responder");
            assert!(status.applicable);
            assert_eq!(status.ports.len(), 4);
        }
    }
}
