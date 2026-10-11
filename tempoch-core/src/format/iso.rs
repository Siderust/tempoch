// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! ISO 8601 / RFC 3339 parsing and formatting for `Time<UTC>`
//! and (via scale conversion) `Time<TAI>`.
//!
//! The civil layer is `chrono`-backed today (chrono is a hard dependency
//! of `tempoch-core`); this module wraps the conversion to provide:
//!
//! * Subsecond precision configurable from 0..9 digits.
//! * `FormatPrecision::{Truncate, RoundHalfToEven}` rounding policy.
//! * Leap-second-aware formatting: `23:59:60[.x]` is emitted *iff* the
//!   instant lands during an announced positive leap second, and accepted on
//!   parse.
//! * A small `FormatOptions` value type so callers can opt into different
//!   subsecond/leap-second/timezone formatting policies without affecting
//!   the existing `chrono` bridge.
//!
//! The conversion goes through `Time<UTC, J2000s>` storage, so the
//! resulting instants are usable on any scale via the unified
//! `to::<Scale>()` / `to_with::<Scale>()` API.
//!
//! # Examples
//!
//! ```
//! use tempoch_core::format::iso::FormatOptions;
//! use tempoch_core::{Time, UTC};
//!
//! let t = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56.789Z").unwrap();
//! let s = t.format_rfc3339(FormatOptions::milliseconds());
//! assert!(s.starts_with("2024-06-15T12:34:56.789"));
//! ```

use chrono::{DateTime, Utc};

use crate::earth::context::TimeContext;
use crate::foundation::error::ConversionError;
use crate::model::scale::UTC;
use crate::model::time::Time;
use alloc::string::{String, ToString};

/// Subsecond rounding policy used by the formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatPrecision {
    /// Round half-to-even at the requested subsecond digit (default).
    RoundHalfToEven,
    /// Truncate toward zero at the requested subsecond digit.
    Truncate,
}

/// Format options for ISO 8601 / RFC 3339 output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    /// Number of subsecond digits to emit (0..=9). Values above 9 are
    /// clamped to 9 because the civil bridge resolves nanosecond labels.
    pub subsecond_digits: u8,
    /// Rounding policy when truncating below the requested precision.
    pub precision: FormatPrecision,
    /// When true, emit the trailing `Z` (RFC 3339); when false, emit no
    /// timezone suffix (bare ISO 8601 naive datetime). UTC offsets other
    /// than `Z` are not supported because the underlying scale is UTC.
    pub include_zulu: bool,
}

impl FormatOptions {
    /// Default RFC 3339 form with seconds resolution and `Z` suffix.
    pub const SECONDS: Self = Self {
        subsecond_digits: 0,
        precision: FormatPrecision::Truncate,
        include_zulu: true,
    };

    /// Milliseconds resolution (3 fractional digits).
    pub const fn milliseconds() -> Self {
        Self {
            subsecond_digits: 3,
            precision: FormatPrecision::RoundHalfToEven,
            include_zulu: true,
        }
    }

    /// Microseconds resolution (6 fractional digits).
    pub const fn microseconds() -> Self {
        Self {
            subsecond_digits: 6,
            precision: FormatPrecision::RoundHalfToEven,
            include_zulu: true,
        }
    }

    /// Nanoseconds resolution (9 fractional digits).
    pub const fn nanoseconds() -> Self {
        Self {
            subsecond_digits: 9,
            precision: FormatPrecision::RoundHalfToEven,
            include_zulu: true,
        }
    }
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self::nanoseconds()
    }
}

/// Parse an RFC 3339 timestamp into the canonical UTC `J2000s` storage.
///
/// Accepts the leap-second form `23:59:60[.x]` during announced positive
/// leap seconds; rejects it otherwise.
#[inline]
pub fn parse_rfc3339_utc(s: &str) -> Result<Time<UTC>, ConversionError> {
    parse_rfc3339_utc_with(s, &TimeContext::new())
}

