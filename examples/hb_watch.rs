fn main() {
    let usb_only = std::env::args().any(|a| a == "usb");
    let api = hidapi::HidApi::new().expect("hid init");
    let mut usb_path: Option<std::ffi::CString> = None;
    let mut bt_path: Option<std::ffi::CString> = None;
    for d in api.device_list() {
        if d.vendor_id() == attackshark_rs::VID
            && attackshark_rs::PIDS.contains(&d.product_id())
            && d.interface_number() == 2
        {
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
    println!(
        "FOUND usb={} bt={}",
        usb_path.is_some(),
        bt_path.is_some()
    );
    let pick = if usb_only { usb_path } else { bt_path.or(usb_path) };
    let dev = match pick {
        Some(p) => api.open_path(&p).expect("open"),
        None => {
            println!("OPEN_FAIL: no device");
            return;
        }
    };
    let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut buf = [0u8; 64];
    while std::time::Instant::now() < end {
        match dev.read_timeout(&mut buf, 1000) {
            Ok(0) => println!("READ: timeout"),
            Ok(len) => println!("EV: {:?}", attackshark_rs::decode_event(&buf[..len])),
            Err(e) => println!("READ: {e}"),
        }
    }
    println!("--- done ---");
}
