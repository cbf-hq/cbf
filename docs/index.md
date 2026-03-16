# CBF Book

CBF (Chromium Browser Framework) is a Rust-oriented browser backend framework built on Chromium.

It provides a browser-generic API while containing Chromium/Mojo internals behind explicit layer boundaries.
By using CBF, you can develop your browser without being tied to Chromium's frequent updates or complex build system.

This book is the primary long-form entry point for users and contributors.

Who this book is for:
- Application developers integrating CBF with prebuilt artifacts.
- Contributors changing crates, bridge code, or Chromium-fork patches.
- Maintainers reviewing architecture boundaries and failure handling rules.

> Note: For API reference, please check docs.rs: [`cbf` crate](https://docs.rs/cbf/latest/cbf/), [`cbf-chrome` crate](https://docs.rs/cbf-chrome/latest/cbf_chrome/), [`cbf-chrome-sys` crate](https://docs.rs/cbf-chrome-sys/latest/cbf_chrome_sys/).
