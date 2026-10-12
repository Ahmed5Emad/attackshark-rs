//! Bluetooth read side: live stage + battery follow over BLE GATT.
//!
//! Proven against ATTACK SHARK V6 (78:87:11:27:D6:6C) 2026-10-12:
//! - HID service (0x1812): vendor control char (fee3, write) takes the
//!   8B queries `a1..` (handshake -> `a1 01 06`) and `a0..` (config dump);
//!   report char (fee4, notify) emits heartbeats `24 41 01 <batt>`
//!   and live stage announcements `24 10 <1-6> 00` on every DPI press.
//! - BlueZ Battery1 mirrors the charge without any subscription.
//! - WRITES of config over BT are uncharted: the vendor channel (0xffc0)
//!   takes <=20B but ignores setter guesses, the HID channel answers
//!   queries only. Edits stay USB-receiver-only.

use std::collections::HashMap;
use std::os::unix::io::{AsRawFd, FromRawFd};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedFd as ZFd, OwnedObjectPath, OwnedValue};

pub const MOUSE_PATH: &str = "/org/bluez/hci0/dev_78_87_11_27_D6_6C";
// NOTE: vendor UUIDs, not the standard HID 2a4c/2a4d — fee3 takes the
// 8B queries, fee4 emits heartbeats/announcements/dumps.
const HID_CTL_UUID: &str = "0000fee3-0000-1000-8000-00805f9b34fb";
const HID_REP_UUID: &str = "0000fee4-0000-1000-8000-00805f9b34fb";

#[derive(Debug, Clone)]
pub enum BtEvent {
    Battery(u8),
    Stage(u8),
}

type Managed = HashMap<
    OwnedObjectPath,
    HashMap<String, HashMap<String, OwnedValue>>,
>;

fn managed(conn: &Connection) -> Option<Managed> {
    let proxy =
        Proxy::new(conn, "org.bluez", "/", "org.freedesktop.DBus.ObjectManager")
            .ok()?;
    let objs: Managed = proxy.call("GetManagedObjects", &()).ok()?;
    Some(objs)
}

/// Find (control_point_path, report_notify_path) for the mouse by UUID
/// (path numbers renumber across reconnects, UUIDs don't).
pub fn find_hid_chars(conn: &Connection) -> Option<(String, String)> {
    let objs = managed(conn)?;
    let (mut ctl, mut rep) = (None, None);
    for (path, ifaces) in &objs {
        if !path.as_str().starts_with(MOUSE_PATH) {
            continue;
        }
        if let Some(ch) = ifaces.get("org.bluez.GattCharacteristic1") {
            let uuid: String =
                ch.get("UUID")?.clone().try_into().unwrap_or_default();
            if uuid.eq_ignore_ascii_case(HID_CTL_UUID) {
                ctl = Some(path.as_str().to_owned());
            } else if uuid.eq_ignore_ascii_case(HID_REP_UUID) {
                rep = Some(path.as_str().to_owned());
            }
        }
    }
    Some((ctl?, rep?))
}

fn write_value(conn: &Connection, path: &str, bytes: &[u8]) -> bool {
    let proxy = match Proxy::new(
        conn,
        "org.bluez",
        path,
        "org.bluez.GattCharacteristic1",
    ) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let opts: HashMap<String, OwnedValue> = HashMap::new();
    let body = (bytes.to_vec(), opts);
    let res: Result<(), _> = proxy.call("WriteValue", &body);
    res.is_ok()
}

/// Blocking monitor: handshake, then heartbeat/announcement stream.
/// Ships events over `tx`; returns on disconnect/error (caller respawns).
pub fn monitor(tx: std::sync::mpsc::Sender<BtEvent>) {
    let conn = match Connection::system() {
        Ok(c) => c,
        Err(_) => return,
    };
    let (ctl, rep) = match find_hid_chars(&conn) {
        Some(v) => v,
        None => return,
    };
    let fd = match Proxy::new(
        &conn,
        "org.bluez",
        rep.as_str(),
        "org.bluez.GattCharacteristic1",
    )
    .ok()
    .and_then(|p: Proxy| {
        let opts: HashMap<String, OwnedValue> = HashMap::new();
        let res: Result<(ZFd, u16), _> = p.call("AcquireNotify", &opts);
        res.ok()
    }) {
        Some((zfd, _)) => {
            let z: ZFd = zfd;
            let o: std::os::fd::OwnedFd = z.into();
            o
        }
        None => return,
    };
    let raw = fd.as_raw_fd();
    std::mem::forget(fd);
    // SAFETY: we own this fd (forgot the OwnedFd); File closes it on drop.
    let mut f = unsafe { std::fs::File::from_raw_fd(raw) };
    use std::io::Read;
    write_value(&conn, &ctl, &[0xa1, 0x08, 0, 0, 0, 0, 0, 0]);
    // BlueZ hands out the notify fd nonblocking: WouldBlock just means
    // quiet air — sleep and retry (this exact bug deafened an early probe).
    // Runs until the receiver hangs up; the UI owns the lifecycle.
    let mut buf = [0u8; 512];
    loop {
        let n = match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(50));
                continue;
            }
            Err(_) => break,
        };
        let d = &buf[..n];
        if d.len() == 4 && d[0] == 0x24 && d[1] == 0x41 && d[3] <= 100 {
            if tx.send(BtEvent::Battery(d[3])).is_err() {
                break;
            }
        } else if d.len() == 4
            && d[0] == 0x24
            && d[1] == 0x10
            && (1..=6).contains(&d[2])
        {
            if tx.send(BtEvent::Stage(d[2])).is_err() {
                break;
            }
        } else if d.len() == 118 && d[0] == 0xa0 && d[1] == 0x01 {
            // Full config dump echo: active stage lives at byte 24
            // (02 on stage 2, 03 on stage 3, cross-checked 2026-10-12;
            // byte 50 mirrors it as 0x67 + stage).
            if let Some(&s) = d.get(24) {
                if (1..=6).contains(&s) && tx.send(BtEvent::Stage(s)).is_err()
                {
                    break;
                }
            }
        }
    }
}
