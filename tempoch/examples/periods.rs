use tempoch::{
    complement_within, intersect_periods, normalize_periods, validate_period_list, Mjd, Period, TT,
};

fn main() {
    let day = Period::<TT, Mjd>::new(61_000.0, 61_001.0);
    let windows = normalize_periods(&[
        Period::<TT, Mjd>::new(61_000.10, 61_000.30),
        Period::<TT, Mjd>::new(61_000.60, 61_000.85),
    ]);
    validate_period_list(&windows).unwrap();

    let gaps = complement_within(day, &windows);
    println!("Visible windows: {}", windows.len());
    println!("Gaps: {}", gaps.len());

    let constraints = vec![
        Period::<TT, Mjd>::new(61_000.00, 61_000.20),
        Period::<TT, Mjd>::new(61_000.70, 61_001.00),
    ];
    let intersection = intersect_periods(&windows, &constraints);
    println!("Intersection windows: {}", intersection.len());
    println!(
        "First overlap starts at MJD {:.5}",
        intersection[0].start.modified_julian_days()
    );
}
