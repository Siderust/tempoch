use chrono::{DateTime, NaiveDate};
use qtty::{Day, Second};
use tempoch::{
    complement_within,
    constats::{J2000_JD_TT, TT_MINUS_TAI},
    intersect_periods, Jd, Mjd, Period, Time, TimeContext, TAI, TT, UT1, UTC,
};

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
    let j2000 = Time::<TT, Jd>::from_julian_days(J2000_JD_TT).unwrap();
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
    let outer = Period::<TT, Mjd>::new(0.0, 10.0);
    let a = vec![
        Period::<TT, Mjd>::new(1.0, 3.0),
        Period::<TT, Mjd>::new(5.0, 9.0),
    ];
    let b = vec![
        Period::<TT, Mjd>::new(2.0, 4.0),
        Period::<TT, Mjd>::new(7.0, 8.0),
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
