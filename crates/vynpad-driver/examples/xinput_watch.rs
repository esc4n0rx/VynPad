//! Mostra o estado dos 4 controles `XInput` a cada 200 ms, para conferir um controle virtual
//! sem abrir o `joy.cpl`.
//!
//! ```text
//! cargo run -p vynpad-driver --example xinput_watch            # até Ctrl+C
//! cargo run -p vynpad-driver --example xinput_watch -- 10      # 10 segundos
//! ```

#[cfg(windows)]
fn main() {
    use std::time::{Duration, Instant};

    let seconds: Option<u64> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    let Ok(xinput) = rusty_xinput::XInputHandle::load_default() else {
        eprintln!("XInput indisponível");
        return;
    };
    let start = Instant::now();
    while seconds.is_none_or(|s| start.elapsed() < Duration::from_secs(s)) {
        let cells: Vec<String> = (0..4)
            .map(|i| match xinput.get_state(i) {
                Ok(state) => {
                    let g = state.raw.Gamepad;
                    format!(
                        "[{i}] btn {:04x} lt {:3} rt {:3} L({:6},{:6}) R({:6},{:6})",
                        g.wButtons,
                        g.bLeftTrigger,
                        g.bRightTrigger,
                        g.sThumbLX,
                        g.sThumbLY,
                        g.sThumbRX,
                        g.sThumbRY
                    )
                }
                Err(_) => format!("[{i}] --"),
            })
            .collect();
        println!("{}", cells.join("  "));
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("O XInput só existe no Windows.");
}
