use chrono::DateTime;
use qtty::{Day, Days, Seconds};
use tempoch::{complement_within, intersect_periods, JulianDate, ModifiedJulianDate, Period, UT};

#[test]
fn utc_roundtrip_j2000_is_stable() {
    let datetime = DateTime::from_timestamp(946_728_000, 0).unwrap();
    let jd = JulianDate::from_utc(datetime);
    let back = jd.to_utc().expect("to_utc");
    let delta_ns = back.timestamp_nanos_opt().unwrap() - datetime.timestamp_nanos_opt().unwrap();
    assert!(delta_ns.abs() < 1_000);
}

#[test]
fn ut_applies_delta_t_near_j2000() {
    let ut = tempoch::Time::<UT>::new(2_451_545.0);
    let jd: JulianDate = ut.to::<tempoch::JD>();
    let offset = (jd.quantity() - ut.quantity()).to::<Day>();
    let offset_s = offset.to::<qtty::Second>();
    assert!((offset_s - Seconds::new(63.83)).abs() < Seconds::new(1.0));
}

#[test]
fn period_set_ops_match_expected_intervals() {
    let outer = Period::new(ModifiedJulianDate::new(0.0), ModifiedJulianDate::new(10.0));
    let a = vec![
        Period::new(ModifiedJulianDate::new(1.0), ModifiedJulianDate::new(3.0)),
        Period::new(ModifiedJulianDate::new(5.0), ModifiedJulianDate::new(9.0)),
    ];
    let b = vec![
        Period::new(ModifiedJulianDate::new(2.0), ModifiedJulianDate::new(4.0)),
        Period::new(ModifiedJulianDate::new(7.0), ModifiedJulianDate::new(8.0)),
    ];

    let below_b = complement_within(outer, &b);
    let between = intersect_periods(&a, &below_b);

    assert_eq!(between.len(), 3);
    assert_eq!(between[0].start.quantity(), Days::new(1.0));
    assert_eq!(between[0].end.quantity(), Days::new(2.0));
    assert_eq!(between[1].start.quantity(), Days::new(5.0));
    assert_eq!(between[1].end.quantity(), Days::new(7.0));
    assert_eq!(between[2].start.quantity(), Days::new(8.0));
    assert_eq!(between[2].end.quantity(), Days::new(9.0));
}

#[cfg(feature = "serde")]
#[test]
fn serde_period_mjd_uses_legacy_field_names() {
    let period = Period::new(
        ModifiedJulianDate::new(59_000.25),
        ModifiedJulianDate::new(59_000.75),
    );
    let json = serde_json::to_string(&period).unwrap();
    assert!(json.contains("start_mjd"));
    assert!(json.contains("end_mjd"));
}
