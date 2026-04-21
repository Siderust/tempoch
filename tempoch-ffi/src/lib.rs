// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! C FFI bindings for **tempoch** — astronomical time primitives.
//!
//! This crate exposes a flat C-compatible API for creating and manipulating
//! Julian Dates, Modified Julian Dates, time periods, and UTC conversions.
//!
//! # ABI conventions
//!
//! - Scalar time values cross the ABI as plain `double`s.
//! - UTC calendar fields remain in `TempochUtc` as raw integer fields.
//! - Duration-related functions return [`QttyQuantity`] from qtty-ffi.
//! - Generic time functions accept raw `int32_t` scale IDs, validated before
//!   dispatch.
//! - Every fallible function returns `TempochStatus`; `InternalPanic` is
//!   reserved exclusively for caught Rust panics.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod carriers;
mod error;
mod period;
mod time;

pub use carriers::*;
pub use error::*;
pub use period::*;
pub use time::*;

// Re-export qtty-ffi types used in our public FFI surface
pub use qtty_ffi::{QttyQuantity, UnitId};

/// Catches any panic and returns an error value instead of unwinding across FFI.
/// For functions returning `TempochStatus`, the fallback is `InternalPanic`.
macro_rules! catch_panic {
    ($default:expr, $body:expr) => {{
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(_) => $default,
        }
    }};
}
pub(crate) use catch_panic;

/// Returns the tempoch-ffi ABI version (major*10000 + minor*100 + patch).
///
/// Current ABI line: 0.4.x -> 400
#[allow(clippy::erasing_op, clippy::identity_op)]
#[no_mangle]
pub extern "C" fn tempoch_ffi_version() -> u32 {
    0 * 10000 + 4 * 100 + 0 // 0.4.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_returns_expected_value() {
        assert_eq!(tempoch_ffi_version(), 400);
    }

    // ── Layout tests ──────────────────────────────────────────────────────
    // These tests guard ABI stability by asserting exact sizes and alignments
    // of every exported `#[repr(C)]` type.

    #[test]
    fn layout_tempoch_status() {
        assert_eq!(std::mem::size_of::<TempochStatus>(), 4);
        assert_eq!(std::mem::align_of::<TempochStatus>(), 4);
    }

    #[test]
    fn layout_tempoch_scale_id() {
        assert_eq!(std::mem::size_of::<TempochScaleId>(), 4);
        assert_eq!(std::mem::align_of::<TempochScaleId>(), 4);
    }

    #[test]
    fn layout_tempoch_utc() {
        // i32 + u8*5 + padding + u32 = 16 with C repr packing
        assert_eq!(std::mem::size_of::<TempochUtc>(), 16);
        assert_eq!(std::mem::align_of::<TempochUtc>(), 4);
    }

    #[test]
    fn layout_tempoch_period_mjd() {
        assert_eq!(std::mem::size_of::<TempochPeriodMjd>(), 16);
        assert_eq!(std::mem::align_of::<TempochPeriodMjd>(), 8);
    }

    // ── Status discriminants are stable ───────────────────────────────────

    #[test]
    fn status_discriminants_are_stable() {
        assert_eq!(TempochStatus::Ok as i32, 0);
        assert_eq!(TempochStatus::NullPointer as i32, 1);
        assert_eq!(TempochStatus::UtcConversionFailed as i32, 2);
        assert_eq!(TempochStatus::InvalidPeriod as i32, 3);
        assert_eq!(TempochStatus::NoIntersection as i32, 4);
        assert_eq!(TempochStatus::InvalidScaleId as i32, 5);
        assert_eq!(TempochStatus::InvalidDurationUnit as i32, 6);
        assert_eq!(TempochStatus::InternalPanic as i32, 7);
        assert_eq!(TempochStatus::Ut1HorizonExceeded as i32, 8);
    }

    // ── ScaleId discriminants are stable ──────────────────────────────────

    #[test]
    fn scale_id_discriminants_are_stable() {
        assert_eq!(TempochScaleId::JD as i32, 0);
        assert_eq!(TempochScaleId::MJD as i32, 1);
        assert_eq!(TempochScaleId::TDB as i32, 2);
        assert_eq!(TempochScaleId::TT as i32, 3);
        assert_eq!(TempochScaleId::TAI as i32, 4);
        assert_eq!(TempochScaleId::TCG as i32, 5);
        assert_eq!(TempochScaleId::TCB as i32, 6);
        assert_eq!(TempochScaleId::GPS as i32, 7);
        assert_eq!(TempochScaleId::UT as i32, 8);
        assert_eq!(TempochScaleId::JDE as i32, 9);
        assert_eq!(TempochScaleId::UnixTime as i32, 10);
    }
}
