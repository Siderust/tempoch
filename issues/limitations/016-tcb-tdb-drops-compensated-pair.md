# TCB↔TDB conversions drop the compensated pair

## Summary
The `TDB→TCB` and `TCB→TDB` conversion implementations discard the low-order
component of the compensated pair after computing with a plain `f64` add. This
is architecturally inconsistent with the rest of the library and causes a
precision loss of roughly **0.7 µs per 100 years** from T0.

## Status
Resolved. Both `TDB→TCB` and `TCB→TDB` now use `add_constant` to preserve the
compensated pair, consistent with the `TT↔TCG` implementation.

## What is the issue
`Time<S>` stores epochs as a compensated `(hi, lo)` pair of J2000 seconds.
The compensated representation gives sub-nanosecond resolution even for epochs
far from J2000, which is the core precision advantage of the architecture.

The TCB↔TDB conversions in `scale/conversion.rs` break this invariant:

```rust
// TDB → TCB
let src = total_seconds(src_hi, src_lo);   // plain f64 add: precision lost here
let target = t0 + delta / (1.0 - L_B);
normalize_pair(target.value(), 0.0)        // lo = 0.0: compensated pair discarded
```

Compare to TT↔TCG which preserves the pair correctly:

```rust
// TT → TCG
let delta = Second::new(L_G * (src - t0).value() / (1.0 - L_G));
add_constant(src_hi, src_lo, delta)        // pair preserved
```

The ULP of `f64` at 100 years from T0 (~3.16 × 10⁹ s) is **~0.70 µs**, so
`normalize_pair(x, 0.0)` silently introduces up to 0.70 µs of representational
error. The error grows linearly with distance from T0.

## Current behavior
- `TDB→TCB` and `TCB→TDB` both call `normalize_pair(target.value(), 0.0)`.
- Precision of the result is limited to the ULP of a plain `f64`, not to the
  sub-nanosecond resolution promised by the compensated-pair architecture.
- The TCB↔TT chain (via TDB) inherits the same loss.

## How it is currently handled
No workaround. The loss is invisible because the ~10 µs TDB accuracy floor
is larger than the ~0.70 µs representational error at typical epoch ranges.

## Pros of the current handling
- Simple arithmetic. No two-sum compensation needed.
- Within the TDB physical accuracy band for epochs within a few centuries of J2000.

## Cons of the current handling
- Violates the library's own precision contract for all scales beyond TAI, TT, TCG.
- As the TCG conversion shows, the correct approach requires no extra complexity
  (just `add_constant` on the compensated pair).
- For epochs far from J2000 or for applications that chain multiple conversions,
  the rounding accumulates.

## Evidence
- `tempoch-core/src/scale/conversion.rs`, lines 145–165 (TCB↔TDB impls)
- `tempoch-core/src/scale/conversion.rs`, lines 130–143 (TT↔TCG, correct reference)

## User impact
- Any `Time<TDB>` or `Time<TCB>` value at ±100 years from J2000 may silently
  accumulate up to ~0.70 µs of representational error.
- Downstream conversions (TDB→TAI, TCB→TT, etc.) inherit the loss.
- Users who rely on the sub-µs precision advertised by the compensated-pair
  design will not receive it for TCB-involving conversions.

## What could be done to solve or reduce it
Rewrite `TDB→TCB` and `TCB→TDB` using the same `add_constant`-style approach
as `TT→TCG`.

For `TCB→TDB`:
```
delta = (src_hi - t0) + src_lo      (compensated difference from T0)
scale  = (1 - L_B)
target_delta = scale * delta + TDB0
// store as (t0 + target_delta_hi, target_delta_lo) using add_constant
```

For `TDB→TCB`:
```
delta = (src_hi - t0 - TDB0) + src_lo
scale = 1 / (1 - L_B)
// similarly preserve the pair
```

The two-sum decomposition of `scale * delta` requires a single Veltkamp split
or equivalent; it is a standard technique already available in the codebase.

## What cannot be solved without tradeoffs
- Any loss introduced during the `TDB←TT` 7-term series evaluation (which also
  uses plain `f64` arithmetic) is a separate issue and limits the ceiling.
- The TCB↔TDB representational fix improves the storage layer independently.

## Acceptance criteria for closing
- Both `TDB→TCB` and `TCB→TDB` preserve the compensated pair through the linear
  scale-and-shift, consistent with the TT↔TCG implementation.
- A test demonstrates sub-µs round-trip accuracy for TCB↔TDB at ±200 years.
