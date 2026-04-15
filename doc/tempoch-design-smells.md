# tempoch Design Smells and qtty-Style Improvements

## Purpose

This note collects design smells in the current `tempoch` implementation and
suggests changes that would move the crate closer to the design discipline used
by `qtty`.

The goal is not to force `tempoch` into a fake unit-conversion model. Time
axes such as `TT`, `TAI`, `UTC`, `UT1`, `TDB`, `TCG`, and `TCB` are not just
scalings of one canonical quantity. Some transforms are affine, some are
periodic, some depend on compiled history, and some require explicit context.

What `tempoch` should borrow from `qtty` is not the math, but the structure:

- one honest core value type,
- local conversion rules,
- explicit encoding boundaries,
- less arithmetic leakage into call sites,
- less public exposure of implementation machinery.

## qtty Qualities Worth Copying

`qtty` works well because its public design has a few strong properties:

- The primary value type is simple and honest: `Quantity<U, S>`.
- `U` has clear semantics. A unit is not just a tag; it drives conversions.
- There is one obvious conversion surface: `.to::<Target>()`.
- Representation-specific details stay local to unit definitions and arithmetic
  helpers, rather than leaking across the whole crate.
- Internal canonicalization exists, but the public API does not force users to
  think in terms of the canonical storage format all the time.

`tempoch` should aim for the same kind of locality and honesty.

## Current Smells

### 1. `Representation` is part of the public type identity, but not of the storage model

Files:

- `tempoch-core/src/time.rs`
- `tempoch-core/src/representation.rs`
- `tempoch-core/src/storage.rs`

Problem:

- `Time<A, R>` suggests that both `A` and `R` are equally real parts of the
  type.
- In practice, for continuous axes, the stored payload is always the same
  `Seconds` value since J2000 TT on axis `A`.
- `repr::<R2>()` is therefore mostly a relabel for continuous axes.

Why this is a smell:

- The type parameter `R` looks semantic, but is often only a view.
- This creates conceptual overhead without giving a correspondingly strong
  invariant.
- It also encourages representation arithmetic to spread into the core API.

Closer-to-`qtty` suggestion:

- Make the core instant type just `Time<A>` or `Instant<A>`.
- Move `JD`, `MJD`, `SISeconds`, `UnixSeconds<POSIX>`, and `GpsSeconds` into
  explicit encoding/decoding APIs.
- Treat representations as codecs or coordinate views, not as part of the
  identity of the instant.

What this would improve:

- The type says what the value fundamentally is.
- Encodings become explicit at boundaries.
- The core model becomes smaller and easier to reason about.

### 2. Three public conversion witness traits expose internal conversion mechanics

Files:

- `tempoch-core/src/conversion.rs`
- `tempoch-core/src/time.rs`

Problem:

- The public API is split across `InfallibleConvertible`,
  `FallibleConvertible`, and `ContextConvertible`.
- These traits exist mainly to encode the conversion graph and gate method
  availability.
- They expose internal route mechanics rather than domain concepts.

Why this is a smell:

- Users have to understand the conversion implementation shape, not just the
  time model.
- It fragments the API into `to`, `try_to`, and `to_with`, where the method
  choice follows internal conversion categories.
- It is harder to discover than a single coherent transform API.

Closer-to-`qtty` suggestion:

- Keep the conversion graph internal and closed.
- Expose one primary transformation surface, for example:

```rust
time.to::<TT>() -> Result<Time<TT>, ConversionError>
time.to_with::<UT1>(&ctx) -> Result<Time<UT1>, ConversionError>
```

- Preserve compile-time invalid routes if desired, but hide the witness traits
  from the public surface.

What this would improve:

- Fewer concepts in the public API.
- A more obvious transformation story.
- Better separation between domain model and implementation strategy.

### 3. Canonical-storage arithmetic leaks all over the code

Files:

- `tempoch-core/src/time.rs`
- `tempoch-core/src/civil.rs`
- `tempoch-core/src/conversion.rs`
- `tempoch-core/src/delta_t.rs`
- `tempoch-core/src/constats.rs`

Problem:

- The code repeatedly performs arithmetic like:
  - `jd - J2000_JD_TT`
  - `mjd + JD_MINUS_MJD - J2000_JD_TT`
  - `jd_ut1 - JD_MINUS_MJD`
- This is correct, but it is implementation arithmetic showing through.

Why this is a smell:

- Readers see coordinate algebra instead of intent.
- The same epoch-offset knowledge is repeated at many call sites.
- It makes representations feel more complicated than they really are.

