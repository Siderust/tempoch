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
    Time, TimeContext,
    TAI, TCB, TCG, TDB, TT, UT1, UTC,
};

fn main() {
    let ctx = TimeContext::new();

    let tt = Time::<TT>::from_julian_days(Day::new(2_451_545.0)).unwrap();

    let tai = tt.to::<TAI>();
    let tdb = tt.to::<TDB>();
    let tcg = tt.to::<TCG>();
    let tcb = tt.to::<TCB>();
    let utc = tt.to::<UTC>().to_chrono().unwrap();
    let ut1 = tt.to_with::<UT1>(&ctx).unwrap();

    println!("Reference epoch: J2000 TT");
    println!("  JD(TT)   : {:.9}", tt.julian_days());
    println!("  MJD(TT)  : {:.9}", tt.modified_julian_days());
    println!("  TT(s)    : {:.3}", tt.si_seconds());
    println!(
        "  UTC      : {}",
        utc.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    );
    println!("  TAI   : {:.3}", tai.si_seconds());
    println!("  TDB   : {:.6}", tdb.si_seconds());
    println!("  TCG   : {:.6}", tcg.si_seconds());
    println!("  TCB   : {:.6}", tcb.si_seconds());
    println!("  UT1   : {:.6}", ut1.si_seconds());
    println!(
        "  TT-UT1   : {:.3}",
        tt.si_seconds() - ut1.si_seconds()
    );

    let posix = Time::<UTC>::from_unix_seconds(Second::new(1_704_067_200.0)).unwrap();
    let gps = posix.to::<TAI>();

    println!();
    println!("Civil / transport representations:");
    println!(
        "  POSIX : {:.3}",
        posix.unix_seconds().unwrap()
    );
    println!("  GPS   : {:.3}", gps.gps_seconds());

    let now_utc = Time::<UTC>::from_chrono(Utc::now());
    let now_tdb = now_utc.to::<TDB>();

    println!();
    println!("Current instant:");
    println!("  UTC   : {}", now_utc.to_chrono().unwrap());
    println!("  TDB   : {:.3}", now_tdb.si_seconds());
}
