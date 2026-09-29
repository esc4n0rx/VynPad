//! Protocolo do `PadGaming` v1 (docs/00-visao-geral/PROTOCOLO.md).
//!
//! - **UDP** ([`packet`], [`crypto`], [`session`]): layout binário, ChaCha20-Poly1305 e
//!   anti-replay. É `no_std`, não aloca e não usa `unsafe`: roda no hot path de input.
//! - **Canal de controle** (`control`, feature `std`): mensagens JSON e framing `u32 LE`.
//!
//! Tudo é conferido contra os test vectors de `protocol/test-vectors/`.
//!
//! Fluxo no server, que tem várias sessões:
//!
//! ```
//! # use padgaming_protocol::*;
//! # let key = Secret::new([1u8; KEY_LEN]);
//! # let mut client = Session::new(Role::Client, 7, &key);
//! # let mut datagram = [0u8; MAX_PACKET_LEN];
//! # let n = client.send(&Payload::Ping(Ping::default()), &mut datagram).unwrap();
//! # let mut sessions = [Session::new(Role::Server, 7, &key)];
//! let header = parse_header(&datagram[..n], Role::Server)?;
//! let session = sessions
//!     .iter_mut()
//!     .find(|s| s.session_id() == header.session_id)
//!     .ok_or(ProtoError::UnknownSession)?;
//! let packet = session.receive_parsed(&header, &datagram[..n])?;
//! assert!(matches!(packet.payload, Payload::Ping(_)));
//! # Ok::<(), ProtoError>(())
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

pub mod buttons;
#[cfg(feature = "std")]
pub mod control;
pub mod crypto;
pub mod error;
pub mod packet;
#[cfg(feature = "std")]
pub mod pairing_uri;
pub mod secret;
pub mod session;

pub use buttons::Buttons;
pub use crypto::{KEY_LEN, SessionCipher, open_packet, seal_packet};
pub use error::ProtoError;
pub use packet::{
    Direction, HEADER_LEN, InputState, MAGIC, MAX_PACKET_LEN, MAX_PAYLOAD_LEN, Motion, Packet,
    PacketHeader, PacketType, Payload, Ping, Pong, Role, Rumble, TAG_LEN, VERSION, parse_header,
};
pub use secret::Secret;
pub use session::{ReplayWindow, SeqCounter, Session};
