fn main() {
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
    println!("cargo:rerun-if-changed=.env");

    if let Err(err) = dotenvy::dotenv() {
        println!("cargo:warn=.env file failed to load: {err}");
    }

    if let Ok(ssid) = std::env::var("SSID") {
        println!("cargo:rustc-env=SSID={ssid}");
    }
    if let Ok(password) = std::env::var("PASSWORD") {
        println!("cargo:rustc-env=PASSWORD={password}");
    }
}