/// Like [`parse_rfc3339_utc`], but uses an explicit [`TimeContext`].
pub fn parse_rfc3339_utc_with(s: &str, ctx: &TimeContext) -> Result<Time<UTC>, ConversionError> {
    // RFC 3339 fields are ASCII. Reject other UTF-8 before parsing.
    if !s.is_ascii() {
        return Err(ConversionError::OutOfRange);
    }
    // Chrono truncates excess fractional digits; enforce the civil bridge's
    // nanosecond resolution before parsing, including lowercase `z` forms.
    if let Some(after_dot) = s.find('.') {
        let digits = s[after_dot + 1..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        if digits == 0 || digits > 9 {
            return Err(ConversionError::OutOfRange);
        }
    }

    // Chrono already parses positive leap-second labels with nanoseconds
    // >= 1e9. The shared civil engine validates them against the context.
    let dt = DateTime::parse_from_rfc3339(s).map_err(|_| ConversionError::OutOfRange)?;
    Time::<UTC>::try_from_chrono_with(dt.with_timezone(&Utc), ctx)
}

impl Time<UTC> {
    /// Parse an RFC 3339 / ISO 8601 timestamp (UTC, `Z` suffix or named
    /// offset). Accepts the leap-second form `23:59:60[.x]` during
    /// announced positive leap seconds.
    #[inline]
    pub fn parse_rfc3339(s: &str) -> Result<Self, ConversionError> {
        parse_rfc3339_utc(s)
    }

    /// Like [`parse_rfc3339`](Self::parse_rfc3339), with an explicit
    /// [`TimeContext`].
    #[inline]
    pub fn parse_rfc3339_with(s: &str, ctx: &TimeContext) -> Result<Self, ConversionError> {
        parse_rfc3339_utc_with(s, ctx)
    }

    /// Format this UTC instant as RFC 3339 with the given options.
    ///
    /// Emits `23:59:60[.fraction]Z` when the instant lies during an announced
    /// positive leap second according to the default [`TimeContext`].
    pub fn format_rfc3339(&self, opts: FormatOptions) -> String {
        self.format_rfc3339_with(opts, &TimeContext::new())
    }

    /// Like [`format_rfc3339`](Self::format_rfc3339), with an explicit
    /// [`TimeContext`].
    ///
    /// Returns `"<invalid>"` if the instant cannot be converted to civil UTC.
    /// Use [`try_format_rfc3339_with`](Self::try_format_rfc3339_with) to
    /// handle that case explicitly.
    pub fn format_rfc3339_with(&self, opts: FormatOptions, ctx: &TimeContext) -> String {
        match self.try_format_rfc3339_with(opts, ctx) {
            Ok(s) => s,
            Err(_) => "<invalid>".to_string(),
        }
    }

    /// Fallible variant of [`format_rfc3339_with`](Self::format_rfc3339_with).
    ///
    /// Returns [`ConversionError`] if the underlying UTC↔chrono conversion
    /// fails (e.g. out-of-range dates).
    pub fn try_format_rfc3339_with(
        &self,
        opts: FormatOptions,
        ctx: &TimeContext,
    ) -> Result<String, ConversionError> {
        let dt = self.try_to_chrono_with(ctx)?;
        let is_leap = self.is_leap_second_with(ctx);
        let digits = opts.subsecond_digits.min(9) as usize;
        let raw_nanos = dt.timestamp_subsec_nanos();
        if is_leap != (raw_nanos >= 1_000_000_000) {
            return Err(ConversionError::InvalidLeapSecond);
        }
        let (frac, carry) = round_subsecond(raw_nanos % 1_000_000_000, digits, opts.precision);
        if carry {
            // Advance the civil second through the context, since chrono's
            // ordinary arithmetic skips a leap second unless already inside it.
            let next = DateTime::<Utc>::from_timestamp(dt.timestamp() + 1, 0)
                .ok_or(ConversionError::OutOfRange)?;
            let next_utc = Self::try_from_chrono_with(next, ctx)?;
            let before = next_utc.sub_exact(crate::ExactDuration::from_nanos(1_000_000_000));
            if !is_leap && before.is_leap_second_with(ctx) {
                return Ok(format_normal_dt(
                    before.try_to_chrono_with(ctx)?,
                    0,
                    digits,
                    opts,
                ));
            }
            return Ok(format_normal_dt(next, 0, digits, opts));
        }
        Ok(format_normal_dt(dt, frac, digits, opts))
    }
}

/// Apply rounding/truncation to `nanos` (0..1_000_000_000) and return
/// `(fractional_value_at_digits, carry_into_next_second)`.
fn round_subsecond(nanos: u32, digits: usize, precision: FormatPrecision) -> (u32, bool) {
    debug_assert!(digits <= 9);
    if digits == 9 {
        return (nanos, false);
    }
    let scale = 10_u32.pow(9 - digits as u32);
    let truncated = nanos / scale;
    let rem = nanos % scale;
    let mut result = truncated;
    if matches!(precision, FormatPrecision::RoundHalfToEven) {
        let half = scale / 2;
        if rem > half || (rem == half && truncated % 2 == 1) {
            result = result.saturating_add(1);
        }
    }
    let threshold = 10_u32.pow(digits as u32);
    let carry = result >= threshold;
    if carry {
        result -= threshold;
    }
    (result, carry)
}

fn format_normal_dt(dt: DateTime<Utc>, frac: u32, digits: usize, opts: FormatOptions) -> String {
    let base = dt.format("%Y-%m-%dT%H:%M:%S");
    if digits == 0 {
        let zulu = if opts.include_zulu { "Z" } else { "" };
        format!("{base}{zulu}")
    } else {
        let zulu = if opts.include_zulu { "Z" } else { "" };
        format!("{base}.{:0width$}{zulu}", frac, width = digits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_z() {
        let t = Time::<UTC>::parse_rfc3339("2000-01-01T12:00:00Z").unwrap();
        let s = t.format_rfc3339(FormatOptions::SECONDS);
        assert_eq!(s, "2000-01-01T12:00:00Z");
    }

    #[test]
    fn parse_with_milliseconds() {
        let t = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56.789Z").unwrap();
        let s = t.format_rfc3339(FormatOptions::milliseconds());
        assert_eq!(s, "2024-06-15T12:34:56.789Z");
    }

    #[test]
    fn parse_with_microseconds_exact() {
        let t = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56.123456Z").unwrap();
        let s = t.format_rfc3339(FormatOptions::microseconds());
        assert_eq!(s, "2024-06-15T12:34:56.123456Z");
    }

    #[test]
    fn parse_with_nanoseconds_exact() {
        let t = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56.123456789Z").unwrap();
        let s = t.format_rfc3339(FormatOptions::nanoseconds());
        assert_eq!(s, "2024-06-15T12:34:56.123456789Z");
        assert_eq!(s.len(), "2024-06-15T12:34:56.123456789Z".len());
    }

    #[test]
    fn parse_with_named_offset_normalizes_to_utc() {
        let t = Time::<UTC>::parse_rfc3339("2024-06-15T14:34:56+02:00").unwrap();
        let s = t.format_rfc3339(FormatOptions::SECONDS);
        assert_eq!(s, "2024-06-15T12:34:56Z");
    }

    #[test]
    fn format_leap_second_emits_colon_sixty() {
        // 2016-12-31T23:59:60Z was an announced positive leap second.
        let t = Time::<UTC>::parse_rfc3339("2016-12-31T23:59:60Z").unwrap();
        let s = t.format_rfc3339(FormatOptions::SECONDS);
        assert_eq!(s, "2016-12-31T23:59:60Z");
    }

    #[test]
    fn format_leap_second_with_fraction() {
        let t = Time::<UTC>::parse_rfc3339("2016-12-31T23:59:60.500Z").unwrap();
        let s = t.format_rfc3339(FormatOptions::milliseconds());
        assert_eq!(s, "2016-12-31T23:59:60.500Z");
    }

    #[test]
    fn reject_malformed_input() {
        assert!(Time::<UTC>::parse_rfc3339("not a date").is_err());
        assert!(Time::<UTC>::parse_rfc3339("2024-13-01T00:00:00Z").is_err());
        assert!(Time::<UTC>::parse_rfc3339("2024-06-15T25:00:00Z").is_err());
    }

    #[test]
    fn reject_empty_fraction() {
        // "2024-06-15T12:34:56.Z" has an empty fraction field — must be rejected.
        let result = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56.Z");
        assert!(result.is_err(), "expected Err for empty fraction, got Ok");
    }

    #[test]
    fn reject_more_than_nine_fractional_digits() {
        // 10 digits — must be rejected.
        let result = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56.1234567890Z");
        assert!(result.is_err(), "expected Err for >9 fractional digits");
    }

    #[test]
    fn round_trip_seconds_precision() {
        for s in ["2000-01-01T00:00:00Z", "1999-12-31T23:59:59Z"] {
            let t = Time::<UTC>::parse_rfc3339(s).unwrap();
            let back = t.format_rfc3339(FormatOptions::SECONDS);
            assert_eq!(back, s, "round trip mismatch for {s}");
        }
    }

    #[test]
    fn format_options_constants_are_consistent() {
        assert_eq!(FormatOptions::SECONDS.subsecond_digits, 0);
        assert_eq!(FormatOptions::milliseconds().subsecond_digits, 3);
        assert_eq!(FormatOptions::microseconds().subsecond_digits, 6);
        assert_eq!(FormatOptions::nanoseconds().subsecond_digits, 9);
    }

    #[test]
    fn arbitrary_precision_digits_are_supported() {
        let t = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56.123456789Z").unwrap();
        let opts = FormatOptions {
            subsecond_digits: 4,
            precision: FormatPrecision::Truncate,
            include_zulu: true,
        };
        let s = t.format_rfc3339(opts);
        assert_eq!(s, "2024-06-15T12:34:56.1234Z");
    }

    #[test]
    fn truncate_vs_round_differs_on_5() {
        let t = Time::<UTC>::parse_rfc3339("2000-06-15T12:34:56.55Z").unwrap();
        let trunc = FormatOptions {
            subsecond_digits: 1,
            precision: FormatPrecision::Truncate,
            include_zulu: true,
        };
        let round = FormatOptions {
            subsecond_digits: 1,
            precision: FormatPrecision::RoundHalfToEven,
            include_zulu: true,
        };
        let st = t.format_rfc3339(trunc);
        let sr = t.format_rfc3339(round);
        assert!(st.ends_with(".5Z"), "truncate got {st}");
        // Half-to-even: .5 with truncated = 5 (odd) rounds up to .6.
        assert!(sr.ends_with(".6Z"), "round-half-to-even got {sr}");
    }

    #[test]
    fn omit_zulu_suffix() {
        let t = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56Z").unwrap();
        let opts = FormatOptions {
            subsecond_digits: 0,
            precision: FormatPrecision::Truncate,
            include_zulu: false,
        };
        let s = t.format_rfc3339(opts);
        assert_eq!(s, "2024-06-15T12:34:56");
    }

    #[test]
    fn reject_invalid_leap_second_date() {
        // 2023-06-15 was NOT a leap-second day; :60 must be rejected.
        let result = Time::<UTC>::parse_rfc3339("2023-06-15T23:59:60Z");
        assert!(
            matches!(result, Err(ConversionError::InvalidLeapSecond)),
            "expected InvalidLeapSecond, got {result:?}"
        );
        // 2016-12-31 WAS a leap-second day; must parse successfully.
        assert!(
            Time::<UTC>::parse_rfc3339("2016-12-31T23:59:60Z").is_ok(),
            "expected Ok for valid leap-second date"
        );
    }

    #[test]
    fn rounding_truncate_standard_digits() {
        // .123Z at 0 digits truncate → no subsecond part (no carry since .123 < .5)
        let t = Time::<UTC>::parse_rfc3339("2000-01-01T12:34:56.123Z").unwrap();
        let opts = FormatOptions {
            subsecond_digits: 0,
            precision: FormatPrecision::Truncate,
            include_zulu: true,
        };
        let s = t.format_rfc3339(opts);
        assert_eq!(s, "2000-01-01T12:34:56Z");
    }

    #[test]
    fn rounding_carry_into_next_second() {
        // .999Z with 0 digits and RoundHalfToEven should carry into next second.
        let t = Time::<UTC>::parse_rfc3339("2000-01-01T12:34:56.999Z").unwrap();
        let opts = FormatOptions {
            subsecond_digits: 0,
            precision: FormatPrecision::RoundHalfToEven,
            include_zulu: true,
        };
        let s = t.format_rfc3339(opts);
        // .999 rounds to 1.0 → carry → 12:34:57
        assert_eq!(s, "2000-01-01T12:34:57Z", "got {s}");
    }

    #[test]
    fn rounding_half_even_milliseconds() {
        // 500.000000 ms at 3 digits, RoundHalfToEven:
        // truncated = 500, remainder = 0 (no tie) → stays .500.
        let t = Time::<UTC>::parse_rfc3339("2000-01-01T12:00:00.500000000Z").unwrap();
        let opts = FormatOptions {
            subsecond_digits: 3,
            precision: FormatPrecision::RoundHalfToEven,
            include_zulu: true,
        };
        let s = t.format_rfc3339(opts);
        assert_eq!(s, "2000-01-01T12:00:00.500Z", "got {s}");
    }

    #[test]
    fn round_subsecond_helper_truncate() {
        assert_eq!(
            round_subsecond(999_999_999, 3, FormatPrecision::Truncate),
            (999, false)
        );
        assert_eq!(
            round_subsecond(500_000_000, 3, FormatPrecision::Truncate),
            (500, false)
        );
        assert_eq!(round_subsecond(0, 0, FormatPrecision::Truncate), (0, false));
    }

    #[test]
    fn round_subsecond_helper_carry() {
        // 999_999_999 at 0 digits with RoundHalfToEven: rounds to 1 (carry).
        let (v, carry) = round_subsecond(999_999_999, 0, FormatPrecision::RoundHalfToEven);
        assert!(carry, "expected carry for 999_999_999 at 0 digits");
        assert_eq!(v, 0);
    }

    #[test]
    fn round_subsecond_helper_half_even() {
        // Exact half: 500_000_000 at 0 digits. truncated = 0 (even) → no round up.
        let (v, carry) = round_subsecond(500_000_000, 0, FormatPrecision::RoundHalfToEven);
        assert!(
            !carry,
            "500_000_000 half-to-even at 0 digits: 0 is even, no carry"
        );
        assert_eq!(v, 0);
        // 1_500_000_000 / 1e9 is not possible but at 9 digits identity is returned.
        let (v9, carry9) = round_subsecond(999_999_999, 9, FormatPrecision::RoundHalfToEven);
        assert!(!carry9);
        assert_eq!(v9, 999_999_999);
    }
}
