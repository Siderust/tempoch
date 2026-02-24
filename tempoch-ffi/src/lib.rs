// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! C FFI bindings for **tempoch** — astronomical time primitives.
//!
//! This crate exposes a flat C-compatible API for creating and manipulating
//! Julian Dates, Modified Julian Dates, time periods, and UTC conversions.
//!
//! Duration-related functions return [`QttyQuantity`] from qtty-ffi, providing
//! type-safe unit information alongside numeric values.

mod error;
mod period;
mod time;

pub use error::*;
pub use period::*;
pub use time::*;

// Re-export qtty-ffi types used in our public FFI surface
pub use qtty_ffi::{QttyQuantity, UnitId};

/// Catches any panic and returns an error value instead of unwinding across FFI.
macro_rules! catch_panic {
    ($default:expr, $body:expr) => {{
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(_) => $default,
        }
    }};
}
pub(crate) use catch_panic;

/// Returns the tempoch-ffi ABI version (semver-encoded: major*10000 + minor*100 + patch).
#[allow(clippy::erasing_op, clippy::identity_op)]
#[no_mangle]
pub extern "C" fn tempoch_ffi_version() -> u32 {
    0 * 10000 + 1 * 100 + 0 // 0.1.0
}
