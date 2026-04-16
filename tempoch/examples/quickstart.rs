use chrono::Utc;
use tempoch::{Jd, Mjd, Time, TT, UTC};

fn main() {
    let utc_now = Time::<UTC>::from_chrono(Utc::now());
    let tt_now: Time<TT> = utc_now.to_scale();

    // Reformat to JD and MJD for display
    let tt_jd: Time<TT, Jd> = tt_now.reformat();
    let tt_mjd: Time<TT, Mjd> = tt_now.reformat();

    println!("UTC       : {}", utc_now.to_chrono().unwrap());
    println!("JD(TT)    : {:.9}", tt_jd.julian_days());
    println!("MJD(TT)   : {:.9}", tt_mjd.modified_julian_days());
}
