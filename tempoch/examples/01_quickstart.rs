use chrono::Utc;
use tempoch::{Jd, Mjd, Time, TT, UTC};

fn main() {
    let utc_now = Time::<UTC>::from_chrono(Utc::now());
    let tt_now: Time<TT> = utc_now.to_scale();

    // Reformat to JD and MJD for display
    let tt_jd: Time<TT, Jd> = tt_now.reformat();
    let tt_mjd: Time<TT, Mjd> = tt_now.reformat();

    println!("UTC       : {}", utc_now.to_chrono().unwrap());
    println!("TT in JD  : {tt_jd:.9}");
    println!("TT in MJD : {tt_mjd:.9}");
}
