use qtty::Second;
use tempoch::{JulianDate, Time, TimeContext, Unix, UnixTime, JD, TT, UT1, UTC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TimeContext::with_builtin_eop();
    let probe_tt = JulianDate::<TT>::new(2_460_000.25).to_j2000s();
    let probe_ut1: Time<UT1> = probe_tt.to_with::<UT1>(&ctx)?;

    let unix = UnixTime::try_new(Second::new(1_700_000_000.0))?.to_j2000s();
    let back = unix.try_to::<Unix>()?;

    println!("probe TT JD  : {:.9}", probe_tt.to::<JD>());
    println!("probe UT1 JD : {:.9}", probe_ut1.to::<JD>());
    println!("Unix roundtrip: {:.3}", back);
    let _ = Time::<UTC>::from_chrono(chrono::Utc::now());
    Ok(())
}
