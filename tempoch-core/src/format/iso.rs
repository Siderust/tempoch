// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! ISO 8601 / RFC 3339 / RFC 2822 parsing and formatting for `Time<UTC>`
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

use chrono::{DateTime, NaiveDateTime, Utc};

use crate::data::runtime_data::time_data_tai_seconds_is_in_leap_window;
use crate::earth::context::TimeContext;
use crate::foundation::error::ConversionError;
use crate::model::scale::UTC;
use crate::model::time::Time;

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
    /// clamped to 9 because tempoch's exact storage is 1 ns.
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
    // Pre-validate: reject more than 9 fractional digits before passing to chrono.
    if let Some(after_dot) = s.find('.') {
        // Find the end of the fractional part (Z, +, or -)
        let frac_start = after_dot + 1;
        if let Some(zone_pos) = s[frac_start..].find(['Z', '+', '-']) {
            let frac_len = zone_pos;
            if frac_len == 0 {
                return Err(ConversionError::OutOfRange);
            }
            if frac_len > 9 {
                return Err(ConversionError::OutOfRange);
            }
        }
    }

    // Try `chrono::DateTime::parse_from_rfc3339` first; it accepts a wide range of valid forms.
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        let utc = dt.with_timezone(&Utc);
        return Time::<UTC>::try_from_chrono_with(utc, ctx);
    }

    // chrono rejects ":60" in the seconds field except via try_parse paths;
    // fall back to a manual leap-second-aware parser for the standard form
    //   YYYY-MM-DDTHH:MM:SS[.fraction](Z|±HH:MM)
    parse_rfc3339_manual(s, ctx)
}

fn parse_rfc3339_manual(s: &str, ctx: &TimeContext) -> Result<Time<UTC>, ConversionError> {
    // Minimum length: "YYYY-MM-DDTHH:MM:SSZ" = 20 chars.
    if s.len() < 20 {
        return Err(ConversionError::OutOfRange);
    }
    let bytes = s.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || (bytes[10] != b'T' && bytes[10] != b' ')
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return Err(ConversionError::OutOfRange);
    }
    let year: i32 = s[..4].parse().map_err(|_| ConversionError::OutOfRange)?;
    let month: u32 = s[5..7].parse().map_err(|_| ConversionError::OutOfRange)?;
    let day: u32 = s[8..10].parse().map_err(|_| ConversionError::OutOfRange)?;
    let hour: u32 = s[11..13].parse().map_err(|_| ConversionError::OutOfRange)?;
    let minute: u32 = s[14..16].parse().map_err(|_| ConversionError::OutOfRange)?;
    let second_str = &s[17..19];
    let second: u32 = second_str
        .parse()
        .map_err(|_| ConversionError::OutOfRange)?;

    // Trailing portion may be: [.fraction][Z|±HH:MM]
    let tail = &s[19..];
    let (frac_str, zone_str) = split_fraction_and_zone(tail)?;
    let frac_nanos = parse_fraction_nanos(frac_str)?;

    if zone_str != "Z" {
        // Only Z is supported in this path (the chrono fast path covers the
        // general timezone case).
        return Err(ConversionError::OutOfRange);
    }

    // Handle leap-second labelling: second == 60 must occur during an
    // announced positive leap second on this UTC date.
    if second == 60 {
        // Construct the instant at HH:59:59.999999999 and add 1s−frac.
        let base = NaiveDateTime::parse_from_str(
            &format!("{year:04}-{month:02}-{day:02}T{hour:02}:59:59.999999999"),
            "%Y-%m-%dT%H:%M:%S%.9f",
        )
        .map_err(|_| ConversionError::OutOfRange)?
        .and_utc();
        let utc_almost = Time::<UTC>::try_from_chrono_with(base, ctx)?;
        // 1ns nudge → instant equivalent to HH:60:00, leap-second second; then add `frac_nanos` ns.
        let shifted =
            utc_almost.add_exact(crate::ExactDuration::from_nanos(1 + frac_nanos as i128));
        // Validate that this date/time actually had an announced positive leap second.
        if !time_data_tai_seconds_is_in_leap_window(
            ctx.time_data(),
            shifted.to_j2000s().total_seconds(),
        ) {
            return Err(ConversionError::InvalidLeapSecond);
        }
        return Ok(shifted);
    }

    if second >= 60 || minute >= 60 || hour >= 24 {
        return Err(ConversionError::OutOfRange);
    }

    let naive_str = if frac_nanos == 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
    } else {
        format!(
            "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{:09}",
            frac_nanos
        )
    };
    let parsed = if frac_nanos == 0 {
        NaiveDateTime::parse_from_str(&naive_str, "%Y-%m-%dT%H:%M:%S")
    } else {
        NaiveDateTime::parse_from_str(&naive_str, "%Y-%m-%dT%H:%M:%S%.9f")
    }
    .map_err(|_| ConversionError::OutOfRange)?
    .and_utc();
    Time::<UTC>::try_from_chrono_with(parsed, ctx)
}

