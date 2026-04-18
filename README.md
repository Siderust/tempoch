# tempoch

[![Crates.io](https://img.shields.io/crates/v/tempoch.svg)](https://crates.io/crates/tempoch)
[![Docs](https://docs.rs/tempoch/badge.svg)](https://docs.rs/tempoch)
[![Code Quality](https://github.com/Siderust/tempoch/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Siderust/tempoch/actions/workflows/ci.yml)

Typed astronomical time primitives for Rust.

`tempoch` provides:

- `Time<S, F>` instants parameterized by a physical or civil scale (`TT`,
  `TAI`, `UTC`, `UT1`, `TDB`, `TCG`, `TCB`) and a numeric format (`J2000s`,
  `JD`, `MJD`).
- Compile-time conversion witnesses for infallible (`.to_scale::<S>()`) and
  context-required (`.to_scale_with::<S>(ctx)`) routes.
- Format re-encoding (`.reformat::<F>()`) to switch between `J2000s`, `JD`,
  and `MJD` representations without changing the physical scale.
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
  respective format types (`Time<S, JD>`, `Time<S, MJD>`, `Time<S, J2000s>`).
- Unix/POSIX timestamps via `Time::<UTC>::from_unix_seconds` / `unix_seconds`.
- GPS transport values via `Time::<TAI>::from_gps_seconds` / `gps_seconds`.
- Compiled time-data tables generated from official UTC-TAI and Delta T
  sources.
- Optional `serde` support for `Time<S, F>` as raw format values and
  `Period<S, F>` / `Interval<T>` as `{start, end}` objects.
- Optional automatic runtime freshness when the `runtime-data` feature is
  enabled, while keeping the same public API.
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

Enable runtime freshness explicitly if you want `tempoch` to prefer a cached
or auto-refreshed time-data bundle at runtime while keeping the ordinary API:

```toml
[dependencies]
tempoch = { version = "0.4", features = ["runtime-data"] }
```

Enable `serde` if you want to serialize typed times and periods:

```toml
[dependencies]
tempoch = { version = "0.4", features = ["serde"] }
```

Features compose normally:

```toml
[dependencies]
tempoch = { version = "0.4", features = ["serde", "runtime-data"] }
```

## Serde

With the `serde` feature enabled:

- `Time<S, F>` serializes as the underlying format value only.
- `Period<S, F>` serializes as `{"start": ..., "end": ...}`.
- Scale and format remain type-level and are not embedded in the payload.

```rust
use tempoch::{MJD, Period, Time, TT};

let tt = Time::<TT>::from(42.5);
let period = Period::<TT, MJD>::new(61_000.0, 61_001.0);

assert_eq!(serde_json::to_string(&tt).unwrap(), "42.5");
assert_eq!(
    serde_json::to_string(&period).unwrap(),
    r#"{"start":61000.0,"end":61001.0}"#
);
```

## Quick Start

```rust
use chrono::Utc;
use tempoch::{JD, MJD, Time, TT, UTC};

let utc_now = Time::<UTC>::from_chrono(Utc::now());
let tt_now: Time<TT> = utc_now.to_scale();

// Reformat to JD / MJD for display (scale unchanged)
let tt_jd: Time<TT, JD> = tt_now.reformat();
let tt_mjd: Time<TT, MJD> = tt_now.reformat();

println!("UTC       : {}", utc_now.to_chrono().unwrap());
println!("TT in JD  : {tt_jd:.9}");
println!("TT in MJD : {tt_mjd:.9}");
```

## Period Operations

```rust
use tempoch::{
  complement_within, intersect_periods, Period, MJD, TT,
};

let day = Period::<TT, MJD>::new(61_000.0, 61_001.0);
let a = vec![
  Period::<TT, MJD>::new(61_000.1, 61_000.4),
  Period::<TT, MJD>::new(61_000.6, 61_000.9),
];
let b = vec![
  Period::<TT, MJD>::new(61_000.2, 61_000.3),
  Period::<TT, MJD>::new(61_000.7, 61_000.8),
];

let overlap = intersect_periods(&a, &b);
let gaps = complement_within(day, &a);

assert_eq!(overlap.len(), 2);
assert_eq!(gaps.len(), 3);
```

## Examples

- `cargo run --example 01_quickstart`
- `cargo run --example 02_timescales`
- `cargo run --example 03_formats`
- `cargo run --example 04_periods`
- `cargo run --example 05_serde --features serde`
- `cargo run -p tempoch --example 06_runtime_tables --features runtime-data`

## Runtime Time Data

The default `tempoch` path remains compile-time and network-free. If you need
fresher UTC-TAI history, modern Delta T, and daily IERS EOP at runtime, enable
the `runtime-data` feature. The public API does not change: `TimeContext`,
`Time::to_scale_with`, and the normal UTC civil helpers automatically consult a
cached bundle in `~/.tempoch/data`, refreshing it once on first use when the
cache is missing, invalid, or older than 24 hours.

Set `TEMPOCH_DATA_DIR` to override the cache location.

For a runnable example that uses the ordinary API under `runtime-data`, run:

```bash
cargo run -p tempoch --example 06_runtime_tables --features runtime-data
```

```rust,no_run
use tempoch::{JD, Time, TimeContext, TT, UT1, UTC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TimeContext::with_builtin_eop();
    let tt = Time::<TT, JD>::from_julian_days(2_460_000.25.into())?;
    let ut1: Time<UT1, JD> = tt.to_scale_with::<UT1>(&ctx)?;

    let unix = Time::<UTC>::from_unix_seconds(1_700_000_000.0.into())?;
    let back = unix.unix_seconds()?;

    println!("UT1 JD     : {ut1:.9}");
    println!("Unix roundtrip: {:.3}", back.value());
    Ok(())
}
```

## Time Data Updates

The compile-time path still uses checked-in generated tables in `tempoch-core`.
The dedicated Rust CLI `tempoch-time-data-updater` regenerates those committed
files from the official UTC-TAI, Delta T, and IERS finals2000A.all sources.
Its fetch/parse/build pipeline now reuses the same shared support crate that
powers the optional `runtime-data` feature. To refresh locally:

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
