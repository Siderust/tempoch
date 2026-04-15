# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- The axis / representation time model is now the primary public API at the
  crate root (`tempoch::*` / `tempoch_core::*`):
  - `Time<A, R = Native>` with sealed `Axis` (`TAI`, `TT`, `TDB`, `TCG`,
    `TCB`, `UTC`, `UT1`) and sealed `Representation` (`Native`,
    `JulianDays`, `ModifiedJulianDays`, `SISeconds`, `UnixSeconds<POSIX>`,
    `GpsSeconds`).
  - Three disjoint witness traits (`InfallibleConvertible`,
    `FallibleConvertible`, `ContextConvertible`) select conversion mode at
    compile time.
  - `TimeContext` carries the compiled-data context required for UT1
    conversions.
  - Civil layer: `Time<UTC>::{from_chrono, to_chrono}` preserves leap-second
    labels; `Time<UTC, UnixSeconds<POSIX>>` exposes POSIX seconds;
    `Time<TAI, GpsSeconds>` exposes GPS seconds with the fixed 19 s offset.
  - Generic `Interval<T: Copy + PartialOrd>` with the same algorithmic set
    (`complement_within`, `intersect_periods`, `normalize_periods`,
    `validate_period_list`).

### Removed

- Deleted the old `Time<S>` compatibility layer entirely, including the
  `tempoch::legacy` / `tempoch_core::legacy` namespaces.
- Dropped the legacy-only `serde` feature from `tempoch-core`, `tempoch`, and
  `tempoch-ffi`.

### Fixed

- **TDB−TT formula**: Removed a spurious Kepler equation correction (`sin(M + e·sin(M))` → `sin(M)`) from the dominant term of the Fairhead & Bretagnon (1990) expression. The `e·sin(M)` factor was converting mean anomaly to eccentric anomaly, which is not part of the standard TDB−TT Fourier series and introduced a ~28 μs systematic error.

### Changed

- Replaced the remaining raw time-quantity public APIs in `tempoch` with
  `qtty` types:
  - `Time::<UTC, UnixSeconds<POSIX>>::from_unix_seconds` and
    `.unix_seconds()` now use `qtty::Second`.
  - `Time::<TAI, GpsSeconds>::from_gps_seconds` and `.gps_seconds()` now use
    `qtty::Second`.
  - `DELTA_T_PREDICTION_HORIZON_MJD` is now exported as a typed `qtty::Day`
    constant.
- **ΔT extrapolation performance**: The quadratic tail-fit coefficients for post-horizon ΔT extrapolation are now computed once and cached via `OnceLock`, instead of solving a 3×3 Gaussian elimination on every call.
- **Deduplicated `TT_MINUS_TAI_SECS`**: The `32.184 s` constant is now defined once in `scales.rs` (`pub(crate)`) and imported by `instant.rs`, eliminating a duplicate definition that could drift.
- **Removed dead `f64::EPSILON` comparisons** in the modern ΔT interpolator. The exact-match shortcuts against MJD-scale values could never trigger (1 ULP at MJD ~50 000 is ≈ 7×10⁻¹² ≫ `f64::EPSILON`); removed in favour of the unconditional linear interpolation that was already the effective code path.

### Deprecated

- `Time::<JD>::julian_millennias()` is deprecated in favour of `julian_millennia()` (correct Latin plural). The old name remains available with a `#[deprecated]` attribute.

### Added

- Automated time-data refresh tooling via `scripts/update_time_data.py` and a scheduled GitHub Actions workflow.

### Changed

- `tempoch-core` now compiles generated UTC-TAI history and modern Delta T tables from official upstream sources instead of relying on hand-maintained constants.
- `tai_minus_utc()` now uses the official pre-1972 UTC frequency-offset history from 1961 onward, while preserving the legacy 10 s fallback for earlier dates.
- `TCB` conversions now compose the linear `TCB ↔ TDB` relation with the existing periodic `TDB ↔ TT` correction, eliminating the previous millisecond-scale `TDB ↔ TCB` round-trip drift.
- Clarified `UnixTime` as the standard Unix / POSIX timestamp contract mapped to physical instants through `UTC → TAI → TT`; docs and examples now explicitly note that equal Unix increments are not guaranteed to equal elapsed SI seconds across leap-second insertions.
- Clarified the public scientific wording for `TCG`, `UT1`, `ΔT`, and `tai_minus_utc()`, and documented the existing modern ΔT horizon at MJD 63871 (`2033-10-01`) plus the pre-1961 `TAI−UTC = 10 s` fallback.

## [0.4.1 - 2026-03-31]

### Changed

- Simplified `tempoch-ffi` to a scalar C ABI centered on `double` time values plus raw scale IDs, with generic `tempoch_time_convert`, `tempoch_time_from_utc`, `tempoch_time_to_utc`, `tempoch_time_difference_*`, and `tempoch_time_add_*` entrypoints.
- Standardized the FFI `UnixTime` contract as POSIX seconds since `1970-01-01T00:00:00 UTC`, instead of exposing the Rust crate's internal day-count representation directly.
- Bumped `tempoch-ffi` to `0.4.0` / ABI version `400` and regenerated the public C header to match the new generic surface.
- Simplified the `tempoch-cpp` wrapper to rebuild typed `Time<S>` operations on top of the generic scale-based FFI calls rather than maintaining a hand-written pairwise conversion matrix.

