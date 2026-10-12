//! attackshark-rs: native Linux tool for ATTACK SHARK V6-family mice.
//! Reverse-engineered from ATTACK SHARK V6 SOFT (AttackShark.exe + hidapi +
//! hiddriver_*.dll, 17 of 18 DLLs byte-identical => one protocol).
//! Device: USB 1d57:fa60/fa65. Config channel: HID iface 2 (report desc 206B,
//! vendor feature reports 04-0C/10/A0/22). Transport: interrupt OUT/IN +
//! GET_REPORT Feature A0 on iface 0 (observed: returns zeros).
//! Write path (DPI/polling/lighting) needs captured SET bytes - see README.
//! This tool only READS unless you explicitly pass bytes to `set-feature`.

use hidapi::{HidApi, HidError};

use attackshark_rs::{protocol, Mouse};

const VID: u16 = 0x1d57;
const PIDS: [u16; 2] = [0xfa60, 0xfa65];

fn api() -> Result<HidApi, HidError> {
    HidApi::new()
}

fn cmd_list() -> i32 {
    let api = match api() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hid init failed: {e}");
            return 1;
        }
    };
    let mut found = 0;
    for d in api.device_list() {
        let usb = d.vendor_id() == VID && PIDS.contains(&d.product_id());
        let bt = d
            .product_string()
            .map(|s| s.contains("ATTACK SHARK"))
            .unwrap_or(false);
        if usb || bt {
            found += 1;
            println!(
                "{}  vid={:04x} pid={:04x} iface={} usage={:04x}/{:04x} {}",
                d.path().to_string_lossy(),
                d.vendor_id(),
                d.product_id(),
                d.interface_number(),
                d.usage_page(),
                d.usage(),
                d.product_string().unwrap_or(""),
            );
        }
    }
    if found == 0 {
        eprintln!("no 1d57:fa60/fa65 device found (need hidraw read access; check udev rule).");
        return 1;
    }
    0
}

/// Open the config channel: the iface-2 node (vendor usage page 0xff00 or
/// usage 0x00 with 206B descriptor). We pick by interface number 2.
fn open_config(api: &HidApi) -> Result<hidapi::HidDevice, String> {
    for d in api.device_list() {
        if d.vendor_id() == VID && PIDS.contains(&d.product_id()) && d.interface_number() == 2 {
            return api.open_path(d.path()).map_err(|e| format!("open failed: {e}"));
        }
    }
    Err("config channel (iface 2) not found. Run `list` first.".into())
}

fn hex(buf: &[u8]) -> String {
    buf.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn cmd_info() -> i32 {
    let api = match api() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hid init failed: {e}");
            return 1;
        }
    };
    let dev = match open_config(&api) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    println!(
        "opened: manufacturer={:?} product={:?}",
        dev.get_manufacturer_string().unwrap_or(None),
        dev.get_product_string().unwrap_or(None)
    );
    // Feature report 0xA0 (8B) - the sync-loop query. Observed via usbmon:
    // GET_REPORT Feature A0 on iface 0 -> a0 00 00 00 00 00 00 00.
    let mut buf = [0u8; 8];
    buf[0] = 0xa0;
    match dev.get_feature_report(&mut buf) {
        Ok(n) => println!("feature A0 ({n}B): {}", hex(&buf[..n])),
        Err(e) => println!("feature A0 read stalled: {e}"),
    }
    0
}

fn cmd_watch(secs: u64) -> i32 {
    let api = match api() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hid init failed: {e}");
            return 1;
        }
    };
    let dev = match open_config(&api) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    println!("streaming input reports {secs}s (move the mouse, watch ep traffic)...");
    let mut buf = [0u8; 64];
    let end = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    let mut n = 0;
    while std::time::Instant::now() < end {
        match dev.read_timeout(&mut buf, 500) {
            Ok(0) => {}
            Ok(len) => {
                n += 1;
                println!("in[{len}]: {}", hex(&buf[..len]));
                if n >= 30 {
                    break;
                }
            }
            Err(e) => {
                eprintln!("read error: {e}");
                break;
            }
        }
    }
    0
}

