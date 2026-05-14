// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Strongly-typed raw time coordinates.
//!
//! [`Coord<S, F>`] is an affine *point* in time on scale `S` (TT, TAI, UTC,
//! …) in format `F` (JD, MJD, J2000 seconds, Unix, GPS, …). [`Offset<S, F>`]
//! is its associated displacement vector.
//!
//! Together, these two types let the crate name *and type-check* values that
//! used to circulate as bare `qtty::Day` / `qtty::Second`. For example,
//! `Coord<TT, JD>` is statically distinct from `Coord<UTC, JD>`, so the
//! compiler now rejects mistakes like reusing a UTC-axis Julian Date as a TT
//! one.
//!
//! The type parameter order `<S, F>` (Scale first, Format second) mirrors
//! [`EncodedTime<S, F>`](crate::EncodedTime) for consistency.
//!
//! # Affine semantics
//!
//! - `Coord - Coord -> Offset`
//! - `Coord + Offset -> Coord`
//! - `Coord - Offset -> Coord`
//! - `Offset + Offset -> Offset`
//! - `Offset - Offset -> Offset`
//! - `-Offset -> Offset`
//!
//! Adding two coordinates is intentionally not modeled — averaging or summing
//! instants in the same coordinate system is not a primitive operation here.
//!
//! # Interop with [`EncodedTime`]
//!
//! `Coord<S, F>` and [`EncodedTime<S, F>`](crate::EncodedTime) carry the
//! same information (a typed quantity, a scale, and a format). Conversion in
//! both directions is zero-cost via [`From`] / [`Into`]. Use `Coord` for raw
//! coordinate arithmetic and constants; use `EncodedTime` for the high-level
//! `to_time*` / `to::<Target>()` conversion machinery.

use core::fmt;
use core::marker::PhantomData;
use core::ops::{Add, Neg, Sub};

use crate::error::ConversionError;
use crate::format::{EncodedTime, TimeFormat};
use crate::scale::Scale;
use qtty::Quantity;

/// A typed time coordinate on scale `S` in format `F`.
///
/// `Coord<S, F>` is an affine point. To shift it, add an [`Offset<S, F>`].
/// To take the directed distance between two coordinates, subtract them.
///
/// `Coord` mirrors [`EncodedTime`] but is intentionally smaller in scope: it
/// only exposes raw-quantity access and affine arithmetic. The
/// `EncodedTime` API (`to_time`, `to::<Target>()`, …) is reachable through
/// the `From`/`Into` conversion below.
pub struct Coord<S: Scale, F: TimeFormat> {
    raw: Quantity<F::Unit>,
    _marker: PhantomData<fn() -> S>,
}

/// A typed displacement between two [`Coord<S, F>`] values.
pub struct Offset<S: Scale, F: TimeFormat> {
    raw: Quantity<F::Unit>,
    _marker: PhantomData<fn() -> S>,
}

// ── Common ZST plumbing (Copy/Clone/PartialEq/PartialOrd/Hash/Debug) ─────

macro_rules! impl_zst_plumbing {
    ($ty:ident, $kind:literal) => {
        impl<S: Scale, F: TimeFormat> Copy for $ty<S, F> {}

        impl<S: Scale, F: TimeFormat> Clone for $ty<S, F> {
            #[inline]
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<S: Scale, F: TimeFormat> PartialEq for $ty<S, F> {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.raw == other.raw
            }
        }

        impl<S: Scale, F: TimeFormat> PartialOrd for $ty<S, F> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                self.raw.partial_cmp(&other.raw)
            }
        }

        impl<S: Scale, F: TimeFormat> fmt::Debug for $ty<S, F> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct($kind)
                    .field("scale", &S::NAME)
                    .field("format", &F::NAME)
                    .field("raw", &self.raw)
                    .finish()
            }
        }

        impl<S: Scale, F: TimeFormat> fmt::Display for $ty<S, F>
        where
            qtty::Quantity<F::Unit>: fmt::Display,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.raw, f)
            }
        }

        impl<S: Scale, F: TimeFormat> fmt::LowerExp for $ty<S, F>
        where
            qtty::Quantity<F::Unit>: fmt::LowerExp,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::LowerExp::fmt(&self.raw, f)
            }
        }

        impl<S: Scale, F: TimeFormat> fmt::UpperExp for $ty<S, F>
        where
            qtty::Quantity<F::Unit>: fmt::UpperExp,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::UpperExp::fmt(&self.raw, f)
            }
        }

        impl<S: Scale, F: TimeFormat> $ty<S, F> {
            /// Wrap a raw quantity without checking finiteness.
            ///
            /// Provided for `const` contexts such as crate-level constants.
            /// The caller is responsible for passing a finite value.
            #[inline]
            pub const fn from_raw_unchecked(raw: Quantity<F::Unit>) -> Self {
                Self {
                    raw,
                    _marker: PhantomData,
                }
            }

            /// Return the underlying typed quantity.
            #[inline]
            pub const fn raw(self) -> Quantity<F::Unit> {
                self.raw
            }

            /// Alias for [`Self::raw`].
            #[inline]
            pub const fn quantity(self) -> Quantity<F::Unit> {
                self.raw
            }
        }
    };
}

