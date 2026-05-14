//! Constructor/accessor showcase for the scale-only API.

use qtty::{Day, Second};
use tempoch::{
    constats::{J2000_JD_TT, UNIX_EPOCH_JD, UNIX_EPOCH_MJD},
    J2000Seconds, J2000s, JulianDate, ModifiedJulianDate, Time, TimeContext, Unix, UnixTime, JD,
    MJD, TT, UTC,
};

fn main() {
    let ctx = TimeContext::new();
    let j2000_tt = J2000Seconds::<TT>::try_new(Second::new(0.0))
        .unwrap()
        .to_time();
    let sample_tt = J2000Seconds::<TT>::try_new(Second::new(123_456.789))
        .unwrap()
        .to_time();

    let j2000_from_jd = JulianDate::<TT>::try_new(J2000_JD_TT.raw())
        .unwrap()
        .to_time();
    let unix_epoch_jd = JulianDate::<TT>::try_new(UNIX_EPOCH_JD.raw())
        .unwrap()
        .to_time();
    let half_day_jd = JulianDate::<TT>::try_new(Day::new(2_451_545.5))
        .unwrap()
        .to_time();
    let unix_epoch_mjd = ModifiedJulianDate::<TT>::try_new(UNIX_EPOCH_MJD.raw())
        .unwrap()
        .to_time();

    let utc = UnixTime::try_new(Second::new(1_700_000_000.25))
        .and_then(|e| e.to_time_with(&ctx))
        .unwrap();

    println!("J2000 TT seconds  : {:.9}", j2000_tt.to::<J2000s>());
    println!("Sample TT JD      : {:.9}", sample_tt.to::<JD>());
    println!("Sample TT MJD     : {:.9}", sample_tt.to::<MJD>());
    println!("J2000 from JD     : {:.9}", j2000_from_jd.to::<J2000s>());
    println!("Unix epoch JD(TT) : {:.9}", unix_epoch_jd.to::<JD>());
    println!("Half-day JD(TT)   : {:.9}", half_day_jd.to::<JD>());
    println!("Unix epoch MJD(TT): {:.9}", unix_epoch_mjd.to::<MJD>());
    println!("UTC POSIX         : {:.3}", utc.try_to::<Unix>().unwrap());

    // suppress unused warnings
    let _ = Time::<UTC>::from_chrono(chrono::Utc::now());
}
