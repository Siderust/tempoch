use tempoch::{Interval, ModifiedJulianDate, Period, TT};

fn mjd(value: f64) -> tempoch::Time<TT> {
    ModifiedJulianDate::<TT>::new(value).to_j2000s()
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

    let overlaps = Period::intersect_many(&a, &b);
    let gaps = day.complement(&a);
    let merged = Interval::normalize(&[a[0], a[1], overlaps[0]]);

    Period::validate(&a).unwrap();

    println!("overlaps: {}", overlaps.len());
    println!("gaps    : {}", gaps.len());
    println!("merged  : {}", merged.len());
}
