// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use chrono::{DateTime, Utc};
use tempoch_core::format::iso::{parse_rfc3339_utc, parse_rfc3339_utc_with};
use tempoch_core::format::{JulianDate, ModifiedJulianDate, UnixTime};
use tempoch_core::qtty::{Day, Second};
use tempoch_core::{ConversionError, Time, TimeContext, UTC};

#[test]
fn all_civil_formats_reject_superseded_historical_labels() {
    let ctx = TimeContext::new();
    for label in [
        "1961-07-31T23:59:59.975Z",
        "1968-01-31T23:59:59.950Z",
        "1968-01-31T23:59:59.999Z",
    ] {
        let dt = label.parse::<DateTime<Utc>>().unwrap();
        let unix = dt.timestamp() as f64 + dt.timestamp_subsec_nanos() as f64 / 1e9;
        assert_eq!(
            Time::<UTC>::try_from_chrono_with(dt, &ctx),
            Err(ConversionError::UtcHistoryUnsupported)
        );
        assert_eq!(
            UnixTime::try_new_with(Second::new(unix), &ctx),
            Err(ConversionError::UtcHistoryUnsupported)
        );
        assert_eq!(
            ModifiedJulianDate::<UTC>::try_new_with(Day::new(40_587.0 + unix / 86_400.0), &ctx),
            Err(ConversionError::UtcHistoryUnsupported)
        );
        assert_eq!(
            JulianDate::<UTC>::try_new_with(Day::new(2_440_587.5 + unix / 86_400.0), &ctx),
            Err(ConversionError::UtcHistoryUnsupported)
        );
        assert_eq!(
            Time::<UTC>::parse_rfc3339_with(label, &ctx),
            Err(ConversionError::UtcHistoryUnsupported)
        );
    }
}

#[test]
fn all_rfc3339_entry_points_reject_unicode_without_panicking() {
    let ctx = TimeContext::new();
    for input in [
        "2024-06-15T12:34:☃Z",
        "2024-06-15T12:34:56☃Z",
        "☃024-06-15T12:34:56Z",
        "2024-☃6-15T12:34:56Z",
        "2024-06-15T12:34:56.☃Z",
    ] {
        let results = std::panic::catch_unwind(|| {
            [
                parse_rfc3339_utc(input),
                parse_rfc3339_utc_with(input, &ctx),
                Time::<UTC>::parse_rfc3339(input),
                Time::<UTC>::parse_rfc3339_with(input, &ctx),
            ]
        })
        .unwrap_or_else(|_| panic!("parser panicked for {input:?}"));
        assert!(results.iter().all(Result::is_err), "{input:?}");
    }
}

// Spacing of the source representation, used to derive rounding bounds.
fn ulp(value: f64) -> f64 {
    let magnitude = value.abs();
    f64::from_bits(magnitude.to_bits() + 1) - magnitude
}