impl_zst_plumbing!(Coord, "Coord");
impl_zst_plumbing!(Offset, "Offset");

// ── Checked constructors ─────────────────────────────────────────────────

impl<S: Scale, F: TimeFormat> Coord<S, F> {
    /// Build a coordinate from a typed quantity, validating finiteness.
    #[inline]
    pub fn try_new(raw: Quantity<F::Unit>) -> Result<Self, ConversionError> {
        if raw.is_finite() {
            Ok(Self::from_raw_unchecked(raw))
        } else {
            Err(ConversionError::NonFinite)
        }
    }
}

impl<S: Scale, F: TimeFormat> Offset<S, F> {
    /// Build an offset from a typed quantity, validating finiteness.
    #[inline]
    pub fn try_new(raw: Quantity<F::Unit>) -> Result<Self, ConversionError> {
        if raw.is_finite() {
            Ok(Self::from_raw_unchecked(raw))
        } else {
            Err(ConversionError::NonFinite)
        }
    }

    /// The zero offset on this `(scale, format)` pair.
    #[inline]
    pub fn zero() -> Self
    where
        Quantity<F::Unit>: Default,
    {
        Self::from_raw_unchecked(Quantity::<F::Unit>::default())
    }
}

// ── Affine arithmetic ────────────────────────────────────────────────────

impl<S: Scale, F: TimeFormat> Sub for Coord<S, F>
where
    Quantity<F::Unit>: Sub<Output = Quantity<F::Unit>>,
{
    type Output = Offset<S, F>;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Offset::from_raw_unchecked(self.raw - rhs.raw)
    }
}

impl<S: Scale, F: TimeFormat> Add<Offset<S, F>> for Coord<S, F>
where
    Quantity<F::Unit>: Add<Output = Quantity<F::Unit>>,
{
    type Output = Coord<S, F>;

    #[inline]
    fn add(self, rhs: Offset<S, F>) -> Self::Output {
        Coord::from_raw_unchecked(self.raw + rhs.raw)
    }
}

impl<S: Scale, F: TimeFormat> Sub<Offset<S, F>> for Coord<S, F>
where
    Quantity<F::Unit>: Sub<Output = Quantity<F::Unit>>,
{
    type Output = Coord<S, F>;

    #[inline]
    fn sub(self, rhs: Offset<S, F>) -> Self::Output {
        Coord::from_raw_unchecked(self.raw - rhs.raw)
    }
}

impl<S: Scale, F: TimeFormat> Add for Offset<S, F>
where
    Quantity<F::Unit>: Add<Output = Quantity<F::Unit>>,
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self::from_raw_unchecked(self.raw + rhs.raw)
    }
}

impl<S: Scale, F: TimeFormat> Sub for Offset<S, F>
where
    Quantity<F::Unit>: Sub<Output = Quantity<F::Unit>>,
{
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::from_raw_unchecked(self.raw - rhs.raw)
    }
}

impl<S: Scale, F: TimeFormat> Neg for Offset<S, F>
where
    Quantity<F::Unit>: Neg<Output = Quantity<F::Unit>>,
{
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self::from_raw_unchecked(-self.raw)
    }
}

// ── Interop with EncodedTime ────────────────────────────────────────────

impl<S: Scale, F: TimeFormat> From<Coord<S, F>> for EncodedTime<S, F> {
    #[inline]
    fn from(value: Coord<S, F>) -> Self {
        EncodedTime::<S, F>::from_raw_unchecked(value.raw)
    }
}

