//! Write a payload to a BLE characteristic on the mouse via BlueZ WriteValue.
//! Usage: ble_write <char-path-suffix> <hex...>
//!   e.g. ble_write service0059/char005a 38 01 00 ...
//! Prints OK or the D-Bus error. Passive until you run it.
use std::collections::HashMap;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::Value;

const DEV: &str = "/org/bluez/hci0/dev_78_87_11_27_D6_6C";

fn main() {
    let mut args = std::env::args().skip(1);
    let char = args.next().expect("char path suffix");
    let bytes: Vec<u8> = args
        .map(|a| u8::from_str_radix(a.trim_start_matches("0x"), 16).expect("hex byte"))
        .collect();
    let path = format!("{DEV}/{char}");
    let conn = Connection::system().expect("system bus");
    let proxy = Proxy::new(
        &conn,
        "org.bluez",
        path.clone(),
        "org.bluez.GattCharacteristic1",
    )
    .expect("proxy");
    let opts: HashMap<String, Value> = HashMap::new();
    let r: Result<(), zbus::Error> = proxy.call("WriteValue", &(bytes, opts));
    match r {
        Ok(()) => println!("WRITE {char} OK"),
        Err(e) => println!("WRITE {char} FAILED: {e}"),
    }
}
