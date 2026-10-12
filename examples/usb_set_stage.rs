//! USB one-shot: resend a full DPI table with chosen active stage.
//! Usage: usb_set_stage <1-6>. Table/colors/prefs are the app defaults —
//! caller confirms they match the mouse before running.
fn main() {
    let stage: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .filter(|s| (1..=6).contains(s))
        .expect("usage: usb_set_stage <1-6>");
    let m = attackshark_rs::Mouse::open_usb().expect("open");
    println!("transport={}", m.transport());
    if m.transport() == "Bluetooth" {
        println!("REFUSED: on Bluetooth, switch to cable/2.4G");
        return;
    }
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
    match m.set_dpi_full(stages, stage, true, false, 0x20, colors) {
        Ok(ack) => println!("SET_OK ack={}", attackshark_rs::hex(&ack)),
        Err(e) => println!("SET_FAIL: {e}"),
    }
}
