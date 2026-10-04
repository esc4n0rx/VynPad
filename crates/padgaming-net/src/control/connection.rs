//! Uma conexão de controle: handshake, sessão e keepalive.

use std::io;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use padgaming_driver::SlotId;
use padgaming_protocol::Secret;
use padgaming_protocol::control::{
    Bye, ByeReason, Config, ControlError, ControlMessage, Hello, Incoming, Reject, RejectReason,
    Uuid, Welcome, decode_body, encode_frame, frame_len,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Instant, sleep_until, timeout, timeout_at};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

use super::server::{Command, Live, Shared};
use super::{ApprovalRequest, ControlEvent, SUPPORTED_VERSIONS, SessionEnd, SessionHealth};
use crate::input::{SessionActivity, SessionSpec};
use crate::pairing::{AuthError, Authenticated};
use crate::random_bytes;

type Reader = ReadHalf<TlsStream<TcpStream>>;
type Writer = WriteHalf<TlsStream<TcpStream>>;

/// Falha ao ler uma mensagem.
enum ReadError {
    Io(io::Error),
    Control(ControlError),
}

impl ReadError {
    /// Depois de `BadMessage` o framing continua íntegro e dá para seguir lendo.
    const fn is_recoverable(&self) -> bool {
        matches!(self, Self::Control(ControlError::BadMessage(_)))
    }
}

async fn read_message<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Incoming, ReadError> {
    let mut prefix = [0u8; 4];
    reader
        .read_exact(&mut prefix)
        .await
        .map_err(ReadError::Io)?;
    let len = frame_len(prefix).map_err(ReadError::Control)?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).await.map_err(ReadError::Io)?;
    decode_body(&body).map_err(ReadError::Control)
}

async fn send<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &ControlMessage,
    limit: Duration,
) -> io::Result<()> {
    let frame = encode_frame(message).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    timeout(limit, async {
        writer.write_all(&frame).await?;
        writer.flush().await
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "escrita expirou"))?
}

fn reject(reason: RejectReason, message: &str) -> Reject {
    Reject {
        reason,
        retry_after_s: None,
        supported_versions: None,
        message: Some(message.to_owned()),
    }
}

/// Por que o handshake não virou sessão.
enum Refusal {
    /// Responder `reject` e fechar.
    Reject(Reject),
    /// Fechar sem dizer nada (erro interno ou client que sumiu).
    Close,
}

/// Uma sessão criada, pronta para o `welcome`.
struct Established {
    session_id: u32,
    slot: SlotId,
    device: Uuid,
    welcome: Welcome,
    paired: bool,
    commands: mpsc::UnboundedReceiver<Command>,
    activity: Arc<SessionActivity>,
}

