fn main() {
    let stages = [800u16, 1600, 2400, 3200, 8000, 25000, 50, 50];
    let active: u8 = 2;
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
    let r = attackshark_rs::protocol::build_dpi_linear(stages, active, true, false, 0x20, colors)
        .expect("build");
    // BLE has no report ID: strip leading 0x04
    print!(
        "{}",
        r[1..]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
}