fn split_fraction_and_zone(tail: &str) -> Result<(&str, &str), ConversionError> {
    if let Some(stripped) = tail.strip_prefix('.') {
        // fraction up to the timezone delimiter (Z, +, -)
        let zone_pos = stripped
            .find(['Z', '+', '-'])
            .ok_or(ConversionError::OutOfRange)?;
        let frac = &stripped[..zone_pos];
        // Reject empty fraction: "2024-06-15T12:34:56.Z" is not valid RFC 3339.
        if frac.is_empty() {
            return Err(ConversionError::OutOfRange);
        }
        let zone = &stripped[zone_pos..];
        Ok((frac, zone))
    } else {
        Ok(("", tail))
    }
}

fn parse_fraction_nanos(s: &str) -> Result<u32, ConversionError> {
    if s.is_empty() {
        return Ok(0);
    }
    // Reject more than 9 fractional digits; tempoch's resolution is 1 ns.
    if s.len() > 9 {
        return Err(ConversionError::OutOfRange);
    }
    let mut padded = [b'0'; 9];
    padded[..s.len()].copy_from_slice(s.as_bytes());
    core::str::from_utf8(&padded)
        .ok()
        .and_then(|p| p.parse::<u32>().ok())
        .ok_or(ConversionError::OutOfRange)
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
        // Use the explicit table/context-driven check as the authoritative source
        // for leap-second detection. This is independent of how the chrono bridge
        // internally represents subsecond nanoseconds.
        let is_leap = self.is_leap_second_with(ctx);
        let dt = self.try_to_chrono_with(ctx)?;
        Ok(format_utc_datetime_rfc3339(dt, is_leap, opts))
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

/// Format a `DateTime<Utc>` as RFC 3339, applying uniform rounding and
/// emitting `23:59:60` for leap-second instants.
///
/// The `is_leap` flag is the authoritative signal: it must be supplied by the
/// caller via `Time<UTC>::is_leap_second_with(ctx)`, which consults the compiled
/// UTC–TAI table. The chrono subsecond-nanos value (≥ 1 × 10⁹) is used only to
/// extract the fractional position within the leap second when `is_leap` is true.
fn format_utc_datetime_rfc3339(dt: DateTime<Utc>, is_leap: bool, opts: FormatOptions) -> String {
    let digits = opts.subsecond_digits.min(9) as usize;
    let raw_nanos = dt.timestamp_subsec_nanos();

    if is_leap {
        // Fractional part within the leap second (0..999_999_999).
        // chrono represents leap-second instants with subsecond_nanos ≥ 1_000_000_000;
        // if for some reason it does not, clamp to 0 rather than underflowing.
        let leap_nanos = raw_nanos.saturating_sub(1_000_000_000);
        let (frac, carry) = round_subsecond(leap_nanos, digits, opts.precision);
        if carry {
            // Rounding caused the leap second itself to overflow into next second
            // (i.e. 23:59:60.999999500 rounded up to 23:59:61 → 2017-01-01T00:00:00).
            let next = dt + chrono::TimeDelta::try_seconds(1).unwrap_or_default();
            return format_normal_dt(next, 0, digits, opts);
        }
        let date = dt.format("%Y-%m-%d");
        if digits == 0 {
            let zulu = if opts.include_zulu { "Z" } else { "" };
            format!("{date}T23:59:60{zulu}")
        } else {
            let zulu = if opts.include_zulu { "Z" } else { "" };
            format!("{date}T23:59:60.{:0width$}{zulu}", frac, width = digits)
        }
    } else {
        let (frac, carry) = round_subsecond(raw_nanos, digits, opts.precision);
        let effective_dt = if carry {
            dt + chrono::TimeDelta::try_seconds(1).unwrap_or_default()
        } else {
            dt
        };
        format_normal_dt(effective_dt, frac, digits, opts)
    }
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
    fn parse_with_microseconds_within_chrono_bridge_precision() {
        // The chrono bridge collapses split storage to f64 J2000 seconds (~150 ns
        // precision near 2024). Test that microsecond round-trip is within
        // single-digit microseconds, which is the documented bridge tolerance.
        let t = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56.123456Z").unwrap();
        let s = t.format_rfc3339(FormatOptions::microseconds());
        assert!(s.starts_with("2024-06-15T12:34:56.1234"), "got {s}");
    }

    #[test]
    fn parse_with_nanoseconds_within_chrono_bridge_precision() {
        // Same bridge precision caveat as above; nanosecond digits will drift by
        // ~150 ns near 2024. The format itself supports 9 digits.
        let t = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56.123456789Z").unwrap();
        let s = t.format_rfc3339(FormatOptions::nanoseconds());
        assert!(s.starts_with("2024-06-15T12:34:56.1234"), "got {s}");
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
        // The chrono bridge has ~150 ns precision near 2016; test at millisecond level.
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
        // 4-digit subsecond resolution survives the chrono-bridge drift (~150 ns).
        assert!(s.starts_with("2024-06-15T12:34:56.1234"), "got {s}");
    }

    #[test]
    fn truncate_vs_round_differs_on_5() {
        // Use a year-2000 epoch where chrono-bridge precision is sub-ms.
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
        // Use a simple value well within chrono-bridge precision (J2000 era).
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
        // Using exactly 500ms avoids a bridge-precision boundary: ±150 ns near J2000
        // cannot shift a 0-remainder to the tie point (500_000 out of 1_000_000).
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
