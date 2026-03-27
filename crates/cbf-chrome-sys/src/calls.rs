#![allow(clippy::too_many_arguments)]

use std::os::raw::c_char;

use crate::{
    bridge::{ArcLibloadingError, BridgeLoadError, bridge},
    ffi::*,
    symbols::for_each_bridge_call,
};

/// Errors returned by runtime-loaded bridge call wrappers.
#[derive(Debug, thiserror::Error, Clone)]
pub enum BridgeCallError {
    /// The bridge library could not be found or opened.
    #[error(transparent)]
    Load(#[from] BridgeLoadError),
    /// A required symbol could not be resolved from the loaded bridge library.
    #[error("failed to load bridge symbol {symbol}: {source}")]
    LoadSymbol {
        /// The bridge symbol name that failed to resolve.
        symbol: &'static str,
        #[source]
        /// The underlying dynamic loader error.
        source: ArcLibloadingError,
    },
}

macro_rules! define_bridge_call {
    ($name:ident, $ty:ident, ($($arg:ident : $arg_ty:ty),*), bool, ($($call_arg:ident),*)) => {
        /// Call the runtime-loaded `cbf_bridge` symbol.
        ///
        /// # Safety
        ///
        /// The caller must uphold the same pointer validity, lifetime, threading,
        /// and ownership requirements as the underlying C ABI function.
        pub unsafe fn $name($($arg: $arg_ty),*) -> Result<bool, BridgeCallError> {
            let bridge = bridge()?;
            let function = unsafe { bridge.get::<$ty>(concat!(stringify!($name), "\0").as_bytes()) }
                .map_err(|source| BridgeCallError::LoadSymbol {
                    symbol: stringify!($name),
                    source: ArcLibloadingError::from(source),
                })?;
            Ok(unsafe { (*function)($($call_arg),*) })
        }
    };
    ($name:ident, $ty:ident, ($($arg:ident : $arg_ty:ty),*), i32, ($($call_arg:ident),*)) => {
        /// Call the runtime-loaded `cbf_bridge` symbol.
        ///
        /// # Safety
        ///
        /// The caller must uphold the same pointer validity, lifetime, threading,
        /// and ownership requirements as the underlying C ABI function.
        pub unsafe fn $name($($arg: $arg_ty),*) -> Result<i32, BridgeCallError> {
            let bridge = bridge()?;
            let function = unsafe { bridge.get::<$ty>(concat!(stringify!($name), "\0").as_bytes()) }
                .map_err(|source| BridgeCallError::LoadSymbol {
                    symbol: stringify!($name),
                    source: ArcLibloadingError::from(source),
                })?;
            Ok(unsafe { (*function)($($call_arg),*) })
        }
    };
    ($name:ident, $ty:ident, ($($arg:ident : $arg_ty:ty),*), *mut CbfBridgeClientHandle, ($($call_arg:ident),*)) => {
        /// Call the runtime-loaded `cbf_bridge` symbol.
        ///
        /// # Safety
        ///
        /// The caller must uphold the same pointer validity, lifetime, threading,
        /// and ownership requirements as the underlying C ABI function.
        pub unsafe fn $name(
            $($arg: $arg_ty),*
        ) -> Result<*mut CbfBridgeClientHandle, BridgeCallError> {
            let bridge = bridge()?;
            let function = unsafe { bridge.get::<$ty>(concat!(stringify!($name), "\0").as_bytes()) }
                .map_err(|source| BridgeCallError::LoadSymbol {
                    symbol: stringify!($name),
                    source: ArcLibloadingError::from(source),
                })?;
            Ok(unsafe { (*function)($($call_arg),*) })
        }
    };
    ($name:ident, $ty:ident, ($($arg:ident : $arg_ty:ty),*), (), ($($call_arg:ident),*)) => {
        /// Call the runtime-loaded `cbf_bridge` symbol.
        ///
        /// # Safety
        ///
        /// The caller must uphold the same pointer validity, lifetime, threading,
        /// and ownership requirements as the underlying C ABI function.
        pub unsafe fn $name($($arg: $arg_ty),*) -> Result<(), BridgeCallError> {
            let bridge = bridge()?;
            let function = unsafe { bridge.get::<$ty>(concat!(stringify!($name), "\0").as_bytes()) }
                .map_err(|source| BridgeCallError::LoadSymbol {
                    symbol: stringify!($name),
                    source: ArcLibloadingError::from(source),
                })?;
            unsafe { (*function)($($call_arg),*) };
            Ok(())
        }
    };
}

for_each_bridge_call!(define_bridge_call);
