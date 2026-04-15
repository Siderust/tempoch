// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! One section per time scale: construction, conversion, and key properties.
//!
//! Each section starts from J2000 TT and shows the corresponding value on
//! that axis, plus whatever is physically meaningful (offsets, drift rates,
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
    Time, TimeContext,
    TAI, TCB, TCG, TDB, TT, UT1, UTC,
};

fn main() {
    // Common reference: J2000.0 epoch expressed on TT.
    let j2000_tt = Time::<TT>::from_julian_days(J2000_JD_TT).unwrap();
    let ctx = TimeContext::new();

    // ─────────────────────────────────────────────────────────────────────────
    // TT — Terrestrial Time
    //
    // The canonical "theory" scale: SI seconds on the rotating geoid.
    // Offset from TAI: TT = TAI + 32.184 s (exact, fixed by convention).
    // All planetary theories (VSOP87, ELP2000, DE4xx) use TT as their
    // independent variable.
    // ─────────────────────────────────────────────────────────────────────────
    println!("── TT: Terrestrial Time ──────────────────────────────────────");
    println!("  JD(TT)  : {:.9}", j2000_tt.julian_days());
    println!("  MJD(TT) : {:.9}", j2000_tt.modified_julian_days());
    println!("  SI(s)   : {:.3}", j2000_tt.si_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // TAI — International Atomic Time
    //
    // Atomic realization produced by averaging hundreds of atomic clocks.
    // TT = TAI + 32.184 s; the same SI second tick rate.
    // TAI is the root of the GPS scale (GPS = TAI − 19 s).
    // ─────────────────────────────────────────────────────────────────────────
    let tai = j2000_tt.to::<TAI>();
    println!();
    println!("── TAI: International Atomic Time ────────────────────────────");
    println!("  SI(s)       : {:.3}", tai.si_seconds());
    println!("  TT − TAI    : {:.3}", j2000_tt.si_seconds() - tai.si_seconds());
    println!("  GPS seconds : {:.3}", tai.gps_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // UTC — Coordinated Universal Time
    //
    // Civil time kept within 0.9 s of UT1 via occasional positive leap
    // seconds.  Internally stored as TAI-equivalent seconds plus a leap label.
    // The only axis that carries a chrono DateTime and a POSIX timestamp.
    // ─────────────────────────────────────────────────────────────────────────
    let utc = j2000_tt.to::<UTC>();
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
    //
    // Tracks Earth's actual rotation angle (UT1 ≈ mean solar time).
    // Differs from TT by ΔT = TT − UT1, which varies irregularly; conversion
    // requires a TimeContext carrying a ΔT model or IERS bulletin data.
    // ─────────────────────────────────────────────────────────────────────────
    let ut1 = j2000_tt.to_with::<UT1>(&ctx).unwrap();
    let delta_t = j2000_tt.si_seconds() - ut1.si_seconds();
    println!();
    println!("── UT1: Universal Time 1 (rotation-angle scale) ─────────────");
    println!("  SI(s)    : {:.6}", ut1.si_seconds());
    println!("  ΔT(TT−UT1): {:.3}", delta_t);

    // ─────────────────────────────────────────────────────────────────────────
    // TDB — Barycentric Dynamical Time
    //
    // Independent variable for Solar System barycentre-based ephemerides
    // (e.g. JPL DE4xx).  Differs from TT by a periodic term up to ≈ 1.7 ms
    // caused by Earth's orbital eccentricity and relativistic time dilation.
    // The instantaneous offset is small; only meaningful for long baselines.
    // ─────────────────────────────────────────────────────────────────────────
    let tdb = j2000_tt.to::<TDB>();
    println!();
    println!("── TDB: Barycentric Dynamical Time ───────────────────────────");
    println!("  SI(s)   : {:.6}", tdb.si_seconds());
    println!("  TT−TDB  : {:.9}", j2000_tt.si_seconds() - tdb.si_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // TCG — Geocentric Coordinate Time
    //
    // Coordinate time for the Geocentric Celestial Reference System (GCRS).
    // Runs faster than TT by L_G = 6.969 290 134 × 10⁻¹⁰ (gravitational
    // blueshift relative to TT's geoid-surface rate).
    // ─────────────────────────────────────────────────────────────────────────
    let tcg = j2000_tt.to::<TCG>();
    let tcg_next_day = (j2000_tt + Second::new(86_400.0)).to::<TCG>();
    let drift_per_day = (tcg_next_day - tcg) - Second::new(86_400.0);
    println!();
    println!("── TCG: Geocentric Coordinate Time ───────────────────────────");
    println!("  SI(s)        : {:.6}", tcg.si_seconds());
    println!("  Drift/day    : {:.4} μs", drift_per_day.value() * 1e6);

    // ─────────────────────────────────────────────────────────────────────────
    // TCB — Barycentric Coordinate Time
    //
    // Coordinate time for the Barycentric Celestial Reference System (BCRS).
    // Runs faster than TDB by L_B = 1.550 519 768 × 10⁻⁸; the offset grows
    // linearly with time and can reach seconds over years.
    // ─────────────────────────────────────────────────────────────────────────
    let tcb = tdb.to::<TCB>();
    println!();
    println!("── TCB: Barycentric Coordinate Time ──────────────────────────");
    println!("  SI(s)    : {:.6}", tcb.si_seconds());
    println!("  TT − TCB : {:.3}", j2000_tt.si_seconds() - tcb.si_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // Live snapshot: current UTC and its TDB equivalent.
    // ─────────────────────────────────────────────────────────────────────────
    let now_utc = Time::<UTC>::from_chrono(Utc::now());
    let now_tdb = now_utc.to::<TDB>();
    println!();
    println!("── Current instant ───────────────────────────────────────────");
    println!("  UTC : {}", now_utc.to_chrono().unwrap());
    println!("  TDB : {:.3}", now_tdb.si_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // Summary: all J2000 instants on one line each.
    // ─────────────────────────────────────────────────────────────────────────
    println!();
    println!("── J2000 summary ─────────────────────────────────────────────");
    let ut1_j2000 = j2000_tt.to_with::<UT1>(&ctx).unwrap();
    for (name, secs) in [
        ("TT ", j2000_tt.si_seconds()),
        ("TAI", tai.si_seconds()),
        ("TDB", tdb.si_seconds()),
        ("TCG", tcg.si_seconds()),
        ("TCB", tcb.si_seconds()),
        ("UT1", ut1_j2000.si_seconds()),
    ] {
        println!("  {name}  {:.3}", secs);
    }
}
