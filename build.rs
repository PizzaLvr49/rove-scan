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

    let st_inc = "STSW-IMG036/VL53L7CX_ULD_driver_2.0.1/VL53L7CX_ULD_API/inc";
    let st_src = "STSW-IMG036/VL53L7CX_ULD_driver_2.0.1/VL53L7CX_ULD_API/src";
    let st_plat = "STSW-IMG036/VL53L7CX_ULD_driver_2.0.1/Platform";

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed={st_inc}");
    println!("cargo:rerun-if-changed={st_src}");
    println!("cargo:rerun-if-changed={st_plat}");

    cc::Build::default()
        .file(format!("{st_src}/vl53l7cx_api.c"))
        .file(format!("{st_src}/vl53l7cx_plugin_detection_thresholds.c"))
        .file(format!("{st_src}/vl53l7cx_plugin_motion_indicator.c"))
        .file(format!("{st_src}/vl53l7cx_plugin_xtalk.c"))
        .include(st_inc)
        .include(st_plat)
        .compile("vl53l7cx");

    let sysroot = std::process::Command::new("arm-none-eabi-gcc")
        .arg("-print-sysroot")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let mut bindgen_builder = bindgen::Builder::default()
        .raw_line("#![allow(warnings, reason = \"Auto-generated FFI bindings via bindgen\")]")
        .raw_line("#![allow(clippy::all, reason = \"Auto-generated FFI bindings via bindgen\")]")
        .raw_line("#![allow(clippy::allow_attributes_without_reason, reason = \"Internal bindgen blocks omit reasons\")]")
        .header("wrapper.h")
        .clang_arg("-ffreestanding")
        .clang_arg(format!("-I{st_inc}"))
        .clang_arg(format!("-I{st_plat}"))
        .allowlist_function("vl53l7cx_.*")
        .allowlist_type("VL53L7CX_.*")
        .allowlist_var("VL53L7CX_.*")
        .use_core();

    if !sysroot.is_empty() {
        bindgen_builder = bindgen_builder.clang_arg(format!("-isystem{sysroot}/include"));
    }

    let bindings = bindgen_builder.generate().expect("bindgen failed");

    bindings
        .write_to_file("src/ffi/bindings.rs")
        .expect("could not write bindings");
}
