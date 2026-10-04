//! A thread `input-rx`: socket UDP bloqueante ligado ao [`InputEngine`].

use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, unbounded};
use socket2::{Domain, Protocol, Socket, Type};
use vynpad_core::SlotManager;
use vynpad_protocol::MAX_PACKET_LEN;

use super::engine::{Handled, InputEngine, SessionActivity, SessionEntry, SessionSpec};
use super::platform;
use super::stats::InputStats;

/// Porta UDP de input (PROTOCOLO §1).
pub const DEFAULT_PORT: u16 = 47801;

/// Configuração do servidor de input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputConfig {
    /// Endereço de escuta. O padrão é `0.0.0.0:47801` (o protocolo é IPv4).
    pub bind: SocketAddr,
    /// Tamanho do buffer de recepção do socket, em bytes.
    pub recv_buffer_bytes: usize,
    /// De quanto em quanto tempo o loop confere o pedido de parada quando não chega nada.
    pub poll_interval: Duration,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from((Ipv4Addr::UNSPECIFIED, DEFAULT_PORT)),
            recv_buffer_bytes: 256 * 1024,
            poll_interval: Duration::from_millis(100),
        }
    }
}

enum Command {
    Add(Box<SessionEntry>),
    Remove(u32),
    Rumble {
        session_id: u32,
        large: u8,
        small: u8,
    },
}

/// O servidor de input em execução.
pub struct InputServer {
    commands: Sender<Command>,
    stop: Arc<AtomicBool>,
    stats: Arc<InputStats>,
    local_addr: SocketAddr,
    epoch: Instant,
    thread: Option<JoinHandle<()>>,
}

impl InputServer {
    /// Abre o socket e inicia a thread `input-rx`.
    pub fn start(config: InputConfig, slots: Arc<SlotManager>) -> io::Result<Self> {
        let socket = open_socket(&config)?;
        let local_addr = socket.local_addr()?;
        let epoch = Instant::now();
        let stats = Arc::new(InputStats::default());
        let stop = Arc::new(AtomicBool::new(false));
        let (commands, inbox) = unbounded();

        let engine = InputEngine::new(slots, Arc::clone(&stats), epoch);
        let thread = std::thread::Builder::new().name("input-rx".into()).spawn({
            let stop = Arc::clone(&stop);
            move || run(&socket, engine, &inbox, &stop)
        })?;
        tracing::info!(%local_addr, "servidor de input UDP iniciado");
        Ok(Self {
            commands,
            stop,
            stats,
            local_addr,
            epoch,
            thread: Some(thread),
        })
    }

    /// Endereço em que o socket está escutando (útil com a porta 0, nos testes).
    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Contadores e histograma do loop.
    pub fn stats(&self) -> &Arc<InputStats> {
        &self.stats
    }

    /// Registra uma sessão (o `welcome` já foi montado). A thread de input a adota antes do
    /// próximo pacote que processar. Devolve o acompanhamento de atividade da sessão.
    pub fn add_session(&self, spec: SessionSpec) -> Arc<SessionActivity> {
        let (entry, activity) = SessionEntry::new(spec, self.epoch);
        let _ = self.commands.send(Command::Add(Box::new(entry)));
        activity
    }

    /// Remove a sessão; os pacotes dela passam a ser descartados.
    pub fn remove_session(&self, session_id: u32) {
        let _ = self.commands.send(Command::Remove(session_id));
    }

    /// Manda o celular da sessão vibrar (`0, 0` para). A thread de input envia o `Rumble` e o
    /// retransmite a cada 100 ms até o client confirmar (T-204).
    pub fn set_rumble(&self, session_id: u32, large: u8, small: u8) {
        let _ = self.commands.send(Command::Rumble {
            session_id,
            large,
            small,
        });
    }

