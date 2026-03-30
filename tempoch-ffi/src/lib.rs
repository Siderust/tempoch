// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! C FFI bindings for **tempoch** — astronomical time primitives.
//!
//! This crate exposes a flat C-compatible API for creating and manipulating
//! Julian Dates, Modified Julian Dates, time periods, and UTC conversions.
//!
//! # ABI conventions (vNext)
//!
//! - Every absolute instant is wrapped in a typed carrier (`TempochJd`,
//!   `TempochMjd`, `TempochTt`, …) rather than a bare `double`.
//! - UTC calendar fields remain in `TempochUtc` as raw integer fields.
//! - Duration-related functions return [`QttyQuantity`] from qtty-ffi.
//! - Generic scale-dispatch functions accept raw `int32_t` scale IDs (not
//!   the `TempochScaleId` enum directly), validated before dispatch.
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
/// Current version: 0.3.0 → 300
#[allow(clippy::erasing_op, clippy::identity_op)]
#[no_mangle]
pub extern "C" fn tempoch_ffi_version() -> u32 {
    0 * 10000 + 3 * 100 + 0 // 0.3.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_returns_expected_value() {
        assert_eq!(tempoch_ffi_version(), 300);
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
    fn layout_tempoch_jd() {
        assert_eq!(std::mem::size_of::<TempochJd>(), 8);
        assert_eq!(std::mem::align_of::<TempochJd>(), 8);
    }

    #[test]
    fn layout_tempoch_mjd() {
        assert_eq!(std::mem::size_of::<TempochMjd>(), 8);
        assert_eq!(std::mem::align_of::<TempochMjd>(), 8);
    }

    #[test]
    fn layout_tempoch_tt() {
        assert_eq!(std::mem::size_of::<TempochTt>(), 8);
        assert_eq!(std::mem::align_of::<TempochTt>(), 8);
    }

    #[test]
    fn layout_tempoch_tdb() {
        assert_eq!(std::mem::size_of::<TempochTdb>(), 8);
        assert_eq!(std::mem::align_of::<TempochTdb>(), 8);
    }

    #[test]
    fn layout_tempoch_tai() {
        assert_eq!(std::mem::size_of::<TempochTai>(), 8);
        assert_eq!(std::mem::align_of::<TempochTai>(), 8);
    }

    #[test]
    fn layout_tempoch_tcg() {
        assert_eq!(std::mem::size_of::<TempochTcg>(), 8);
        assert_eq!(std::mem::align_of::<TempochTcg>(), 8);
    }

    #[test]
    fn layout_tempoch_tcb() {
        assert_eq!(std::mem::size_of::<TempochTcb>(), 8);
        assert_eq!(std::mem::align_of::<TempochTcb>(), 8);
    }

    #[test]
    fn layout_tempoch_gps() {
        assert_eq!(std::mem::size_of::<TempochGps>(), 8);
        assert_eq!(std::mem::align_of::<TempochGps>(), 8);
    }

    #[test]
    fn layout_tempoch_ut() {
        assert_eq!(std::mem::size_of::<TempochUt>(), 8);
        assert_eq!(std::mem::align_of::<TempochUt>(), 8);
    }

    #[test]
    fn layout_tempoch_jde() {
        assert_eq!(std::mem::size_of::<TempochJde>(), 8);
        assert_eq!(std::mem::align_of::<TempochJde>(), 8);
    }

    #[test]
    fn layout_tempoch_unix_time() {
        assert_eq!(std::mem::size_of::<TempochUnixTime>(), 8);
        assert_eq!(std::mem::align_of::<TempochUnixTime>(), 8);
    }

    #[test]
    fn layout_tempoch_period_mjd() {
        // Two TempochMjd (each 8 bytes) → 16 bytes, 8-aligned
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