fn cmd_get_feature(rid: u8) -> i32 {
    let m = match Mouse::open() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    println!("opened via {}", m.transport());
    match m.get_feature(rid, 32) {
        Ok(b) => {
            println!("feature {rid:#04x} ({}B): {}", b.len(), attackshark_rs::hex(&b));
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn cmd_set_feature(report: u8, payload_hex: &str) -> i32 {    let payload: Vec<u8> = match payload_hex
        .split(|c| c == ' ' || c == ',' || c == ':')
        .filter(|s| !s.is_empty())
        .map(|s| u8::from_str_radix(s, 16))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(p) => p,
        Err(_) => {
            eprintln!("bad hex payload");
            return 1;
        }
    };
    let api = match api() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hid init failed: {e}");
            return 1;
        }
    };
    let dev = match open_config(&api) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let mut buf = vec![report];
    buf.extend_from_slice(&payload);
    match dev.send_feature_report(&buf) {
        Ok(_) => {
            println!("sent feature {:#04x} ({}B). Verify on the mouse.", report, buf.len());
            0
        }
        Err(e) => {
            eprintln!("send failed: {e}");
            1
        }
    }
}

fn cmd_battery(secs: u64) -> i32 {
    let api = match api() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hid init failed: {e}");
            return 1;
        }
    };
    let dev = match open_config(&api) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    println!("listening {secs}s for battery reports...");
    let mut buf = [0u8; 64];
    let end = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    while std::time::Instant::now() < end {
        match dev.read_timeout(&mut buf, 500) {
            Ok(0) => {}
            Ok(len) => {
                let r = &buf[..len];
                if let Some(pct) = protocol::decode_battery_x6(r) {
                    println!("battery: {pct}%  [{}]", hex(r));
                    return 0;
                }
                println!("other[{}]: {}", len, hex(r));
            }
            Err(e) => {
                eprintln!("read error: {e}");
                return 1;
            }
        }
    }
    println!("no battery report seen (V6 heartbeat may use a different form).");
    0
}

fn cmd_set_polling(hz: u16) -> i32 {
    let report = match protocol::build_polling(hz) {
        Some(r) => r,
        None => {
            eprintln!("rate must be 125, 250, 500 or 1000");
            return 2;
        }
    };
    let api = match api() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hid init failed: {e}");
            return 1;
        }
    };
    let dev = match open_config(&api) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    println!("sending polling {hz}Hz: [{}]", hex(&report));
    if let Err(e) = dev.send_feature_report(&report) {
        eprintln!("send failed: {e}");
        return 1;
    }
    let mut buf = [0u8; 64];
    let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < end {
        match dev.read_timeout(&mut buf, 500) {
            Ok(0) => {}
            Ok(len) => {
                let r = &buf[..len];
                println!("reply[{}]: {}", len, hex(r));
                if protocol::is_ack(r) {
                    println!("ACK: mouse accepted polling {hz}Hz.");
                    return 0;
                }
            }
            Err(e) => {
                eprintln!("read error: {e}");
                return 1;
            }
        }
    }
    println!("no ACK within 3s (V6 may ACK differently; check polling in games).");
    0
}

