// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use chrono::{DateTime, Utc};
use tempoch_core::format::iso::{FormatOptions, FormatPrecision};
use tempoch_core::{ConversionError, ExactDuration, Time, TimeContext, TAI, UTC};

#[test]
fn historical_nanosecond_regressions() {
    let ctx = TimeContext::new();
    for label in [
        "1961-01-01T00:00:00Z",
        "1965-07-01T00:00:00.123456789Z",
        "1968-02-01T00:00:00.123456789Z",
        "1970-01-01T00:00:00Z",
        "1970-01-01T00:00:00.000000001Z",
        "1971-12-30T23:59:59.123456789Z",
        "1971-12-31T23:59:59.999999999Z",
    ] {
        let dt = label.parse::<DateTime<Utc>>().unwrap();
        let utc = Time::<UTC>::try_from_chrono_with(dt, &ctx).unwrap();
        let back = utc
            .to::<TAI>()
            .to::<UTC>()
            .try_to_chrono_with(&ctx)
            .unwrap();
        assert_eq!(
            back,
            dt,
            "{label}: error {:?}",
            back.signed_duration_since(dt)
        );
    }
}

#[test]
fn historical_gap_is_not_a_leap_second() {
    let ctx = TimeContext::new();
    let midnight = Time::<UTC>::parse_rfc3339_with("1972-01-01T00:00:00Z", &ctx).unwrap();
    let gap = midnight.sub_exact(ExactDuration::from_nanos(50_000_000));
    assert!(!gap.is_leap_second_with(&ctx));
    assert_eq!(
        gap.try_to_chrono_with(&ctx),
        Err(ConversionError::UtcHistoryUnsupported)
    );
    assert_eq!(
        gap.try_format_rfc3339_with(FormatOptions::nanoseconds(), &ctx),
        Err(ConversionError::UtcHistoryUnsupported)
    );
    for label in [
        "1971-12-31T23:59:60Z",
        "1965-06-30T23:59:60Z",
        "2023-06-15T23:59:60Z",
        "2016-12-31T12:59:60Z",
    ] {
        let dt = label.parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            Time::<UTC>::try_from_chrono_with(dt, &ctx),
            Err(ConversionError::InvalidLeapSecond)
        );
        assert_eq!(
            Time::<UTC>::parse_rfc3339_with(label, &ctx),
            Err(ConversionError::InvalidLeapSecond)
        );
    }
}

#[test]
fn exact_iso_and_chrono_round_trips() {
    let ctx = TimeContext::new();
    for base in [
        "1965-07-01T00:00:00",
        "1970-01-01T00:00:00",
        "1999-12-31T23:59:59",
        "2000-01-01T12:00:00",
        "2024-06-15T12:34:56",
        "2024-06-30T23:59:59",
        "2024-07-01T00:00:00",
        "2016-12-31T23:59:59",
        "2016-12-31T23:59:60",
        "2017-01-01T00:00:00",
        "1972-06-30T23:59:60",
    ] {
        for nanos in [
            0,
            1,
            2,
            10,
            999,
            1_000,
            123_456_789,
            999_999_998,
            999_999_999,
        ] {
            let label = format!("{base}.{nanos:09}Z");
            let dt = label.parse::<DateTime<Utc>>().unwrap();
            let utc = Time::<UTC>::parse_rfc3339_with(&label, &ctx).unwrap();
            assert_eq!(
                utc.try_to_chrono_with(&ctx)
                    .unwrap_or_else(|e| panic!("{label}: {e:?}")),
                dt,
                "{label}"
            );
            assert_eq!(
                utc.is_leap_second_with(&ctx),
                dt.timestamp_subsec_nanos() >= 1_000_000_000
            );
            assert_eq!(
                utc.try_format_rfc3339_with(FormatOptions::nanoseconds(), &ctx)
                    .unwrap(),
                label
            );
        }
    }
}

