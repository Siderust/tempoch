# Nominal civil-UTC offsets versus elapsed SI time

Tempoch already exposes the time-scale and calendar bridges needed by consumers
such as the SGP4 adapter in Siderust. **No new temporal arithmetic API is
introduced by this change.** Keep the distinction between calendar coordinates
and physical elapsed time explicit.

## Two different quantities

- **Elapsed SI time:** subtract two `Time<UTC>` instants (or use
  `ExactDuration` for checked integral duration conversions). UTC is decoded
  using the time-data snapshot in `TimeContext`; elapsed arithmetic includes
  positive leap seconds.
- **Nominal civil-UTC time:** manipulate two *ordinary civil calendar labels*
  with `chrono::NaiveDateTime` or `chrono::DateTime<Utc>` and checked chrono
  durations. This uses 86,400 nominal seconds / 1,440 nominal minutes per
  civil day, regardless of leap seconds. It models a calendar coordinate,
  **not** elapsed SI seconds.

At the 2016 positive leap second:

| Civil labels | Nominal calendar difference | Actual elapsed time |
| --- | ---: | ---: |
| 2016-12-31 23:59:00 → 2017-01-01 00:00:00 UTC | 60 seconds | 61 SI seconds |

This is why the SGP4 model's “minutes since epoch” must not be computed by
subtracting native `Time<UTC>` instants: SGP4 requires nominal civil minutes,
not physical elapsed minutes.

## Explicit-context conversion and checked calendar offset

```rust
use chrono::{DateTime, TimeDelta, Utc};
use tempoch::{Time, TimeContext, UTC};

let ctx = TimeContext::new();
let label: DateTime<Utc> = "2017-01-01T00:00:00.123456789Z".parse()?;
let offset = TimeDelta::seconds(-30);
let shifted = label.checked_add_signed(offset)
    .ok_or("civil calendar offset is out of range")?;

// Native UTC instant, decoded with one explicit time-data snapshot:
let native = Time::<UTC>::try_from_chrono_with(shifted, &ctx)?;
let civil_again = native.try_to_chrono_with(&ctx)?;
assert_eq!(civil_again, shifted);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The Rust snippet is illustrative (the example is not an executable doctest).
For subsecond offsets, use the checked chrono duration constructors rather
than rounding through a large floating-point Julian date. The chrono
calendar representation supports nanosecond labels within its valid range.
Use `checked_add_signed` / `checked_sub_signed` and propagate failures
rather than saturating or overflowing.

To *measure* nominal minutes between ordinary civil labels, use their chrono
difference and convert using checked integer nanoseconds or a carefully
validated typed quantity. In contrast, `Time<UTC>` subtraction gives
**elapsed SI time**, which differs at leap seconds.

## Leap-second labels and Julian coordinates

A positive leap label such as `2016-12-31T23:59:60Z` can be represented
by the supported chrono/Tempoch UTC bridge. Chrono uses its leap-second
encoding, including a nanosecond field of `1_000_000_000`. Do **not** feed
that special label to ordinary calendar-offset calculations without an
explicit policy. Civil UTC JD/MJD cannot encode the leap label; the fallible
`to_with::<JD>(&ctx)` / `to_with::<MJD>(&ctx)` path rejects it.

For normal labels, `Time::<UTC>::try_from_chrono_with(label, &ctx)` and
`try_to_chrono_with(&ctx)` preserve the supported nanosecond precision.
Raw `f64` JD/MJD scalar arithmetic over a large epoch does **not**
guarantee nanosecond precision; keep the compensated native representation
and use checked calendar operations for nominal offsets.

## Validation and ownership

`ExactDuration::try_from_quantity` rejects non-finite and unrepresentable
quantities. Boundary projections such as `as_nanoseconds_i()` remain
fallible when the result cannot fit the selected integer carrier. However,
`ExactDuration` represents elapsed time, not an alternative nominal UTC
calendar offset.

Keep SGP4's `minutes_between` and `offset_epoch` adapters, including
their model-specific error mapping, in Siderust. Do not introduce new
temporal units, change existing subtraction semantics, or relocate the
Vallado/AFSPC calendar conventions into Tempoch. Both conversion directions
should use the same explicit `TimeContext` snapshot when reproducibility
matters.
