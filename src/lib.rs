//! attackshark-rs library: native Linux driver for ATTACK SHARK V6-family mice.
//!
//! egui frontends (and anything else) should use [`Mouse`]. The `protocol`
//! module holds the byte-level report builders and decoders; every layout
//! cites its provenance and live-test status.
//!
//! ```no_run
//! use attackshark_rs::Mouse;
//! let mouse = Mouse::open()?;
//! mouse.set_polling(1000)?;
//! # Ok::<(), String>(())
//! ```

use hidapi::{HidApi, HidDevice, HidError};

pub mod protocol;
pub mod bt;

pub const VID: u16 = 0x1d57;
pub const PIDS: [u16; 3] = [0xfa60, 0xfa65, 0x2224];

/// Battery % over Bluetooth, via BlueZ `org.bluez.Battery1` (the same
/// Battery Service the desktop reads — NOT the HID channel, which carries
/// no battery data on this mouse). Prefers a device whose name mentions the
/// mouse, otherwise takes the first BT battery found. `None` = unavailable.
pub fn bt_battery_pct() -> Option<u8> {
    use std::collections::HashMap;
    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::OwnedObjectPath;
    let conn = Connection::system().ok()?;
    let root = Proxy::new(&conn, "org.bluez", "/", "org.freedesktop.DBus.ObjectManager").ok()?;
    let objs: HashMap<
        OwnedObjectPath,
        HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>,
    > = root.call("GetManagedObjects", &()).ok()?;
    let mut fallback: Option<u8> = None;
    for (path, _ifaces) in &objs {
        let batt = match Proxy::new(&conn, "org.bluez", path.clone(), "org.bluez.Battery1") {
            Ok(p) => p,
            Err(_) => continue,
        };
        let pct: u8 = match batt.get_property("Percentage") {
            Ok(p) => p,
            Err(_) => continue,
        };
        let name: String = Proxy::new(&conn, "org.bluez", path.clone(), "org.bluez.Device1")
            .ok()
            .and_then(|d| d.get_property("Name").ok())
            .unwrap_or_default();
        let n = name.to_uppercase();
        if n.contains("SHARK") || n.contains("ATTACK") || n.contains("V6") {
            return Some(pct);
        }
        fallback = fallback.or(Some(pct));
    }
    fallback
}

/// Anything the dongle pushes on the interrupt pipe, decoded as far as known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// V6 heartbeat, e.g. 03 24 41 01 23 (meaning of bytes 1..4 open).
    Heartbeat(Vec<u8>),
    /// Config-write ACK: 03 (10|24) 50 00 <report-id>.
    Ack(u8),
    /// X6-form battery: 03 10 40 .. pct10.
    Battery(u8),
    /// X6-form DPI-button stage event: 03 10 10 <stage> _.
    StageEvent(u8),
    /// Anything else, raw.
    Other(Vec<u8>),
}

pub fn decode_event(report: &[u8]) -> Event {
    if protocol::is_ack(report) {
        return Event::Ack(report[4]);
    }
    if let Some(pct) = protocol::decode_battery_x6(report) {
        return Event::Battery(pct);
    }
    if let Some(pct) = protocol::decode_battery_x11(report) {
        return Event::Battery(pct);
    }
    if let Some(pct) = protocol::decode_battery_v6(report) {
        return Event::Battery(pct);
    }
    if let Some(pct) = protocol::decode_battery_v6_hb(report) {
        return Event::Battery(pct);
    }
    if report.len() == 5 && report[0] == 0x03 && report[1] == 0x10 && report[2] == 0x10 {
        return Event::StageEvent(report[3]);
    }
    if !report.is_empty() && report[0] == 0x03 {
        return Event::Heartbeat(report.to_vec());
    }
    Event::Other(report.to_vec())
}