/// Factory reset, mirroring the original app's "restore defaults":
/// 0x0C internal-state reset, default DPI, default prefs, 1000Hz polling,
/// default 0x08 button map, empty BACKWARD macro (three 0x09 pages).
/// Macro page bytes are the driver's own resetCustomMacro output.
fn cmd_reset() -> i32 {
    // Factory defaults, mirroring the original app's "restore defaults".
    // Macro page bytes are the driver's own resetCustomMacro output.
    const PREFS: &str = "0f 01 50 03 a8 00 ff 00 0a 04 01 c9 00 00";
    const MAP08: &str = "3b 01 02 00 00 03 00 00 04 00 00 01 00 00 01 00 00 0d 00 00 06 00 00 12 00 08 01 00 00 01 00 00 01 00 00 01 00 00 01 00 00 01 00 00 01 00 00 01 00 00 09 00 00 0a 00 00 00 53";
    const MAC1: &str = "40 08 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00";
    const MAC2: &str = "40 08 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00";
    const MAC3: &str = "0c 08 02 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00";
    let mut fails = 0;
    let mut step = |name: &str, code: i32| {
        if code != 0 {
            eprintln!("reset step '{name}' FAILED");
            fails += 1;
        } else {
            println!("reset step '{name}' ok");
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    };
    step("state-reset", cmd_set_feature(0x0c, "0a 01 fe 01 fe 00 00 00 00"));
    step("dpi-defaults", cmd_set_dpi_v6state());
    step("prefs-defaults", cmd_set_feature(0x05, PREFS));
    step("polling-1000", cmd_set_polling(1000));
    step("button-map", cmd_set_feature(0x08, MAP08));
    step("macro-p1", cmd_set_feature(0x09, MAC1));
    step("macro-p2", cmd_set_feature(0x09, MAC2));
    step("macro-p3", cmd_set_feature(0x09, MAC3));
    if fails > 0 {
        eprintln!("reset finished with {fails} failed step(s)");
        return 1;
    }
    println!("reset finished OK");
    0
}
/// Re-send the state shown in the Windows app screenshot
/// (stages 800/1600/2400/3200/8000/25000, active=stage 2).
/// No-op if the X6-linear layout matches V6: watch for 03 24 50 00 04.
fn cmd_set_dpi_v6state() -> i32 {
    let stages: [u16; 8] = [800, 1600, 2400, 3200, 8000, 25000, 50, 50];
    let colors: [[u8; 3]; 8] = [
        [0xff, 0, 0],
        [0, 0xff, 0],
        [0, 0, 0xff],
        [0xff, 0xff, 0],
        [0, 0xff, 0xff],
        [0xff, 0, 0xff],
        [0xff, 0x40, 0],
        [0xff, 0xff, 0xff],
    ];
    let report = match protocol::build_dpi_linear(stages, 2, true, false, 0x20, colors) {
        Some(r) => r,
        None => {
            eprintln!("bad dpi params");
            return 2;
        }
    };
    let api = match api() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hid init failed: {e}");
            return 1;
        }
    };
    let dev = match open_config(&api) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    println!("sending dpi state: [{}]", hex(&report));
    if let Err(e) = dev.send_feature_report(&report) {
        eprintln!("send failed: {e}");
        return 1;
    }
    let mut buf = [0u8; 64];
    let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < end {
        match dev.read_timeout(&mut buf, 500) {
            Ok(0) => {}
            Ok(len) => {
                let r = &buf[..len];
                println!("reply[{}]: {}", len, hex(r));
                if protocol::is_ack(r) {
                    println!("ACK: dpi layout accepted.");
                    return 0;
                }
            }
            Err(e) => {
                eprintln!("read error: {e}");
                return 1;
            }
        }
    }
    println!("no ACK within 3s.");
    0
}

fn cmd_set_dpi_custom(stages6: [u16; 6], active: u8) -> i32 {
    let stages: [u16; 8] = [
        stages6[0], stages6[1], stages6[2], stages6[3], stages6[4], stages6[5], 50, 50,
    ];
    let colors: [[u8; 3]; 8] = [
        [0xff, 0, 0],
        [0, 0xff, 0],
        [0, 0, 0xff],
        [0xff, 0xff, 0],
        [0, 0xff, 0xff],
        [0xff, 0, 0xff],
        [0xff, 0x40, 0],
        [0xff, 0xff, 0xff],
    ];
    let report = match protocol::build_dpi_linear(stages, active, true, false, 0x20, colors) {
        Some(r) => r,
        None => {
            eprintln!("bad dpi params (values must be multiples of 50, >= 50)");
            return 2;
        }
    };
    let api = match api() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hid init failed: {e}");
            return 1;
        }
    };
    let dev = match open_config(&api) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    println!("sending dpi {stages6:?} active={active}: [{}]", hex(&report));
    if let Err(e) = dev.send_feature_report(&report) {
        eprintln!("send failed: {e}");
        return 1;
    }
    let mut buf = [0u8; 64];
    let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < end {
        match dev.read_timeout(&mut buf, 500) {
            Ok(0) => {}
            Ok(len) => {
                let r = &buf[..len];
                println!("reply[{}]: {}", len, hex(r));
                if protocol::is_ack(r) {
                    println!("ACK: dpi layout accepted.");
                    return 0;
                }
            }
            Err(e) => {
                eprintln!("read error: {e}");
                return 1;
            }
        }
    }
    println!("no ACK within 3s.");
    0
}

