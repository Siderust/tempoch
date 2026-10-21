use chrono::{DateTime, Timelike, Utc};
use tempoch::qtty::{Minute, Second};
use tempoch::{ExactDuration, Time, TimeContext, JD, UTC};

fn label(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

#[test]
fn existing_bridges_distinguish_nominal_from_elapsed_seconds() {
    let ctx = TimeContext::new();
    let start = label("2016-12-31T23:59:00Z");
    let end = label("2017-01-01T00:00:00Z");
    let a = Time::<UTC>::try_from_chrono_with(start, &ctx).unwrap();
    let b = Time::<UTC>::try_from_chrono_with(end, &ctx).unwrap();
    assert_eq!(b - a, Second::new(61.0));
    let ca = a.try_to_chrono_with(&ctx).unwrap();
    let cb = b.try_to_chrono_with(&ctx).unwrap();
    assert_eq!(
        cb.signed_duration_since(ca).num_nanoseconds(),
        Some(60_000_000_000)
    );
    let leap = Time::<UTC>::try_from_chrono_with(label("2016-12-31T23:59:60Z"), &ctx).unwrap();
    assert_eq!(
        leap.try_to_chrono_with(&ctx).unwrap().nanosecond(),
        1_000_000_000
    );
    assert!(leap.to_with::<JD>(&ctx).is_err());
}

#[test]
fn ordinary_calendar_offsets_preserve_nanoseconds_and_check_ranges() {
    let ctx = TimeContext::new();
    let epoch = label("2017-01-01T00:00:00.123456789Z");
    for ns in [
        -86_400_000_000_000_i64,
        -30_000_000_001,
        -1,
        0,
        1,
        30_000_000_001,
        86_400_000_000_000,
    ] {
        let shifted = epoch
            .checked_add_signed(chrono::TimeDelta::nanoseconds(ns))
            .unwrap();
        let native = Time::<UTC>::try_from_chrono_with(shifted, &ctx).unwrap();
        let round = native
            .to_with::<JD>(&ctx)
            .unwrap()
            .try_to_chrono_with(&ctx)
            .unwrap();
        assert_eq!(round, shifted);
        assert_eq!(
            round.signed_duration_since(epoch).num_nanoseconds(),
            Some(ns)
        );
    }
    let negative = ExactDuration::try_from_quantity(Minute::new(-0.5)).unwrap();
    assert_eq!(
        negative.as_nanoseconds_i().unwrap().value(),
        -30_000_000_000
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
        assert!(ExactDuration::try_from_quantity(Minute::new(value)).is_err());
    }
    let too_large = ExactDuration::try_from_quantity(Second::new(1e10)).unwrap();
    assert!(too_large.as_nanoseconds_i().is_err());
}
