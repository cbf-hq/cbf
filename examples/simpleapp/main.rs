#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {
    eprintln!("simpleapp currently supports only macOS (Windows planned)");
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod app;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod cli;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod core;

#[cfg(target_os = "macos")]
mod platform_macos;

#[cfg(target_os = "macos")]
fn main() {
    platform_macos::run();
}

#[cfg(target_os = "windows")]
fn main() {
    eprintln!("simpleapp Windows support is not implemented yet");
}
