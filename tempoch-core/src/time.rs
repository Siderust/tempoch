// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! `Time<S, F>` — the core public type.

use core::marker::PhantomData;

use super::context::TimeContext;
use super::error::ConversionError;
use super::format::{DayCount, Format, GpsSecs, J2000s, Jd, Mjd, UnixSecs};
use super::format_conversion::{CanonicalRoundtrip, FormatConvertible};
use super::scale::{ContinuousScale, Scale};
use super::scale_conversion::{ContextScaleConvert, InfallibleScaleConvert};
use qtty::time::Seconds;
use qtty::{Day, QuantityI32, QuantityI64, Second};

// ═══════════════════════════════════════════════════════════════════════════
// Time
// ═══════════════════════════════════════════════════════════════════════════

/// A point in time on scale `S` in format `F`.
///
/// `S` determines the physical time scale (`TT`, `TAI`, `UTC`, etc.).
/// `F` determines the numerical representation and storage type via
/// `qtty::Quantity`. Defaults to [`J2000s`](super::format::J2000s)
/// (SI seconds since J2000 TT) so `Time<TT>` works without specifying
/// a format.
///
/// # Scale conversions
///
/// - `.to_scale::<S2>()` — infallible closed-form routes (TT↔TAI, TT↔TDB, etc.)
/// - `.to_scale_with::<S2>(&ctx)` — context-required routes (UT1, via ΔT)
///
/// Scale conversions require `F: CanonicalRoundtrip` — they go through
/// the canonical J2000s representation internally. Integer-based formats
/// (`UnixSecs`, `DayCount`) must `.reformat::<J2000s>()` first.
///
/// # Format conversions
///
/// - `.reformat::<F2>()` — convert to a different format on the same scale
pub struct Time<S: Scale, F: Format = super::format::J2000s> {
    value: F::Storage,
    _scale: PhantomData<S>,
}

impl<S: Scale, F: Format> Copy for Time<S, F> where F::Storage: Copy {}
impl<S: Scale, F: Format> Clone for Time<S, F>
where
    F::Storage: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: Scale, F: Format> PartialEq for Time<S, F>
where
    F::Storage: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<S: Scale, F: Format> PartialOrd for Time<S, F>
where
    F::Storage: PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<S: Scale, F: Format> core::fmt::Debug for Time<S, F>
where
    F::Storage: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Time<{}, {}>({:?})", S::NAME, F::NAME, self.value)
    }
}

// ── Generic constructors and accessors ───────────────────────────────────

impl<S: Scale, F: Format> Time<S, F> {
    /// Build a `Time<S, F>` from a raw storage value.
    #[inline]
    pub fn new(value: F::Storage) -> Self {
        Self {
            value,
            _scale: PhantomData,
        }
    }

    /// Extract the raw storage value.
    #[inline]
    pub fn value(self) -> F::Storage {
        self.value
    }

    /// Convert to a different format on the same scale.
    #[allow(private_bounds)]
    #[inline]
    pub fn reformat<F2: Format>(self) -> Time<S, F2>
    where
        F: FormatConvertible<F2>,
    {
        Time::new(F::convert(self.value))
    }
}

// ── Raw value conversions for ergonomic construction ─────────────────────

impl<S: Scale> From<Second> for Time<S, J2000s> {
    #[inline]
    fn from(value: Second) -> Self {
        Self::new(value)
    }
}

impl<S: Scale> From<f64> for Time<S, J2000s> {
    #[inline]
    fn from(value: f64) -> Self {
        Self::new(Second::new(value))
    }
}

impl<S: Scale> From<Day> for Time<S, Jd> {
    #[inline]
    fn from(value: Day) -> Self {
        Self::new(value)
    }
}

impl<S: Scale> From<f64> for Time<S, Jd> {
    #[inline]
    fn from(value: f64) -> Self {
        Self::new(Day::new(value))
    }
}

impl<S: Scale> From<Day> for Time<S, Mjd> {
    #[inline]
    fn from(value: Day) -> Self {
        Self::new(value)
    }
}