    /// Modo de desenvolvimento (só em builds de debug): registra a sessão fixa
    /// [`DEV_SESSION_ID`] com a chave pública [`DEV_KEY`], para o M1 funcionar antes do
    /// handshake TLS (T-302). Quem usa isso não tem autenticação nenhuma.
    #[cfg(debug_assertions)]
    pub fn add_insecure_dev_session(
        &self,
        profile: Arc<vynpad_mapping::ProfileCell>,
    ) -> Arc<SessionActivity> {
        tracing::warn!("sessão de desenvolvimento SEM autenticação registrada (--insecure-dev)");
        self.add_session(SessionSpec {
            session_id: DEV_SESSION_ID,
            key: vynpad_protocol::Secret::new(DEV_KEY),
            profile,
        })
    }

    /// Pede a parada e espera a thread terminar (em até `poll_interval`).
    pub fn shutdown(mut self) {
        self.stop_and_join();
    }

    fn stop_and_join(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
            tracing::info!("servidor de input UDP encerrado");
        }
    }
}

impl Drop for InputServer {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

/// `session_id` da sessão de desenvolvimento.
#[cfg(debug_assertions)]
pub const DEV_SESSION_ID: u32 = 1;
/// Chave da sessão de desenvolvimento. **Pública de propósito**: não use fora de testes.
#[cfg(debug_assertions)]
pub const DEV_KEY: [u8; vynpad_protocol::KEY_LEN] = [0x42; vynpad_protocol::KEY_LEN];

fn open_socket(config: &InputConfig) -> io::Result<UdpSocket> {
    let socket = Socket::new(
        Domain::for_address(config.bind),
        Type::DGRAM,
        Some(Protocol::UDP),
    )?;
    // O SO manda no que é possível; se recusar o tamanho, seguimos com o padrão dele.
    if let Err(error) = socket.set_recv_buffer_size(config.recv_buffer_bytes) {
        tracing::warn!(%error, "não consegui ajustar o buffer de recepção do socket UDP");
    }
    socket.bind(&config.bind.into())?;
    socket.set_read_timeout(Some(config.poll_interval))?;
    Ok(socket.into())
}

fn drain(engine: &mut InputEngine, inbox: &Receiver<Command>) {
    while let Ok(command) = inbox.try_recv() {
        match command {
            Command::Add(entry) => {
                if !engine.add_session(*entry) {
                    tracing::error!("tabela de sessões UDP cheia; sessão ignorada");
                }
            }
            Command::Remove(id) => engine.remove_session(id),
            Command::Rumble {
                session_id,
                large,
                small,
            } => {
                engine.set_rumble(session_id, large, small);
            }
        }
    }
}

fn run(socket: &UdpSocket, mut engine: InputEngine, inbox: &Receiver<Command>, stop: &AtomicBool) {
    let tuning = platform::tune_current_thread();
    tracing::info!(?tuning, "thread input-rx ajustada");

    let mut buf = [0u8; 1500];
    let mut out = [0u8; MAX_PACKET_LEN];
    while !stop.load(Ordering::Relaxed) {
        match socket.recv_from(&mut buf) {
            Ok((len, src)) => {
                let rx = Instant::now();
                // Sessões novas valem antes deste pacote (o client só manda depois do `welcome`).
                drain(&mut engine, inbox);
                if let Handled::Pong { reply_len, to } =
                    engine.handle(&buf[..len], src, rx, &mut out)
                {
                    let _ = socket.send_to(&out[..reply_len], to);
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::WouldBlock | ErrorKind::TimedOut => drain(&mut engine, inbox),
                // No Windows, um ICMP "porta inacessível" de um envio anterior aparece aqui.
                ErrorKind::ConnectionReset | ErrorKind::Interrupted => {}
                _ => {
                    tracing::error!(%error, "erro no socket UDP de input");
                    std::thread::sleep(Duration::from_millis(10));
                }
            },
        }
        // A vibração devida (nova ou ainda sem confirmação) sai a cada volta: a espera do socket
        // limita o atraso ao `poll_interval`.
        engine.send_rumble(Instant::now(), &mut out, |bytes, to| {
            let _ = socket.send_to(bytes, to);
        });
    }
}
