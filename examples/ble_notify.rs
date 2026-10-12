use zbus::blocking::{Connection, Proxy};

const DEV: &str = "/org/bluez/hci0/dev_78_87_11_27_D6_6C";
// (service, char) pairs to subscribe
const CHARS: &[&str] = &[
    "service0059/char005a", // ffc1 vendor write+notify
    "service0059/char005e", // ffc2 vendor write+notify
    "service0012/char001a", // fee4 notify
];

fn main() {
    let conn = Connection::system().unwrap();
    for c in CHARS {
        let path = format!("{DEV}/{c}");
        let proxy = Proxy::new(
            &conn,
            "org.bluez",
            path.clone(),
            "org.bluez.GattCharacteristic1",
        )
        .unwrap();
        match proxy.call::<&str, (), ()>("StartNotify", &()) {
            Ok(()) => println!("notify on {c}"),
            Err(e) => println!("notify {c} FAILED: {e}"),
        }
    }
    println!("--- subscribed, holding 150s ---");
    std::thread::sleep(std::time::Duration::from_secs(150));
    println!("--- done ---");
}