impl<S: Scale> From<f64> for Time<S, Mjd> {
    #[inline]
    fn from(value: f64) -> Self {
        Self::new(Day::new(value))
    }
}

impl<S: Scale> From<Second> for Time<S, GpsSecs> {
    #[inline]
    fn from(value: Second) -> Self {
        Self::new(value)
    }
}

impl<S: Scale> From<f64> for Time<S, GpsSecs> {
    #[inline]
    fn from(value: f64) -> Self {
        Self::new(Second::new(value))
    }
}

impl<S: Scale> From<QuantityI64<qtty::unit::Second>> for Time<S, UnixSecs> {
    #[inline]
    fn from(value: QuantityI64<qtty::unit::Second>) -> Self {
        Self::new(value)
    }
}

impl<S: Scale> From<i64> for Time<S, UnixSecs> {
    #[inline]
    fn from(value: i64) -> Self {
        Self::new(QuantityI64::new(value))
    }
}

impl<S: Scale> From<QuantityI32<qtty::unit::Day>> for Time<S, DayCount> {
    #[inline]
    fn from(value: QuantityI32<qtty::unit::Day>) -> Self {
        Self::new(value)
    }
}

impl<S: Scale> From<i32> for Time<S, DayCount> {
    #[inline]
    fn from(value: i32) -> Self {
        Self::new(QuantityI32::new(value))
    }
}

// ── Validated constructors (f64 formats with finiteness check) ───────────

impl<S: Scale> Time<S, super::format::J2000s> {
    /// Build from SI seconds since J2000 TT. Fails on non-finite input.
    #[inline]
    pub fn from_si_seconds(seconds: Seconds) -> Result<Self, ConversionError> {
        if seconds.is_finite() {
            Ok(Self::new(seconds))
        } else {
            Err(ConversionError::NonFinite)
        }
    }

    /// SI seconds since J2000 TT on the internal (TAI-based) axis.
    ///
    /// For most scales this is directly the scale-coordinate value.
    ///
    /// **UTC caveat:** `Time<UTC>` stores the same numerical value as
    /// `Time<TAI>` for the same instant (see the [`UTC`](super::scale::UTC)
    /// scale doc). Therefore `.si_seconds()` on a `Time<UTC>` is **not** a
    /// UTC coordinate value — it differs from a true UTC timestamp by the
    /// accumulated leap-second offset (TAI − UTC). Use the civil API
    /// ([`unix_seconds`](super::civil)) for a POSIX-compatible timestamp.
    #[inline]
    pub fn si_seconds(self) -> Seconds {
        self.value
    }
}

impl<S: Scale> Time<S, super::format::Jd> {
    /// Build from a Julian Day number. Fails on non-finite input.
    #[inline]
    pub fn from_julian_days(jd: qtty::Day) -> Result<Self, ConversionError> {
        if jd.is_finite() {
            Ok(Self::new(jd))
        } else {
            Err(ConversionError::NonFinite)
        }
    }

    /// Julian Day number.
    #[inline]
    pub fn julian_days(self) -> qtty::Day {
        self.value
    }
}

impl<S: Scale> Time<S, super::format::Mjd> {
    /// Build from a Modified Julian Day value. Fails on non-finite input.
    #[inline]
    pub fn from_modified_julian_days(mjd: qtty::Day) -> Result<Self, ConversionError> {
        if mjd.is_finite() {
            Ok(Self::new(mjd))
        } else {
            Err(ConversionError::NonFinite)
        }
    }

    /// Modified Julian Day.
    #[inline]
    pub fn modified_julian_days(self) -> qtty::Day {
        self.value
    }
}

// ── Scale conversions ────────────────────────────────────────────────────

