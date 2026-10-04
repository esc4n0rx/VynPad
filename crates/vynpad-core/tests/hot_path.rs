//! O hot path do input (`SlotManager::submit`) não aloca nem trava, mesmo com a UI mexendo
//! nos slots ao mesmo tempo.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use vynpad_core::SlotManager;
use vynpad_driver::{MockDriver, SlotId, XReport, xbuttons};
use vynpad_protocol::control::Uuid;

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

fn device(n: u8) -> Uuid {
    Uuid::parse(&format!("123e4567-e89b-42d3-a456-4266141740{n:02}")).unwrap()
}

const REPORT: XReport = XReport {
    buttons: xbuttons::A,
    lt: 1,
    rt: 2,
    lx: 3,
    ly: 4,
    rx: 5,
    ry: 6,
};

#[test]
fn submit_does_not_allocate_after_the_pad_is_plugged() {
    let driver = Arc::new(MockDriver::with_capacity(1024));
    let manager = SlotManager::new(driver);
    manager.assign(&device(1), 7, None).unwrap();
    manager.submit(7, &REPORT).unwrap(); // pluga

    ALLOCS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    for _ in 0..2000 {
        manager.submit(7, &REPORT).unwrap();
    }
    COUNTING.with(|c| c.set(false));
    let allocs = ALLOCS.with(Cell::get);
    assert_eq!(allocs, 0, "submit() alocou {allocs} vezes");
}

/// Uma thread de input martela `submit` enquanto outra faz assign/suspend/swap/pause.
/// O resultado tem de ser consistente e sem pânico.
#[test]
fn input_thread_survives_concurrent_slot_operations() {
    let driver = Arc::new(MockDriver::with_capacity(16));
    let manager = Arc::new(SlotManager::new(driver.clone()));
    manager.assign(&device(1), 1, None).unwrap();
    manager.assign(&device(2), 2, None).unwrap();
    manager.submit(1, &REPORT).unwrap();
    manager.submit(2, &REPORT).unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let input = {
        let (manager, stop) = (manager.clone(), stop.clone());
        thread::spawn(move || {
            let mut ok = 0u64;
            while !stop.load(Ordering::Relaxed) {
                // Erros (slot em troca, pausado...) são esperados; só não pode haver pânico.
                ok += u64::from(manager.submit(1, &REPORT).is_ok());
                ok += u64::from(manager.submit(2, &REPORT).is_ok());
            }
            ok
        })
    };

    let slot = |i| SlotId::new(i).unwrap();
    for round in 0..200 {
        manager.swap(slot(0), slot(1)).unwrap();
        manager.set_paused(slot(0), round % 2 == 0).unwrap();
        manager.set_paused(slot(1), false).unwrap();
        let _ = manager.tick();
    }
    stop.store(true, Ordering::Relaxed);
    let ok = input.join().expect("a thread de input não pode dar pânico");
    assert!(ok > 0, "nenhum input passou");

    // Estado final coerente: os dois dispositivos ainda têm slot.
    let snap = manager.snapshot();
    assert_eq!(snap.free_count(), 2);
    assert_eq!(driver.plugged_slots().len(), 2);
}