/// Ponto de entrada da conexão.
pub(super) async fn run(
    shared: Arc<Shared>,
    acceptor: TlsAcceptor,
    stream: TcpStream,
    peer: std::net::SocketAddr,
    permit: super::server::Pending,
) {
    let limit = shared.config.write_timeout;
    let deadline = Instant::now() + shared.config.hello_timeout;

    // TLS + hello dentro do mesmo prazo de 10 s.
    let Ok(Ok(tls)) = timeout_at(deadline, acceptor.accept(stream)).await else {
        tracing::debug!(%peer, "handshake TLS não concluiu");
        return;
    };
    let (mut reader, mut writer) = tokio::io::split(tls);
    let hello = match timeout_at(deadline, read_message(&mut reader)).await {
        Ok(Ok(Incoming::Message(ControlMessage::Hello(hello)))) => hello,
        Ok(Err(ReadError::Io(_))) | Err(_) => return,
        // Mensagem malformada, frame inválido ou outra coisa antes do `hello` (§5.1).
        Ok(_) => {
            let message = ControlMessage::Reject(reject(
                RejectReason::BadRequest,
                "esperado um hello válido",
            ));
            let _ = send(&mut writer, &message, limit).await;
            shared.emit(ControlEvent::Rejected {
                peer: peer.ip(),
                device_id: None,
                reason: RejectReason::BadRequest,
            });
            return;
        }
    };

    let established = match establish(&shared, &hello, peer.ip()).await {
        Ok(established) => established,
        Err(Refusal::Reject(rejection)) => {
            tracing::info!(%peer, reason = ?rejection.reason, "conexão recusada");
            let reason = rejection.reason;
            let _ = send(&mut writer, &ControlMessage::Reject(rejection), limit).await;
            shared.emit(ControlEvent::Rejected {
                peer: peer.ip(),
                device_id: Some(hello.device_id.clone()),
                reason,
            });
            return;
        }
        Err(Refusal::Close) => return,
    };
    drop(permit); // O handshake acabou; a vaga pendente volta.

    let Established {
        session_id,
        slot,
        device,
        welcome,
        paired,
        commands,
        activity,
    } = established;
    let end = if let Err(error) = send(&mut writer, &ControlMessage::Welcome(welcome), limit).await
    {
        tracing::debug!(%peer, %error, "não consegui enviar o welcome");
        SessionEnd::Dropped
    } else {
        tracing::info!(%peer, session_id, slot = slot.index(), "sessão iniciada");
        if paired {
            shared.emit(ControlEvent::Paired {
                device_id: device.clone(),
                name: hello.name.clone(),
                model: hello.model.clone(),
            });
        }
        shared.emit(ControlEvent::SessionStarted {
            session_id,
            slot,
            device_id: device.clone(),
            peer: peer.ip(),
        });
        serve(&shared, reader, &mut writer, commands, &activity).await
    };
    finish(&shared, session_id, slot, &device, end);
    let _ = timeout(limit, writer.shutdown()).await;
}

/// Autentica, aloca o slot e registra a sessão UDP.
async fn establish(
    shared: &Arc<Shared>,
    hello: &Hello,
    peer: IpAddr,
) -> Result<Established, Refusal> {
    if !SUPPORTED_VERSIONS.contains(&hello.proto_ver) {
        return Err(Refusal::Reject(Reject {
            reason: RejectReason::Version,
            retry_after_s: None,
            supported_versions: Some(SUPPORTED_VERSIONS.to_vec()),
            message: Some(format!(
                "versão {} do protocolo não é suportada; o server aceita {SUPPORTED_VERSIONS:?}",
                hello.proto_ver
            )),
        }));
    }

    let auth = authenticate(shared, hello, peer).await?;
    let paired = auth.device_token.is_some();
    let device = auth.device_id.clone();

    let session_id;
    let slot;
    let key = random_bytes::<32>();
    let (command_tx, commands) = mpsc::unbounded_channel();
    // Consultado antes de pegar os locks: pode ir ao banco.
    let preferred = shared.profiles.preferred_slot(&device).await;
    {
        // Sorteio do id e alocação do slot sob o lock: dois handshakes simultâneos não podem
        // escolher o mesmo `session_id`. O id fica em `reserved` até a inserção em `live`, pois
        // o `setup` abaixo é assíncrono e o lock não pode atravessar o `await`.
        let live = shared.live();
        let mut reserved = shared.reserved();
        session_id = loop {
            let id = u32::from_le_bytes(random_bytes::<4>()) & 0x7FFF_FFFF;
            if id != 0 && !live.contains_key(&id) && !reserved.contains(&id) {
                break id;
            }
        };
        slot = match shared.slots.assign(&device, session_id, preferred) {
            Ok(slot) => slot,
            Err(padgaming_core::SlotError::NoSlot) => {
                return Err(Refusal::Reject(reject(
                    RejectReason::NoSlot,
                    "todos os controles estão em uso",
                )));
            }
            Err(error) => {
                tracing::error!(%error, "falha ao alocar o slot");
                return Err(Refusal::Close);
            }
        };
        reserved.insert(session_id);
    }

    let setup = shared.profiles.setup(slot, &device).await;
    let config = setup.config;
    let activity = shared.input.add_session(SessionSpec {
        session_id,
        key: Secret::new(key),
        profile: setup.profile,
    });
    {
        let mut live = shared.live();
        shared.reserved().remove(&session_id);
        live.insert(
            session_id,
            Live {
                device: device.clone(),
                slot,
                commands: command_tx,
                activity: Arc::clone(&activity),
                health: SessionHealth::Live,
            },
        );
        // Uma conexão anterior do mesmo dispositivo (que ainda não percebeu a queda) sai.
        for (id, other) in live.iter() {
            if *id != session_id && other.device == device {
                let _ = other.commands.send(Command::Bye(ByeReason::Kicked));
            }
        }
    }

    let config = Config {
        slot: slot.index(),
        ..config
    };
    Ok(Established {
        session_id,
        slot,
        device,
        paired,
        commands,
        activity,
        welcome: Welcome {
            proto_ver: hello.proto_ver,
            session_id,
            udp_port: shared.udp_port,
            udp_key: Secret::new(padgaming_protocol::control::Bytes(key)),
            device_token: auth.device_token,
            server_id: shared.server_id.clone(),
            server_name: shared.server_name.clone(),
            config,
        },
    })
}

