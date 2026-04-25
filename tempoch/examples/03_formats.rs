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

    let j2000_from_jd = JulianDate::<TT>::try_new(J2000_JD_TT).unwrap().to_time();
    let unix_epoch_jd = JulianDate::<TT>::try_new(UNIX_EPOCH_JD).unwrap().to_time();
    let half_day_jd = JulianDate::<TT>::try_new(Day::new(2_451_545.5))
        .unwrap()
        .to_time();
    let unix_epoch_mjd = ModifiedJulianDate::<TT>::try_new(UNIX_EPOCH_MJD)
        .unwrap()
        .to_time();

    let utc = UnixTime::try_new(Second::new(1_700_000_000.25))
        .and_then(|e| e.to_time_with(&ctx))
        .unwrap();

    println!("J2000 TT seconds  : {:.9}", j2000_tt.to::<J2000s>().raw());
    println!("Sample TT JD      : {:.9}", sample_tt.to::<JD>().raw());
    println!("Sample TT MJD     : {:.9}", sample_tt.to::<MJD>().raw());
    println!(
        "J2000 from JD     : {:.9}",
        j2000_from_jd.to::<J2000s>().raw()
    );
    println!(
        "Unix epoch JD(TT) : {:.9}",
        unix_epoch_jd.to::<JD>().raw()
    );
    println!("Half-day JD(TT)   : {:.9}", half_day_jd.to::<JD>().raw());
    println!(
        "Unix epoch MJD(TT): {:.9}",
        unix_epoch_mjd.to::<MJD>().raw()
    );
    println!(
        "UTC POSIX         : {:.3}",
        utc.try_to::<Unix>().unwrap().raw()
    );

    // suppress unused warnings
    let _ = Time::<UTC>::from_chrono(chrono::Utc::now());
}
