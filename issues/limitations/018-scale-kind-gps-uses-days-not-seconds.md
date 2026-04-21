# ScaleKind::Gps encodes days, not conventional GPS seconds

## Summary
`ScaleKind::Gps` in the scalar dispatch layer uses **Julian days on the TAI
axis since the GPS epoch**, which is not the conventional GPS time representation.
Standard GPS time is expressed as GPS seconds (or GPS week + seconds-of-week).
The mismatch is not signaled by the name or the first-encountered API surface.

## Status
Pending. The current encoding works internally but may surprise integrators
who expect conventional GPS seconds.

## What is the issue
In `tempoch-core/src/scalar.rs`, `ScaleKind::Gps` is documented and
implemented as:

> GPS Julian days on the TAI axis, measured from `GPS_EPOCH_JD_TAI`.

This means a value of `1.0` represents **one Julian day** (86 400 s) since
the GPS epoch, not one GPS second. The GPS standard defines GPS time as an
integer number of seconds (or weeks + seconds) since the GPS epoch
(1980-01-06 00:00:00 UTC = 1980-01-06 00:00:19 TAI). No published GPS
standard uses Julian days as the native unit.

The constant `GPS_EPOCH_JD_TAI = 2 444 244.500 219 9 JD(TAI)` is correctly
computed, but the unit attached to user-facing scalars is days, not seconds.

## Current behavior
- `ScaleKind::Gps` scalars represent Julian-day counts from the GPS epoch.
- A value of `1.0` is `1 JD = 86 400 s` of GPS time.
- The C ABI (via `TempochScaleId`) inherits the same unit.
- No conversion or renaming is applied for users expecting GPS seconds.

## How it is currently handled
The documentation in `scalar.rs` states "GPS days" explicitly. The C header
generated from the FFI should carry the same documentation. The encoding is
consistent within the library.

## Pros of the current handling
- Julian days are a uniform unit across all `ScaleKind` variants, making
  the dispatch matrix uniform.
- The GPS-as-JD encoding composes cleanly with the rest of the scalar API.

## Cons of the current handling
- GPS receivers, GNSS libraries, and navigation software universally output
  GPS seconds (or week + ToW). An integrator connecting `tempoch` to a GPS
  receiver must manually scale by 86 400.
- The name `Gps` does not hint at the day-based unit; it implies the
  conventional GPS time representation.
- Confusion risk grows at the FFI boundary, where C callers are less likely
  to read Rust doc comments.

## Evidence
- `tempoch-core/src/scalar.rs`, `ScaleKind::Gps` and `GPS_EPOCH_JD_TAI`
- `tempoch-ffi/src/carriers.rs`, `TempochScaleId::Gps` dispatch
- GNSS convention: GPS seconds are the authoritative scalar representation
  in receiver datasheets, RINEX files, and navigation message definitions.

## User impact
- Integrators who connect a GPS receiver to `tempoch` via the scalar API
  must know to divide GPS seconds by 86 400 before passing them in. This is
  non-obvious and error-prone.
- A GPS receiver reporting `t = 1_377_461_234 s` must be supplied as
  `~15 943.8 days`, which is surprising.

## What could be done to solve or reduce it
1. **Rename to `GpsJulianDays`** (or `GpsDays`) to make the unit explicit
   in the identifier, keeping the current encoding.
2. **Switch to GPS seconds** by changing `GPS_EPOCH_JD_TAI` to a TAI-seconds
   epoch constant and adjusting the scalar dispatch to use seconds.
   This matches the conventional representation at the cost of a uniform-day
   guarantee.
3. **Add a companion `GpsSeconds` variant** alongside `GpsDays` if both
   encodings are needed by different callers.

## What cannot be solved without tradeoffs
- The current uniform-day dispatch is elegant; moving GPS to seconds would
  require GPS to be a special case in the dispatch matrix.
- Any rename or encoding change is a breaking API change at the scalar layer.

## Acceptance criteria for closing
- Either `ScaleKind::Gps` (and the corresponding FFI id) clearly signals its
  day-based unit in its name or prominent documentation, or the variant
  switches to conventional GPS seconds and is renamed accordingly.
