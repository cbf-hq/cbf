//! CBF (Chromium Browser Framework) is a reusable Rust browser backend framework.
//!
//! This crate provides a browser-generic high-level API for controlling browser
//! backends and handling browser events. Chromium-specific integration and FFI
//! details are kept in lower layers.
//!
//! For setup, architecture, and implementation details, see the repository
//! documentation under `docs/`.
//!
//! Dialog abstractions are available under [`dialogs`].
//! Optional convenience helpers:
//! - `native-dialogs`: native helper functions and [`dialogs::NativeDialogPresenter`]
//!   for `alert` / `confirm`, plus macOS `prompt` support.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, doc(auto_cfg))]

pub mod backend_event_loop;
pub mod browser;
pub mod command;
pub mod data;
pub mod delegate;
pub mod dialogs;
pub mod error;
pub mod event;
pub mod middleware;
