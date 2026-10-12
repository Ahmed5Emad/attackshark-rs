fn main() {
    let p = attackshark_rs::protocol::Prefs {
        light: attackshark_rs::protocol::LightMode::Static,
        led_speed: 3,
        rgb: [255, 0, 0],
        sleep_min: 5.0,
        deep_sleep_min: 10,
        debounce_ms: 8,
    };
    let r = attackshark_rs::protocol::build_prefs(p).expect("build");
    print!(
        "{}",
        r.iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
}
