// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Axis / representation showcase for the promoted tempoch API.
//!
//! Run with:
//! ```sh
//! cargo run -p tempoch --example timescales
//! ```

use chrono::Utc;
use qtty::{Day, Second};
use tempoch::{
    GpsSeconds, JulianDays, ModifiedJulianDays, SISeconds, Time, TimeContext, UnixSeconds, POSIX,
    TAI, TCB, TCG, TDB, TT, UT1, UTC,
};

fn main() {
    let ctx = TimeContext::new();

    let tt_j2000: Time<TT, JulianDays> = Time::from_julian_days(Day::new(2_451_545.0)).unwrap();
    let tt: Time<TT> = tt_j2000.repr();
    let mjd: Time<TT, ModifiedJulianDays> = tt.repr();
    let si: Time<TT, SISeconds> = tt.repr();

    let tai = tt.to::<TAI>();
    let tdb = tt.to::<TDB>();
    let tcg = tt.to::<TCG>();
    let tcb = tt.to::<TCB>();
    let utc = tt.to::<UTC>().to_chrono().unwrap();
    let ut1 = tt.to_with::<UT1>(&ctx).unwrap();

    println!("Reference epoch: J2000 TT");
    println!("  JD(TT)   : {:.9}", tt_j2000.julian_days().value());
    println!("  MJD(TT)  : {:.9}", mjd.modified_julian_days().value());
    println!("  TT(s)    : {:.3}", si.seconds().value());
    println!(
        "  UTC      : {}",
        utc.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    );
    println!("  TAI(s)   : {:.3}", tai.si_seconds().value());
    println!("  TDB(s)   : {:.6}", tdb.si_seconds().value());
    println!("  TCG(s)   : {:.6}", tcg.si_seconds().value());
    println!("  TCB(s)   : {:.6}", tcb.si_seconds().value());
    println!("  UT1(s)   : {:.6}", ut1.si_seconds().value());
    println!(
        "  TT-UT1   : {:.3} s",
        (tt.si_seconds() - ut1.si_seconds()).value()
    );

    let posix =
        Time::<UTC, UnixSeconds<POSIX>>::from_unix_seconds(Second::new(1_704_067_200.0)).unwrap();
    let gps: Time<TAI, GpsSeconds> = posix.to::<TAI>().repr();

    println!();
    println!("Civil / transport representations:");
    println!(
        "  POSIX seconds : {:.3}",
        posix.unix_seconds().unwrap().value()
    );
    println!("  GPS seconds   : {:.3}", gps.gps_seconds().value());

    let now_utc = Time::<UTC>::from_chrono(Utc::now());
    let now_tdb = now_utc.to::<TDB>();

    println!();
    println!("Current instant:");
    println!("  UTC      : {}", now_utc.to_chrono().unwrap());
    println!("  TDB(s)   : {:.3}", now_tdb.si_seconds().value());
}
