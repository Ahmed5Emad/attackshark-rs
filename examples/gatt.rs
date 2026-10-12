use std::collections::HashMap;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

fn main() {
    let conn = Connection::system().unwrap();
    let root = Proxy::new(
        &conn,
        "org.bluez",
        "/",
        "org.freedesktop.DBus.ObjectManager",
    )
    .unwrap();
    let objs: HashMap<
        OwnedObjectPath,
        HashMap<String, HashMap<String, OwnedValue>>,
    > = root.call("GetManagedObjects", &()).unwrap();
    let mut paths: Vec<_> = objs
        .keys()
        .filter(|p| p.as_str().contains("78_87_11_27_D6_6C"))
        .collect();
    paths.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    for p in paths {
        let ifs = &objs[p];
        if let Some(s) = ifs.get("org.bluez.GattService1") {
            println!("SVC {} uuid={}", p.as_str(), val_str(s.get("UUID")));
        }
        if let Some(c) = ifs.get("org.bluez.GattCharacteristic1") {
            println!(
                "CHR {} uuid={} flags={}",
                p.as_str(),
                val_str(c.get("UUID")),
                val_str(c.get("Flags"))
            );
        }
    }
}

fn val_str(v: Option<&OwnedValue>) -> String {
    match v {
        Some(x) => format!("{x:?}"),
        None => "?".into(),
    }
}
