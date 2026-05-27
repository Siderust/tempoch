// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Exact-precision duration container.
//!
//! [`ExactDuration`] is the canonical duration type for `tempoch`. Its
//! representation is **deliberately opaque**: today it is backed by a single
//! `i128` of nanoseconds (range ≈ ±1.7 × 10²¹ years, ~170 Gyr; exact resolution
//! 1 ns). A future internal migration to `i128` attoseconds or another
//! sub-nanosecond representation is a non-breaking change as long as callers go
//! through the named accessors (`as_seconds_i64_nanos`, `as_seconds_f64`, …).
//!
//! # Design choices
//!
//! * **Sign convention** — a single signed integer carries the sign uniformly.
//!   This avoids the classic `{whole_seconds: i64, sub_nanos: u32}` pitfall
//!   where `-0.5 s` must be represented as `{-1, 500_000_000}` and negation
//!   becomes asymmetric near zero.
//! * **No `f64` in the public exact API** — `f64` boundaries are reachable only
//!   through explicitly named methods (`from_seconds_f64_lossy`,
//!   `as_seconds_f64`) so users see the lossy step in code review.
//! * **qtty interop** — [`ExactDuration::from_quantity`] /
//!   [`ExactDuration::as_quantity`] bridge to typed `Quantity<U>` for any
//!   [`qtty::time::TimeUnit`]. The bridge through `f64` is intentional: `qtty`
//!   itself is a floating-point quantity system; users wanting exact duration
//!   math should keep values inside [`ExactDuration`].
//! * **Overflow** — arithmetic uses checked operations and reports
//!   [`DurationError::Overflow`] when the result leaves the i128 range; the
//!   public `+`/`-` operators panic on overflow (debug + release) to match
//!   `Duration`/`std::time` ergonomics. Use [`ExactDuration::checked_add`] /
//!   [`ExactDuration::checked_sub`] / [`ExactDuration::checked_neg`] for
//!   non-panicking callers (FFI, parsers, formal-verification harnesses).
//!
//! # Future-proofing
//!
//! Because the storage is opaque and the boundary projection
//! `(seconds: i64, nanos: u32)` is the only serde shape, migrating to a
//! sub-nanosecond representation is non-breaking; callers requesting
//! attosecond precision in serde will opt in through a future
//! `serde-attos` feature.

use core::cmp::Ordering;
use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use qtty::time::TimeUnit;
use qtty::unit::Second as SecondUnit;
use qtty::{Quantity, Second};

/// Nanoseconds per second; convenience constant for boundary code.
pub const NANOS_PER_SECOND: i128 = 1_000_000_000;

/// Error type for fallible [`ExactDuration`] operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationError {
    /// Arithmetic overflowed the `i128`-nanosecond representation.
    Overflow,
    /// Input scalar was NaN or infinite.
    NonFinite,
}

impl core::fmt::Display for DurationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Overflow => f.write_str("ExactDuration arithmetic overflowed i128 nanoseconds"),
            Self::NonFinite => f.write_str("ExactDuration input was NaN or infinite"),
        }
    }
}

impl std::error::Error for DurationError {}

/// Exact-precision signed duration.
///
/// Internally an `i128` of nanoseconds. Range ≈ ±170 Gyr at 1 ns resolution.
///
/// Construction:
///
/// * [`ExactDuration::ZERO`]
/// * [`ExactDuration::from_nanos`]
/// * [`ExactDuration::from_seconds_and_nanos`]
/// * [`ExactDuration::from_quantity`] / [`ExactDuration::try_from_quantity`]
/// * [`ExactDuration::from_seconds_f64_lossy`] (explicit lossy boundary)
///
/// Accessors:
///
/// * [`ExactDuration::as_nanos_i128`]
/// * [`ExactDuration::as_seconds_i64_nanos`] (boundary projection: `(i64, u32)`)
/// * [`ExactDuration::as_seconds_f64`]
/// * [`ExactDuration::as_quantity`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExactDuration {
    nanos: i128,
}

impl ExactDuration {
    /// Zero duration.
    pub const ZERO: Self = Self { nanos: 0 };

    /// Smallest representable positive duration (1 ns).
    pub const NANOSECOND: Self = Self { nanos: 1 };

    /// One second.
    pub const SECOND: Self = Self {
        nanos: NANOS_PER_SECOND,
    };

