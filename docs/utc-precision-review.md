# UTC precision review for PR #18

Reviewed on `fix/context-utc-offset`, starting at `9b6d90e`.

## Confirmed defects

The original branch failed four new regression tests before implementation:

- `1965-07-01T00:00:00.123456789Z` returned `.123456791Z` (2 ns error).
- Historical midnight without a fraction could return 2 ns after midnight.
- The January 1972 historical gap was classified as a leap second.
- Rounding `2016-12-31T23:59:59.999999999Z` to milliseconds skipped the
  leap second and returned January 1 instead of `23:59:60.000Z`.

`SplitQuantity::new` normalizes with compensated summation, but `scale`
multiplies each component separately without retaining the high product's
rounding residual. In addition, forming `1 + k` and its reciprocal rounds the
coefficient before multiplication by a large displacement. Compensation cannot
recover either discarded error. The inverse used in PR #18 therefore failed its
nanosecond goal; the pre-PR scalar day implementation already had historical
precision limitations. Arbitrary-gap leap classification, the 0.5 s parsing
threshold, and leap-boundary formatting errors predate this PR. The weak ISO
tests also predate it.

## Changes and numerical argument

For UTC coordinate `u`, TAI coordinate `t`, reference `r`, base offset `b`, and
frequency offset `k = slope / 86400`, the segment equation is

```text
t = u + b + k (u - r)
u = (t - b) - [k / (1 + k)] (t - b - r)
```

The inverse now subtracts the small drift correction from the compensated
coordinate. It never scales the coordinate by a number near one. Over the
bundled historical segments, `|k| <= 3e-8` and the reference displacement is
less than 1.9e8 s. Rounding that displacement to f64 contributes less than
`1e-6 ns` after multiplication by `k`; the correction itself is below 6 s.
Coefficient, correction, and compensated-addition errors remain comfortably
below the 0.5 ns margin needed to recover nanosecond labels. This is a bounded
range argument, not a claim of exact real-number arithmetic or exhaustive
checking of every possible nanosecond.

The forward conversion and boundary calculations share an offset calculation
from seconds, avoiding scalar Julian dates except for segment selection and
ensuring identical arithmetic at exact midnights. The ISO formatter now resolves
carry through the context and preserves the intervening leap second. Its manual
parser reuses chrono conversion and the same leap-label validation.

Affected implementation files: `data/runtime_data/utc_tai.rs`, `format/iso.rs`,
and civil API documentation in `model/civil.rs`. The tests in `earth/context.rs`
now capture the compiled history directly, preventing interference from other
tests' temporary global fixtures. No public signatures, dependencies, unsafe
code, or global production state were added.

## Historical semantics and limitations

The [IERS UTC–TAI history](https://hpiers.obspm.fr/iers/bul/bulc/UTC-TAI.history)
defines the drift segments and the 1972 switch to integral offsets and SI
seconds. Only subsequent integral +1 s steps between zero-drift segments are
recognized as positive leap seconds in the captured table. The January 1972
step is 0.107758 s, not an ordinary leap second. Historical positive gaps have
no civil label in this model and return `UtcHistoryUnsupported`.

Negative historical steps overlap: at February 1968 the offset decreases by
0.1 s. Explicitly encoding the earlier branch's
`1968-01-31T23:59:59.950000000Z` decodes canonically as
`1968-02-01T00:00:00.049999997Z`. A regression demonstrates this quantitatively.
The stored atomic coordinate has no branch identifier. The later segment
remains authoritative; conversions of superseded earlier labels now explicitly
return `UtcHistoryUnsupported` rather than silently changing their labels.
This is an intentional behavioral tightening of the existing fallible API.

Accepted nanosecond civil labels preserve exact chrono and normalized nine-digit
RFC 3339 round-trips over the tested 1961–2030 range, including genuine positive
leap seconds. Pre-definition extrapolation remains opt-in. Arbitrarily distant
epochs, scalar JD/MJD precision, and arbitrary replacement histories are outside
this guarantee. Future dates use the captured table and cannot predict future
announced leap seconds.

## Tests and validation

Coverage includes every requested historical example, all compiled segment
boundaries and adjacent nanoseconds, every compiled positive leap second,
invalid leap labels, historical gaps and overlaps, context policy, ordinary
calendar boundaries, rounding/truncation, and exact ISO assertions replacing
prefix checks. A daily sweep checks 409,070 accepted timestamps for exactness
and monotonicity. Fixed-seed properties add 512 ordinary historical/modern
scenarios and 256 generated leap-boundary scenarios.

All commands completed successfully on the final code:

```sh
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
cargo check -p tempoch-core --no-default-features --features alloc
cargo check -p tempoch-core --no-default-features --features std
cargo check -p tempoch --no-default-features --features alloc
cargo check -p tempoch --no-default-features --features std
cargo check -p tempoch-core -p tempoch --no-default-features --features alloc,serde
cargo check -p tempoch-core -p tempoch --target thumbv7em-none-eabihf --no-default-features --features alloc,serde
cargo +nightly llvm-cov --workspace --all-features --doctests --no-report
cargo +nightly llvm-cov report --ignore-filename-regex 'tempoch-time-data(-updater)?/' --cobertura --output-path /tmp/tempoch-utc-validation/coverage.xml
cargo +nightly llvm-cov report --ignore-filename-regex 'tempoch-time-data(-updater)?/' --html --output-dir /tmp/tempoch-utc-validation/coverage_html
cargo +nightly llvm-cov report --ignore-filename-regex 'tempoch-time-data(-updater)?/' --fail-under-lines 90
```

Results: 371 all-target tests and 9 doctests passed. The repository coverage gate
passed with **93.86% lines** (7,879 / 8,394), above its 90% requirement. XML and
HTML reports were generated. Command logs are in `/tmp/tempoch-utc-validation`.

The three requested issues are addressed. From local numerical and validation
results the PR is ready to merge with the documented historical ambiguity
policy; remote PR checks remain a separate merge gate. The PR has not been
merged.