#[test]
fn valid_civil_formats_agree_within_their_input_precision() {
    let ctx = TimeContext::new();
    for label in [
        "1961-01-01T00:00:00Z",
        "1965-07-01T00:00:00Z",
        "1968-02-01T00:00:00.050Z",
        "1970-01-01T00:00:00Z",
        "1971-12-31T23:59:59Z",
        "1972-01-01T00:00:00Z",
        "2016-12-31T23:59:59Z",
        "2017-01-01T00:00:00Z",
        "2024-06-15T12:34:56Z",
        "2024-06-15T12:34:56.123456789Z",
        "1961-07-31T23:59:59Z",
        "1961-08-01T00:00:00Z",
        "1961-08-01T00:00:01Z",
        "1965-06-30T23:59:59Z",
        "1965-07-01T00:00:01Z",
        "1968-01-31T23:59:59Z",
        "1968-02-01T00:00:00Z",
        "1968-02-01T00:00:01Z",
    ] {
        let dt = label.parse::<DateTime<Utc>>().unwrap();
        let expected = Time::<UTC>::try_from_chrono_with(dt, &ctx).unwrap();
        let fraction = dt.timestamp_subsec_nanos() as f64 / 1e9;
        let unix = dt.timestamp() as f64 + fraction;
        let days = unix / 86_400.0;
        let mjd = 40_587.0 + days;
        let jd = 2_440_587.5 + days;
        // Half an ULP per rounded input-encoding operation, plus 1 ns for
        // the engine's nanosecond guarantee and diff_exact's projection.
        let unix_error = 0.5 * ulp(unix) + 0.5 * ulp(fraction);
        let day_division_error = 0.5 * ulp(days) * 86_400.0;
        for (actual, seconds_error) in [
            (
                UnixTime::try_new_with(Second::new(unix), &ctx)
                    .unwrap()
                    .to_j2000s(),
                unix_error,
            ),
            (
                ModifiedJulianDate::<UTC>::try_new_with(Day::new(mjd), &ctx)
                    .unwrap()
                    .to_j2000s(),
                unix_error
                    + day_division_error
                    + 0.5 * ulp(mjd) * 86_400.0
                    + 0.5 * ulp((mjd - 40_587.0) * 86_400.0),
            ),
            (
                JulianDate::<UTC>::try_new_with(Day::new(jd), &ctx)
                    .unwrap()
                    .to_j2000s(),
                unix_error
                    + day_division_error
                    + 0.5 * ulp(jd) * 86_400.0
                    + 0.5 * ulp((jd - 2_440_587.5) * 86_400.0),
            ),
        ] {
            let bound_ns = (seconds_error * 1e9).ceil() as i128 + 1;
            assert!(
                actual.diff_exact(expected).unwrap().as_nanos_i128().abs() <= bound_ns,
                "{label}: {bound_ns} ns bound"
            );
            assert!(!actual.is_leap_second_with(&ctx));
        }
    }
}

#[test]
fn scalar_unix_roundtrips_without_chrono_range_or_nanosecond_quantization() {
    let ctx = TimeContext::new();
    for seconds in [
        0.0,
        0.125,
        -0.125,
        1.25,
        -1.25,
        1e15 + 0.25,
        1_718_454_896.1234567,
    ] {
        let time = UnixTime::try_new_with(Second::new(seconds), &ctx).unwrap();
        let back = time.try_raw_with(&ctx).unwrap().value();
        assert_eq!(back, seconds, "{seconds}: error {} s", back - seconds);
    }
}

#[test]
fn scalar_formats_share_nonfinite_and_predefinition_policy() {
    let ctx = TimeContext::new();
    for scalar in [f64::NAN, f64::NEG_INFINITY, f64::INFINITY] {
        assert_eq!(
            UnixTime::try_new_with(Second::new(scalar), &ctx),
            Err(ConversionError::NonFinite)
        );
        assert_eq!(
            JulianDate::<UTC>::try_new_with(Day::new(scalar), &ctx),
            Err(ConversionError::NonFinite)
        );
        assert_eq!(
            ModifiedJulianDate::<UTC>::try_new_with(Day::new(scalar), &ctx),
            Err(ConversionError::NonFinite)
        );
    }
    let old = "1960-12-31T23:59:59Z".parse::<DateTime<Utc>>().unwrap();
    let unix = old.timestamp() as f64;
    assert_eq!(
        UnixTime::try_new_with(Second::new(unix), &ctx),
        Err(ConversionError::UtcBeforeDefinition)
    );
    assert_eq!(
        ModifiedJulianDate::<UTC>::try_new_with(Day::new(40_587.0 + unix / 86_400.0), &ctx),
        Err(ConversionError::UtcBeforeDefinition)
    );
    assert_eq!(
        JulianDate::<UTC>::try_new_with(Day::new(2_440_587.5 + unix / 86_400.0), &ctx),
        Err(ConversionError::UtcBeforeDefinition)
    );
    let permissive = ctx.allow_pre_definition_utc();
    let expected = Time::<UTC>::try_from_chrono_with(old, &permissive).unwrap();
    let actual = UnixTime::try_new_with(Second::new(unix), &permissive).unwrap();
    assert_eq!(actual.to_j2000s(), expected);
}

