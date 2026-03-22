#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("simpleapp currently supports only macOS");
}

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod browser;
#[cfg(target_os = "macos")]
mod cli;
#[cfg(target_os = "macos")]
mod ipc;
#[cfg(target_os = "macos")]
mod platform;
#[cfg(target_os = "macos")]
mod scene;

#[cfg(target_os = "macos")]
fn main() {
    platform::macos::run();
}
