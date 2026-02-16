# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