#[test]
fn malformed_ascii_and_unicode_at_every_position_are_rejected() {
    let ctx = TimeContext::new();
    for input in [
        "",
        "2024",
        "2024-06-15T12:34:",
        "2024/06-15T12:34:56Z",
        "2024-xx-15T12:34:56Z",
        "2024-06-15T12:34:56+25:00",
        "2024-06-15T12:34:56Zjunk",
        "2024-06-15T12:34:56.1234567890Z",
        "2024-06-15T12:34:56.Z",
        "2016-12-30T23:59:60Z",
        "1971-12-31T23:59:60Z",
    ] {
        assert!(
            Time::<UTC>::parse_rfc3339_with(input, &ctx).is_err(),
            "{input:?}"
        );
    }
    let valid = "2024-06-15T12:34:56.123456789Z";
    for index in 0..valid.len() {
        for unicode in ['☃', 'é', '🕰', '\0'] {
            let mut invalid = valid.to_owned();
            invalid.replace_range(index..index + 1, &unicode.to_string());
            assert!(
                Time::<UTC>::parse_rfc3339_with(&invalid, &ctx).is_err(),
                "{invalid:?}"
            );
        }
    }
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: 512,
        failure_persistence: None,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x5554_4638),
        ..proptest::test_runner::Config::default()
    })]
    #[test]
    fn arbitrary_utf8_is_safe(input in proptest::collection::vec(proptest::prelude::any::<char>(), 0..160)) {
        let input: String = input.into_iter().collect();
        let ctx = TimeContext::new();
        let result = parse_rfc3339_utc(&input);
        proptest::prop_assert_eq!(parse_rfc3339_utc_with(&input, &ctx), result);
        proptest::prop_assert_eq!(Time::<UTC>::parse_rfc3339(&input), result);
        proptest::prop_assert_eq!(Time::<UTC>::parse_rfc3339_with(&input, &ctx), result);
        if !input.is_ascii() { proptest::prop_assert!(result.is_err()); }
    }
}

#[test]
fn subnanosecond_unix_precision_is_bounded_by_split_storage() {
    let ctx = TimeContext::new();
    let source = 1e-10; // 0.1 ns is finer than chrono's civil representation.
    let time = UnixTime::try_new_with(Second::new(source), &ctx).unwrap();
    let back = time.try_raw_with(&ctx).unwrap().value();
    let epoch = UnixTime::try_new_with(Second::new(0.0), &ctx).unwrap();
    let offset = ctx.tai_minus_utc(epoch).unwrap().value();
    // The offset's residual after normalization against the large J2000
    // origin sets the low word's grid. Two residual additions each round by
    // at most half an ULP. This is a storage bound, not a source-f64 guarantee.
    let residual = offset - ((-946_728_000.0 + offset) + 946_728_000.0);
    let bound = ulp(residual) + ulp(source);
    assert!(back > 0.0); // The sub-nanosecond fraction was not quantized away.
    assert!(
        (back - source).abs() <= bound,
        "error {} s, bound {bound} s",
        back - source
    );
}

#[test]
fn gaps_and_leaps_are_not_scalar_civil_labels() {
    use tempoch_core::format::{Unix, JD, MJD};
    use tempoch_core::ExactDuration;
    let ctx = TimeContext::new();
    let midnight = Time::<UTC>::parse_rfc3339_with("1972-01-01T00:00:00Z", &ctx).unwrap();
    let gap = midnight.sub_exact(ExactDuration::from_nanos(50_000_000));
    let leap = Time::<UTC>::parse_rfc3339_with("2016-12-31T23:59:60.123456789Z", &ctx).unwrap();
    for (time, error) in [
        (gap, ConversionError::UtcHistoryUnsupported),
        (leap, ConversionError::InvalidLeapSecond),
    ] {
        assert_eq!(time.to_with::<Unix>(&ctx), Err(error));
        assert_eq!(time.to_with::<JD>(&ctx), Err(error));
        assert_eq!(time.to_with::<MJD>(&ctx), Err(error));
    }
}