### Fixed

- `tempoch-ffi` period APIs now reject malformed `MJD` intervals consistently, including non-finite endpoints passed across the C boundary.
- Rust and C++ FFI-facing civil-time docs now agree on the supported `second` range (`0..=59`), removing the earlier leap-second mismatch at the adapter boundary.

## [0.4.0 - 2026-03-08]

### Added

- Typed `qtty-ffi` duration helpers in `tempoch-ffi`, including `tempoch_jd_difference_qty`, `tempoch_jd_add_qty`, `tempoch_mjd_difference_qty`, `tempoch_mjd_add_qty`, `tempoch_jd_julian_centuries_qty`, and `tempoch_period_mjd_duration_qty`.
- FFI time-scale conversion functions for `TDB`, `TT`, `TAI`, `TCG`, `TCB`, `GPS`, `UT`, `JDE`, and `UnixTime`.
- FFI `TempochScale` enum plus generic `tempoch_jd_to_scale` / `tempoch_scale_to_jd` dispatch helpers.
- `timescales` example covering the supported scale conversions.

### Changed

- Updated the public `qtty` dependency to `0.4.1`.
- `Time<S>` display output now omits the redundant trailing day-unit suffix and forwards standard formatting flags to the raw numeric value.
- Generated FFI headers now include `qtty_ffi.h` instead of duplicating shared quantity declarations.
- Improved formatting and consistency in FFI time conversion implementations.

### Fixed

- `Interval<DateTime<Utc>>::duration_days()` now preserves sub-second precision instead of truncating through whole seconds.

## [0.3.0 - 2026-02-19]

### Added

- `Time::try_new` and `Time::try_from_days` validated constructors that reject `NaN`/`±∞`.
- `Interval::try_new` validated constructor that rejects `start > end` (and `NaN` endpoints).
- `NonFiniteTimeError`, `InvalidIntervalError`, `PeriodListError` error types.
- `validate_period_list` — checks sorted/non-overlapping invariants on a period slice.
- `normalize_periods` — sorts and merges overlapping intervals into a valid list.
- FFI: generated C header is now also written to `tempoch-ffi/include/tempoch_ffi.h`.

### Changed

- `Time::new` and `Interval::new` now carry documentation warnings about
  accepting unchecked input; prefer the new `try_*` constructors for
  untrusted data.
- Serde deserialization of `Time<S>`, `Period<MJD>`, and `Period<JD>` now
  rejects non-finite values and invalid intervals (start > end).
- `Time::<JD>::tt_to_tdb` now delegates to the shared `tdb_minus_tt_days`
  function (previously duplicated the Fairhead & Bretagnon math).

### Fixed

- Eliminated duplicated TDB correction logic between `scales.rs` and
  `julian_date_ext.rs`, reducing drift risk.

## [0.2.1 - 2026-02-19]

### Added

- FFI support

### Fixed

- Preserve sub-second precision in `Interval<DateTime<Utc>>::duration_days()` by computing from nanoseconds instead of truncating via whole seconds.

## [0.2.0 - 2026-02-16]

### Added

- New coordinate time scales: `TCG` (Geocentric Coordinate Time) and `TCB` (Barycentric Coordinate Time).
- Crate-root exports for `TCG` and `TCB` (`tempoch::TCG`, `tempoch::TCB`).
- Additional conversion tests covering `TDB`, `TCG`, `TCB`, and leap-second-aware Unix conversions.

### Changed

- `TDB` conversions no longer treat `TT` as strictly identical; they now apply periodic Fairhead & Bretagnon correction terms.
- `UnixTime` conversions now apply leap-second-aware UTC/TAI/TT offsets from an IERS Bulletin C table (1972–2017).
- `ΔT` now uses observed annual values for 1992–2025 and linear near-term extrapolation after 2026, replacing the prior modern-year approximation.

## [0.1.0 - 2026-02-12]

### Added

- Initial standalone release extracted from `siderust::time`.
- Generic time instants and scales via `Time<S>` and `TimeScale`.
- Marker scales: `JD`, `JDE`, `MJD`, `TDB`, `TT`, `TAI`, `GPS`, `UnixTime`, `UT`.
- `JulianDate`, `JulianEphemerisDay`, `ModifiedJulianDate`, `UniversalTime` aliases.
- UTC conversion helpers (`from_utc`, `to_utc`) using `chrono`.
- Automatic `ΔT` correction layer for `UT` conversions.
- Generic intervals (`Interval<T>`) and scale-based periods (`Period<S>`).
- Set-like period utilities: `intersect_periods` and `complement_within`.
- GitHub Actions CI workflow with check/fmt/clippy/test/doctest/coverage jobs.
- Runnable examples and integration tests.
