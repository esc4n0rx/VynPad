//! `update()` roda no hot path e não pode alocar (T-201).
#![cfg(feature = "mock")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use vynpad_driver::{MockDriver, SlotId, VirtualPadDriver, XReport, xbuttons};

struct CountingAlloc;

thread_local! {
    /// Alocações da thread atual enquanto `COUNTING` está ligado.
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

fn count_allocs(f: impl FnOnce()) -> u64 {
    ALLOCS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    f();
    COUNTING.with(|c| c.set(false));
    ALLOCS.with(Cell::get)
}

#[test]
fn update_does_not_allocate() {
    let driver = MockDriver::with_capacity(1024);
    let pad = driver.plug(SlotId::new(0).unwrap()).unwrap();
    let report = XReport {
        buttons: xbuttons::A | xbuttons::START,
        lt: 1,
        rt: 2,
        lx: 3,
        ly: 4,
        rx: 5,
        ry: 6,
    };

    // Aquece qualquer inicialização preguiçosa.
    driver.update(&pad, &report).unwrap();

    // 2000 chamadas com capacidade 1024: cobre também o descarte dos mais antigos.
    let allocs = count_allocs(|| {
        for _ in 0..2000 {
            driver.update(&pad, &report).unwrap();
        }
    });
    assert_eq!(allocs, 0, "update() alocou {allocs} vezes");
}

#[test]
fn the_counter_actually_counts() {
    let allocs = count_allocs(|| {
        let v = vec![1u8; 64];
        std::hint::black_box(v);
    });
    assert!(allocs > 0, "o contador de alocações não está funcionando");
}
