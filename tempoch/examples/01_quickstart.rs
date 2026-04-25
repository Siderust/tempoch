use chrono::Utc;
use tempoch::{J2000s, Time, TimeContext, JD, MJD, TT, UTC};

fn main() {
    let ctx = TimeContext::new();
    let utc_now = Time::<UTC>::try_from_chrono_with(Utc::now(), &ctx).unwrap();
    let tt_now: Time<TT> = utc_now.to::<TT>();

    println!("UTC chrono : {}", utc_now.to_chrono_with(&ctx).unwrap());
    println!("TT seconds : {:.6}", tt_now.to::<J2000s>());
    println!("TT JD      : {:.9}", tt_now.to::<JD>());
    println!("TT MJD     : {:.9}", tt_now.to::<MJD>());
}