#[allow(private_bounds)]
impl<S: Scale, F: Format + CanonicalRoundtrip> Time<S, F> {
    /// Infallible scale conversion. Compiles only for pairs with a
    /// closed-form, context-free conversion (e.g. TT↔TAI, TT↔TDB).
    ///
    /// Requires `F: CanonicalRoundtrip` — the format must support
    /// round-tripping through J2000 SI seconds.
    #[allow(private_bounds)]
    #[inline]
    pub fn to_scale<S2: Scale>(self) -> Time<S2, F>
    where
        S: InfallibleScaleConvert<S2>,
    {
        let j2000s = F::to_j2000s(self.value);
        let converted = <S as InfallibleScaleConvert<S2>>::convert(j2000s);
        Time::new(F::from_j2000s(converted))
    }

    /// Context-required scale conversion (UT1 routes).
    #[allow(private_bounds)]
    #[inline]
    pub fn to_scale_with<S2: Scale>(self, ctx: &TimeContext) -> Result<Time<S2, F>, ConversionError>
    where
        S: ContextScaleConvert<S2>,
    {
        let j2000s = F::to_j2000s(self.value);
        let converted = <S as ContextScaleConvert<S2>>::convert_with(j2000s, ctx)?;
        Ok(Time::new(F::from_j2000s(converted)))
    }

    /// Convert both scale and format at once.
    #[allow(private_bounds)]
    #[inline]
    pub fn convert<S2: Scale, F2: Format + CanonicalRoundtrip>(self) -> Time<S2, F2>
    where
        S: InfallibleScaleConvert<S2>,
    {
        let j2000s = F::to_j2000s(self.value);
        let converted = <S as InfallibleScaleConvert<S2>>::convert(j2000s);
        Time::new(F2::from_j2000s(converted))
    }
}

// ── Arithmetic (continuous scales only) ──────────────────────────────────

impl<S: ContinuousScale, F: Format> core::ops::Sub for Time<S, F>
where
    F::Storage: core::ops::Sub<Output = F::Storage>,
{
    type Output = F::Storage;
    #[inline]
    fn sub(self, rhs: Self) -> F::Storage {
        self.value - rhs.value
    }
}

impl<S: ContinuousScale, F: Format> core::ops::Add<F::Storage> for Time<S, F>
where
    F::Storage: core::ops::Add<Output = F::Storage>,
{
    type Output = Self;
    #[inline]
    fn add(self, rhs: F::Storage) -> Self {
        Self::new(self.value + rhs)
    }
}

// Time - Duration uses Add with negation or explicit methods.
// Direct `Sub<F::Storage>` conflicts with `Sub for Time` due to
// potential overlap in generic resolution.

impl<S: ContinuousScale, F: Format> core::ops::AddAssign<F::Storage> for Time<S, F>
where
    F::Storage: core::ops::AddAssign,
{
    #[inline]
    fn add_assign(&mut self, rhs: F::Storage) {
        self.value += rhs;
    }
}

impl<S: ContinuousScale, F: Format> core::ops::SubAssign<F::Storage> for Time<S, F>
where
    F::Storage: core::ops::SubAssign,
{
    #[inline]
    fn sub_assign(&mut self, rhs: F::Storage) {
        self.value -= rhs;
    }
}

// No arithmetic for Time<UTC, _> — that is deliberate (RFC §9).

#[cfg(test)]
mod tests {
    use super::super::format::{J2000s, Jd, Mjd};
    use super::super::scale::{TAI, TCB, TCG, TDB, TT};
    use super::*;
    use qtty::{Day, Second};

    const SECONDS_PER_DAY: Second = Second::new(86_400.0);

    #[test]
    fn tt_tai_round_trip_exact() {
        let tt = Time::<TT>::from_si_seconds(Second::new(0.0)).unwrap();
        let tai = tt.to_scale::<TAI>();
        let tt2 = tai.to_scale::<TT>();
        assert_eq!(tt.si_seconds(), tt2.si_seconds());
        assert!((tai.si_seconds() - Second::new(-32.184)).abs() < Second::new(1e-15));
    }

    #[test]
    fn tt_tdb_round_trip_model_error() {
        let tt = Time::<TT>::from_si_seconds(Second::new(1_000_000.0)).unwrap();
        let tdb = tt.to_scale::<TDB>();
        let tt2 = tdb.to_scale::<TT>();
        assert!(
            (tt.si_seconds() - tt2.si_seconds()).abs() < Second::new(1e-6),
            "round-trip error {:?}",
            tt - tt2
        );
    }

