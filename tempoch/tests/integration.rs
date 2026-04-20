use chrono::{DateTime, NaiveDate};
use qtty::{Day, Second};
#[cfg(feature = "serde")]
use serde_json::json;
use tempoch::{
    complement_within,
    constats::{J2000_JD_TT, TT_MINUS_TAI},
    intersect_periods, J2000s, JD, MJD, Period, Time, TimeContext, TAI, TT, UT1, UTC,
};

#[test]
fn utc_roundtrip_j2000_is_stable() {
    let datetime = DateTime::from_timestamp(946_728_000, 0).unwrap();
    let utc = Time::<UTC>::try_from_chrono(datetime).unwrap();
    let jd_tt: Time<TT> = utc.to::<TT>();
    let back = jd_tt.to::<UTC>().try_to_chrono().unwrap();
    let delta_ns = back.timestamp_nanos_opt().unwrap() - datetime.timestamp_nanos_opt().unwrap();
    assert!(delta_ns.abs() < 50_000);
}

#[test]
fn ut1_context_roundtrip_near_j2000() {
    let ctx = TimeContext::new();
    let tt = Time::<TT>::from_j2000_seconds(Second::new(0.0)).unwrap();
    let ut1 = tt.to_with::<UT1>(&ctx).unwrap();
    let tt_back = ut1.to_with::<TT>(&ctx).unwrap();

    let offset_s = tt.to::<J2000s>() - ut1.to::<J2000s>();
    assert!((offset_s - Second::new(63.83)).abs() < Second::new(1.0));
    assert!((tt - tt_back).abs() < Second::new(1e-9));
}

#[test]
fn public_constats_epochs_are_usable() {
    let j2000 = Time::<TT>::from_julian_days(J2000_JD_TT).unwrap();
    let tai_s: Time<TAI> = j2000.to::<TAI>();

    assert_eq!(j2000.to::<JD>(), J2000_JD_TT);
    assert!(
        ((j2000.to::<J2000s>() - tai_s.to::<J2000s>()) - TT_MINUS_TAI).abs() < Second::new(1e-12)
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
    let outer = Period::<TT>::new(
        Time::<TT>::from_modified_julian_days(0.0.into()).unwrap(),
        Time::<TT>::from_modified_julian_days(10.0.into()).unwrap(),
    );
    let a = vec![
        Period::<TT>::new(
            Time::<TT>::from_modified_julian_days(1.0.into()).unwrap(),
            Time::<TT>::from_modified_julian_days(3.0.into()).unwrap(),
        ),
        Period::<TT>::new(
            Time::<TT>::from_modified_julian_days(5.0.into()).unwrap(),
            Time::<TT>::from_modified_julian_days(9.0.into()).unwrap(),
        ),
    ];
    let b = vec![
        Period::<TT>::new(
            Time::<TT>::from_modified_julian_days(2.0.into()).unwrap(),
            Time::<TT>::from_modified_julian_days(4.0.into()).unwrap(),
        ),
        Period::<TT>::new(
            Time::<TT>::from_modified_julian_days(7.0.into()).unwrap(),
            Time::<TT>::from_modified_julian_days(8.0.into()).unwrap(),
        ),
    ];

    let below_b = complement_within(outer, &b);
    let between = intersect_periods(&a, &below_b);

    assert_eq!(between.len(), 3);
    assert_eq!(between[0].start.to::<MJD>(), Day::new(1.0));
    assert_eq!(between[0].end.to::<MJD>(), Day::new(2.0));
    assert_eq!(between[1].start.to::<MJD>(), Day::new(5.0));
    assert_eq!(between[1].end.to::<MJD>(), Day::new(7.0));
    assert_eq!(between[2].start.to::<MJD>(), Day::new(8.0));
    assert_eq!(between[2].end.to::<MJD>(), Day::new(9.0));
}

#[cfg(feature = "serde")]
#[test]
fn public_serde_roundtrips_time_and_periods() {
    let tt = Time::<TT>::from_j2000_seconds(Second::new(12.5)).unwrap();
    let jd = Time::<TT>::from_julian_days(Day::new(2_451_545.25)).unwrap();
    let mjd_period = Period::<TT>::new(
        Time::<TT>::from_modified_julian_days(61_000.0.into()).unwrap(),
        Time::<TT>::from_modified_julian_days(61_001.0.into()).unwrap(),
    );

    assert_eq!(
        serde_json::to_value(tt).unwrap(),
        json!({"hi": 12.5, "lo": 0.0})
    );
    assert_eq!(
        serde_json::to_value(jd).unwrap(),
        json!({"hi": 21600.0, "lo": 0.0})
    );

    assert_eq!(
        serde_json::from_value::<Time<TT>>(json!({"hi": 12.5, "lo": 0.0})).unwrap(),
        tt
    );
    assert_eq!(
        serde_json::from_value::<Time<TT>>(json!({"hi": 21600.0, "lo": 0.0})).unwrap(),
        jd
    );
    assert_eq!(
        serde_json::from_value::<Period<TT>>(json!({
            "start": {"hi": 816091200.0, "lo": 0.0},
            "end": {"hi": 816177600.0, "lo": 0.0}
        })).unwrap(),
        mjd_period
    );
}

#[cfg(feature = "serde")]
#[test]
fn serde_still_works_with_current_shape() {
    let tt = Time::<TT>::from_j2000_seconds(Second::new(1.25)).unwrap();
    let period = Period::<TT>::new(1.25, 2.5);

    assert_eq!(
        serde_json::from_value::<Time<TT>>(json!({"hi": 1.25, "lo": 0.0})).unwrap(),
        tt
    );
    assert_eq!(
        serde_json::from_value::<Period<TT>>(json!({
            "start": {"hi": 1.25, "lo": 0.0},
            "end": {"hi": 2.5, "lo": 0.0}
        })).unwrap(),
        period
    );
}
