use chrono::Utc;
use tempoch::{Time, TT, UTC};

fn main() {
    let utc_now = Time::<UTC>::from_chrono(Utc::now());
    let tt_now = utc_now.to::<TT>();

    println!("UTC       : {}", utc_now.to_chrono().unwrap());
    println!("JD(TT)    : {:.9}", tt_now.julian_days());
    println!("MJD(TT)   : {:.9}", tt_now.modified_julian_days());
}