    /// Maximum representable duration.
    pub const MAX: Self = Self { nanos: i128::MAX };

    /// Minimum (most negative) representable duration.
    pub const MIN: Self = Self { nanos: i128::MIN };

    /// Build from a raw nanosecond count.
    #[inline]
    pub const fn from_nanos(nanos: i128) -> Self {
        Self { nanos }
    }

    /// Build from `(seconds, nanos)` boundary projection.
    ///
    /// The fractional `nanos` is interpreted with the same sign as `seconds`
    /// when `seconds != 0`; when `seconds == 0`, `nanos` carries the sign
    /// directly. This matches the unambiguous total
    /// `result_nanos = seconds * 1e9 + nanos`.
    ///
    /// Returns [`DurationError::Overflow`] if the multiplication overflows.
    #[inline]
    pub const fn from_seconds_and_nanos(seconds: i64, nanos: i32) -> Result<Self, DurationError> {
        // `seconds as i128 * NANOS_PER_SECOND` cannot overflow i128 because
        // i64::MAX * 1e9 < i128::MAX, but the addition can if nanos has the
        // same sign and a sufficiently extreme value — practically not, but we
        // still go through checked_add for correctness.
        let secs_nanos = (seconds as i128).wrapping_mul(NANOS_PER_SECOND);
        match secs_nanos.checked_add(nanos as i128) {
            Some(n) => Ok(Self { nanos: n }),
            None => Err(DurationError::Overflow),
        }
    }

    /// Build from a `qtty::Quantity<U>` of any time unit. Returns
    /// [`DurationError::NonFinite`] for NaN/inf inputs and
    /// [`DurationError::Overflow`] if the value does not fit in i128 ns.
    #[inline]
    pub fn try_from_quantity<U: TimeUnit>(q: Quantity<U>) -> Result<Self, DurationError> {
        let secs = q.to::<SecondUnit>().value();
        if !secs.is_finite() {
            return Err(DurationError::NonFinite);
        }
        // f64 mantissa is 53 bits; for |secs| < 2^53 / 1e9 ≈ 9.0e6 the conversion
        // is exact. Outside that range we still produce the closest i128 ns
        // representation; callers needing better precision should construct via
        // `from_seconds_and_nanos` or `from_nanos`.
        let nanos_f = secs * (NANOS_PER_SECOND as f64);
        if nanos_f >= (i128::MAX as f64) || nanos_f <= (i128::MIN as f64) {
            return Err(DurationError::Overflow);
        }
        Ok(Self {
            nanos: nanos_f as i128,
        })
    }

    /// Infallible variant for callers that already know the input is finite
    /// and in-range. Panics on non-finite or overflowing input.
    /// For fallible conversion, use [`try_from_quantity`](Self::try_from_quantity).
    #[inline]
    pub fn from_quantity<U: TimeUnit>(q: Quantity<U>) -> Self {
        Self::try_from_quantity(q).unwrap_or_else(|e| panic!("ExactDuration::from_quantity: {e}"))
    }

    /// Explicit lossy `f64` → `ExactDuration` boundary. Named so the lossy
    /// step is visible in code review. Returns `None` on non-finite input or
    /// when the value does not fit in i128 ns.
    #[inline]
    pub fn from_seconds_f64_lossy(seconds: f64) -> Option<Self> {
        if !seconds.is_finite() {
            return None;
        }
        let nanos_f = seconds * (NANOS_PER_SECOND as f64);
        if nanos_f >= (i128::MAX as f64) || nanos_f <= (i128::MIN as f64) {
            return None;
        }
        Some(Self {
            nanos: nanos_f as i128,
        })
    }

    /// Raw signed nanosecond count.
    #[inline]
    pub const fn as_nanos_i128(self) -> i128 {
        self.nanos
    }

    /// Boundary projection `(seconds, nanos)` where
    /// `seconds * 1e9 + nanos == as_nanos_i128()` and the pair has the same
    /// sign. `nanos` is in `(-1_000_000_000, 1_000_000_000)`.
    ///
    /// This is the canonical serde/FFI shape for [`ExactDuration`].
    #[inline]
    pub const fn as_seconds_i64_nanos(self) -> (i64, i32) {
        let secs = self.nanos / NANOS_PER_SECOND;
        let rem = (self.nanos - secs * NANOS_PER_SECOND) as i32;
        // secs comes from i128 / 1e9; for any practical durations representable
        // in this crate it fits in i64; saturate otherwise.
        let secs_i64 = if secs > i64::MAX as i128 {
            i64::MAX
        } else if secs < i64::MIN as i128 {
            i64::MIN
        } else {
            secs as i64
        };
        (secs_i64, rem)
    }

