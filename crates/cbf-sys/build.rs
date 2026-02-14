fn main() {
    println!("cargo:rerun-if-env-changed=CBF_BRIDGE_LIB_DIR");

    if let Ok(cbf_bridge_lib_dir) = std::env::var("CBF_BRIDGE_LIB_DIR") {
        println!("cargo:rustc-link-search=native={cbf_bridge_lib_dir}");
        println!("cargo:rustc-link-lib=dylib=cbf_bridge");
    }
}
