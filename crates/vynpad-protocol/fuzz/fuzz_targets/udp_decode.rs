//! Datagramas arbitrários: a recepção nunca pode entrar em pânico, nos dois papéis.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vynpad_protocol::{KEY_LEN, ProtoError, Role, Secret, Session, parse_header};

fuzz_target!(|data: &[u8]| {
    for role in [Role::Server, Role::Client] {
        // Sessão com o session_id do próprio datagrama, para chegar até o AEAD.
        let session_id = match parse_header(data, role) {
            Ok(h) => h.session_id,
            Err(_) => 1,
        };
        let mut session = Session::new(role, session_id, &Secret::new([0x5A; KEY_LEN]));
        match session.receive(data) {
            // Forjar uma tag válida é inviável; se acontecer, é bug.
            Ok(p) => panic!("pacote arbitrário aceito: {p:?}"),
            Err(ProtoError::BufferTooSmall | ProtoError::SeqExhausted) => {
                panic!("erro de envio na recepção")
            }
            Err(_) => {}
        }
    }
});
