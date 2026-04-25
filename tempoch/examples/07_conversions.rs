use qtty::Second;
use tempoch::{GpsSecs, J2000s, Time, TimeContext, UnixSecs, JD, MJD, TAI, TDB, TT, UT1, UTC};

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
    println!("UTC unix     : {:.3}", utc.try_to::<UnixSecs>()?);
    println!("TAI J2000 s  : {:.9}", tai.to::<J2000s>());
    println!("TT JD        : {:.9}", tt.to::<JD>());
    println!("TDB JD       : {:.9}", tdb.to::<JD>());
    println!("UT1 MJD      : {:.9}", ut1.to::<MJD>());
    println!("GPS seconds  : {:.3}", gps);
    println!("Leap second? : {}", utc.is_leap_second());

    assert!((tai_from_gps.to::<J2000s>() - tai.to::<J2000s>()).abs() < Second::new(1e-9));

    Ok(())
}
