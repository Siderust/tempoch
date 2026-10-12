// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use tempoch_core::qtty::{Day, Second};
use tempoch_core::{
    ConversionError, ExactDuration, Interval, J2000s, ModifiedJulianDate, Period, Time,
    TimeContext, TimeSeries, Unix, JD, MJD, TAI, TT, UT1, UTC,
};

fn mjd(value: f64) -> Time<TT, MJD> {
    ModifiedJulianDate::<TT>::new(value)
}

#[test]
fn period_formats_are_typed_and_share_canonical_storage() {
    let p: Period<TT, MJD> = Period::try_new(mjd(60_000.0), mjd(60_001.0)).unwrap();
    let jd: Period<TT, JD> = p.to::<JD>();
    let seconds: Period<TT, J2000s> = jd.to::<J2000s>();
    let default: Period<TT> = seconds;
    assert_eq!(default.to::<MJD>(), p);
    assert_eq!(p.start.raw_seconds_pair(), jd.start.raw_seconds_pair());
    assert_eq!(p.end.raw_seconds_pair(), jd.end.raw_seconds_pair());
    assert_eq!(
        core::mem::size_of::<Period<TT, MJD>>(),
        core::mem::size_of::<Period<TT>>()
    );
    assert_eq!(
        core::mem::size_of::<Period<TT, MJD>>(),
        core::mem::size_of::<Interval<Time<TT, MJD>>>()
    );
    let _: Period<TAI, J2000s> = Period::new(Time::<TAI>::new(0.0), Time::<TAI>::new(10.0));
    let _: Period<UTC> = Period::new(Time::<UTC>::new(0.0), Time::<UTC>::new(10.0));
}

#[test]
fn format_roundtrip_preserves_compensated_low_word_and_interval_bounds() {
    let start = Time::<TT>::try_from_raw_j2000_seconds_split(
        Second::new(1_000_000_000.0),
        Second::new(1e-9),
    )
    .unwrap();
    let end = Time::<TT>::try_from_raw_j2000_seconds_split(
        Second::new(1_000_000_010.0),
        Second::new(2e-9),
    )
    .unwrap();
    let source = Period::<TT>::try_new(start, end).unwrap();
    let encoded: Period<TT, MJD> = source.to::<MJD>();
    let jd: Period<TT, JD> = encoded.to::<JD>();
    let back: Period<TT> = jd.to::<J2000s>();
    assert_eq!(back, source);
    assert_eq!(
        back.start.raw_seconds_pair(),
        source.start.raw_seconds_pair()
    );
    assert_eq!(back.end.raw_seconds_pair(), source.end.raw_seconds_pair());
    assert_eq!(back.length(), Second::new(10.0 + 1e-9));
}

#[test]
fn non_default_format_reuses_all_interval_algebra() {
    let outer = Period::<TT, MJD>::try_new(mjd(60_000.0), mjd(60_010.0)).unwrap();
    let a = Period::<TT, MJD>::try_new(mjd(60_001.0), mjd(60_004.0)).unwrap();
    let b = Period::<TT, MJD>::try_new(mjd(60_004.0), mjd(60_006.0)).unwrap();
    let c = Period::<TT, MJD>::try_new(mjd(60_005.0), mjd(60_008.0)).unwrap();

    assert!(a.intersection(&b).is_none());
    assert_eq!(
        b.intersection(&c),
        Some(Period::new(mjd(60_005.0), mjd(60_006.0)))
    );
    assert_eq!(a.union(&b), vec![Period::new(mjd(60_001.0), mjd(60_006.0))]);

    let covered = Period::normalize(&[c, b, a]);
    assert_eq!(covered, vec![Period::new(mjd(60_001.0), mjd(60_008.0))]);
    Period::validate(&covered).unwrap();
    assert_eq!(
        outer.try_complement(&covered).unwrap(),
        vec![
            Period::new(mjd(60_000.0), mjd(60_001.0)),
            Period::new(mjd(60_008.0), mjd(60_010.0)),
        ]
    );
    assert_eq!(
        Period::try_intersect_many(&[outer], &covered).unwrap(),
        covered
    );
    assert_eq!(outer.length(), Day::new(10.0));
    assert_eq!(outer.duration(), Day::new(10.0));
}

#[test]
fn non_default_format_time_series_matches_period_endpoints() {
    let start = Time::<TT>::new(0.0).to::<MJD>();
    let end = Time::<TT>::new(3.0).to::<MJD>();
    let p = Period::<TT, MJD>::new(start, end);
    let values: Vec<_> = TimeSeries::<TT, MJD>::new(p.start, p.end, ExactDuration::SECOND)
        .unwrap()
        .collect();
    assert_eq!(values.len(), 3);
    assert_eq!(values[0], p.start);
    assert_eq!(values[2], Time::<TT>::new(2.0).to::<MJD>());
}