async fn authenticate(
    shared: &Arc<Shared>,
    hello: &Hello,
    peer: IpAddr,
) -> Result<Authenticated, Refusal> {
    let runtime = Handle::current();
    let (shared, hello) = (Arc::clone(shared), hello.clone());
    // O `PairingService` é síncrono (pode usar o SQLite) e o gate de aprovação espera o usuário.
    let result = tokio::task::spawn_blocking(move || {
        shared
            .pairing
            .authenticate_with(&hello, peer, &|h| approve(&shared, &runtime, h, peer))
    })
    .await;
    match result {
        Ok(Ok(auth)) => Ok(auth),
        Ok(Err(AuthError::Rejected(rejection))) => Err(Refusal::Reject(rejection)),
        Ok(Err(AuthError::Store(error))) => {
            tracing::error!(%error, "falha no armazenamento de dispositivos");
            Err(Refusal::Close)
        }
        Err(error) => {
            tracing::error!(%error, "a autenticação falhou");
            Err(Refusal::Close)
        }
    }
}

/// O gate do pareamento: pergunta à UI, se a opção estiver ligada.
fn approve(shared: &Shared, runtime: &Handle, hello: &Hello, peer: IpAddr) -> bool {
    if !shared.require_approval.load(Ordering::Relaxed) {
        return true;
    }
    let Some(approver) = shared.approver() else {
        tracing::warn!("confirmação exigida, mas não há UI para perguntar: recusado");
        return false;
    };
    runtime.block_on(async {
        let (reply, answer) = oneshot::channel();
        let request = ApprovalRequest {
            device_id: hello.device_id.clone(),
            name: hello.name.clone(),
            model: hello.model.clone(),
            peer,
            reply,
        };
        if approver.send(request).await.is_err() {
            return false;
        }
        matches!(
            timeout(shared.config.approval_timeout, answer).await,
            Ok(Ok(true))
        )
    })
}

/// Uma sonda de RTT sem resposta por este tempo é dada como perdida.
const PROBE_LOST_AFTER: Duration = Duration::from_secs(2);

