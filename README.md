# tempoch

[![Crates.io](https://img.shields.io/crates/v/tempoch.svg)](https://crates.io/crates/tempoch)
[![Docs](https://docs.rs/tempoch/badge.svg)](https://docs.rs/tempoch)
[![Code Quality](https://github.com/Siderust/tempoch/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Siderust/tempoch/actions/workflows/ci.yml)

Typed astronomical time primitives for Rust.

`tempoch` provides:

- `Time<S, F>` instants parameterized by a physical or civil scale (`TT`,
  `TAI`, `UTC`, `UT1`, `TDB`, `TCG`, `TCB`) and a numeric format (`J2000s`,
  `Jd`, `Mjd`).
- Compile-time conversion witnesses for infallible (`.to_scale::<S>()`) and
  context-required (`.to_scale_with::<S>(ctx)`) routes.
- Format re-encoding (`.reformat::<F>()`) to switch between `J2000s`, `Jd`,
  and `Mjd` representations without changing the physical scale.
- UTC conversion through `chrono`, covering 1961 onward and leap-second
  aware; round-trip precision is limited by `f64` storage (~100 µs near J2000).
- Automatic `ΔT = TT - UT1` handling for `UT1` conversions via an explicit
  `TimeContext`. Opt into daily IERS Earth Orientation Parameters (finals2000A.all)
  with `TimeContext::with_builtin_eop()` for ≲ 10 ms UT1 accuracy inside
  the compiled coverage window; raw values are available under
  `tempoch::eop` and bracketed by the public `EOP_START_MJD` /
  `EOP_OBSERVED_END_MJD` / `EOP_END_MJD` constants.
- TT↔TDB conversion via the built-in seven-term Fairhead–Bretagnon
  approximation. The crate documents microsecond-level accuracy only inside
  the public `constats::TDB_TT_MODEL_HIGH_ACCURACY_START_JD` →
  `constats::TDB_TT_MODEL_HIGH_ACCURACY_END_JD` interval (about
  1600-01-01 to 2200-01-01 TT).
- Julian Day, Modified Julian Day, and SI-second accessors on their
  respective format types (`Time<S, Jd>`, `Time<S, Mjd>`, `Time<S, J2000s>`).
- Unix/POSIX timestamps via `Time::<UTC>::from_unix_seconds` / `unix_seconds`.
- GPS transport values via `Time::<TAI>::from_gps_seconds` / `gps_seconds`.
- Compiled time-data tables generated from official UTC-TAI and Delta T
  sources.
- Optional runtime refresh and cache management under `tempoch::runtime_data`
  when the `runtime-data` feature is enabled.
- Public typed epoch/offset constants under `tempoch::constats`, such as
  `J2000_JD_TT`, `TT_MINUS_TAI`, and `DELTA_T_PREDICTION_HORIZON_MJD`.
- A utility `Interval<T>` type for half-open time ranges over `Time<A>`,
  with intersection, normalization, validation, and complement helpers.

**Storage model:** `Time<A>` stores a single `f64` second count since J2000 TT
on the target axis. Precision therefore depends on the epoch magnitude; around
contemporary dates the floor is sub-microsecond, but it degrades as the
absolute second count grows.

The compiled modern ΔT series runs through MJD 63871 (`2033-10-01`). Beyond
that date UT1 conversions fail with `ConversionError::Ut1HorizonExceeded`.
Use the exported `DELTA_T_PREDICTION_HORIZON_MJD` typed `qtty::Day` constant
to reference the compiled boundary programmatically.

## Installation

```toml
[dependencies]
tempoch = "0.4"
```

Enable runtime refresh support explicitly if you want to fetch current
timekeeping data at runtime:

```toml
[dependencies]
tempoch = { version = "0.4", features = ["runtime-data"] }
```

## Quick Start

```rust
use chrono::Utc;
use tempoch::{Jd, Mjd, Time, TT, UTC};

let utc_now = Time::<UTC>::from_chrono(Utc::now());
let tt_now: Time<TT> = utc_now.to_scale();

// Reformat to JD / MJD for display (scale unchanged)
let tt_jd: Time<TT, Jd> = tt_now.reformat();
let tt_mjd: Time<TT, Mjd> = tt_now.reformat();

println!("UTC       : {}", utc_now.to_chrono().unwrap());
println!("JD(TT)    : {:.9}", tt_jd.julian_days());
println!("MJD(TT)   : {:.9}", tt_mjd.modified_julian_days());
```

## Period Operations

```rust
use tempoch::{
    complement_within, intersect_periods, Period, Mjd, TT,
};

let day = Period::<TT, Mjd>::new(61_000.0, 61_001.0);
let a = vec![
    Period::<TT, Mjd>::new(61_000.1, 61_000.4),
    Period::<TT, Mjd>::new(61_000.6, 61_000.9),
];
let b = vec![
    Period::<TT, Mjd>::new(61_000.2, 61_000.3),
    Period::<TT, Mjd>::new(61_000.7, 61_000.8),
];

let overlap = intersect_periods(&a, &b);
let gaps = complement_within(day, &a);

assert_eq!(overlap.len(), 2);
assert_eq!(gaps.len(), 3);
```

## Examples

- `cargo run --example quickstart`
- `cargo run --example periods`
- `cargo run --example timescales`

## Runtime Time Data

The default `tempoch` path remains compile-time and network-free. If you need
fresh UTC-TAI history, modern Delta T, and daily IERS EOP at runtime, enable
the `runtime-data` feature and use `tempoch::runtime_data` explicitly.

Downloaded raw upstream files are cached under `~/.tempoch/data` by default.
Set `TEMPOCH_DATA_DIR` to override that location.

```rust,no_run
use tempoch::runtime_data::{RuntimeTimeContext, TimeDataManager};
use tempoch::{Jd, Time, TT, UT1, UTC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = TimeDataManager::new()?;
    let data = manager.refresh_and_load()?;
    let ctx = RuntimeTimeContext::new(data);

    let tt = Time::<TT, Jd>::from_julian_days(2_460_000.25.into())?;
    let ut1: Time<UT1, Jd> = tt.to_scale_with_runtime(&ctx)?;

    let unix = Time::<UTC>::from_unix_seconds_with_runtime(1_700_000_000.0.into(), &ctx)?;
    let back = unix.unix_seconds_with_runtime(&ctx)?;

    println!("UT1 JD     : {:.9}", ut1.julian_days());
    println!("Unix roundtrip: {:.3}", back.value());
    Ok(())
}
```

## Time Data Updates

The compile-time path still uses checked-in generated tables in `tempoch-core`.
The dedicated Rust CLI `tempoch-time-data-updater` regenerates those committed
files from the official UTC-TAI, Delta T, and IERS finals2000A.all sources.
Its fetch/parse/build pipeline now reuses the same shared runtime-data logic
that powers the optional `tempoch::runtime_data` feature. To refresh locally:

```bash
cargo run -p tempoch-time-data-updater
cargo test --all-features
```

To verify that the committed generated files are still in sync with upstream
(this is also enforced in CI):

```bash
cargo run -p tempoch-time-data-updater -- --check
```

A scheduled GitHub Actions workflow runs the refresh automatically and pushes
the resulting commit directly to `main` when the generated tables or their
source hashes change.

## Tests and Coverage

```bash
cargo test --all-targets
cargo test --doc
cargo +nightly llvm-cov --workspace --all-features --doctests --summary-only
```

Coverage is gated in CI at **>= 90% line coverage**.

## License

AGPL-3.0-only
