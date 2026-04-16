// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! One section per time scale: construction, conversion, and key properties.
//!
//! Each section starts from J2000 TT and shows the corresponding value on
//! that scale, plus whatever is physically meaningful (offsets, drift rates,
//! civil representations, etc.).
//!
//! Run with:
//! ```sh
//! cargo run -p tempoch --example timescales
//! ```

use chrono::Utc;
use qtty::Second;
use tempoch::{
    constats::J2000_JD_TT,
    Jd, Mjd, Time, TimeContext, TAI, TCB, TCG, TDB, TT, UT1, UTC,
};

fn main() {
    // Common reference: J2000.0 epoch expressed on TT in J2000s format.
    let j2000_tt = Time::<TT, Jd>::from_julian_days(J2000_JD_TT).unwrap();
    let j2000_tt_s: Time<TT> = j2000_tt.reformat();
    let j2000_tt_mjd: Time<TT, Mjd> = j2000_tt.reformat();
    let ctx = TimeContext::new();

    // ─────────────────────────────────────────────────────────────────────────
    // TT — Terrestrial Time
    // ─────────────────────────────────────────────────────────────────────────
    println!("── TT: Terrestrial Time ──────────────────────────────────────");
    println!("  JD(TT)  : {:.9}", j2000_tt.julian_days());
    println!("  MJD(TT) : {:.9}", j2000_tt_mjd.modified_julian_days());
    println!("  SI(s)   : {:.3}", j2000_tt_s.si_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // TAI — International Atomic Time
    // ─────────────────────────────────────────────────────────────────────────
    let tai: Time<TAI> = j2000_tt_s.to_scale();
    println!();
    println!("── TAI: International Atomic Time ────────────────────────────");
    println!("  SI(s)       : {:.3}", tai.si_seconds());
    println!("  TT − TAI    : {:.3}", j2000_tt_s.si_seconds() - tai.si_seconds());
    println!("  GPS seconds : {:.3}", tai.gps_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // UTC — Coordinated Universal Time
    // ─────────────────────────────────────────────────────────────────────────
    let utc: Time<UTC> = j2000_tt_s.to_scale();
    println!();
    println!("── UTC: Coordinated Universal Time ──────────────────────────");
    println!(
        "  DateTime     : {}",
        utc.to_chrono()
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    );
    println!("  Unix seconds : {:.3}", utc.unix_seconds().unwrap());
    println!("  Is leap sec  : {}", utc.is_leap_second());

    // ─────────────────────────────────────────────────────────────────────────
    // UT1 — Universal Time 1
    // ─────────────────────────────────────────────────────────────────────────
    let ut1: Time<UT1> = j2000_tt_s.to_scale_with(&ctx).unwrap();
    let delta_t = j2000_tt_s.si_seconds() - ut1.si_seconds();
    println!();
    println!("── UT1: Universal Time 1 (rotation-angle scale) ─────────────");
    println!("  SI(s)    : {:.6}", ut1.si_seconds());
    println!("  ΔT(TT−UT1): {:.3}", delta_t);

    // ─────────────────────────────────────────────────────────────────────────
    // TDB — Barycentric Dynamical Time
    // ─────────────────────────────────────────────────────────────────────────
    let tdb: Time<TDB> = j2000_tt_s.to_scale();
    println!();
    println!("── TDB: Barycentric Dynamical Time ───────────────────────────");
    println!("  SI(s)   : {:.6}", tdb.si_seconds());
    println!("  TT−TDB  : {:.9}", j2000_tt_s.si_seconds() - tdb.si_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // TCG — Geocentric Coordinate Time
    // ─────────────────────────────────────────────────────────────────────────
    let tcg: Time<TCG> = j2000_tt_s.to_scale();
    let tcg_next_day: Time<TCG> = (j2000_tt_s + Second::new(86_400.0)).to_scale();
    let drift_per_day = (tcg_next_day - tcg) - Second::new(86_400.0);
    println!();
    println!("── TCG: Geocentric Coordinate Time ───────────────────────────");
    println!("  SI(s)        : {:.6}", tcg.si_seconds());
    println!("  Drift/day    : {:.4} μs", drift_per_day.value() * 1e6);

    // ─────────────────────────────────────────────────────────────────────────
    // TCB — Barycentric Coordinate Time
    // ─────────────────────────────────────────────────────────────────────────
    let tcb: Time<TCB> = tdb.to_scale();
    println!();
    println!("── TCB: Barycentric Coordinate Time ──────────────────────────");
    println!("  SI(s)    : {:.6}", tcb.si_seconds());
    println!("  TT − TCB : {:.3}", j2000_tt_s.si_seconds() - tcb.si_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // Live snapshot: current UTC and its TDB equivalent.
    // ─────────────────────────────────────────────────────────────────────────
    let now_utc = Time::<UTC>::from_chrono(Utc::now());
    let now_tdb: Time<TDB> = now_utc.to_scale();
    println!();
    println!("── Current instant ───────────────────────────────────────────");
    println!("  UTC : {}", now_utc.to_chrono().unwrap());
    println!("  TDB : {:.3}", now_tdb.si_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // Summary
    // ─────────────────────────────────────────────────────────────────────────
    println!();
    println!("── J2000 summary ─────────────────────────────────────────────");
    let ut1_j2000: Time<UT1> = j2000_tt_s.to_scale_with(&ctx).unwrap();
    for (name, secs) in [
        ("TT ", j2000_tt_s.si_seconds()),
        ("TAI", tai.si_seconds()),
        ("TDB", tdb.si_seconds()),
        ("TCG", tcg.si_seconds()),
        ("TCB", tcb.si_seconds()),
        ("UT1", ut1_j2000.si_seconds()),
    ] {
        println!("  {name}  {:.3}", secs);
    }
}