Closer-to-`qtty` suggestion:

- Centralize representation math behind helpers or encoding types:
  - `jd_to_storage_seconds(jd)`
  - `mjd_to_storage_seconds(mjd)`
  - `storage_seconds_to_mjd(seconds)`
  - `jd_to_mjd(jd)`
  - `mjd_to_jd(mjd)`

- Prefer names that express intent instead of exposing raw offset arithmetic.

What this would improve:

- Code becomes easier to scan.
- Epoch logic becomes testable in one place.
- Constants stay as internal source-of-truth values instead of user-facing
  arithmetic ingredients.

### 4. Civil-time concerns are mixed into the same abstraction layer as continuous dynamical axes

Files:

- `tempoch-core/src/civil.rs`
- `tempoch-core/src/storage.rs`
- `tempoch-core/src/representation.rs`
- `tempoch-core/src/axis.rs`

Problem:

- `UTC` is discontinuous and leap-aware.
- The other main axes are continuous.
- The crate still tries to make them feel like variants within one mostly
  uniform model, even though `UTC` needs special storage semantics and
  dedicated conversion logic.

Why this is a smell:

- The model looks simpler than it really is.
- `UTC` special cases leak into storage and API decisions.
- It becomes harder to explain what `Time<UTC, R>` means relative to
  continuous axes.

Closer-to-`qtty` suggestion:

- Keep `UTC` as a first-class axis, but isolate civil encodings and leap-second
  behavior behind dedicated modules or traits.
- Make it explicit that `UTC` is not just another continuous time coordinate.
- Consider a stronger separation between:
  - physical/dynamical axes,
  - civil labels,
  - transport encodings.

What this would improve:

- Better conceptual boundaries.
- Less accidental complexity in the core instant model.
- Easier future extension for additional civil-time features.

### 5. The crate mixes “what instant is this?” with “how is it encoded?” and “how do I transport it?”

Files:

- `tempoch-core/src/time.rs`
- `tempoch-core/src/representation.rs`
- `tempoch-core/src/civil.rs`

Problem:

- `JulianDays`, `ModifiedJulianDays`, `SISeconds`, `UnixSeconds<POSIX>`, and
  `GpsSeconds` are all grouped under `Representation`.
- These are not all the same kind of thing.
- Some are coordinate systems on an axis, some are transport encodings, and
  some are civil conventions.

Why this is a smell:

- One abstraction is carrying several different meanings.
- It weakens the usefulness of the abstraction itself.
- It makes extension harder because new concepts are forced into the same slot.

Closer-to-`qtty` suggestion:

- Split these ideas explicitly:
  - coordinate encodings: `JD`, `MJD`, native seconds,
  - civil encodings: chrono/UTC label interop,
  - transport encodings: POSIX seconds, GPS seconds.

- Give each category its own small, explicit API.

What this would improve:

- Cleaner concept boundaries.
- Better naming and docs.
- Less pressure on one catch-all `Representation` model.

### 6. Conversion routing logic is encoded in large pairwise impl blocks instead of local transform definitions

Files:

- `tempoch-core/src/conversion.rs`

Problem:

- The conversion graph is expressed as many trait impls and macros for pairwise
  routes.
- Some transforms are direct, some go through TT, some go through TAI, some
  require context.

Why this is a smell:

- The routing logic is correct but visually heavy.
- It is difficult to see the small number of true primitive transforms.
- Adding new axes or changing route policy risks touching a broad matrix.

Closer-to-`qtty` suggestion:

- Define a small set of primitive transforms:
  - `TAI <-> TT`
  - `TT <-> TDB`
  - `TT <-> TCG`
  - `TDB <-> TCB`
  - `UTC <-> TAI`
  - `UT1 <-> TT`

- Build routing on top of those primitives in one place.
- Keep route composition internal, not public.

What this would improve:

- The true model becomes visible.
- The code reads like a transform graph, not an impl matrix.
- Future maintenance gets easier.

### 7. `Native` is doing too much conceptual work

Files:

- `tempoch-core/src/representation.rs`
- `tempoch-core/src/time.rs`

Problem:

- `Native` stands in for the canonical user-facing representation for an axis.
- It also relies on hidden private storage.
- On UTC it means one thing; on continuous axes it effectively means another.

Why this is a smell:

- The name is generic, but the semantics are axis-dependent.
- It papers over an important distinction between “internal storage” and
  “canonical public encoding”.

