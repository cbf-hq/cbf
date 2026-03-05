fn main() {
    println!("cargo:rerun-if-env-changed=CBF_BRIDGE_LIB_DIR");

    let Ok(cbf_bridge_lib_dir) = std::env::var("CBF_BRIDGE_LIB_DIR") else {
        return;
    };

    println!("cargo:rustc-link-search=native={cbf_bridge_lib_dir}");

    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{cbf_bridge_lib_dir}");
    }
}
