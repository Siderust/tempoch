//! Constructor/accessor showcase for the scale-only API.

use qtty::Second;
use tempoch::{
    J2000Seconds, J2000s, JulianDate, Time, Unix, UnixTime, J2000_JD_TT_DAY, JD, MJD, TT,
    UNIX_EPOCH_JD_DAY, UTC,
};

fn main() {
    // `J2000Seconds<S>` is `Time<S>` with an explicit alias — no relabel step.
    let j2000_tt = J2000Seconds::<TT>::new(0.0);
    let sample_tt = J2000Seconds::<TT>::new(123_456.789);

    // Keep JD/MJD scalars in their typed aliases; unified `.to::<Target>()` works on any `Time<S, F>`.
    let j2000_from_jd = JulianDate::<TT>::new(J2000_JD_TT_DAY.value());
    let unix_epoch_jd = JulianDate::<TT>::new(UNIX_EPOCH_JD_DAY.value());
    let half_day_jd = JulianDate::<TT>::new(2_451_545.5);
    let unix_epoch_mjd = tempoch::unix_epoch_mjd();

    let utc: Time<UTC> = UnixTime::try_new(Second::new(1_700_000_000.25))
        .unwrap()
        .into();

    println!("J2000 TT seconds  : {:.9}", j2000_tt.to::<J2000s>());
    println!("Sample TT JD      : {:.9}", sample_tt.to::<JD>());
    println!("Sample TT MJD     : {:.9}", sample_tt.to::<MJD>());
    println!("J2000 from JD     : {:.9}", j2000_from_jd.to::<J2000s>());
    println!("Unix epoch JD(TT) : {:.9}", unix_epoch_jd.to::<JD>());
    println!("Half-day JD(TT)   : {:.9}", half_day_jd.to::<JD>());
    println!("Unix epoch MJD(UTC): {:.9}", unix_epoch_mjd.to::<MJD>());
    println!("UTC POSIX         : {:.3}", utc.try_to::<Unix>().unwrap());

    // suppress unused warnings
    let _ = Time::<UTC>::from_chrono(chrono::Utc::now());
}
