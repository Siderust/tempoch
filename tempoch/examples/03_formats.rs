//! Constructor/accessor showcase for the scale-only API.

use qtty::{Day, Second};
use tempoch::{
    constats::{J2000_JD_TT, UNIX_EPOCH_JD, UNIX_EPOCH_MJD},
    J2000s, JD, MJD, Time, UnixSecs, TT, UTC,
};

fn main() {
    let j2000_tt = Time::<TT>::from_j2000_seconds(Second::new(0.0)).unwrap();
    let sample_tt = Time::<TT>::from_j2000_seconds(Second::new(123_456.789)).unwrap();

    let j2000_from_jd = Time::<TT>::from_julian_days(J2000_JD_TT).unwrap();
    let unix_epoch_jd = Time::<TT>::from_julian_days(UNIX_EPOCH_JD).unwrap();
    let half_day_jd = Time::<TT>::from_julian_days(Day::new(2_451_545.5)).unwrap();
    let unix_epoch_mjd = Time::<TT>::from_modified_julian_days(UNIX_EPOCH_MJD).unwrap();

    let utc = Time::<UTC>::from_unix_seconds(Second::new(1_700_000_000.25)).unwrap();

    println!("J2000 TT seconds  : {:.9}", j2000_tt.to::<J2000s>().value());
    println!("Sample TT JD      : {:.9}", sample_tt.to::<JD>().value());
    println!("Sample TT MJD     : {:.9}", sample_tt.to::<MJD>().value());
    println!("J2000 from JD     : {:.9}", j2000_from_jd.to::<J2000s>().value());
    println!("Unix epoch JD(TT) : {:.9}", unix_epoch_jd.to::<JD>().value());
    println!("Half-day JD(TT)   : {:.9}", half_day_jd.to::<JD>().value());
    println!("Unix epoch MJD(TT): {:.9}", unix_epoch_mjd.to::<MJD>().value());
    println!("UTC POSIX         : {:.3}", utc.try_to::<UnixSecs>().unwrap().value());
}
