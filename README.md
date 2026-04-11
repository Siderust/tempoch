# tempoch

[![Crates.io](https://img.shields.io/crates/v/tempoch.svg)](https://crates.io/crates/tempoch)
[![Docs](https://docs.rs/tempoch/badge.svg)](https://docs.rs/tempoch)
[![Code Quality](https://github.com/Siderust/tempoch/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Siderust/tempoch/actions/workflows/ci.yml)

Typed astronomical time primitives for Rust.

`tempoch` provides:

- Generic `Time<S>` instants parameterized by time-scale markers (`JD`, `MJD`, `TT`, `UT`, `TAI`, `GPS`, `UnixTime`, ...).
- UTC conversion through `chrono`, exact from 1961 onward for TAI-adjacent scales (TT, TAI, JD,
  MJD, JDE, GPS, UnixTime); model-limited for UT (ΔT model) and TDB/TCB/TCG (periodic
  approximation).  All conversions are leap-second aware.
- Automatic `ΔT = TT - UT1` handling for the `UT` scale.
- Standard Unix/POSIX timestamps via `UnixTime`, mapped to physical instants through `UTC -> TAI -> TT`.
- Compiled time-data tables generated from official UTC-TAI and Delta T sources.
- Generic intervals with `Interval<T>` and scale-aware alias `Period<S>`.
- Utility operations like period intersection and complement.

`UnixTime` keeps the usual Unix/POSIX timestamp contract for representable UTC
instants. When converted to physical scales, it is mapped through the compiled
`UTC -> TAI -> TT` history, so equal Unix increments are not guaranteed to
equal elapsed SI seconds across leap-second insertions.

`from_utc()` / `to_utc()` conversions are exact (table inversion) from 1961
onward for TAI-adjacent scales.  For `UT` they are limited by the ΔT model
accuracy; for `TDB`/`TCB`/`TCG` they are limited by the periodic
approximation (Fairhead & Bretagnon 1990, formula accuracy <30 μs).

**Precision floor:** `Time<S>` stores a single `f64` Julian Day.  Near J2000,
one `f64` ULP at JD 2 451 545.0 ≈ 4.66 × 10⁻¹⁰ d ≈ 40 μs.  All scales share
this storage-precision ceiling; claims of "sub-microsecond" or
"nanosecond-level" precision do not apply to the current representation.

The compiled modern ΔT series runs through MJD 63871 (`2033-10-01`).  Beyond
that date a quadratic continuation of the official prediction tail is used
automatically; extrapolation uncertainty grows without bound.  Call
`Time::<UT>::is_within_delta_t_horizon()` to check whether an epoch is covered
by compiled data, and use the exported `DELTA_T_PREDICTION_HORIZON_MJD`
constant to reference the boundary programmatically.

## Installation

```toml
[dependencies]
tempoch = "0.4"
```

Optional features:

- `serde`: serialization support for public time types
- `ffi`: marks `Time<S>` as `repr(transparent)` for Rust-side FFI layout guarantees

## Quick Start

```rust
use chrono::Utc;
use tempoch::{JulianDate, MJD, Time};

let now_jd = JulianDate::from_utc(Utc::now());
let now_mjd: Time<MJD> = now_jd.to::<MJD>();

println!("JD(TT): {now_jd}");
println!("MJD(TT): {now_mjd}");
```

## Period Operations

```rust
use tempoch::{complement_within, intersect_periods, ModifiedJulianDate, Period};

let outer = Period::new(ModifiedJulianDate::new(0.0), ModifiedJulianDate::new(10.0));
let a = vec![
    Period::new(ModifiedJulianDate::new(1.0), ModifiedJulianDate::new(4.0)),
    Period::new(ModifiedJulianDate::new(6.0), ModifiedJulianDate::new(9.0)),
];
let b = vec![
    Period::new(ModifiedJulianDate::new(2.0), ModifiedJulianDate::new(3.0)),
    Period::new(ModifiedJulianDate::new(7.0), ModifiedJulianDate::new(8.0)),
];

let overlap = intersect_periods(&a, &b);
let gaps = complement_within(outer, &a);

assert_eq!(overlap.len(), 2);
assert_eq!(gaps.len(), 3);
```

## Examples

- `cargo run --example quickstart`
- `cargo run --example periods`

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
