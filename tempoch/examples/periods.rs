use chrono::{DateTime, Utc};
use tempoch::{
    complement_within, intersect_periods, Interval, ModifiedJulianDate, Period, UtcPeriod,
};

fn main() {
    let day = Period::new(
        ModifiedJulianDate::new(61_000.0),
        ModifiedJulianDate::new(61_001.0),
    );
    let windows = vec![
        Period::new(
            ModifiedJulianDate::new(61_000.10),
            ModifiedJulianDate::new(61_000.30),
        ),
        Period::new(
            ModifiedJulianDate::new(61_000.60),
            ModifiedJulianDate::new(61_000.85),
        ),
    ];

    let gaps = complement_within(day, &windows);
    println!("Visible windows: {}", windows.len());
    println!("Gaps: {}", gaps.len());

    let constraints = vec![
        Period::new(
            ModifiedJulianDate::new(61_000.00),
            ModifiedJulianDate::new(61_000.20),
        ),
        Period::new(
            ModifiedJulianDate::new(61_000.70),
            ModifiedJulianDate::new(61_001.00),
        ),
    ];
    let intersection = intersect_periods(&windows, &constraints);
    println!("Intersection windows: {}", intersection.len());

    let utc_day: UtcPeriod = day.to::<DateTime<Utc>>().unwrap();
    let roundtrip: Interval<ModifiedJulianDate> = utc_day.to::<ModifiedJulianDate>();
    println!(
        "Roundtrip drift (days): {:.3e}",
        (roundtrip.start.value() - day.start.value()).abs()
    );
}
