//! Scanner de broadcast contra um "server" em loopback.

use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use vynpad_client_core::{BroadcastScanner, DiscoveredServer, ScanListener};

#[derive(Default)]
struct Collect(Mutex<Vec<DiscoveredServer>>);

impl ScanListener for Collect {
    fn on_server(&self, server: DiscoveredServer) {
        self.0.lock().unwrap().push(server);
    }
}

const REPLY: &[u8] = br#"{"id":"0b8e6a52-7d1c-4c0e-9f3e-0a1b2c3d4e5f","name":"PC-Sala","ver":1,"tcp":47800,"udp":47801,"os":"windows","free":3,"fp":"af8beea5aef0bef6"}"#;

/// O server responde ao `VPDISC1` (e também manda lixo); o scanner entrega uma vez só, com o
/// IP de origem, mesmo repetindo o pedido a cada segundo.
#[test]
fn finds_a_server_once_even_when_the_request_is_repeated() {
    let server = UdpSocket::bind("127.0.0.1:0").unwrap();
    server
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let addr = server.local_addr().unwrap();
    let replier = thread::spawn(move || {
        let mut buf = [0u8; 64];
        let stop = Instant::now() + Duration::from_millis(2500);
        while Instant::now() < stop {
            if let Ok((n, from)) = server.recv_from(&mut buf)
                && &buf[..n] == b"VPDISC1"
            {
                server.send_to(b"isto nao e json", from).unwrap();
                server.send_to(REPLY, from).unwrap();
            }
        }
    });

    let listener = Arc::new(Collect::default());
    let scanner = BroadcastScanner::with_targets(vec![addr]);
    scanner.scan(2200, listener.clone()).unwrap();
    replier.join().unwrap();

    let found = listener.0.lock().unwrap();
    assert_eq!(found.len(), 1, "o mesmo server não pode repetir: {found:?}");
    assert_eq!(found[0].name, "PC-Sala");
    assert_eq!(found[0].hosts, vec!["127.0.0.1".to_owned()]);
}

#[test]
fn cancel_ends_the_scan_early() {
    let scanner = BroadcastScanner::with_targets(vec![]);
    let canceller = scanner.clone();
    let started = Instant::now();
    let t = thread::spawn(move || {
        thread::sleep(Duration::from_millis(250));
        canceller.cancel();
    });
    scanner.scan(10_000, Arc::new(Collect::default())).unwrap();
    t.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn invalid_extra_targets_are_ignored() {
    let _ = BroadcastScanner::new(&["abc".to_owned(), "192.168.0.255".to_owned()]);
}
