//! Hold BlueZ notification FDs (AcquireNotify) on the vendor chars and dump
//! every ATT notification for 150s. Passive listener — reveals the BLE
//! config/heartbeat format without sending anything.
use std::io::Read;
use std::os::fd::{FromRawFd, OwnedFd};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::Value;

const DEV: &str = "/org/bluez/hci0/dev_78_87_11_27_D6_6C";
const CHARS: &[&str] = &[
    "service0059/char005a", // ffc1
    "service0059/char005e", // ffc2
    "service0012/char001a", // fee4
];

fn watch(conn: Connection, char: String) {
    let path = format!("{DEV}/{char}");
    let proxy = match Proxy::new(
        &conn,
        "org.bluez",
        path.clone(),
        "org.bluez.GattCharacteristic1",
    ) {
        Ok(p) => p,
        Err(e) => {
            println!("proxy {char} FAILED: {e}");
            return;
        }
    };
    let opts: std::collections::HashMap<String, Value> = Default::default();
    let (zfd, mtu): (zbus::zvariant::OwnedFd, u16) = match proxy.call("AcquireNotify", &(opts,)) {
        Ok(v) => v,
        Err(e) => {
            println!("acquire {char} FAILED: {e}");
            return;
        }
    };
    let fd: std::os::fd::OwnedFd = zfd.into();
    println!("acquired {char} mtu={mtu}");
    let raw = std::os::fd::AsRawFd::as_raw_fd(&fd);
    let mut f = unsafe { std::fs::File::from_raw_fd(raw) };
    std::mem::forget(fd); // f now owns it
    let mut buf = [0u8; 512];
    let end = std::time::Instant::now() + std::time::Duration::from_secs(150);
    while std::time::Instant::now() < end {
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => println!(
                "NOTIFY {char} len={n}: {}",
                buf[..n]
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                println!("read {char} err: {e}");
                break;
            }
        }
    }
}

fn main() {
    let mut handles = Vec::new();
    for c in CHARS {
        let conn = Connection::system().unwrap();
        let name = c.to_string();
        handles.push(std::thread::spawn(move || watch(conn, name)));
    }
    std::thread::sleep(std::time::Duration::from_secs(150));
    println!("--- done ---");
}
