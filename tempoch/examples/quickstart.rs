use chrono::Utc;
use tempoch::{JulianDate, Time, MJD, UT};

fn main() {
    let now_jd = JulianDate::from_utc(Utc::now());
    let now_mjd: Time<MJD> = now_jd.to::<MJD>();
    let now_ut: Time<UT> = now_jd.to::<UT>();

    println!("JD(TT): {now_jd}");
    println!("MJD(TT): {now_mjd}");
    println!("UT: {now_ut}");
    println!("ΔT: {}", now_ut.delta_t());
}
