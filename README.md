# tempoch

[![Crates.io](https://img.shields.io/crates/v/tempoch.svg)](https://crates.io/crates/tempoch)
[![Docs](https://docs.rs/tempoch/badge.svg)](https://docs.rs/tempoch)
[![Code Quality](https://github.com/Siderust/tempoch/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Siderust/tempoch/actions/workflows/ci.yml)

Typed astronomical time primitives for Rust.

`tempoch` provides:

- `Time<S>` instants parameterized by a physical or civil scale (`TT`,
  `TAI`, `UTC`, `UT1`, `TDB`, `TCG`, `TCB`).
- Unified target-based conversions:
  - `.to::<TT>()`, `.to::<UTC>()`, `.to::<TDB>()` for infallible scale routes
  - `.to_with::<UT1>(&ctx)` for context-backed routes
  - `.to::<JD>()`, `.to::<MJD>()`, `.to::<J2000s>()` for coordinate views
  - `.try_to::<UnixSecs>()` and `.to::<GpsSecs>()` for transport encodings
- UTC conversion through `chrono`, covering 1961 onward and leap-second aware.
- Automatic `ΔT = TT - UT1` handling for `UT1` conversions via an explicit
  `TimeContext`. For the currently compiled bundle fetched 2026-04-18, the
  default monthly-ΔT path stays within 10 ms of the bundled daily IERS-derived
  path over the observed overlap through 2026-04-16, and within 0.2 s over the
  compiled short-range prediction overlap through 2027-04-24. Opt into
  `TimeContext::with_builtin_eop()` when you want the highest-fidelity bundled
  UT1 path; raw EOP values are available under `tempoch::eop` and bracketed by
  the public `EOP_START_MJD` / `EOP_OBSERVED_END_MJD` / `EOP_END_MJD`
  constants.
- TT↔TDB conversion via the built-in seven-term Fairhead–Bretagnon
  approximation from USNO Circular 179. The crate documents about 10 µs
  accuracy only inside the public
  `constats::TDB_TT_MODEL_HIGH_ACCURACY_START_JD` →
  `constats::TDB_TT_MODEL_HIGH_ACCURACY_END_JD` interval (about 1600-01-01 to
  2200-01-01 TT).
- Julian Day, Modified Julian Day, and SI-second views via `JD`, `MJD`, and
  `J2000s` conversion targets.
- Unix/POSIX timestamps via `Time::<UTC>::from_unix_seconds` and
  `.try_to::<UnixSecs>()`.
- GPS transport values via `Time::<TAI>::from_gps_seconds` and `.to::<GpsSecs>()`.
- Compiled time-data tables generated from official UTC-TAI and Delta T
  sources.
- Optional `serde` support for `Time<S>` as `{"hi","lo"}` and
  `Period<S>` / `Interval<T>` as `{start, end}` objects.
- Optional automatic runtime freshness when the `runtime-data` feature is
  enabled, while keeping the same public API.
- Public typed epoch/offset constants under `tempoch::constats`, such as
  `J2000_JD_TT`, `TT_MINUS_TAI`, and `DELTA_T_PREDICTION_HORIZON_MJD`.
- A utility `Interval<T>` type for half-open time ranges over `Time<A>`,
  with intersection, normalization, validation, and complement helpers.

**Storage model:** `Time<S>` stores a compensated `(hi, lo)` pair of seconds
since J2000 TT on the target axis. Tags such as `JD`, `MJD`, `UnixSecs`, and
`GpsSecs` are conversion targets, not storage types.

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

- `Time<S>` serializes as `{"hi": ..., "lo": ...}`.
- `Period<S>` serializes as `{"start": ..., "end": ...}`.
- The scale remains type-level and is not embedded in the payload.

```rust
use qtty::Second;
use tempoch::{Period, Time, TT};

let tt = Time::<TT>::from_j2000_seconds(Second::new(42.5)).unwrap();
let period = Period::<TT>::new(42.5, 43.5);

assert_eq!(serde_json::to_string(&tt).unwrap(), r#"{"hi":42.5,"lo":0.0}"#);
assert_eq!(
    serde_json::to_string(&period).unwrap(),
    r#"{"start":{"hi":42.5,"lo":0.0},"end":{"hi":43.5,"lo":0.0}}"#
);
```

## Quick Start

```rust
use chrono::Utc;
use tempoch::{JD, MJD, Time, TT, UTC};

let utc_now = Time::<UTC>::from_chrono(Utc::now());
let tt_now: Time<TT> = utc_now.to::<TT>();

println!("UTC       : {}", utc_now.to_chrono().unwrap());
println!("TT in JD  : {:.9}", tt_now.to::<JD>().value());
println!("TT in MJD : {:.9}", tt_now.to::<MJD>().value());
```

## Period Operations

```rust
use qtty::Day;
use tempoch::{complement_within, intersect_periods, Period, Time, TT};

let day = Period::<TT>::new(
  Time::<TT>::from_modified_julian_days(Day::new(61_000.0)).unwrap(),
  Time::<TT>::from_modified_julian_days(Day::new(61_001.0)).unwrap(),
);
let a = vec![
  Period::<TT>::new(
    Time::<TT>::from_modified_julian_days(Day::new(61_000.1)).unwrap(),
    Time::<TT>::from_modified_julian_days(Day::new(61_000.4)).unwrap(),
  ),
  Period::<TT>::new(
    Time::<TT>::from_modified_julian_days(Day::new(61_000.6)).unwrap(),
    Time::<TT>::from_modified_julian_days(Day::new(61_000.9)).unwrap(),
  ),
];
let b = vec![
  Period::<TT>::new(
    Time::<TT>::from_modified_julian_days(Day::new(61_000.2)).unwrap(),
    Time::<TT>::from_modified_julian_days(Day::new(61_000.3)).unwrap(),
  ),
  Period::<TT>::new(
    Time::<TT>::from_modified_julian_days(Day::new(61_000.7)).unwrap(),
    Time::<TT>::from_modified_julian_days(Day::new(61_000.8)).unwrap(),
  ),
];

let overlap = intersect_periods(&a, &b);
let gaps = complement_within(day, &a);

assert_eq!(overlap.len(), 2);
assert_eq!(gaps.len(), 3);
```

## Examples

- `cargo run --example 01_quickstart`
- `cargo run --example 02_scales`
- `cargo run --example 03_formats`
- `cargo run --example 04_periods`
- `cargo run --example 05_serde --features serde`
- `cargo run -p tempoch --example 06_runtime_tables --features runtime-data`
- `cargo run --example 07_conversions`

## Runtime Time Data

The default `tempoch` path remains compile-time and network-free. If you need
fresher UTC-TAI history, modern Delta T, and daily IERS EOP at runtime, enable
the `runtime-data` feature. The public API does not change: `TimeContext`,
`Time::to_with`, and the normal UTC civil helpers automatically consult a
cached bundle in `~/.tempoch/data`, refreshing it once on first use when the
cache is missing, invalid, or older than 24 hours.

Set `TEMPOCH_DATA_DIR` to override the cache location.

For a runnable example that uses the ordinary API under `runtime-data`, run:

```bash
cargo run -p tempoch --example 06_runtime_tables --features runtime-data
```

```rust,no_run
use tempoch::{JD, UnixSecs, Time, TimeContext, TT, UT1, UTC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TimeContext::with_builtin_eop();
    let tt = Time::<TT>::from_julian_days(2_460_000.25.into())?;
    let ut1: Time<UT1> = tt.to_with::<UT1>(&ctx)?;

    let unix = Time::<UTC>::from_unix_seconds(1_700_000_000.0.into())?;
    let back = unix.try_to::<UnixSecs>()?;

    println!("UT1 JD     : {:.9}", ut1.to::<JD>().value());
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