pub fn hex(buf: &[u8]) -> String {
    buf.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Open handle on the V6 config channel: USB 1d57:fa60/fa65 HID iface 2
/// preferred, Bluetooth "ATTACK SHARK V6" as fallback.
pub struct Mouse {
    dev: HidDevice,
    #[allow(dead_code)]
    api: HidApi,
    bt: bool,
}

impl Mouse {
    pub fn open() -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| format!("hid init failed: {e}"))?;
        let mut usb_path: Option<std::ffi::CString> = None;
        let mut bt_path: Option<std::ffi::CString> = None;
        for d in api.device_list() {
            if d.vendor_id() == VID && PIDS.contains(&d.product_id()) && d.interface_number() == 2 {
                usb_path = Some(d.path().to_owned());
            }
            if bt_path.is_none()
                && d.product_string()
                    .map(|s| s.contains("ATTACK SHARK"))
                    .unwrap_or(false)
            {
                bt_path = Some(d.path().to_owned());
            }
        }
        if let Some(p) = bt_path {
            let dev = api.open_path(&p).map_err(|e| format!("open failed: {e}"))?;
            return Ok(Mouse { dev, api, bt: true });
        }
        if let Some(p) = usb_path {
            let dev = api.open_path(&p).map_err(|e| format!("open failed: {e}"))?;
            return Ok(Mouse { dev, api, bt: false });
        }
        Err("no V6 found: plug the receiver (USB) or connect Bluetooth".into())
    }

    /// Open handle on the USB V6 only (receiver fa60/fa65 or wired 2224,
    /// iface 2). Unlike [`Mouse::open`] this never takes the Bluetooth
    /// node — needed when BT is paired but the mouse sits on cable/2.4G.
    pub fn open_usb() -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| format!("hid init failed: {e}"))?;
        let mut usb_path: Option<std::ffi::CString> = None;
        for d in api.device_list() {
            if d.vendor_id() == VID && PIDS.contains(&d.product_id()) && d.interface_number() == 2 {
                usb_path = Some(d.path().to_owned());
            }
        }
        if let Some(p) = usb_path {
            let dev = api.open_path(&p).map_err(|e| format!("open failed: {e}"))?;
            return Ok(Mouse { dev, api, bt: false });
        }
        Err("no USB V6 found: plug the cable/receiver (wired=2224, 2.4G=fa60/fa65)".into())
    }

    /// Transport label for the UI: "2.4G wireless" or "Bluetooth".
    pub fn transport(&self) -> &'static str {
        if self.bt {
            "Bluetooth"
        } else {
            "2.4G wireless"
        }
    }

    /// Channel label for the UI.
    pub fn channel(&self) -> &'static str {
        if self.bt {
            "bluetooth hid · 045e:0224"
        } else {
            "iface 2 · VID 1d57"
        }
    }

    pub fn manufacturer(&self) -> Option<String> {
        self.dev.get_manufacturer_string().unwrap_or(None)
    }

    pub fn product(&self) -> Option<String> {
        self.dev.get_product_string().unwrap_or(None)
    }

    /// Send one SET_REPORT feature buffer (first byte = report ID),
    /// then wait up to `ack_timeout_ms` for its ACK. Returns the ACK bytes.
    pub fn send_report(&self, report: &[u8], ack_timeout_ms: u64) -> Result<Vec<u8>, String> {
        // The Bluetooth hidraw node is read-only for vendor SET_REPORT
        // (kernel: IOCTL SFEATURE I/O error). Refuse before touching HW —
        // proven 2026-10-12; BT writes go over GATT fee3, not hidraw.
        if self.bt {
            return Err("needs the 2.4G receiver — Bluetooth is follow-only".into());
        }
        self.dev
            .send_feature_report(report)
            .map_err(|e| format!("send failed: {e}"))?;
        let mut buf = [0u8; 64];
        let end = std::time::Instant::now() + std::time::Duration::from_millis(ack_timeout_ms);
        while std::time::Instant::now() < end {
            match self.dev.read_timeout(&mut buf, 200) {
                Ok(0) => {}
                Ok(len) => {
                    let r = &buf[..len];
                    if protocol::is_ack(r) && r[4] == report[0] {
                        return Ok(r.to_vec());
                    }
                }
                Err(e) => return Err(format!("read error: {e}")),
            }
        }
        Err("no ACK within timeout".into())
    }

    /// Blocking read of one interrupt report, decoded. Timeout in ms.
    pub fn read_event(&self, timeout_ms: i32) -> Result<Event, String> {        let mut buf = [0u8; 64];
        match self.dev.read_timeout(&mut buf, timeout_ms) {
            Ok(0) => Err("timeout".into()),
            Ok(len) => Ok(decode_event(&buf[..len])),
            Err(e) => Err(format!("read error: {e}")),
        }
    }

    /// GET_REPORT Feature read by report ID (read-only probe).
    pub fn get_feature(&self, rid: u8, len: usize) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; len.max(2)];
        buf[0] = rid;
        match self.dev.get_feature_report(&mut buf) {
            Ok(n) => Ok(buf[..n].to_vec()),
            Err(e) => Err(format!("get feature {rid:#04x} failed: {e}")),
        }
    }

    /// GET_REPORT Feature 0xA0 (the Windows sync-loop query; all zeros).
    pub fn feature_a0(&self) -> Result<Vec<u8>, String> {
        let mut buf = [0u8; 8];
        buf[0] = 0xa0;
        match self.dev.get_feature_report(&mut buf) {
            Ok(n) => Ok(buf[..n].to_vec()),
            Err(HidError::HidApiErrorEmpty) => Err("stall".into()),
            Err(e) => Err(format!("{e}")),
        }
    }

    /// V6 factory-visible DPI state (matches the Windows app screenshot).
    pub fn set_dpi_v6state(&self) -> Result<Vec<u8>, String> {
        let stages = [800u16, 1600, 2400, 3200, 8000, 25000, 50, 50];
        let colors = [
            [0xff, 0, 0],
            [0, 0xff, 0],
            [0, 0, 0xff],
            [0xff, 0xff, 0],
            [0, 0xff, 0xff],
            [0xff, 0, 0xff],
            [0xff, 0x40, 0],
            [0xff, 0xff, 0xff],
        ];
        let r = protocol::build_dpi_linear(stages, 2, true, false, 0x20, colors)
            .ok_or("bad dpi params")?;
        self.send_report(&r, 3000)
    }

    pub fn set_dpi(
        &self,
        stages6: [u16; 6],
        active: u8,
        angle: bool,
        ripple: bool,
    ) -> Result<Vec<u8>, String> {
        self.set_dpi_full(
            [stages6[0], stages6[1], stages6[2], stages6[3], stages6[4], stages6[5], 50, 50],
            active,
            angle,
            ripple,
            0x20,
            [
                [0xff, 0, 0],
                [0, 0xff, 0],
                [0, 0, 0xff],
                [0xff, 0xff, 0],
                [0, 0xff, 0xff],
                [0xff, 0, 0xff],
                [0xff, 0x40, 0],
                [0xff, 0xff, 0xff],
            ],
        )
    }

    /// Full DPI report with caller-supplied 8 stages + 8 per-stage RGB colors
    /// (UI sends 6 real stages + 2 hidden 50-DPI fillers, like the X6 driver).
    /// b7 is the reserved header byte — the vendor app always sends 0x20.
    pub fn set_dpi_full(
        &self,
        stages: [u16; 8],
        active: u8,
        angle: bool,
        ripple: bool,
        b7: u8,
        colors: [[u8; 3]; 8],
    ) -> Result<Vec<u8>, String> {
        let r = protocol::build_dpi_linear(stages, active, angle, ripple, b7, colors)
            .ok_or("bad dpi params (multiples of 50, >= 50; active 1-8)")?;
        self.send_report(&r, 3000)
    }

    pub fn set_polling(&self, hz: u16) -> Result<Vec<u8>, String> {
        let r = protocol::build_polling(hz).ok_or("rate must be 125, 250, 500 or 1000")?;
        self.send_report(&r, 3000)
    }

    pub fn set_prefs(&self, prefs: protocol::Prefs) -> Result<Vec<u8>, String> {
        let r = protocol::build_prefs(prefs).ok_or("bad prefs params")?;
        self.send_report(&r, 3000)
    }

    /// Static per-Apply commit header (report 0x0c). The vendor app sends it
    /// once per Apply before the 04/05/06/08 pages.
    pub fn send_commit(&self) -> Result<Vec<u8>, String> {
        self.send_report(&protocol::COMMIT_0C, 3000)
    }

    /// Factory reset: replays the vendor app's own default Apply — commit +
    /// DPI + prefs + polling + button map — with the exact default bytes
    /// from `tools/hiddriver_capture_2026-10-12.log`: DPI
    /// 800/1600/2400/3200/8000/25000 (active S2, angle-snap on), prefs
    /// Off / green / sleep 0.5 min / deep-sleep 10 min / debounce 8 ms,
    /// polling 1000 Hz, and the factory button map
    /// (`protocol::buttons::FACTORY_MAP`).
    pub fn reset_factory(&self) -> Result<(), String> {
        self.send_commit().map_err(|e| format!("reset commit: {e}"))?;
        self.set_dpi_v6state()
            .map_err(|e| format!("reset dpi: {e}"))?;
        self.set_prefs(protocol::Prefs {
            light: protocol::LightMode::Off,
            led_speed: 3,
            rgb: [0, 0xff, 0],
            sleep_min: 0.5,
            deep_sleep_min: 10,
            debounce_ms: 8,
        })
        .map_err(|e| format!("reset prefs: {e}"))?;
        self.set_polling(1000)
            .map_err(|e| format!("reset polling: {e}"))?;
        self.set_buttons_raw(&protocol::buttons::FACTORY_MAP)
            .map_err(|e| format!("reset buttons: {e}"))?;
        Ok(())
    }

    /// Raw 59-byte button-map report (0x08). No defaults are applied:
    /// callers must build the full map (see protocol::buttons) because the
    /// mouse cannot be read back.
    pub fn set_buttons_raw(&self, report59: &[u8; 59]) -> Result<Vec<u8>, String> {
        self.send_report(report59, 3000)
    }

    /// Raw macro packet sequence (0x08 map + three 0x09 pages).
    pub fn send_macro_packets(&self, packets: &[[u8; 64]; 3]) -> Result<(), String> {
        if self.bt {
            return Err("needs the 2.4G receiver — Bluetooth is follow-only".into());
        }
        for p in packets {
            self.dev
                .send_feature_report(p)
                .map_err(|e| format!("macro send failed: {e}"))?;
        }
        Ok(())
    }
}
