use qtty::Second;
use tempoch::{GpsSecs, J2000s, JD, MJD, Time, TimeContext, TAI, TDB, TT, UT1, UTC, UnixSecs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TimeContext::with_builtin_eop();

    // Start from a civil/transport representation.
    let utc = Time::<UTC>::from_unix_seconds(Second::new(1_700_000_000.25))?;

    // Convert across continuous scales.
    let tai: Time<TAI> = utc.to::<TAI>();
    let tt: Time<TT> = tai.to::<TT>();
    let tdb: Time<TDB> = tt.to::<TDB>();
    let ut1: Time<UT1> = utc.to_with::<UT1>(&ctx)?;

    // TAI also exposes the GPS bridge.
    let gps = tai.to::<GpsSecs>();
    let tai_from_gps = Time::<TAI>::from_gps_seconds(gps)?;

    println!("UTC chrono   : {}", utc.to_chrono().unwrap());
    println!("UTC unix     : {:.3}", utc.try_to::<UnixSecs>()?.value());
    println!("TAI J2000 s  : {:.9}", tai.to::<J2000s>().value());
    println!("TT JD        : {:.9}", tt.to::<JD>().value());
    println!("TDB JD       : {:.9}", tdb.to::<JD>().value());
    println!("UT1 MJD      : {:.9}", ut1.to::<MJD>().value());
    println!("GPS seconds  : {:.3}", gps.value());
    println!("Leap second? : {}", utc.is_leap_second());

    assert!((tai_from_gps.to::<J2000s>() - tai.to::<J2000s>()).abs() < Second::new(1e-9));

    Ok(())
}
