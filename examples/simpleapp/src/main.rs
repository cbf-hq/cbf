#[cfg(not(any(target_os = "macos")))]
fn main() {
    eprintln!("simpleapp currently supports only macOS");
}

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod cli;
#[cfg(target_os = "macos")]
mod core;

#[cfg(target_os = "macos")]
mod platform_macos;

#[cfg(target_os = "macos")]
fn main() {
    platform_macos::run();
}