#[test]
fn utc_civil_format_conversions_share_explicit_context() {
    let ctx = TimeContext::new();
    let start = Time::<UTC, MJD>::try_new_with(Day::new(57_754.25), &ctx).unwrap();
    let end = Time::<UTC, MJD>::try_new_with(Day::new(57_755.25), &ctx).unwrap();
    let p = Period::<UTC, MJD>::try_new(start, end).unwrap();

    let jd: Period<UTC, JD> = p.to_with::<JD>(&ctx).unwrap();
    let back: Period<UTC, MJD> = jd.to_with::<MJD>(&ctx).unwrap();
    assert_eq!(back, p);
    assert_eq!(p.try_to::<JD>().unwrap(), jd);
    assert_eq!(back.start.raw_seconds_pair(), p.start.raw_seconds_pair());
}

#[test]
fn conversion_errors_are_not_swallowed_at_either_endpoint() {
    let normal = Time::<UTC>::parse_rfc3339("2016-12-31T23:59:59Z").unwrap();
    let leap = Time::<UTC>::parse_rfc3339("2016-12-31T23:59:60Z").unwrap();
    let after = Time::<UTC>::parse_rfc3339("2017-01-01T00:00:00Z").unwrap();

    let bad_start = Period::<UTC>::try_new(leap, after).unwrap();
    let bad_end = Period::<UTC>::try_new(normal, leap).unwrap();
    let ctx = TimeContext::new();
    for p in [bad_start, bad_end] {
        assert_eq!(p.try_to::<Unix>(), Err(ConversionError::InvalidLeapSecond));
        assert_eq!(
            p.to_with::<MJD>(&ctx),
            Err(ConversionError::InvalidLeapSecond)
        );
    }
}

#[test]
fn scale_conversions_delegate_and_keep_format_tag() {
    let ctx = TimeContext::new();
    let p = Period::<TT, MJD>::try_new(mjd(51_544.5), mjd(51_544.500_1)).unwrap();
    let ut1: Period<UT1, MJD> = p.to_with::<UT1>(&ctx).unwrap();
    let back: Period<TT, MJD> = ut1.to_with::<TT>(&ctx).unwrap();
    assert!((back.start.to::<J2000s>() - p.start.to::<J2000s>()).abs() < Second::new(1e-6));
    assert!((back.end.to::<J2000s>() - p.end.to::<J2000s>()).abs() < Second::new(1e-6));
    let tai: Period<TAI, MJD> = p.to::<TAI>();
    assert_eq!(tai.to::<TT>(), p);
}

#[cfg(feature = "serde")]
#[test]
fn scalar_period_serde_uses_selected_endpoint_format() {
    let p: Period<TT, MJD> = Period::new(mjd(60_000.25), mjd(60_001.5));
    let wire = serde_json::to_value(p).unwrap();
    assert_eq!(
        wire,
        serde_json::json!({"start": 60_000.25, "end": 60_001.5})
    );
    let decoded: Period<TT, MJD> = serde_json::from_value(wire).unwrap();
    assert_eq!(decoded, p);
    assert!(serde_json::from_value::<Period<TT, MJD>>(
        serde_json::json!({"start": 60_010.0, "end": 60_000.0})
    )
    .is_err());
}

#[cfg(feature = "serde")]
#[test]
fn tagged_period_serde_preserves_split_seconds_independent_of_format() {
    use tempoch_core::tagged::{TaggedPeriod, TaggedTime};

    let p = Period::<TT, MJD>::new(mjd(60_000.25), mjd(60_001.5));
    let tagged = TaggedPeriod(p);
    let encoded = serde_json::to_value(tagged).unwrap();
    assert_eq!(encoded["scale"], "TT");
    assert_eq!(encoded["start"]["scale"], "TT");
    assert!(encoded["start"]["hi"].is_number());
    assert!(encoded["start"]["lo"].is_number());

    let decoded: TaggedPeriod<TT, MJD> = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(decoded.0, p);
    let canonical: TaggedPeriod<TT> = serde_json::from_value(encoded).unwrap();
    assert_eq!(canonical.0, p.to::<J2000s>());

    let single: TaggedTime<TT, MJD> =
        serde_json::from_value(serde_json::to_value(TaggedTime(p.start)).unwrap()).unwrap();
    assert_eq!(single.0, p.start);
}

#[cfg(feature = "serde")]
#[test]
fn utc_non_default_period_serde_rejects_leap_encoding() {
    let ctx = TimeContext::new();
    let start = Time::<UTC, MJD>::try_new_with(Day::new(57_754.0), &ctx).unwrap();
    let end = Time::<UTC, MJD>::try_new_with(Day::new(57_754.125), &ctx).unwrap();
    let p = Period::<UTC, MJD>::new(start, end);
    let wire = serde_json::to_string(&p).unwrap();
    assert_eq!(serde_json::from_str::<Period<UTC, MJD>>(&wire).unwrap(), p);

    let leap = Time::<UTC>::parse_rfc3339("2016-12-31T23:59:60Z").unwrap();
    let illegal: Period<UTC, MJD> = Period::new(leap.reinterpret(), end);
    assert!(serde_json::to_string(&illegal).is_err());
}
