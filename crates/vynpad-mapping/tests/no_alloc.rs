//! `apply` roda no hot path e não pode alocar (T-206).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use vynpad_driver::xbuttons;
use vynpad_mapping::{
    Button, ButtonTarget, CompiledProfile, Curve, Gyro, MapperState, Profile, RawInput, Turbo,
    apply,
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
fn apply_does_not_allocate_with_every_stage_enabled() {
    let mut p = Profile::standard();
    p.sticks.left.curve = Curve::Exponential { exponent: 1.8 };
    p.sticks.right.curve = Curve::Custom {
        points: [(0.2, 0.1), (0.4, 0.3), (0.6, 0.6), (0.8, 0.85)],
    };
    p.gyro.enabled = true;
    p.buttons
        .remap
        .insert(Button::A, ButtonTarget::RightTrigger);
    p.buttons.turbo.push(Turbo {
        button: Button::X,
        hz: 12.0,
    });
    let compiled = CompiledProfile::compile(&p).unwrap();
    let mut state = MapperState::new();
    let raw = RawInput {
        buttons: xbuttons::A | xbuttons::X,
        lx: 9000,
        ly: -9000,
        rx: 4000,
        ry: 2000,
        gyro: Some(Gyro {
            x: 0.2,
            y: -0.4,
            z: 0.1,
        }),
        ..RawInput::default()
    };
    apply(&compiled, &raw, &mut state, 0.004); // aquece

    ALLOCS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    for _ in 0..5000 {
        std::hint::black_box(apply(&compiled, &raw, &mut state, 0.004));
    }
    COUNTING.with(|c| c.set(false));
    let allocs = ALLOCS.with(Cell::get);
    assert_eq!(allocs, 0, "apply() alocou {allocs} vezes");
}