/// Depois do `welcome`: keepalive, `config` e `bye`. Devolve por que a sessão acabou.
async fn serve(
    shared: &Arc<Shared>,
    mut reader: Reader,
    writer: &mut Writer,
    mut commands: mpsc::UnboundedReceiver<Command>,
    activity: &SessionActivity,
) -> SessionEnd {
    let (ping_after, dead_after, limit, probe_every) = (
        shared.config.ping_after,
        shared.config.dead_after,
        shared.config.write_timeout,
        shared.config.rtt_probe_every,
    );
    let mut stop = shared.shutdown.subscribe();

    // Uma tarefa lê (a leitura de um frame não é cancelável dentro do `select!`).
    let (incoming_tx, mut incoming) = mpsc::channel::<Result<Incoming, ReadError>>(8);
    let reader_task = tokio::spawn(async move {
        loop {
            let message = read_message(&mut reader).await;
            if let Err(ReadError::Io(error)) = &message {
                tracing::debug!(%error, "leitura do canal de controle terminou");
            }
            let keep_going =
                message.is_ok() || message.as_ref().is_err_and(ReadError::is_recoverable);
            if incoming_tx.send(message).await.is_err() || !keep_going {
                break;
            }
        }
    });

    let mut last_recv = Instant::now();
    let mut last_sent = last_recv;
    // RTT: um `ping` de sonda por vez; o `pong` seguinte fecha a medida.
    let mut last_probe = last_recv;
    let mut probe_sent: Option<Instant> = None;
    let end = loop {
        let outcome = tokio::select! {
            message = incoming.recv() => match message {
                Some(Err(error)) if error.is_recoverable() => {
                    // `bad_message` depois do `hello`: só registra e descarta (§5.1).
                    tracing::debug!("mensagem de controle inválida descartada");
                    last_recv = Instant::now();
                    continue;
                }
                None | Some(Err(_)) => break SessionEnd::Dropped,
                Some(Ok(message)) => {
                    last_recv = Instant::now();
                    match message {
                        Incoming::Message(ControlMessage::Ping) => Some(ControlMessage::Pong),
                        Incoming::Message(ControlMessage::Bye(_)) => break SessionEnd::ClientBye,
                        Incoming::Message(ControlMessage::Pong) => {
                            if let Some(sent) = probe_sent.take() {
                                activity.metrics.record_rtt(sent.elapsed());
                            }
                            None
                        }
                        // O resto não faz sentido aqui.
                        Incoming::Message(_) | Incoming::Ignored => None,
                    }
                }
            },
            command = commands.recv() => match command {
                Some(Command::Config(config)) => Some(ControlMessage::Config(config)),
                Some(Command::Bye(reason)) => {
                    let _ = send(writer, &bye(reason), limit).await;
                    break match reason {
                        ByeReason::Revoked => SessionEnd::Revoked,
                        ByeReason::Shutdown => SessionEnd::Shutdown,
                        _ => SessionEnd::Kicked,
                    };
                }
                None => break SessionEnd::Shutdown,
            },
            () = sleep_until(last_sent + ping_after) => Some(ControlMessage::Ping),
            () = sleep_until(last_probe + probe_every) => {
                last_probe = Instant::now();
                // Sem resposta há mais de 2 s: a sonda foi perdida, manda outra.
                if probe_sent.is_some_and(|sent| sent.elapsed() < PROBE_LOST_AFTER) {
                    None
                } else {
                    probe_sent = Some(last_probe);
                    Some(ControlMessage::Ping)
                }
            }
            () = sleep_until(last_recv + dead_after) => {
                let _ = send(writer, &bye(ByeReason::Timeout), limit).await;
                break SessionEnd::Timeout;
            }
            _ = stop.changed() => {
                let _ = send(writer, &bye(ByeReason::Shutdown), limit).await;
                break SessionEnd::Shutdown;
            }
        };
        if let Some(message) = outcome {
            if send(writer, &message, limit).await.is_err() {
                break SessionEnd::Dropped;
            }
            last_sent = Instant::now();
        }
    };
    reader_task.abort();
    end
}

fn bye(reason: ByeReason) -> ControlMessage {
    ControlMessage::Bye(Bye {
        reason,
        message: None,
    })
}

/// Encerra a sessão: tira do UDP, do registro e resolve o slot.
fn finish(shared: &Shared, session_id: u32, slot: SlotId, device: &Uuid, end: SessionEnd) {
    shared.input.remove_session(session_id);
    // O slot pode ter mudado de lugar (`relocate`) desde o início da sessão.
    let slot = shared
        .live()
        .remove(&session_id)
        .map_or(slot, |live| live.slot);
    match end {
        // O TLS caiu ou ficou mudo: o slot espera a reconexão (30 s).
        SessionEnd::Dropped | SessionEnd::Timeout => {
            shared.slots.suspend(session_id);
        }
        SessionEnd::ClientBye | SessionEnd::Kicked | SessionEnd::Revoked | SessionEnd::Shutdown => {
            shared.slots.release(session_id);
        }
    }
    tracing::info!(session_id, ?end, "sessão encerrada");
    shared.emit(ControlEvent::SessionEnded {
        session_id,
        slot,
        device_id: device.clone(),
        reason: end,
    });
}
