// SPDX-License-Identifier: AGPL-3.0-only

/// Status codes returned by tempoch-ffi functions.
///
/// Callers must inspect this value before reading any output parameters.
///
/// # ABI Contract
///
/// Discriminant values are frozen; new variants may only be added at the end.
///
/// cbindgen:prefix-with-name
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempochStatus {
    /// Operation completed successfully.
    Ok = 0,
    /// A required output pointer was null.
    NullPointer = 1,
    /// UTC conversion failed (date out of range or invalid).
    UtcConversionFailed = 2,
    /// The period is invalid (start > end).
    InvalidPeriod = 3,
    /// The two periods do not intersect.
    NoIntersection = 4,
    /// The provided scale ID is not a recognized `TempochScale` discriminant.
    InvalidScaleId = 5,
    /// The quantity's unit is not a time-compatible duration unit.
    InvalidDurationUnit = 6,
    /// A Rust panic was caught at the FFI boundary.
    ///
    /// This indicates a bug in the underlying library; the panic payload is
    /// discarded.  Domain errors (`UtcConversionFailed`, `InvalidPeriod`, etc.)
    /// are never reported via this variant.
    InternalPanic = 7,
    /// A UT1 / ΔT conversion was requested for a date outside the compiled
    /// ΔT data horizon.  The output value has not been written.
    Ut1HorizonExceeded = 8,
}
