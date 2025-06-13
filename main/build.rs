fn main() {
    // use std::env;
    // let target = env::var("STAMP").unwrap();
    // println!("{target}");
    // #[cfg(variable = "BAR")]
    // if target == "esp32" {
    // println!("Configured for ESP32");
    // feature="esp32
    // println!("cargo:rustc-cfg=feature=\"esp32\" ");
    // } else {
    // println!("NOT configured for ESP32");
    // }
    // println!("cargo:rustc-target=target=\"xtensa-esp32-none-elf\" ");
    // println!("cargo:rustc-target=target=\"riscv32imac-unknown-none-elf\" ");

    println!("cargo:rustc-link-arg-bins=-Tlinkall.x");
}
