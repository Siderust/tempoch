// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Refresh runtime time-data tables and swap in the updated context.
//!
//! This example shows the intended runtime update flow:
//!   1. Try the current cached bundle, if any.
//!   2. Refresh the on-disk bundle from the upstream sources.
//!   3. Build a new `RuntimeTimeContext` from the refreshed tables.
//!   4. Use the refreshed context for UT1 and UTC helpers.
//!
//! Run with:
//! ```sh
//! cargo run -p tempoch --example 06_runtime_tables --features runtime-data
//! ```

#[cfg(feature = "runtime-data")]
use qtty::{Day, Second};
#[cfg(feature = "runtime-data")]
use tempoch::runtime_data::{RuntimeTimeContext, RuntimeTimeData, TimeDataManager};
#[cfg(feature = "runtime-data")]
use tempoch::{Time, JD, TT, UT1, UTC};

#[cfg(not(feature = "runtime-data"))]
fn main() {
    eprintln!("run with: cargo run -p tempoch --example 06_runtime_tables --features runtime-data");
}

#[cfg(feature = "runtime-data")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = TimeDataManager::new()?;
    println!("Cache directory : {}", manager.data_dir().display());

    let probe_tt = Time::<TT, JD>::from_julian_days(Day::new(2_460_000.25))?;
    let cached_ctx = match manager.load_cached() {
        Ok(cached) => {
            print_bundle("Cached bundle", &cached);
            print_runtime_values("Cached context", &cached.context(), probe_tt)?;
            Some(cached.context())
        }
        Err(err) => {
            println!("Cached bundle  : none ({err})");
            None
        }
    };

    println!();
    println!("Refreshing bundle from upstream sources...");
    let refreshed = manager.refresh_and_load()?;
    print_bundle("Refreshed bundle", &refreshed);

    let refreshed_ctx = refreshed.context();
    print_runtime_values("Refreshed context", &refreshed_ctx, probe_tt)?;

    if let Some(previous_ctx) = cached_ctx.as_ref() {
        let before: Time<UT1, JD> = probe_tt.to_scale_with_runtime(previous_ctx)?;
        let after: Time<UT1, JD> = probe_tt.to_scale_with_runtime(&refreshed_ctx)?;
        let delta_seconds = (after.julian_days() - before.julian_days()).value() * 86_400.0;
        println!("UT1 delta       : {:.6} s", delta_seconds);
    } else {
        println!("UT1 delta       : unavailable on first run (no previous cache)");
    }

    println!();
    println!("Runtime update means replacing the old RuntimeTimeContext with the refreshed one.");
    Ok(())
}

#[cfg(feature = "runtime-data")]
fn print_bundle(label: &str, data: &RuntimeTimeData) {
    let provenance = data.provenance();
    println!("{label}:");
    println!("  fetched_utc   : {}", provenance.fetched_utc());
    println!(
        "  delta-T end   : MJD {:.0}",
        data.delta_t_prediction_horizon_mjd().value()
    );
    println!(
        "  EOP observed  : MJD {:.0}",
        data.eop_observed_end_mjd().value()
    );
}

#[cfg(feature = "runtime-data")]
fn print_runtime_values(
    label: &str,
    ctx: &RuntimeTimeContext,
    probe_tt: Time<TT, JD>,
) -> Result<(), Box<dyn std::error::Error>> {
    let probe_ut1: Time<UT1, JD> = probe_tt.to_scale_with_runtime(ctx)?;
    let unix_utc = Time::<UTC>::from_unix_seconds_with_runtime(Second::new(1_700_000_000.0), ctx)?;
    let unix_roundtrip = unix_utc.unix_seconds_with_runtime(ctx)?;

    println!("{label}:");
    println!("  probe TT JD   : {probe_tt:.9}");
    println!("  probe UT1 JD  : {probe_ut1:.9}");
    println!("  Unix roundtrip: {:.3}", unix_roundtrip.value());
    Ok(())
}
