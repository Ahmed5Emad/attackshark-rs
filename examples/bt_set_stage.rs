//! Single-session BT stage-set test: handshake + set + verify dump,
//! all on one BlueZ connection. Earlier attempts used one ble_write
//! process per packet (fresh session each); if the mouse needs the a1
//! handshake armed in-session, those never had a chance.
//! Usage: bt_set_stage <stage 1-6>
use std::collections::HashMap;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedFd, Value};

const DEV: &str = "/org/bluez/hci0/dev_78_87_11_27_D6_6C";
const CTL: &str = "service0012/char0018"; // fee3 write
const REP: &str = "service0012/char001a"; // fee4 notify

fn w(conn: &Connection, bytes: &[u8]) {
    let proxy = Proxy::new(
        &conn,
        "org.bluez",
        format!("{DEV}/{CTL}"),
        "org.bluez.GattCharacteristic1",
    )
    .expect("ctl proxy");
    let opts: HashMap<String, Value> = HashMap::new();
    let r: Result<(), _> = proxy.call("WriteValue", &(bytes.to_vec(), opts));
    println!("WRITE {} -> {:?}", hex(bytes), r.is_ok());
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}

fn main() {
    let stage: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .filter(|s| (1..=6).contains(s))
        .expect("usage: bt_set_stage <1-6>");
    let conn = Connection::system().expect("system bus");
    // subscribe notify fd
    let proxy = Proxy::new(
        &conn,
        "org.bluez",
        format!("{DEV}/{REP}"),
        "org.bluez.GattCharacteristic1",
    )
    .expect("rep proxy");
    let opts: HashMap<String, Value> = HashMap::new();
    let (zfd, _mtu): (OwnedFd, u16) = proxy
        .call("AcquireNotify", &(opts,))
        .expect("acquire");
    // Subscribe companions exactly like ble_watch (ffc1, ffc2): fee4 alone
    // yields zero inbound on this mouse — companions seem to arm the stream.
    // Held open for the whole run (dropping unsubscribes).
    let mut _held: Vec<std::os::fd::OwnedFd> = Vec::new();
    for extra in ["service0059/char005a", "service0059/char005e"] {
        let p = Proxy::new(
            &conn,
            "org.bluez",
            format!("{DEV}/{extra}"),
            "org.bluez.GattCharacteristic1",
        )
        .expect("extra proxy");
        let o: HashMap<String, Value> = HashMap::new();
        let r: Result<(OwnedFd, u16), _> = p.call("AcquireNotify", &(o,));
        eprintln!("EXTRA {extra} -> {}", r.is_ok());
        if let Ok((z, _)) = r {
            _held.push(z.into());
        }
    }
    let fd: std::os::fd::OwnedFd = zfd.into();
    let raw = fd.as_raw_fd();
    std::mem::forget(fd);
    let mut f = unsafe { std::fs::File::from_raw_fd(raw) };
    // Timed writes from a helper thread; READS stay on the main thread
    // (proven ble_watch pattern — a reader thread got zero inbound here).
    let conn2 = Connection::system().expect("system bus");
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        w(&conn2, &[0xa1, 0x08, 0, 0, 0, 0, 0, 0]);
        std::thread::sleep(std::time::Duration::from_secs(3));
        w(&conn2, &[0x24, 0x10, stage, 0x00]);
        std::thread::sleep(std::time::Duration::from_secs(4));
        w(&conn2, &[0xa0, 0x08, 0, 0, 0, 0, 0, 0]);
    });
    let t0 = std::time::Instant::now();
    let mut buf = [0u8; 512];
    loop {
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => println!(
                "{:>5.1}s {}",
                t0.elapsed().as_secs_f32(),
                hex(&buf[..n])
            ),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                println!("read err: {e}");
                break;
            }
        }
        if t0.elapsed().as_secs() > 18 {
            break;
        }
    }
    println!("--- done ---");
}
