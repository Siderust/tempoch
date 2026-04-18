use chrono::{DateTime, NaiveDate};
use qtty::{Day, Second};
#[cfg(feature = "serde")]
use serde_json::json;
use tempoch::{
    complement_within,
    constats::{J2000_JD_TT, TT_MINUS_TAI},
    intersect_periods, JD, MJD, Period, Time, TimeContext, TAI, TT, UT1, UTC,
};
#[cfg(feature = "serde")]
use tempoch::{DayCount, GpsSecs, UnixSecs};

#[test]
fn utc_roundtrip_j2000_is_stable() {
    let datetime = DateTime::from_timestamp(946_728_000, 0).unwrap();
    let utc = Time::<UTC>::try_from_chrono(datetime).unwrap();
    let jd_tt: Time<TT> = utc.to_scale::<TT>();
    let back = jd_tt.to_scale::<UTC>().try_to_chrono().unwrap();
    let delta_ns = back.timestamp_nanos_opt().unwrap() - datetime.timestamp_nanos_opt().unwrap();
    assert!(delta_ns.abs() < 50_000);
}

#[test]
fn ut1_context_roundtrip_near_j2000() {
    let ctx = TimeContext::new();
    let tt = Time::<TT>::from_si_seconds(Second::new(0.0)).unwrap();
    let ut1 = tt.to_scale_with::<UT1>(&ctx).unwrap();
    let tt_back = ut1.to_scale_with::<TT>(&ctx).unwrap();

    let offset_s = tt.si_seconds() - ut1.si_seconds();
    assert!((offset_s - Second::new(63.83)).abs() < Second::new(1.0));
    assert!((tt - tt_back).abs() < Second::new(1e-9));
}

#[test]
fn public_constats_epochs_are_usable() {
    let j2000 = Time::<TT, JD>::from_julian_days(J2000_JD_TT).unwrap();
    let j2000_s: Time<TT> = j2000.reformat();
    let tai_s: Time<TAI> = j2000_s.to_scale();

    assert_eq!(j2000.julian_days(), J2000_JD_TT);
    assert!(
        ((j2000_s.si_seconds() - tai_s.si_seconds()) - TT_MINUS_TAI).abs() < Second::new(1e-12)
    );
}

#[test]
fn utc_leap_second_roundtrip_is_preserved() {
    let leap = NaiveDate::from_ymd_opt(2016, 12, 31)
        .unwrap()
        .and_hms_nano_opt(23, 59, 59, 1_250_000_000)
        .unwrap()
        .and_utc();

    let utc = Time::<UTC>::try_from_chrono(leap).unwrap();
    let back = utc.try_to_chrono().unwrap();

    assert!(utc.is_leap_second());
    assert_eq!(back.timestamp(), leap.timestamp());
    assert!(
        (back.timestamp_subsec_nanos() as i64 - leap.timestamp_subsec_nanos() as i64).abs()
            < 50_000
    );
    assert!(format!("{back:?}").starts_with("2016-12-31T23:59:60."));
}

#[test]
fn interval_set_ops_match_expected_intervals() {
    let outer = Period::<TT, MJD>::new(0.0, 10.0);
    let a = vec![
        Period::<TT, MJD>::new(1.0, 3.0),
        Period::<TT, MJD>::new(5.0, 9.0),
    ];
    let b = vec![
        Period::<TT, MJD>::new(2.0, 4.0),
        Period::<TT, MJD>::new(7.0, 8.0),
    ];

    let below_b = complement_within(outer, &b);
    let between = intersect_periods(&a, &below_b);

    assert_eq!(between.len(), 3);
    assert_eq!(between[0].start.modified_julian_days(), Day::new(1.0));
    assert_eq!(between[0].end.modified_julian_days(), Day::new(2.0));
    assert_eq!(between[1].start.modified_julian_days(), Day::new(5.0));
    assert_eq!(between[1].end.modified_julian_days(), Day::new(7.0));
    assert_eq!(between[2].start.modified_julian_days(), Day::new(8.0));
    assert_eq!(between[2].end.modified_julian_days(), Day::new(9.0));
}

#[cfg(feature = "serde")]
#[test]
fn public_serde_roundtrips_time_and_periods() {
    let tt = Time::<TT>::from_si_seconds(Second::new(12.5)).unwrap();
    let jd = Time::<TT, JD>::from_julian_days(Day::new(2_451_545.25)).unwrap();
    let mjd_period = Period::<TT, MJD>::new(61_000.0, 61_001.0);
    let unix = Time::<UTC, UnixSecs>::from(1_700_000_000_i64);
    let gps = Time::<TAI, GpsSecs>::from(345.25_f64);
    let daycount = Time::<TT, DayCount>::from(42_i32);

    assert_eq!(serde_json::to_value(tt).unwrap(), json!(12.5));
    assert_eq!(serde_json::to_value(jd).unwrap(), json!(2_451_545.25));
    assert_eq!(
        serde_json::to_value(mjd_period).unwrap(),
        json!({"start": 61_000.0, "end": 61_001.0})
    );
    assert_eq!(
        serde_json::to_value(unix).unwrap(),
        json!(1_700_000_000_i64)
    );
    assert_eq!(serde_json::to_value(gps).unwrap(), json!(345.25));
    assert_eq!(serde_json::to_value(daycount).unwrap(), json!(42));

    assert_eq!(serde_json::from_value::<Time<TT>>(json!(12.5)).unwrap(), tt);
    assert_eq!(
        serde_json::from_value::<Time<TT, JD>>(json!(2_451_545.25)).unwrap(),
        jd
    );
    assert_eq!(
        serde_json::from_value::<Period<TT, MJD>>(json!({"start": 61_000.0, "end": 61_001.0}))
            .unwrap(),
        mjd_period
    );
}

#[cfg(all(feature = "serde", feature = "runtime-data"))]
#[test]
fn serde_still_works_with_all_features_enabled() {
    let tt = Time::<TT>::from_si_seconds(Second::new(1.25)).unwrap();
    let period = Period::<TT>::new(1.25, 2.5);

    assert_eq!(serde_json::from_value::<Time<TT>>(json!(1.25)).unwrap(), tt);
    assert_eq!(
        serde_json::from_value::<Period<TT>>(json!({"start": 1.25, "end": 2.5})).unwrap(),
        period
    );
}
