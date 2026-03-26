#![allow(clippy::too_many_arguments)]

use std::os::raw::c_char;
use std::ptr;

use crate::{bridge::bridge, ffi::*, symbols::for_each_bridge_call};

macro_rules! define_bridge_call {
    ($name:ident, $ty:ident, ($($arg:ident : $arg_ty:ty),*), bool, ($($call_arg:ident),*)) => {
        /// Call the runtime-loaded `cbf_bridge` symbol.
        ///
        /// # Safety
        ///
        /// The caller must uphold the same pointer validity, lifetime, threading,
        /// and ownership requirements as the underlying C ABI function.
        pub unsafe fn $name($($arg: $arg_ty),*) -> bool {
            let Ok(bridge) = bridge() else {
                return false;
            };
            let Ok(function) = (unsafe { bridge.get::<$ty>(concat!(stringify!($name), "\0").as_bytes()) }) else {
                return false;
            };
            unsafe { (*function)($($call_arg),*) }
        }
    };
    ($name:ident, $ty:ident, ($($arg:ident : $arg_ty:ty),*), i32, ($($call_arg:ident),*)) => {
        /// Call the runtime-loaded `cbf_bridge` symbol.
        ///
        /// # Safety
        ///
        /// The caller must uphold the same pointer validity, lifetime, threading,
        /// and ownership requirements as the underlying C ABI function.
        pub unsafe fn $name($($arg: $arg_ty),*) -> i32 {
            let Ok(bridge) = bridge() else {
                return if stringify!($name) == "cbf_bridge_client_wait_for_event" {
                    CBF_BRIDGE_EVENT_WAIT_STATUS_CLOSED
                } else {
                    -1
                };
            };
            let Ok(function) = (unsafe { bridge.get::<$ty>(concat!(stringify!($name), "\0").as_bytes()) }) else {
                return if stringify!($name) == "cbf_bridge_client_wait_for_event" {
                    CBF_BRIDGE_EVENT_WAIT_STATUS_CLOSED
                } else {
                    -1
                };
            };
            unsafe { (*function)($($call_arg),*) }
        }
    };
    ($name:ident, $ty:ident, ($($arg:ident : $arg_ty:ty),*), *mut CbfBridgeClientHandle, ($($call_arg:ident),*)) => {
        /// Call the runtime-loaded `cbf_bridge` symbol.
        ///
        /// # Safety
        ///
        /// The caller must uphold the same pointer validity, lifetime, threading,
        /// and ownership requirements as the underlying C ABI function.
        pub unsafe fn $name($($arg: $arg_ty),*) -> *mut CbfBridgeClientHandle {
            let Ok(bridge) = bridge() else {
                return ptr::null_mut();
            };
            let Ok(function) = (unsafe { bridge.get::<$ty>(concat!(stringify!($name), "\0").as_bytes()) }) else {
                return ptr::null_mut();
            };
            unsafe { (*function)($($call_arg),*) }
        }
    };
    ($name:ident, $ty:ident, ($($arg:ident : $arg_ty:ty),*), (), ($($call_arg:ident),*)) => {
        /// Call the runtime-loaded `cbf_bridge` symbol.
        ///
        /// # Safety
        ///
        /// The caller must uphold the same pointer validity, lifetime, threading,
        /// and ownership requirements as the underlying C ABI function.
        pub unsafe fn $name($($arg: $arg_ty),*) {
            let Ok(bridge) = bridge() else {
                return;
            };
            let Ok(function) = (unsafe { bridge.get::<$ty>(concat!(stringify!($name), "\0").as_bytes()) }) else {
                return;
            };
            unsafe { (*function)($($call_arg),*) }
        }
    };
}

for_each_bridge_call!(define_bridge_call);
