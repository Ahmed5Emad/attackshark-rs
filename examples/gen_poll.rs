fn main() {
    let r = attackshark_rs::protocol::build_polling(1000).expect("build");
    print!(
        "{}",
        r.iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
}
