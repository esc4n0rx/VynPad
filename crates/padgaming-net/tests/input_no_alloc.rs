//! O loop de input não pode alocar (T-301): mede o `InputEngine` com um allocator contador.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use padgaming_core::SlotManager;
use padgaming_driver::MockDriver;
use padgaming_mapping::{CompiledProfile, Profile, ProfileCell};
use padgaming_net::input::{Handled, InputEngine, InputStats, SessionEntry, SessionSpec};
use padgaming_protocol::control::Uuid;
use padgaming_protocol::{
    Buttons, InputState, MAX_PACKET_LEN, Motion, Payload, Ping, Role, Secret, Session,
};

struct CountingAlloc;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
    static COUNTING: Cell<bool> = const { Cell::new(false) };
}

fn note_allocation() {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
    }
}

// SAFETY: só repassa ao alocador do sistema, contando as chamadas.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note_allocation();
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note_allocation();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

#[test]
fn handling_datagrams_does_not_allocate() {
    const SESSION: u32 = 7;
    let key = Secret::new([0x33; 32]);
    let slots = Arc::new(SlotManager::new(Arc::new(MockDriver::with_capacity(16))));
    let device = Uuid::parse("123e4567-e89b-42d3-a456-426614174000").unwrap();
    slots.assign(&device, SESSION, None).unwrap();

    let mut profile = Profile::standard();
    profile.gyro.enabled = true;
    let cell = Arc::new(ProfileCell::new(
        CompiledProfile::compile(&profile).unwrap(),
    ));
    let stats = Arc::new(InputStats::default());
    let epoch = Instant::now();
    let mut engine = InputEngine::new(slots, stats.clone(), epoch);
    let (entry, _activity) = SessionEntry::new(
        SessionSpec {
            session_id: SESSION,
            key: key.clone(),
            profile: cell,
        },
        epoch,
    );
    assert!(engine.add_session(entry));

    // Pacotes prontos (seq crescente), de três tipos, criados antes de contar.
    let mut client = Session::new(Role::Client, SESSION, &key);
    let pad_state = InputState {
        buttons: Buttons::A | Buttons::RIGHT_SHOULDER,
        lx: 12_000,
        ly: -9_000,
        rt: 200,
        ..InputState::default()
    };
    let motion = Motion {
        gx: 300,
        gy: -800,
        ..Motion::default()
    };
    let mut buf = [0u8; MAX_PACKET_LEN];
    let packets: Vec<(Vec<u8>, bool)> = (0..3000)
        .map(|i| {
            let (payload, is_ping) = match i % 3 {
                0 => (Payload::Input(pad_state), false),
                1 => (Payload::InputMotion(pad_state, motion), false),
                _ => (
                    Payload::Ping(Ping {
                        t_client_us: i64::from(i),
                    }),
                    true,
                ),
            };
            let n = client.send(&payload, &mut buf).unwrap();
            (buf[..n].to_vec(), is_ping)
        })
        .collect();

    let src: SocketAddr = "127.0.0.1:50000".parse().unwrap();
    let mut out = [0u8; MAX_PACKET_LEN];
    // O primeiro pacote publica o endereço de origem (aloca uma vez); aquece.
    assert_eq!(
        engine.handle(&packets[0].0, src, Instant::now(), &mut out),
        Handled::Input
    );

    ALLOCS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    let mut pongs = 0;
    for (packet, is_ping) in &packets[1..] {
        let handled = engine.handle(packet, src, Instant::now(), &mut out);
        match (handled, is_ping) {
            (Handled::Pong { .. }, true) => pongs += 1,
            (Handled::Input, false) => {}
            other => panic!("resultado inesperado: {other:?}"),
        }
    }
    COUNTING.with(|c| c.set(false));
    let allocs = ALLOCS.with(Cell::get);

    assert_eq!(
        allocs,
        0,
        "o loop alocou {allocs} vezes em {} pacotes",
        packets.len() - 1
    );
    assert_eq!(pongs, 1000);
    let snap = stats.snapshot();
    assert_eq!(snap.dropped(), 0);
    assert_eq!(snap.inputs, 2000);
    assert!(stats.processing.count() >= 2000);
}