    /// Explicit lossy `ExactDuration` → `f64 seconds` boundary.
    #[inline]
    pub fn as_seconds_f64(self) -> f64 {
        (self.nanos as f64) / (NANOS_PER_SECOND as f64)
    }

    /// Project back into a `qtty::Quantity<U>`. Lossy in general (f64).
    #[inline]
    pub fn as_quantity<U: TimeUnit>(self) -> Quantity<U> {
        Second::new(self.as_seconds_f64()).to::<U>()
    }

    /// True iff exactly zero.
    #[inline]
    pub const fn is_zero(self) -> bool {
        self.nanos == 0
    }

    /// True iff strictly negative.
    #[inline]
    pub const fn is_negative(self) -> bool {
        self.nanos < 0
    }

    /// Absolute value. Returns [`DurationError::Overflow`] on
    /// [`ExactDuration::MIN`] (i128::MIN has no representable positive).
    #[inline]
    pub const fn checked_abs(self) -> Result<Self, DurationError> {
        match self.nanos.checked_abs() {
            Some(n) => Ok(Self { nanos: n }),
            None => Err(DurationError::Overflow),
        }
    }

    /// Checked addition.
    #[inline]
    pub const fn checked_add(self, rhs: Self) -> Result<Self, DurationError> {
        match self.nanos.checked_add(rhs.nanos) {
            Some(n) => Ok(Self { nanos: n }),
            None => Err(DurationError::Overflow),
        }
    }

    /// Checked subtraction.
    #[inline]
    pub const fn checked_sub(self, rhs: Self) -> Result<Self, DurationError> {
        match self.nanos.checked_sub(rhs.nanos) {
            Some(n) => Ok(Self { nanos: n }),
            None => Err(DurationError::Overflow),
        }
    }

    /// Checked negation.
    #[inline]
    pub const fn checked_neg(self) -> Result<Self, DurationError> {
        match self.nanos.checked_neg() {
            Some(n) => Ok(Self { nanos: n }),
            None => Err(DurationError::Overflow),
        }
    }

    /// Saturating addition.
    #[inline]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Self {
            nanos: self.nanos.saturating_add(rhs.nanos),
        }
    }

    /// Saturating subtraction.
    #[inline]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self {
            nanos: self.nanos.saturating_sub(rhs.nanos),
        }
    }

    /// Round this duration to the nearest multiple of `quantum` (banker's
    /// rounding / half-to-even). `quantum` must be strictly positive; a
    /// non-positive quantum returns `self` unchanged to avoid surprising
    /// errors in formatting paths.
    #[inline]
    pub const fn round_to(self, quantum: ExactDuration) -> Self {
        let q = quantum.nanos;
        if q <= 0 {
            return self;
        }
        let n = self.nanos;
        // Round-half-to-even on positive quantum, treating negative `n` symmetrically.
        let div = n / q;
        let rem = n - div * q;
        let abs_rem = if rem < 0 { -rem } else { rem };
        let half = q / 2;
        let result = if abs_rem.saturating_mul(2) < q {
            div
        } else if abs_rem.saturating_mul(2) > q {
            if n >= 0 {
                div.saturating_add(1)
            } else {
                div.saturating_sub(1)
            }
        } else {
            // Exact half — banker's rounding to even.
            let _ = half;
            if div % 2 == 0 {
                div
            } else if n >= 0 {
                div.saturating_add(1)
            } else {
                div.saturating_sub(1)
            }
        };
        Self {
            nanos: result.saturating_mul(q),
        }
    }

    /// Floor this duration toward negative infinity at `quantum`.
    #[inline]
    pub const fn floor_to(self, quantum: ExactDuration) -> Self {
        let q = quantum.nanos;
        if q <= 0 {
            return self;
        }
        let n = self.nanos;
        let div = n / q;
        let rem = n - div * q;
        let floor_div = if rem < 0 { div.saturating_sub(1) } else { div };
        Self {
            nanos: floor_div.saturating_mul(q),
        }
    }

    /// Ceil this duration toward positive infinity at `quantum`.
    #[inline]
    pub const fn ceil_to(self, quantum: ExactDuration) -> Self {
        let q = quantum.nanos;
        if q <= 0 {
            return self;
        }
        let n = self.nanos;
        let div = n / q;
        let rem = n - div * q;
        let ceil_div = if rem > 0 { div.saturating_add(1) } else { div };
        Self {
            nanos: ceil_div.saturating_mul(q),
        }
    }
}

