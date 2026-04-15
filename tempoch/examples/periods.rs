use qtty::Day;
use tempoch::{
    complement_within, intersect_periods, normalize_periods, validate_period_list, Interval,
    ModifiedJulianDays, Time, TT,
};

type MjdTt = Time<TT, ModifiedJulianDays>;

fn mjd(value: f64) -> MjdTt {
    Time::<TT, ModifiedJulianDays>::from_modified_julian_days(Day::new(value)).unwrap()
}

fn main() {
    let day = Interval::new(mjd(61_000.0), mjd(61_001.0));
    let windows = normalize_periods(&[
        Interval::new(mjd(61_000.10), mjd(61_000.30)),
        Interval::new(mjd(61_000.60), mjd(61_000.85)),
    ]);
    validate_period_list(&windows).unwrap();

    let gaps = complement_within(day, &windows);
    println!("Visible windows: {}", windows.len());
    println!("Gaps: {}", gaps.len());

    let constraints = vec![
        Interval::new(mjd(61_000.00), mjd(61_000.20)),
        Interval::new(mjd(61_000.70), mjd(61_001.00)),
    ];
    let intersection = intersect_periods(&windows, &constraints);
    println!("Intersection windows: {}", intersection.len());
    println!(
        "First overlap starts at MJD {:.5}",
        intersection[0].start.modified_julian_days().value()
    );
}
