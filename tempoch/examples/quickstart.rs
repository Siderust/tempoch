use chrono::Utc;
use tempoch::{JulianDays, ModifiedJulianDays, Time, TT, UTC};

fn main() {
    let utc_now = Time::<UTC>::from_chrono(Utc::now());
    let tt_now = utc_now.to::<TT>();
    let jd_tt: Time<TT, JulianDays> = tt_now.repr();
    let mjd_tt: Time<TT, ModifiedJulianDays> = tt_now.repr();

    println!("UTC       : {}", utc_now.to_chrono().unwrap());
    println!("JD(TT)    : {:.9}", jd_tt.julian_days().value());
    println!("MJD(TT)   : {:.9}", mjd_tt.modified_julian_days().value());
}