impl Default for ExactDuration {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialOrd for ExactDuration {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExactDuration {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.nanos.cmp(&other.nanos)
    }
}

impl core::fmt::Display for ExactDuration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (s, n) = self.as_seconds_i64_nanos();
        if n == 0 {
            write!(f, "{s} s")
        } else {
            // Render seconds as decimal with up to 9 fractional digits.
            // Sign carried by `s` when |duration| >= 1 s; when |duration| < 1 s,
            // sign comes from `n`.
            if s == 0 {
                // Sub-second magnitude — sign carried by `n`.
                if n < 0 {
                    write!(f, "-0.{:09} s", (-n))
                } else {
                    write!(f, "0.{:09} s", n)
                }
            } else {
                write!(f, "{s}.{:09} s", n.abs())
            }
        }
    }
}

// ───────────────── Operators ─────────────────
// Panics on overflow to match `Duration` ergonomics; use `checked_*` to opt out.

impl Add for ExactDuration {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.checked_add(rhs)
            .expect("ExactDuration::add overflowed i128 ns")
    }
}

impl Sub for ExactDuration {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.checked_sub(rhs)
            .expect("ExactDuration::sub overflowed i128 ns")
    }
}

impl Neg for ExactDuration {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        self.checked_neg()
            .expect("ExactDuration::neg overflowed i128 ns")
    }
}

impl AddAssign for ExactDuration {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for ExactDuration {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::{ExactDuration, NANOS_PER_SECOND};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Boundary {
        sec: i64,
        ns: i32,
    }

