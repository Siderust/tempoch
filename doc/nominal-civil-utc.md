# Nominal civil UTC offsets (#23)

Decision: reuse existing APIs; keep the model-specific adapter in Siderust.

`Time<UTC>` stores a continuous instant. Subtraction and addition, including
`JulianDate<UTC>` operations, measure elapsed SI time, not nominal civil time.
`try_raw_with` encodes UTC JD/MJD on an 86,400 nominal-second day, rejecting
leap-second labels. Subtracting those large rounded scalar dates sacrifices
nanosecond precision and is not the right adapter.

For a model requiring a nominal calendar grid:

1. Capture one `TimeContext` and reuse it for every decode and encode.
2. Validate grid membership with `try_raw_with(&ctx)` (leap labels are rejected),
   then decode with `try_to_chrono_with(&ctx)`. The decode preserves nanoseconds;
   do not use the rounded JD scalar as input to a second conversion.
3. On ordinary, non-leap labels, chrono `signed_duration_since` gives the
   nominal civil difference. `num_nanoseconds` checks its i64 range (~292 years).
   Convert once to QTTY minutes if that is the model's required boundary.
4. For offsets, `ExactDuration::try_from_quantity` validates finite values and
   `as_nanoseconds_i` checks i64 representability. This is a validation/conversion
   tool, not an assertion that the nominal grid has physical duration semantics.
   Use chrono `checked_add_signed` on the ordinary civil label and re-encode with
   `Time::<UTC>::try_from_chrono_with`, using the same context.

For example 2016-12-31 23:59:00 → 2017-01-01 00:00:00 is 60 nominal seconds but
61 elapsed SI seconds. An explicit 23:59:60 label cannot be represented on the
nominal JD grid. Tests exercise the existing methods, integer nanosecond round
trips and both kinds of arithmetic; they introduce no duplicate calendar engine.
Floating QTTY minutes cannot carry arbitrary nanosecond distinctions at very
large offsets. Do not claim more precision than their f64 representation allows.

Siderust's two short SGP4 adapters already implement these steps. They also own
SGP4's 1,440-minutes/day contract, error mapping and bounded model offsets. Moving
them here would create a new public API used by one caller without removing any
independent algorithm (chrono performs the calendar arithmetic today). Vallado
sidereal initialization, model epochs and UTC rejection policy remain in Siderust.
The bridges and ExactDuration support `no_std`; the core crate currently requires
alloc. No new dependency, broad std requirement or additional wrapper is needed.
