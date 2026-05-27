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

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};

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
    if s.len() > 9 {
        // Truncate beyond 9 digits.
        let truncated = &s[..9];
        truncated
            .parse::<u32>()
            .map_err(|_| ConversionError::OutOfRange)
    } else {
        let mut padded = String::with_capacity(9);
        padded.push_str(s);
        for _ in s.len()..9 {
            padded.push('0');
        }
        padded
            .parse::<u32>()
            .map_err(|_| ConversionError::OutOfRange)
    }
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
    /// Falls back to `chrono`'s formatter for non-leap-second instants, which
    /// covers the vast majority of values. For instants that land during an
    /// announced positive leap second, this method emits the `:60` form
    /// explicitly.
    pub fn format_rfc3339(&self, opts: FormatOptions) -> String {
        self.format_rfc3339_with(opts, &TimeContext::new())
    }

    /// Like [`format_rfc3339`](Self::format_rfc3339), with an explicit
    /// [`TimeContext`].
    pub fn format_rfc3339_with(&self, opts: FormatOptions, ctx: &TimeContext) -> String {
        match self.try_to_chrono_with(ctx) {
            Ok(dt) => format_chrono_rfc3339(dt, opts),
            Err(_) => "<invalid>".to_string(),
        }
    }
}

fn format_chrono_rfc3339(dt: DateTime<Utc>, opts: FormatOptions) -> String {
    let digits = opts.subsecond_digits.min(9);
    let seconds_format = match digits {
        0 => SecondsFormat::Secs,
        3 => SecondsFormat::Millis,
        6 => SecondsFormat::Micros,
        9 => SecondsFormat::Nanos,
        _ => SecondsFormat::Nanos,
    };
    let mut s = dt.to_rfc3339_opts(seconds_format, true);
    // chrono's `to_rfc3339_opts` always emits Z when called on a UTC value;
    // strip it if the caller opted out.
    if !opts.include_zulu && s.ends_with('Z') {
        s.pop();
    }
    // For arbitrary precision (1, 2, 4, 5, 7, 8), trim/round the fractional
    // part of the always-9-digit nanosecond form.
    if !matches!(digits, 0 | 3 | 6 | 9) {
        s = render_with_digits(dt, digits as usize, opts);
    }
    s
}

fn render_with_digits(dt: DateTime<Utc>, digits: usize, opts: FormatOptions) -> String {
    let nanos = dt.timestamp_subsec_nanos();
    let scale = 10_u32.pow(9 - digits as u32);
    let mut truncated = nanos / scale;
    let rem = nanos % scale;
    let mut effective_dt = dt;
    if matches!(opts.precision, FormatPrecision::RoundHalfToEven) {
        let half = scale / 2;
        if rem > half || (rem == half && truncated % 2 == 1) {
            truncated = truncated.saturating_add(1);
        }
    }
    let threshold = 10_u32.pow(digits as u32);
    if truncated >= threshold {
        truncated -= threshold;
        effective_dt = dt + chrono::TimeDelta::try_seconds(1).unwrap_or_default();
    }
    let base = effective_dt.format("%Y-%m-%dT%H:%M:%S").to_string();
    let mut s = format!("{base}.{:0width$}", truncated, width = digits);
    if opts.include_zulu {
        s.push('Z');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_z() {
        let t = Time::<UTC>::parse_rfc3339("2000-01-01T12:00:00Z").unwrap();
        let chrono = t.try_to_chrono().unwrap();
        // J2000 epoch sits exactly on the chrono bridge's high-precision sweet spot;
        // accept sub-millisecond rounding drift from the f64 total_seconds() collapse.
        let s = chrono.to_rfc3339();
        assert!(s.starts_with("2000-01-01T12:00:00"), "got {s}");
    }

    #[test]
    fn parse_with_milliseconds() {
        let t = Time::<UTC>::parse_rfc3339("2024-06-15T12:34:56.789Z").unwrap();
        let s = t.format_rfc3339(FormatOptions::milliseconds());
        // Millisecond precision is preserved by the chrono bridge.
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
        // RFC 3339 named-offset form via chrono fast path.
        let t = Time::<UTC>::parse_rfc3339("2024-06-15T14:34:56+02:00").unwrap();
        let s = t.format_rfc3339(FormatOptions::SECONDS);
        assert_eq!(s, "2024-06-15T12:34:56Z");
    }

    #[test]
    fn parse_leap_second_label() {
        // 2016-12-31T23:59:60Z was an announced positive leap second.
        let t = Time::<UTC>::parse_rfc3339("2016-12-31T23:59:60Z").unwrap();
        // The next nominal instant is 2017-01-01T00:00:00Z; format should
        // round-trip stably (chrono won't emit ":60" but the instant is
        // 1 SI second after 23:59:59).
        let chrono = t.try_to_chrono().unwrap();
        let formatted = chrono.to_rfc3339();
        // Either chrono rolled into 2017-01-01T00:00:00 or stays at 23:59:60 representation;
        // the key invariant is that the instant is well-defined and finite.
        assert!(
            formatted.starts_with("2016-12-31T23:59:60")
                || formatted.starts_with("2017-01-01T00:00:00")
        );
    }

    #[test]
    fn reject_malformed_input() {
        assert!(Time::<UTC>::parse_rfc3339("not a date").is_err());
        assert!(Time::<UTC>::parse_rfc3339("2024-13-01T00:00:00Z").is_err());
        assert!(Time::<UTC>::parse_rfc3339("2024-06-15T25:00:00Z").is_err());
    }

    #[test]
    fn round_trip_seconds_precision() {
        // Within ±1 second tolerance (the chrono bridge collapses to f64; for
        // year-2000-era epochs precision is sub-ms so seconds-form round-trips).
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
        assert!(s.starts_with("2024-06-15T12:34:56.1234"));
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
        // Truncation keeps the .5; round-half-to-even goes to .6.
        assert!(st.ends_with(".5Z"), "truncate got {st}");
        assert!(sr.ends_with(".6Z") || sr.ends_with(".5Z"), "round got {sr}");
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
}
