# tempoch-ffi

C-compatible ABI bridge for [`tempoch`](https://github.com/Siderust/tempoch) — astronomical time primitives.

`tempoch-ffi` exposes stable carrier structs, time-scale discriminants, status codes, and
conversion functions so C and C++ consumers can construct, inspect, and convert `tempoch`
typed instants (`Time<S>`, JD, MJD, Unix, GPS) without reimplementing scale-conversion logic.

## Highlights

- `TempochInstant` POD carrier (epoch + scale tag)
- `TimeScaleId` and `TimeFormatId` C-compatible discriminants
- `extern "C"` entry points for construction, conversion, and formatting
- generated header at `include/tempoch_ffi.h`

## C example

```c
#include "tempoch_ffi.h"

TempochInstant t;
tempoch_from_unix(1_700_000_000.0, &t);

TempochInstant t_tai;
if (tempoch_to_tai(&t, &t_tai) == TEMPOCH_OK) {
    /* t_tai holds the same instant in TAI */
}
```

## Rust example

```rust,no_run
use tempoch_ffi::{TempochInstant, TimeScaleId};

// constructed via the C ABI from a Unix timestamp
let raw = TempochInstant { seconds: 1_700_000_000.0, scale: TimeScaleId::Utc as u32 };
```

## Publishing Policy

This crate is not published by default; publish only when C API/ABI changes.

**Manual publish procedure:**

1. Flip `publish = false` to `publish = true` in `Cargo.toml`.
2. Ensure every `unsafe` block in `src/` carries a `// SAFETY:` rationale comment.
3. Run `cargo test -p tempoch-ffi --all-features` and `cargo clippy -p tempoch-ffi -- -D warnings`.
4. Run `cargo publish --manifest-path tempoch/tempoch-ffi/Cargo.toml`.
5. Revert `publish` back to `false`.

Alternatively, use `scripts/publish-changed.sh --confirm-ffi` from the repo root, which
enforces the soundness check and skips `publish = false` crates automatically.

## Related crates

- `tempoch`: user-facing Rust time types
- `qtty-ffi`: C ABI for `qtty` quantities (shared numeric carriers)
- `siderust-ffi`: C ABI for `siderust` astronomy (depends on this crate)