#[test]
fn rounding_across_leap_boundaries() {
    let ctx = TimeContext::new();
    for (input, rounded, truncated) in [
        (
            "2016-12-31T23:59:59.999999999Z",
            "2016-12-31T23:59:60.000Z",
            "2016-12-31T23:59:59.999Z",
        ),
        (
            "2016-12-31T23:59:60.999999999Z",
            "2017-01-01T00:00:00.000Z",
            "2016-12-31T23:59:60.999Z",
        ),
        (
            "2024-12-31T23:59:59.999999999Z",
            "2025-01-01T00:00:00.000Z",
            "2024-12-31T23:59:59.999Z",
        ),
    ] {
        let utc = Time::<UTC>::parse_rfc3339_with(input, &ctx).unwrap();
        assert_eq!(
            utc.try_format_rfc3339_with(FormatOptions::milliseconds(), &ctx)
                .unwrap(),
            rounded
        );
        let opts = FormatOptions {
            precision: FormatPrecision::Truncate,
            ..FormatOptions::milliseconds()
        };
        assert_eq!(utc.try_format_rfc3339_with(opts, &ctx).unwrap(), truncated);
    }
}

#[test]
fn historical_negative_steps_have_an_explicit_canonical_label() {
    let ctx = TimeContext::new();
    // At 1968-02-01 the offset decreases by 0.1 s: the last ~0.1
    // UTC seconds of January share atomic instants with February.
    let earlier = "1968-01-31T23:59:59.950000000Z"
        .parse::<DateTime<Utc>>()
        .unwrap();
    assert_eq!(
        Time::<UTC>::try_from_chrono_with(earlier, &ctx),
        Err(ConversionError::UtcHistoryUnsupported)
    );
    let later = "1968-02-01T00:00:00.050000000Z"
        .parse::<DateTime<Utc>>()
        .unwrap();
    let utc = Time::<UTC>::try_from_chrono_with(later, &ctx).unwrap();
    assert_eq!(utc.try_to_chrono_with(&ctx).unwrap(), later);
    assert!(!utc.is_leap_second_with(&ctx));
}

#[test]
fn utc_definition_policy_is_preserved() {
    let ctx = TimeContext::new();
    let dt = "1960-12-31T23:59:59.999999999Z"
        .parse::<DateTime<Utc>>()
        .unwrap();
    assert_eq!(
        Time::<UTC>::try_from_chrono_with(dt, &ctx),
        Err(ConversionError::UtcBeforeDefinition)
    );
    let permissive = ctx.clone().allow_pre_definition_utc();
    let utc = Time::<UTC>::try_from_chrono_with(dt, &permissive).unwrap();
    assert_eq!(utc.try_to_chrono_with(&permissive).unwrap(), dt);
    assert_eq!(
        utc.try_to_chrono_with(&ctx),
        Err(ConversionError::UtcBeforeDefinition)
    );
    assert!(!utc.is_leap_second_with(&permissive));
}

#[test]
fn daily_sweep_1961_through_2030_is_exact_and_monotone() {
    let ctx = TimeContext::new();
    let first = "1961-01-01T00:00:00Z"
        .parse::<DateTime<Utc>>()
        .unwrap()
        .timestamp();
    let end = "2031-01-01T00:00:00Z"
        .parse::<DateTime<Utc>>()
        .unwrap()
        .timestamp();
    let mut previous = None;
    for midnight in (first..end).step_by(86_400) {
        for second in [0, 1, 43_199, 86_399] {
            for nanos in [0, 1, 123_456_789, 999_999_999] {
                let dt = DateTime::<Utc>::from_timestamp(midnight + second, nanos).unwrap();
                let result = Time::<UTC>::try_from_chrono_with(dt, &ctx);
                if nanos == 999_999_999
                    && second == 86_399
                    && ["1961-07-31", "1968-01-31"]
                        .contains(&dt.format("%Y-%m-%d").to_string().as_str())
                {
                    assert_eq!(result, Err(ConversionError::UtcHistoryUnsupported));
                    continue;
                }
                let utc = result.unwrap_or_else(|e| panic!("{dt}: {e:?}"));
                assert_eq!(utc.try_to_chrono_with(&ctx).unwrap(), dt, "{dt}");
                assert!(!utc.is_leap_second_with(&ctx));
                if let Some(prior) = previous {
                    assert!(utc > prior, "{dt}");
                }
                previous = Some(utc);
            }
        }
    }
}
