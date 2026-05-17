use chrono::{DateTime, NaiveDate};
use qtty::{Day, Second};
#[cfg(feature = "serde")]
use serde_json::json;
use tempoch::{
    constats::{J2000_JD_TT, TT_MINUS_TAI},
    ConversionError, CoordinateScale, J2000Seconds, J2000s, JulianDate, ModifiedJulianDate, Period,
    Time, TimeContext, Unix, JD, MJD, TAI, TT, UT1, UTC,
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
fn tt_encoded_dates_support_chrono_roundtrips() {
    let datetime = DateTime::from_timestamp(1_784_617_200, 0).unwrap();

    let jd = JulianDate::<TT>::from_chrono(datetime);
    let mjd = ModifiedJulianDate::<TT>::from_chrono(datetime);

    let jd_back = jd.to_chrono().unwrap();
    let mjd_back = mjd.to_chrono().unwrap();

    let jd_delta_ns =
        jd_back.timestamp_nanos_opt().unwrap() - datetime.timestamp_nanos_opt().unwrap();
    let mjd_delta_ns =
        mjd_back.timestamp_nanos_opt().unwrap() - datetime.timestamp_nanos_opt().unwrap();

    assert!(jd_delta_ns.abs() < 50_000);
    assert!(mjd_delta_ns.abs() < 50_000);
}

#[test]
fn ut1_context_roundtrip_near_j2000() {
    let ctx = TimeContext::new();
    let tt = J2000Seconds::<TT>::try_new(Second::new(0.0))
        .unwrap()
        .to_time();
    let ut1 = tt.to_with::<UT1>(&ctx).unwrap();
    let tt_back = ut1.to_with::<TT>(&ctx).unwrap();

    let offset_s = tt.to::<J2000s>().raw() - ut1.to::<J2000s>().raw();
    assert!((offset_s - Second::new(63.83)).abs() < Second::new(1.0));
    assert!((tt - tt_back).abs() < Second::new(1e-9));
}

#[test]
fn default_try_to_ut1_uses_default_context() {
    let tt = J2000Seconds::<TT>::try_new(Second::new(0.0))
        .unwrap()
        .to_time();
    let via_default: Time<UT1> = tt.try_to::<UT1>().unwrap();
    let via_context: Time<UT1> = tt.to_with::<UT1>(&TimeContext::new()).unwrap();
    assert!(
        (via_default.to::<J2000s>().raw() - via_context.to::<J2000s>().raw()).abs()
            < Second::new(1e-12)
    );
}

#[test]
fn public_constats_epochs_are_usable() {
    let j2000 = JulianDate::<TT>::try_new(J2000_JD_TT.raw())
        .unwrap()
        .to_time();
    let tai_s: Time<TAI> = j2000.to::<TAI>();

    assert_eq!(j2000.to::<JD>().raw(), J2000_JD_TT.raw());
    assert!(
        ((j2000.to::<J2000s>().raw() - tai_s.to::<J2000s>().raw()) - TT_MINUS_TAI).abs()
            < Second::new(1e-12)
    );
}

#[test]
fn encoded_date_helpers_and_assign_arithmetic_are_available() {
    let mut jd = JulianDate::<TT>::try_new(J2000_JD_TT.raw()).unwrap();
    jd += Day::new(2.0);
    jd -= Day::new(0.5);

    assert_eq!(jd.value(), 2_451_546.5);
    assert_eq!(jd.to::<MJD>().value(), jd.value() - 2_400_000.5);
    assert!(
        (JulianDate::<TT>::try_new(Day::new(2_451_545.0))
            .unwrap()
            .julian_centuries())
        .abs()
            < 1e-12
    );

    let mut mjd = ModifiedJulianDate::<TT>::try_new(Day::new(51_544.5)).unwrap();
    mjd += Day::new(1.0);
    mjd -= Day::new(0.25);
    assert_eq!(mjd.to::<JD>().value(), 2_451_545.75);
}

#[test]
fn encoded_date_min_max_and_mean_are_available() {
    let a = JulianDate::<TT>::try_new(Day::new(2_451_545.0)).unwrap();
    let b = JulianDate::<TT>::try_new(Day::new(2_451_547.0)).unwrap();

    assert_eq!(a.min(b), a);
    assert_eq!(a.max(b), b);
    assert_eq!(a.mean(b).value(), 2_451_546.0);
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
    assert!(utc.try_to::<Unix>().is_err());
    assert_eq!(back.timestamp(), leap.timestamp());
    assert!(
        (back.timestamp_subsec_nanos() as i64 - leap.timestamp_subsec_nanos() as i64).abs()
            < 50_000
    );
    assert!(format!("{back:?}").starts_with("2016-12-31T23:59:60."));
}

#[test]
fn utc_supports_coordinate_views_and_pre_1961_roundtrips() {
    fn needs_coordinate_scale<S: CoordinateScale>(time: Time<S>) -> Day {
        time.to::<MJD>().raw()
    }

    let utc = J2000Seconds::<UTC>::try_new(Second::new(0.0))
        .unwrap()
        .to_time();
    assert_eq!(needs_coordinate_scale(utc), utc.to::<MJD>().raw());

    let pre_1961 = DateTime::from_timestamp(-631_152_000, 500_000_000).unwrap();

    // Default: pre-1961 dates must be rejected.
    assert!(matches!(
        Time::<UTC>::try_from_chrono(pre_1961),
        Err(ConversionError::UtcBeforeDefinition)
    ));

    // Opt-in: round-trip must close.
    let ctx = TimeContext::new().allow_pre_definition_utc();
    let encoded = Time::<UTC>::try_from_chrono_with(pre_1961, &ctx).unwrap();
    let back = encoded.try_to_chrono_with(&ctx).unwrap();
    let delta_ns = back.timestamp_nanos_opt().unwrap() - pre_1961.timestamp_nanos_opt().unwrap();
    assert!(delta_ns.abs() < 50_000);
}

#[test]
fn interval_set_ops_match_expected_intervals() {
    let outer = Period::<TT>::new(
        ModifiedJulianDate::<TT>::try_new(Day::new(0.0))
            .unwrap()
            .to_time(),
        ModifiedJulianDate::<TT>::try_new(Day::new(10.0))
            .unwrap()
            .to_time(),
    );
    let a = vec![
        Period::<TT>::new(
            ModifiedJulianDate::<TT>::try_new(Day::new(1.0))
                .unwrap()
                .to_time(),
            ModifiedJulianDate::<TT>::try_new(Day::new(3.0))
                .unwrap()
                .to_time(),
        ),
        Period::<TT>::new(
            ModifiedJulianDate::<TT>::try_new(Day::new(5.0))
                .unwrap()
                .to_time(),
            ModifiedJulianDate::<TT>::try_new(Day::new(9.0))
                .unwrap()
                .to_time(),
        ),
    ];
    let b = vec![
        Period::<TT>::new(
            ModifiedJulianDate::<TT>::try_new(Day::new(2.0))
                .unwrap()
                .to_time(),
            ModifiedJulianDate::<TT>::try_new(Day::new(4.0))
                .unwrap()
                .to_time(),
        ),
        Period::<TT>::new(
            ModifiedJulianDate::<TT>::try_new(Day::new(7.0))
                .unwrap()
                .to_time(),
            ModifiedJulianDate::<TT>::try_new(Day::new(8.0))
                .unwrap()
                .to_time(),
        ),
    ];

    let below_b = outer.complement(&b);
    let between = Period::intersect_many(&a, &below_b);

    assert_eq!(between.len(), 3);
    assert_eq!(between[0].start.to::<MJD>().raw(), Day::new(1.0));
    assert_eq!(between[0].end.to::<MJD>().raw(), Day::new(2.0));
    assert_eq!(between[1].start.to::<MJD>().raw(), Day::new(5.0));
    assert_eq!(between[1].end.to::<MJD>().raw(), Day::new(7.0));
    assert_eq!(between[2].start.to::<MJD>().raw(), Day::new(8.0));
    assert_eq!(between[2].end.to::<MJD>().raw(), Day::new(9.0));
}

#[cfg(feature = "serde")]
#[test]
fn public_serde_roundtrips_time_and_periods() {
    let tt = J2000Seconds::<TT>::try_new(Second::new(12.5))
        .unwrap()
        .to_time();
    let jd = JulianDate::<TT>::try_new(Day::new(2_451_545.25))
        .unwrap()
        .to_time();
    let mjd_period = Period::<TT>::new(
        ModifiedJulianDate::<TT>::try_new(Day::new(61_000.0))
            .unwrap()
            .to_time(),
        ModifiedJulianDate::<TT>::try_new(Day::new(61_001.0))
            .unwrap()
            .to_time(),
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
        serde_json::to_value(mjd_period).unwrap(),
        json!({
            "start": {"hi": 816955200.0, "lo": 0.0},
            "end": {"hi": 817041600.0, "lo": 0.0}
        })
    );
    assert_eq!(
        serde_json::from_value::<Period<TT>>(json!({
            "start": {"hi": 816955200.0, "lo": 0.0},
            "end": {"hi": 817041600.0, "lo": 0.0}
        }))
        .unwrap(),
        mjd_period
    );
}

#[cfg(feature = "serde")]
#[test]
fn serde_still_works_with_current_shape() {
    let tt = J2000Seconds::<TT>::try_new(Second::new(1.25))
        .unwrap()
        .to_time();
    let period = Period::<TT>::new(
        J2000Seconds::<TT>::try_new(Second::new(1.25))
            .unwrap()
            .to_time(),
        J2000Seconds::<TT>::try_new(Second::new(2.5))
            .unwrap()
            .to_time(),
    );

    assert_eq!(
        serde_json::from_value::<Time<TT>>(json!({"hi": 1.25, "lo": 0.0})).unwrap(),
        tt
    );
    assert_eq!(
        serde_json::from_value::<Period<TT>>(json!({
            "start": {"hi": 1.25, "lo": 0.0},
            "end": {"hi": 2.5, "lo": 0.0}
        }))
        .unwrap(),
        period
    );
}

#[cfg(feature = "serde")]
#[test]
fn tagged_serde_preserves_scale_in_payload() {
    use tempoch::tagged::{TaggedPeriod, TaggedTime};

    let tt = J2000Seconds::<TT>::try_new(Second::new(1.25))
        .unwrap()
        .to_time();
    let period = Period::<TT>::new(
        J2000Seconds::<TT>::try_new(Second::new(1.25))
            .unwrap()
            .to_time(),
        J2000Seconds::<TT>::try_new(Second::new(2.5))
            .unwrap()
            .to_time(),
    );

    assert_eq!(
        serde_json::to_value(TaggedTime(tt)).unwrap(),
        json!({"scale": "TT", "hi": 1.25, "lo": 0.0})
    );
    assert_eq!(
        serde_json::to_value(TaggedPeriod(period)).unwrap(),
        json!({
            "scale": "TT",
            "start": {"scale": "TT", "hi": 1.25, "lo": 0.0},
            "end": {"scale": "TT", "hi": 2.5, "lo": 0.0}
        })
    );
}