#[test]
fn adjacent_scalar_values_select_the_correct_segment_and_are_monotone() {
    use tempoch_core::format::{FormatForScale, TimeFormat, Unix, JD, MJD};
    use tempoch_core::qtty::Quantity;
    fn check<F: TimeFormat + FormatForScale<UTC>>(
        raw: f64,
        before_error: Option<ConversionError>,
        ctx: &TimeContext,
    ) {
        let bits = raw.to_bits();
        let before = f64::from_bits(if raw > 0.0 { bits - 1 } else { bits + 1 });
        let after = f64::from_bits(if raw > 0.0 { bits + 1 } else { bits - 1 });
        let current = Time::<UTC, F>::try_new_with(Quantity::new(raw), ctx).unwrap();
        let next = Time::<UTC, F>::try_new_with(Quantity::new(after), ctx).unwrap();
        assert!(next > current);
        assert!(!next.is_leap_second_with(ctx));
        let previous = Time::<UTC, F>::try_new_with(Quantity::new(before), ctx);
        if let Some(error) = before_error {
            assert_eq!(previous, Err(error));
        } else {
            let previous = previous.unwrap();
            assert!(current > previous);
            assert!(!previous.is_leap_second_with(ctx));
        }
    }
    let ctx = TimeContext::new();
    for (label, before_error) in [
        (
            "1961-01-01T00:00:00Z",
            Some(ConversionError::UtcBeforeDefinition),
        ),
        (
            "1961-08-01T00:00:00Z",
            Some(ConversionError::UtcHistoryUnsupported),
        ),
        ("1965-07-01T00:00:00Z", None),
        (
            "1968-02-01T00:00:00Z",
            Some(ConversionError::UtcHistoryUnsupported),
        ),
        ("1972-01-01T00:00:00Z", None),
        ("2017-01-01T00:00:00Z", None),
    ] {
        let unix = label.parse::<DateTime<Utc>>().unwrap().timestamp() as f64;
        check::<Unix>(unix, before_error, &ctx);
        check::<MJD>(40_587.0 + unix / 86_400.0, before_error, &ctx);
        check::<JD>(2_440_587.5 + unix / 86_400.0, before_error, &ctx);
    }
}

#[test]
fn chrono_parser_preserves_supported_ascii_forms_and_fraction_limits() {
    use tempoch_core::format::iso::FormatOptions;
    let ctx = TimeContext::new();
    for (input, normalized) in [
        (
            "2024-06-15t12:34:56.123456789z",
            "2024-06-15T12:34:56.123456789Z",
        ),
        (
            "2024-06-15T14:34:56.123456789+02:00",
            "2024-06-15T12:34:56.123456789Z",
        ),
        (
            "2017-01-01T00:59:60.123456789+01:00",
            "2016-12-31T23:59:60.123456789Z",
        ),
    ] {
        let time = Time::<UTC>::parse_rfc3339_with(input, &ctx).unwrap();
        assert_eq!(
            time.try_format_rfc3339_with(FormatOptions::nanoseconds(), &ctx)
                .unwrap(),
            normalized
        );
    }
    for input in [
        "2024-06-15t12:34:56.1234567890z",
        "2024-06-15T12:34:56.1234567890+02:00",
    ] {
        assert_eq!(
            Time::<UTC>::parse_rfc3339_with(input, &ctx),
            Err(ConversionError::OutOfRange)
        );
    }
}