    #[test]
    fn tt_tcg_rate_difference() {
        let tt0 = Time::<TT>::from_si_seconds(Second::new(0.0)).unwrap();
        let tt1 = Time::<TT>::from_si_seconds(SECONDS_PER_DAY).unwrap();
        let tcg0 = tt0.to_scale::<TCG>();
        let tcg1 = tt1.to_scale::<TCG>();
        let drift: Second = (tcg1 - tcg0) - SECONDS_PER_DAY;
        let l_g = 6.969_290_134e-10_f64;
        let expected: Second = SECONDS_PER_DAY * (l_g / (1.0 - l_g));
        assert!(
            (drift - expected).abs() < Second::new(1e-11),
            "drift = {:?}, expected = {:?}",
            drift,
            expected
        );
    }

    #[test]
    fn tdb_tcb_linear() {
        let tdb = Time::<TDB>::from_si_seconds(Second::new(1_000_000.0)).unwrap();
        let tcb = tdb.to_scale::<TCB>();
        let tdb2 = tcb.to_scale::<TDB>();
        assert!(
            (tdb.si_seconds() - tdb2.si_seconds()).abs() < Second::new(1e-6),
            "round-trip diff {:?}",
            tdb.si_seconds() - tdb2.si_seconds()
        );
    }

    #[test]
    fn julian_days_round_trip() {
        let jd = Day::new(2_451_545.0);
        let t = Time::<TT, Jd>::from_julian_days(jd).unwrap();
        assert_eq!(t.julian_days(), jd);
    }

    #[test]
    fn reformat_jd_to_mjd() {
        let jd = Day::new(2_451_545.0);
        let t_jd = Time::<TT, Jd>::from_julian_days(jd).unwrap();
        let t_mjd: Time<TT, Mjd> = t_jd.reformat();
        let expected_mjd = Day::new(2_451_545.0 - 2_400_000.5);
        assert!((t_mjd.modified_julian_days() - expected_mjd).abs() < Day::new(1e-9));
    }

    #[test]
    fn si_seconds_and_julian_days_consistent() {
        let t_jd = Time::<TT, Jd>::from_julian_days(Day::new(2_451_545.5)).unwrap();
        let t_s: Time<TT, J2000s> = t_jd.reformat();
        assert!((t_s.si_seconds() - SECONDS_PER_DAY / 2.0).abs() < Second::new(1e-10));
    }

    #[test]
    fn arithmetic_in_seconds() {
        let a = Time::<TT>::from_si_seconds(Second::new(0.0)).unwrap();
        let b = a + Second::new(10.0);
        let diff: Second = b - a;
        assert_eq!(diff, Second::new(10.0));
    }

    #[test]
    fn nonfinite_rejected() {
        assert_eq!(
            Time::<TT>::from_si_seconds(Second::new(f64::NAN)).unwrap_err(),
            ConversionError::NonFinite
        );
        assert_eq!(
            Time::<TT>::from_si_seconds(Second::new(f64::INFINITY)).unwrap_err(),
            ConversionError::NonFinite
        );
    }

    #[test]
    fn scale_conversion_in_jd_format() {
        let tt_jd = Time::<TT, Jd>::from_julian_days(Day::new(2_451_545.0)).unwrap();
        let tai_jd: Time<TAI, Jd> = tt_jd.to_scale();
        // TT = TAI + 32.184 s, so TAI JD should be slightly less
        let diff_days = tt_jd.julian_days() - tai_jd.julian_days();
        let diff_secs = diff_days.to::<qtty::unit::Second>();
        // JD values are ~2.4M, so we lose ~4 ULPs ≈ 40 µs in day arithmetic.
        assert!(
            (diff_secs - Second::new(32.184)).abs() < Second::new(1e-4),
            "diff = {:?}",
            diff_secs
        );
    }
}
