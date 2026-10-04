//! Frames de controle arbitrários: sem pânico, e toda mensagem aceita sobrevive à ida e volta.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vynpad_protocol::control::{Incoming, decode_frame, encode_frame};

fuzz_target!(|data: &[u8]| {
    if let Ok(Incoming::Message(msg)) = decode_frame(data) {
        let frame = encode_frame(&msg).expect("mensagem aceita precisa ser codificável");
        assert_eq!(decode_frame(&frame), Ok(Incoming::Message(msg)));
    }
});