Closer-to-`qtty` suggestion:

- Reduce reliance on `Native` as a public concept.
- If a canonical external view is needed, name it specifically.
- Otherwise let the core instant type be opaque and expose explicit encoding
  methods.

What this would improve:

- Better API honesty.
- Less need for users to reverse-engineer what “native” means.

### 8. Constants have started to become part of the everyday API instead of supporting hidden encoders

Files:

- `tempoch-core/src/constats.rs`

Problem:

- Epoch and offset constants are useful, but once exposed broadly they invite
  users and internal code to write coordinate arithmetic manually.

Why this is a smell:

- It shifts the API toward “assemble your own calendar algebra”.
- It is easy to produce correct-but-opaque code.

Closer-to-`qtty` suggestion:

- Keep one internal source of truth for epoch constants.
- Prefer public helper methods or encoding types over exposing raw conversion
  ingredients as the main workflow.

What this would improve:

- Less arithmetic duplication.
- Stronger high-level API.

## Suggested Direction

### A smaller, more honest core

Recommended target shape:

```rust
pub struct Time<A: Axis> { ... }
```

Then layer explicit views/codecs on top:

```rust
pub trait TimeEncoding<A: Axis> {
    type Repr;

    fn encode(time: Time<A>) -> Result<Self::Repr, ConversionError>;
    fn decode(value: Self::Repr) -> Result<Time<A>, ConversionError>;
}
```

This is closer to `qtty` in spirit:

- one core value,
- separate transformation/encoding logic,
- explicit boundaries.

### A clearer taxonomy

A useful split would be:

- `Axis`
  - what physical or civil time domain the instant lives on.
- `Encoding`
  - how the instant is represented externally on that axis.
- `Transport`
  - special external interchange forms such as POSIX and GPS.

This would remove pressure from `Representation` to mean too many things.

### One obvious transform API

Preferred surface:

- `time.to::<TT>()`
- `time.to_with::<UT1>(&ctx)`
- `time.encode::<JulianDate>()`
- `Time::<TT>::decode::<JulianDate>(value)`

The important part is that the public API should express operations users care
about, not the internal classification of transform implementations.

### Encoders should own epoch arithmetic

Representation modules should own arithmetic such as:

- J2000 offset handling,
- JD/MJD offset handling,
- POSIX epoch handling,
- GPS epoch handling.

This keeps the rest of the crate focused on time semantics rather than
coordinate math.

### Keep the closed-world type safety

`qtty` works well partly because the set of supported units and dimensions is
 explicit and local. `tempoch` should keep the same discipline:

- sealed axes,
- closed transform graph,
- explicit supported encodings,
- no adapter-only semantic shortcuts in the core crate.

The goal is not to make the model more dynamic. The goal is to make the static
model more coherent.

## Incremental Refactor Plan

### Phase 1: Local cleanup without public breakage

- Add internal helper functions for JD/MJD/J2000 conversions.
- Stop using raw epoch arithmetic at call sites.
- Group primitive transforms more explicitly inside `conversion.rs`.
- Reduce the number of places that directly reason about storage format.

### Phase 2: Reframe representations as encodings

- Introduce an internal encoding layer for `JD`, `MJD`, SI seconds, POSIX, and
  GPS.
- Make `Time<A, R>` wrappers delegate to encoding implementations.
- Deprecate direct representation-heavy call patterns where appropriate.

### Phase 3: Simplify the public core type

- Move toward `Time<A>` as the primary type.
- Replace `repr::<R>()` with explicit `encode` / `decode` APIs.
- Hide the witness traits or collapse them into a cleaner public transform API.

### Phase 4: Separate civil labeling from continuous-axis encoding more clearly

- Keep `UTC` first-class, but isolate leap-second and chrono interop concerns.
- Clarify which APIs are about physical instants and which are about civil
  labels.

## Non-Goals

These changes should not try to:

- pretend all axis transforms are simple unit scalings,
- erase the scientific distinction between axes,
- hide failure modes such as unsupported UTC history or UT1 horizon limits,
- trade away type-level safety for a stringly typed runtime model.

## Bottom Line

`tempoch` should become more like `qtty` in architectural style:

- smaller public core,
- sharper concept boundaries,
- one obvious conversion story,
- local handling of representation math,
- less leakage of canonical storage details.

It should not become more like `qtty` in mathematical assumptions. Time-axis
transforms are richer and messier than unit-ratio conversions. The design
should acknowledge that explicitly rather than flattening it into a misleading
uniform abstraction.