    impl Serialize for ExactDuration {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let secs = self.nanos / NANOS_PER_SECOND;
            if secs > i64::MAX as i128 || secs < i64::MIN as i128 {
                return Err(serde::ser::Error::custom(
                    "ExactDuration out of i64 seconds range; duration cannot be serialized",
                ));
            }
            let (sec, ns) = self.as_seconds_i64_nanos();
            Boundary { sec, ns }.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for ExactDuration {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let b = Boundary::deserialize(deserializer)?;
            let total = (b.sec as i128)
                .checked_mul(NANOS_PER_SECOND)
                .and_then(|s| s.checked_add(b.ns as i128))
                .ok_or_else(|| serde::de::Error::custom("ExactDuration overflow"))?;
            Ok(Self::from_nanos(total))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtty::unit::{Day as DayUnit, Millisecond as MsUnit};

    #[test]
    fn zero_and_constants() {
        assert_eq!(ExactDuration::ZERO.as_nanos_i128(), 0);
        assert_eq!(ExactDuration::NANOSECOND.as_nanos_i128(), 1);
        assert_eq!(ExactDuration::SECOND.as_nanos_i128(), NANOS_PER_SECOND);
        assert!(ExactDuration::ZERO.is_zero());
        assert!(!ExactDuration::SECOND.is_negative());
        assert!((-ExactDuration::SECOND).is_negative());
    }

    #[test]
    fn from_seconds_and_nanos_signs() {
        let half_neg = ExactDuration::from_seconds_and_nanos(-1, 500_000_000).unwrap();
        assert_eq!(half_neg.as_nanos_i128(), -NANOS_PER_SECOND + 500_000_000);
        let half_pos = ExactDuration::from_seconds_and_nanos(0, 500_000_000).unwrap();
        assert_eq!(half_pos.as_nanos_i128(), 500_000_000);
    }

    #[test]
    fn boundary_projection_round_trip() {
        for nanos in [
            0_i128,
            1,
            -1,
            NANOS_PER_SECOND,
            -NANOS_PER_SECOND,
            1_234_567_890,
            -9_876_543_210,
        ] {
            let d = ExactDuration::from_nanos(nanos);
            let (s, n) = d.as_seconds_i64_nanos();
            let recovered = (s as i128) * NANOS_PER_SECOND + n as i128;
            assert_eq!(recovered, nanos, "round trip failed for {nanos}");
        }
    }

    #[test]
    fn neg_round_trip_and_min_overflow() {
        let d = ExactDuration::from_nanos(1_500_000_000);
        assert_eq!((-(-d)), d);
        assert!(matches!(
            ExactDuration::MIN.checked_neg(),
            Err(DurationError::Overflow)
        ));
    }

    #[test]
    fn ordering_matches_i128() {
        let a = ExactDuration::from_nanos(-5);
        let b = ExactDuration::from_nanos(0);
        let c = ExactDuration::from_nanos(5);
        assert!(a < b && b < c);
        assert_eq!(a.cmp(&a), Ordering::Equal);
    }

    #[test]
    fn checked_add_sub_overflow() {
        assert_eq!(
            ExactDuration::MAX.checked_add(ExactDuration::NANOSECOND),
            Err(DurationError::Overflow)
        );
        assert_eq!(
            ExactDuration::MIN.checked_sub(ExactDuration::NANOSECOND),
            Err(DurationError::Overflow)
        );
        assert_eq!(
            ExactDuration::ZERO
                .checked_add(ExactDuration::SECOND)
                .unwrap(),
            ExactDuration::SECOND
        );
    }

    #[test]
    fn saturating_add_sub() {
        assert_eq!(
            ExactDuration::MAX.saturating_add(ExactDuration::SECOND),
            ExactDuration::MAX
        );
        assert_eq!(
            ExactDuration::MIN.saturating_sub(ExactDuration::SECOND),
            ExactDuration::MIN
        );
    }

    #[test]
    fn quantity_round_trip_within_mantissa() {
        let q = Second::new(123.456_789_012_345);
        let d = ExactDuration::try_from_quantity(q).unwrap();
        let back = d.as_quantity::<SecondUnit>();
        assert!((back.value() - q.value()).abs() < 1e-9);
    }

    #[test]
    fn quantity_non_finite_errors() {
        assert_eq!(
            ExactDuration::try_from_quantity(Second::new(f64::NAN)),
            Err(DurationError::NonFinite)
        );
        assert_eq!(
            ExactDuration::try_from_quantity(Second::new(f64::INFINITY)),
            Err(DurationError::NonFinite)
        );
    }

    #[test]
    fn quantity_overflow_errors() {
        // 1e25 seconds is far outside i128 ns range (1.7e29 ns max).
        // Use a value that triggers overflow when multiplied by 1e9.
        let q = Second::new(1.0e30);
        assert_eq!(
            ExactDuration::try_from_quantity(q),
            Err(DurationError::Overflow)
        );
    }

    #[test]
    fn quantity_unit_conversion() {
        let ms = Quantity::<MsUnit>::new(1500.0);
        let d = ExactDuration::try_from_quantity(ms).unwrap();
        assert_eq!(d.as_nanos_i128(), 1_500_000_000);

        let day = Quantity::<DayUnit>::new(1.0);
        let d2 = ExactDuration::try_from_quantity(day).unwrap();
        assert_eq!(d2.as_nanos_i128(), 86_400 * NANOS_PER_SECOND);
    }

    #[test]
    fn from_seconds_f64_lossy_handles_edges() {
        assert!(ExactDuration::from_seconds_f64_lossy(f64::NAN).is_none());
        assert!(ExactDuration::from_seconds_f64_lossy(f64::INFINITY).is_none());
        assert_eq!(
            ExactDuration::from_seconds_f64_lossy(1.5)
                .unwrap()
                .as_nanos_i128(),
            1_500_000_000
        );
    }

    #[test]
    fn display_basic() {
        assert_eq!(ExactDuration::SECOND.to_string(), "1 s");
        assert_eq!(ExactDuration::from_nanos(0).to_string(), "0 s");
        assert_eq!(
            ExactDuration::from_seconds_and_nanos(3, 250_000_000)
                .unwrap()
                .to_string(),
            "3.250000000 s"
        );
    }

    #[test]
    fn add_sub_neg_operators() {
        let a = ExactDuration::SECOND;
        let b = ExactDuration::NANOSECOND;
        assert_eq!((a + b).as_nanos_i128(), 1_000_000_001);
        assert_eq!((a - b).as_nanos_i128(), 999_999_999);
        assert_eq!((-a).as_nanos_i128(), -1_000_000_000);

        let mut c = a;
        c += b;
        assert_eq!(c.as_nanos_i128(), 1_000_000_001);
        c -= b;
        assert_eq!(c.as_nanos_i128(), 1_000_000_000);
    }

    #[test]
    #[should_panic(expected = "overflowed")]
    fn add_panics_on_overflow() {
        let _ = ExactDuration::MAX + ExactDuration::NANOSECOND;
    }

    #[test]
    fn checked_abs_works() {
        assert_eq!(
            ExactDuration::from_nanos(-5)
                .checked_abs()
                .unwrap()
                .as_nanos_i128(),
            5
        );
        assert!(matches!(
            ExactDuration::MIN.checked_abs(),
            Err(DurationError::Overflow)
        ));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let cases = [0_i128, 1, -1, 1_500_000_000, -2_345_678_901];
        for n in cases {
            let d = ExactDuration::from_nanos(n);
            let s = serde_json::to_string(&d).unwrap();
            let back: ExactDuration = serde_json::from_str(&s).unwrap();
            assert_eq!(back, d, "serde round-trip {n}");
        }
    }

    #[test]
    fn floor_ceil_round_basic() {
        let q = ExactDuration::from_nanos(1_000_000_000); // 1 s
        assert_eq!(
            ExactDuration::from_nanos(1_500_000_000)
                .floor_to(q)
                .as_nanos_i128(),
            1_000_000_000
        );
        assert_eq!(
            ExactDuration::from_nanos(1_500_000_000)
                .ceil_to(q)
                .as_nanos_i128(),
            2_000_000_000
        );
        // Half-to-even: 1.5 rounds to 2 (even); 2.5 rounds to 2 (even); 0.5 rounds to 0 (even).
        assert_eq!(
            ExactDuration::from_nanos(1_500_000_000)
                .round_to(q)
                .as_nanos_i128(),
            2_000_000_000
        );
        assert_eq!(
            ExactDuration::from_nanos(2_500_000_000)
                .round_to(q)
                .as_nanos_i128(),
            2_000_000_000
        );
        assert_eq!(
            ExactDuration::from_nanos(500_000_000)
                .round_to(q)
                .as_nanos_i128(),
            0
        );
    }

    #[test]
    fn floor_ceil_round_negative() {
        let q = ExactDuration::from_nanos(1_000_000_000);
        // -1.5 s
        let n = ExactDuration::from_nanos(-1_500_000_000);
        assert_eq!(n.floor_to(q).as_nanos_i128(), -2_000_000_000);
        assert_eq!(n.ceil_to(q).as_nanos_i128(), -1_000_000_000);
        // half-to-even on -1.5 → -2 (even)
        assert_eq!(n.round_to(q).as_nanos_i128(), -2_000_000_000);
    }

    #[test]
    fn round_with_non_positive_quantum_is_identity() {
        let n = ExactDuration::from_nanos(123);
        assert_eq!(n.round_to(ExactDuration::ZERO), n);
        assert_eq!(n.floor_to(ExactDuration::from_nanos(-1)), n);
        assert_eq!(n.ceil_to(ExactDuration::ZERO), n);
    }

    #[test]
    fn round_floor_ceil_saturate_at_extremes() {
        let q = ExactDuration::SECOND;
        // Near i128::MAX: result should not panic, may saturate.
        let near_max = ExactDuration::MAX;
        let _ = near_max.round_to(q);
        let _ = near_max.floor_to(q);
        let _ = near_max.ceil_to(q);
        let near_min = ExactDuration::MIN;
        let _ = near_min.round_to(q);
        let _ = near_min.floor_to(q);
        let _ = near_min.ceil_to(q);
    }

    #[test]
    #[should_panic(expected = "ExactDuration::from_quantity")]
    fn from_quantity_panics_on_nan() {
        let _ = ExactDuration::from_quantity(Second::new(f64::NAN));
    }

    #[test]
    #[should_panic(expected = "ExactDuration::from_quantity")]
    fn from_quantity_panics_on_overflow() {
        let _ = ExactDuration::from_quantity(Second::new(1.0e40));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_serialize_fails_on_out_of_range() {
        // Duration of ~300 billion years — exceeds i64 seconds range.
        let huge = ExactDuration::MAX;
        let result = serde_json::to_string(&huge);
        assert!(
            result.is_err(),
            "expected serde error for out-of-range duration"
        );
    }
}
