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
  `TimeContext`.
- Julian Day, Modified Julian Day, and SI-second accessors on their
  respective format types (`Time<S, Jd>`, `Time<S, Mjd>`, `Time<S, J2000s>`).
- Unix/POSIX timestamps via `Time::<UTC>::from_unix_seconds` / `unix_seconds`.
- GPS transport values via `Time::<TAI>::from_gps_seconds` / `gps_seconds`.
- Compiled time-data tables generated from official UTC-TAI and Delta T
  sources.
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

## Time Data Updates

The crate compiles generated time-data tables into `tempoch-core`, rather than
fetching them at runtime. To refresh the checked-in data locally:

```bash
python3 scripts/update_time_data.py
cargo test --all-features
```

A scheduled GitHub Actions workflow also runs this refresh automatically and
opens a pull request when the generated tables change.

## Tests and Coverage

```bash
cargo test --all-targets
cargo test --doc
cargo +nightly llvm-cov --workspace --all-features --doctests --summary-only
```

Coverage is gated in CI at **>= 90% line coverage**.

## License

AGPL-3.0-only
