use tempoch::{Interval, ModifiedJulianDate, Period, TT, JD, MJD};

fn mjd(value: f64) -> ModifiedJulianDate<TT> {
    ModifiedJulianDate::<TT>::new(value)
}

fn main() {
    // This is still Interval<Time<TT, MJD>>.
    let day: Period<TT, MJD> = Period::new(mjd(61_000.0), mjd(61_001.0));
    let a = vec![
        Period::<TT, MJD>::new(mjd(61_000.10), mjd(61_000.30)),
        Period::<TT, MJD>::new(mjd(61_000.60), mjd(61_000.85)),
    ];
    let b = vec![
        Period::<TT, MJD>::new(mjd(61_000.00), mjd(61_000.20)),
        Period::<TT, MJD>::new(mjd(61_000.70), mjd(61_001.00)),
    ];

    let jd_day: Period<TT, JD> = day.to::<JD>();
    assert_eq!(jd_day.to::<MJD>(), day);

    let overlaps = Period::intersect_many(&a, &b);
    let gaps = day.complement(&a);
    let merged = Interval::normalize(&[a[0], a[1], overlaps[0]]);
    Period::validate(&a).unwrap();

    println!("overlaps: {}", overlaps.len());
    println!("gaps    : {}", gaps.len());
    println!("merged  : {}", merged.len());
}
