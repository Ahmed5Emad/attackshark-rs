//! USB read-only probe: open the wired/receiver V6 (never Bluetooth),
//! send the a1 handshake + a0 dump query as FEATURE reports, print every
//! interrupt reply. Pure reads — changes nothing on the mouse.
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}

fn main() {
    let api = hidapi::HidApi::new().expect("hid init");
    let mut pick: Option<std::ffi::CString> = None;
    for d in api.device_list() {
        let bt = d.product_string().map(|s| s.contains("ATTACK SHARK")).unwrap_or(false)
            && d.vendor_id() != attackshark_rs::VID;
        if bt {
            continue;
        }
        if d.vendor_id() == attackshark_rs::VID
            && attackshark_rs::PIDS.contains(&d.product_id())
            && d.interface_number() == 2
        {
            println!("CANDIDATE pid={:04x} iface={}", d.product_id(), d.interface_number());
            pick = Some(d.path().to_owned());
        }
    }
    let dev = match pick {
        Some(p) => api.open_path(&p).expect("open"),
        None => {
            println!("OPEN_FAIL: no USB V6");
            return;
        }
    };
    println!("OPEN_OK");
    let ask = |r: &[u8], tag: &str| {
        match dev.send_feature_report(r) {
            Ok(_) => println!("{tag} sent {}", hex(r)),
            Err(e) => println!("{tag} SEND_FAIL: {e}"),
        }
        let t0 = std::time::Instant::now();
        let mut buf = [0u8; 256];
        while t0.elapsed().as_secs() < 4 {
            match dev.read_timeout(&mut buf, 500) {
                Ok(0) => {}
                Ok(n) => println!("{tag} REPLY len={n}: {}", hex(&buf[..n])),
                Err(e) => {
                    println!("{tag} read err: {e}");
                    break;
                }
            }
        }
    };
    ask(&[0xa1, 0x08, 0, 0, 0, 0, 0, 0], "HS");
    ask(&[0xa0, 0x08, 0, 0, 0, 0, 0, 0], "DUMP");
    // Same queries as OUTPUT reports (dev.write): BT fee3-write maps to
    // one of these paths — find which one the dump answers on.
    for (tag, r) in [
        ("O-HS", [0xa1u8, 0x08, 0, 0, 0, 0, 0, 0]),
        ("O-DUMP", [0xa0u8, 0x08, 0, 0, 0, 0, 0, 0]),
    ] {
        match dev.write(&r) {
            Ok(_) => println!("{tag} sent {}", hex(&r)),
            Err(e) => println!("{tag} SEND_FAIL: {e}"),
        }
        let t0 = std::time::Instant::now();
        let mut buf = [0u8; 256];
        while t0.elapsed().as_secs() < 4 {
            match dev.read_timeout(&mut buf, 500) {
                Ok(0) => {}
                Ok(n) => println!("{tag} REPLY len={n}: {}", hex(&buf[..n])),
                Err(e) => {
                    println!("{tag} read err: {e}");
                    break;
                }
            }
        }
    }
    println!("--- done ---");
}