impl<S: Scale, F: TimeFormat> From<EncodedTime<S, F>> for Coord<S, F> {
    #[inline]
    fn from(value: EncodedTime<S, F>) -> Self {
        Coord::<S, F>::from_raw_unchecked(value.raw())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{JD, MJD};
    use crate::scale::{TT, UTC};
    use qtty::Day;

    #[test]
    fn coord_round_trip_with_encoded_time() {
        let c = Coord::<TT, JD>::try_new(Day::new(2_451_545.5)).unwrap();
        let e: EncodedTime<TT, JD> = c.into();
        let back: Coord<TT, JD> = e.into();
        assert_eq!(c, back);
    }

    #[test]
    fn coord_minus_coord_yields_offset() {
        let a = Coord::<TT, JD>::from_raw_unchecked(Day::new(2_451_545.5));
        let b = Coord::<TT, JD>::from_raw_unchecked(Day::new(2_451_545.0));
        let v: Offset<TT, JD> = a - b;
        assert_eq!(v.raw(), Day::new(0.5));
    }

    #[test]
    fn coord_plus_offset_yields_coord() {
        let a = Coord::<TT, JD>::from_raw_unchecked(Day::new(2_451_545.0));
        let v = Offset::<TT, JD>::from_raw_unchecked(Day::new(1.5));
        let b = a + v;
        assert_eq!(b.raw(), Day::new(2_451_546.5));
    }

    #[test]
    fn offset_arithmetic() {
        let v = Offset::<TT, JD>::from_raw_unchecked(Day::new(1.0));
        let w = Offset::<TT, JD>::from_raw_unchecked(Day::new(0.25));
        assert_eq!((v + w).raw(), Day::new(1.25));
        assert_eq!((v - w).raw(), Day::new(0.75));
        assert_eq!((-v).raw(), Day::new(-1.0));
    }

    #[test]
    fn try_new_rejects_non_finite() {
        let nan = Coord::<TT, JD>::try_new(Day::new(f64::NAN));
        assert!(matches!(nan, Err(ConversionError::NonFinite)));
        let inf = Offset::<UTC, MJD>::try_new(Day::new(f64::INFINITY));
        assert!(matches!(inf, Err(ConversionError::NonFinite)));
    }

    #[test]
    fn debug_includes_scale_and_format() {
        let c = Coord::<TT, JD>::from_raw_unchecked(Day::new(2_451_545.0));
        let dbg = format!("{c:?}");
        assert!(dbg.contains("TT"));
        assert!(dbg.contains("JD"));
    }

    #[test]
    fn display_delegates_to_quantity() {
        let c = Coord::<TT, JD>::from_raw_unchecked(Day::new(2_451_545.5));
        assert_eq!(format!("{c:.1}"), "2451545.5 d");
    }

    #[test]
    fn coord_scale_phantom_prevents_mixing() {
        fn accept_tt_jd(c: Coord<TT, JD>) -> Day {
            c.raw()
        }
        fn accept_utc_jd(c: Coord<UTC, JD>) -> Day {
            c.raw()
        }

        let tt = Coord::<TT, JD>::from_raw_unchecked(Day::new(2_451_545.0));
        let utc = Coord::<UTC, JD>::from_raw_unchecked(Day::new(2_451_545.0));

        let _ = accept_tt_jd(tt);
        let _ = accept_utc_jd(utc);
    }

    #[test]
    fn coord_quantity_is_alias_for_raw() {
        let c = Coord::<TT, JD>::from_raw_unchecked(Day::new(2_451_545.5));
        assert_eq!(c.raw(), c.quantity());
    }

    #[test]
    fn offset_quantity_is_alias_for_raw() {
        let v = Offset::<TT, JD>::from_raw_unchecked(Day::new(0.5));
        assert_eq!(v.raw(), v.quantity());
    }

    #[test]
    fn coord_minus_offset_yields_coord() {
        let a = Coord::<TT, JD>::from_raw_unchecked(Day::new(2_451_546.0));
        let v = Offset::<TT, JD>::from_raw_unchecked(Day::new(1.0));
        let b = a - v;
        assert_eq!(b.raw(), Day::new(2_451_545.0));
    }

    #[test]
    fn coord_lower_exp_delegates_to_quantity() {
        let c = Coord::<TT, JD>::from_raw_unchecked(Day::new(2_451_545.5));
        assert_eq!(format!("{c:.2e}"), format!("{:.2e}", c.raw()));
    }

    #[test]
    fn coord_upper_exp_delegates_to_quantity() {
        let c = Coord::<TT, JD>::from_raw_unchecked(Day::new(2_451_545.5));
        assert_eq!(format!("{c:.2E}"), format!("{:.2E}", c.raw()));
    }

    #[test]
    fn offset_lower_exp_delegates_to_quantity() {
        let v = Offset::<TT, JD>::from_raw_unchecked(Day::new(1.5));
        assert_eq!(format!("{v:.2e}"), format!("{:.2e}", v.raw()));
    }

    #[test]
    fn offset_upper_exp_delegates_to_quantity() {
        let v = Offset::<TT, JD>::from_raw_unchecked(Day::new(1.5));
        assert_eq!(format!("{v:.2E}"), format!("{:.2E}", v.raw()));
    }

    #[test]
    fn offset_debug_includes_scale_and_format() {
        let v = Offset::<TT, JD>::from_raw_unchecked(Day::new(1.0));
        let dbg = format!("{v:?}");
        assert!(dbg.contains("TT"));
        assert!(dbg.contains("JD"));
    }
}
