//! Build script for Vehicle Diagnostics Pipeline
//!
//! Compiles the C++ CAN/OBD driver library and links it with Rust.

fn main() {
    // Tell Cargo to rerun if C++ sources change
    println!("cargo:rerun-if-changed=src_cpp/");
    println!("cargo:rerun-if-changed=include/");
    println!("cargo:rerun-if-changed=CMakeLists.txt");

    // Build C++ library using cc
    let mut build = cc::Build::new();

    build
        .cpp(true)
        .std("c++17")
        .include("include")
        .file("src_cpp/can_driver.cpp")
        .file("src_cpp/obd_client.cpp")
        .file("src_cpp/frame_decoder.cpp")
        .file("src_cpp/timing_service.cpp")
        .file("src_cpp/c_api.cpp")
        .warnings(true)
        .extra_warnings(true);

    // Platform-specific flags
    #[cfg(target_os = "linux")]
    {
        build.define("HAS_SOCKETCAN", "1");
    }

    #[cfg(not(target_os = "linux"))]
    {
        build.define("HAS_SOCKETCAN", "0");
    }

    // Compile
    build.compile("can_obd_driver");

    // Link with system libraries on Linux
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=rt");  // For clock_gettime
    }
}
