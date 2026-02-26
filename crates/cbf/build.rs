use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=CBF_BRIDGE_LIB_DIR");
    let _ = env::var("CBF_BRIDGE_LIB_DIR");
}
