//! Wired write visibility: send the real DPI feature report, dump every
//! interrupt reply for 6s. Read-only except the one DPI table write.
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}

fn main() {
    let m = attackshark_rs::Mouse::open_usb().expect("open");
    let stages = [800u16, 1600, 2400, 3200, 8000, 25000, 50, 50];
    let colors = [
        [0xff, 0, 0],
        [0, 0xff, 0],
        [0, 0, 0xff],
        [0xff, 0xff, 0],
        [0, 0xff, 0xff],
        [0xff, 0, 0xff],
        [0, 0, 0],
        [0, 0, 0],
    ];
    let r = attackshark_rs::protocol::build_dpi_linear(stages, 6, true, false, 0x20, colors)
        .expect("build");
    println!("REPORT len={}: {}", r.len(), hex(&r));
    // Pad to full 64B: the X6 driver sends fixed-size reports; a short
    // feature report may be ignored without an answer.
    let mut r64 = [0u8; 64];
    r64[..r.len()].copy_from_slice(&r);
    // send raw via a second handle so we can listen freely
    let api = hidapi::HidApi::new().expect("hid");
    let mut dev = None;
    for d in api.device_list() {
        if d.vendor_id() == attackshark_rs::VID
            && d.product_id() == 0x2224
            && d.interface_number() == 2
        {
            dev = Some(api.open_path(d.path()).expect("open2"));
        }
    }
    let dev = dev.expect("wired handle");
    let _ = &m;
    // Arm first: the a1 handshake may gate writes (queries answer
    // without it, but the DPI report got silence twice).
    match dev.send_feature_report(&[0xa1, 0x08, 0, 0, 0, 0, 0, 0]) {
        Ok(_) => println!("HS sent"),
        Err(e) => println!("HS SEND_FAIL: {e}"),
    }
    std::thread::sleep(std::time::Duration::from_secs(1));
    match dev.send_feature_report(&r64) {
        Ok(_) => println!("SENT 64B feature"),
        Err(e) => println!("SEND_FAIL: {e}"),
    }
    let t0 = std::time::Instant::now();
    let mut buf = [0u8; 256];
    while t0.elapsed().as_secs() < 6 {
        match dev.read_timeout(&mut buf, 500) {
            Ok(0) => {}
            Ok(n) => println!("REPLY len={n}: {}", hex(&buf[..n])),
            Err(e) => {
                println!("read err: {e}");
                break;
            }
        }
    }
    println!("--- done ---");
}
