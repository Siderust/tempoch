// SPDX-License-Identifier: AGPL-3.0-or-later

/// Status codes returned by tempoch-ffi functions.
///
/// cbindgen:prefix-with-name
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempochStatus {
    /// Success.
    Ok = 0,
    /// A required output pointer was null.
    NullPointer = 1,
    /// UTC conversion failed (date out of range or invalid).
    UtcConversionFailed = 2,
    /// The period is invalid (start > end).
    InvalidPeriod = 3,
    /// The two periods do not intersect.
    NoIntersection = 4,
}
