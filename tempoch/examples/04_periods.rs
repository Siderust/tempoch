use tempoch::{
    complement_within, intersect_periods, normalize_periods, validate_period_list, Period, Time, TT,
};

fn mjd(value: f64) -> Time<TT> {
    Time::<TT>::from_modified_julian_days(value.into()).unwrap()
}

fn main() {
    let day = Period::<TT>::new(mjd(61_000.0), mjd(61_001.0));
    let a = vec![
        Period::<TT>::new(mjd(61_000.10), mjd(61_000.30)),
        Period::<TT>::new(mjd(61_000.60), mjd(61_000.85)),
    ];
    let b = vec![
        Period::<TT>::new(mjd(61_000.00), mjd(61_000.20)),
        Period::<TT>::new(mjd(61_000.70), mjd(61_001.00)),
    ];

    let overlaps = intersect_periods(&a, &b);
    let gaps = complement_within(day, &a);
    let merged = normalize_periods(&[a[0], a[1], overlaps[0]]);

    validate_period_list(&a).unwrap();

    println!("overlaps: {}", overlaps.len());
    println!("gaps    : {}", gaps.len());
    println!("merged  : {}", merged.len());
}