/// set-light <off|static|breathing|neon|color|staticdpi|breathingdpi> [rrggbb] [speed1-5]
fn cmd_set_light(mode: &str) -> i32 {
    let full: Vec<String> = std::env::args().collect();
    let rgb_hex = full.get(3).map(|s| s.as_str()).unwrap_or("00ff00");
    let speed: u8 = full.get(4).and_then(|s| s.parse().ok()).unwrap_or(3);
    let light = match mode {
        "off" => protocol::LightMode::Off,
        "static" => protocol::LightMode::Static,
        "breathing" => protocol::LightMode::Breathing,
        "neon" => protocol::LightMode::Neon,
        "color" => protocol::LightMode::ColorBreathing,
        "staticdpi" => protocol::LightMode::StaticDpi,
        "breathingdpi" => protocol::LightMode::BreathingDpi,
        _ => {
            eprintln!("mode: off static breathing neon color staticdpi breathingdpi");
            return 2;
        }
    };
    let rgb = match u32::from_str_radix(rgb_hex.trim_start_matches('#'), 16) {
        Ok(v) => [(v >> 16) as u8, (v >> 8) as u8, v as u8],
        Err(_) => {
            eprintln!("bad rgb hex (rrggbb)");
            return 2;
        }
    };
    let mouse = match Mouse::open() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let prefs = protocol::Prefs {
        light,
        led_speed: speed.clamp(1, 5),
        rgb,
        sleep_min: 5.0,
        deep_sleep_min: 10,
        debounce_ms: 8,
    };
    match mouse.set_prefs(prefs) {
        Ok(ack) => {
            println!("ACK: lighting accepted: [{}]", attackshark_rs::hex(&ack));
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// set-prefs <sleep_min> <deep_sleep_min> <debounce_ms> [mode] [rrggbb] [speed]
fn cmd_set_prefs_full(args: &[String]) -> i32 {
    if args.len() < 3 {
        eprintln!("usage: set-prefs <sleep_min 0.5-30> <deep_sleep_min 1-60> <debounce_ms 4-50-even> [mode] [rrggbb] [speed1-5]");
        return 2;
    }
    let sleep: f32 = args[0].parse().unwrap_or(-1.0);
    let deep: u8 = args[1].parse().unwrap_or(0);
    let deb: u8 = args[2].parse().unwrap_or(0);
    let light = match args.get(3).map(|s| s.as_str()).unwrap_or("staticdpi") {
        "off" => protocol::LightMode::Off,
        "static" => protocol::LightMode::Static,
        "breathing" => protocol::LightMode::Breathing,
        "neon" => protocol::LightMode::Neon,
        "color" => protocol::LightMode::ColorBreathing,
        "staticdpi" => protocol::LightMode::StaticDpi,
        "breathingdpi" => protocol::LightMode::BreathingDpi,
        _ => {
            eprintln!("bad mode");
            return 2;
        }
    };
    let rgb = args
        .get(4)
        .and_then(|s| u32::from_str_radix(s.trim_start_matches('#'), 16).ok())
        .map(|v| [(v >> 16) as u8, (v >> 8) as u8, v as u8])
        .unwrap_or([0, 255, 0]);
    let speed: u8 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(3);
    let mouse = match Mouse::open() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let prefs = protocol::Prefs {
        light,
        led_speed: speed.clamp(1, 5),
        rgb,
        sleep_min: sleep,
        deep_sleep_min: deep,
        debounce_ms: deb,
    };
    match mouse.set_prefs(prefs) {
        Ok(ack) => {
            println!("ACK: prefs accepted: [{}]", attackshark_rs::hex(&ack));
            0
        }
        Err(e) => {
            eprintln!("{e} (note: prefs layout is UNTESTED on V6)");
            1
        }
    }
}

/// monitor: timestamped log of every decoded interrupt event. Use it to
/// learn unknown reports (battery, DPI-button stage events): run it while
/// charging, pressing the DPI button, changing LED in the Windows app, etc.
fn cmd_monitor(secs: u64) -> i32 {
    let mouse = match Mouse::open() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    println!("monitoring {secs}s (press DPI button / plug charger to learn events)...");
    let start = std::time::Instant::now();
    let end = start + std::time::Duration::from_secs(secs);
    while std::time::Instant::now() < end {
        match mouse.read_event(500) {
            Ok(ev) => println!("[{:>6.1}s] {ev:?}", start.elapsed().as_secs_f32()),
            Err(_) => {}
        }
    }
    0
}

fn usage() {
    eprintln!("usage: attackshark-rs <list|info|watch [secs]|battery [secs]|set-polling <125|250|500|1000>|set-dpi|set-feature <report-id> <hex...>>");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
        std::process::exit(2);
    }
    let code = match args[1].as_str() {
        "list" => cmd_list(),
        "info" => cmd_info(),
        "watch" => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            cmd_watch(secs)
        }
        "get-feature" => {
            if args.len() < 3 {
                usage();
                std::process::exit(2);
            }
            let rid = u8::from_str_radix(args[2].trim_start_matches("0x"), 16).unwrap_or_else(|_| {
                eprintln!("bad report id");
                std::process::exit(2);
            });
            cmd_get_feature(rid)
        }
        "set-feature" => {
            if args.len() < 4 {
                usage();
                std::process::exit(2);
            }
            let rid = u8::from_str_radix(args[2].trim_start_matches("0x"), 16).unwrap_or_else(|_| {
                eprintln!("bad report id");
                std::process::exit(2);
            });
            cmd_set_feature(rid, &args[3..].join(" "))
        }
        "battery" => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            cmd_battery(secs)
        }
        "set-polling" => {
            if args.len() < 3 {
                usage();
                std::process::exit(2);
            }
            let hz: u16 = args[2].parse().unwrap_or_else(|_| {
                eprintln!("rate must be 125, 250, 500 or 1000");
                std::process::exit(2);
            });
            cmd_set_polling(hz)
        }
        "set-dpi" => cmd_set_dpi_v6state(),
        "set-dpi-raw" => {
            if args.len() != 9 {
                eprintln!("usage: set-dpi-raw <s1> <s2> <s3> <s4> <s5> <s6> <active 1-6>");
                std::process::exit(2);
            }
            let nums: Vec<u16> = args[2..8]
                .iter()
                .map(|s| {
                    s.parse().unwrap_or_else(|_| {
                        eprintln!("bad dpi value: {s}");
                        std::process::exit(2);
                    })
                })
                .collect();
            let active: u8 = args[8].parse().unwrap_or_else(|_| {
                eprintln!("bad active stage");
                std::process::exit(2);
            });
            cmd_set_dpi_custom([nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]], active)
        }
        "set-light" => cmd_set_light(args.get(2).map(|s| s.as_str()).unwrap_or("")),
        "reset" => cmd_reset(),
        "set-prefs" => cmd_set_prefs_full(&args[2..]),
        "monitor" => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20);
            cmd_monitor(secs)
        }
        _ => {
            usage();
            2
        }
    };
    std::process::exit(code);
}
