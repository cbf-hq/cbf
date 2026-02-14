use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=CBF_BRIDGE_LIB_DIR");

    if let Ok(cbf_bridge_lib_dir) = env::var("CBF_BRIDGE_LIB_DIR")
        && !cbf_bridge_lib_dir.is_empty()
    {
        println!(
            "cargo:rustc-link-arg-examples=-Wl,-rpath,{}",
            cbf_bridge_lib_dir
        );
    }
}
