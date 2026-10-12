//! Live lighting test: send Static-red prefs (report 0x05) and print the ACK.
//! Visible proof = mouse LED turns solid red. Restore from the UI after.
fn main() {
    let m = attackshark_rs::Mouse::open_usb()
        .or_else(|_| attackshark_rs::Mouse::open())
        .expect("no device");
    println!("link: {} {}", m.transport(), m.channel());
    let p = attackshark_rs::protocol::Prefs {
        light: attackshark_rs::protocol::LightMode::Static,
        led_speed: 3,
        rgb: [128, 0, 0],
        sleep_min: 5.0,
        deep_sleep_min: 10,
        debounce_ms: 8,
    };
    match m.send_commit() {
        Ok(ack) => println!(
            "COMMIT_ACK [{}]",
            ack.iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Err(e) => println!("COMMIT_FAIL: {e}"),
    }
    match m.set_prefs(p) {
        Ok(ack) => println!(
            "LIGHT_ACK [{}]",
            ack.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
        ),
        Err(e) => println!("LIGHT_FAIL: {e}"),
    }
}
