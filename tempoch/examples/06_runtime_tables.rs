use qtty::Day;
use tempoch::{Time, TimeContext, UnixSecs, JD, TT, UT1, UTC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TimeContext::with_builtin_eop();
    let probe_tt = Time::<TT>::from_julian_days(Day::new(2_460_000.25))?;
    let probe_ut1: Time<UT1> = probe_tt.to_with::<UT1>(&ctx)?;

    let unix = Time::<UTC>::from_unix_seconds(1_700_000_000.0.into())?;
    let back = unix.try_to::<UnixSecs>()?;

    println!("probe TT JD  : {:.9}", probe_tt.to::<JD>());
    println!("probe UT1 JD : {:.9}", probe_ut1.to::<JD>());
    println!("Unix roundtrip: {:.3}", back);
    Ok(())
}
