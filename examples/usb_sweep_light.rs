//! Sweep distinct RGB values through commit+prefs so the LED palette can be
//! observed on hardware. 3 s per color; names printed as each is sent.
fn main() {
    let m = attackshark_rs::Mouse::open_usb()
        .or_else(|_| attackshark_rs::Mouse::open())
        .expect("no device");
    let colors: &[(&str, [u8; 3])] = &[
        ("red", [255, 0, 0]),
        ("orange", [255, 128, 0]),
        ("yellow", [255, 255, 0]),
        ("green", [0, 255, 0]),
        ("cyan", [0, 255, 255]),
        ("blue", [0, 0, 255]),
        ("magenta", [255, 0, 255]),
        ("white", [255, 255, 255]),
    ];
    for (name, rgb) in colors {
        let p = attackshark_rs::protocol::Prefs {
            light: attackshark_rs::protocol::LightMode::Static,
            led_speed: 3,
            rgb: *rgb,
            sleep_min: 5.0,
            deep_sleep_min: 10,
            debounce_ms: 8,
        };
        let _ = m.send_commit();
        match m.set_prefs(p) {
            Ok(_) => println!("sent {name}"),
            Err(e) => println!("{name} FAILED: {e}"),
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    println!("SWEEP_DONE");
}
