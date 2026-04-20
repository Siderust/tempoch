use qtty::Day;
use tempoch::{refresh_runtime_time_data, update_runtime_time_data, JD, UnixSecs, Time, TimeContext, TT, UT1, UTC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = update_runtime_time_data();
    let _ = refresh_runtime_time_data();

    let ctx = TimeContext::with_builtin_eop();
    let probe_tt = Time::<TT>::from_julian_days(Day::new(2_460_000.25))?;
    let probe_ut1: Time<UT1> = probe_tt.to_with::<UT1>(&ctx)?;

    let unix = Time::<UTC>::from_unix_seconds(1_700_000_000.0.into())?;
    let back = unix.try_to::<UnixSecs>()?;

    println!("probe TT JD  : {:.9}", probe_tt.to::<JD>().value());
    println!("probe UT1 JD : {:.9}", probe_ut1.to::<JD>().value());
    println!("Unix roundtrip: {:.3}", back.value());
    Ok(())
}
