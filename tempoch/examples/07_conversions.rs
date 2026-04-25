use qtty::Second;
use tempoch::{
    GPS, GpsTime, J2000s, Time, TimeContext, Unix, UnixTime, JD, MJD, TAI, TDB, TT, UT1,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TimeContext::with_builtin_eop();

    // Start from a civil/transport representation.
    let utc = UnixTime::try_new(Second::new(1_700_000_000.25))
        .and_then(|e| e.to_time_with(&ctx))?;

    // Convert across continuous scales.
    let tai: Time<TAI> = utc.to::<TAI>();
    let tt: Time<TT> = tai.to::<TT>();
    let tdb: Time<TDB> = tt.to::<TDB>();
    let ut1: Time<UT1> = utc.to_with::<UT1>(&ctx)?;

    // TAI also exposes the GPS bridge.
    let gps: GpsTime = tai.to::<GPS>();
    let tai_from_gps: Time<TAI> = gps.to_time();

    println!("UTC chrono   : {}", utc.to_chrono().unwrap());
    println!("UTC unix     : {:.3}", utc.try_to::<Unix>()?);
    println!("TAI J2000 s  : {:.9}", tai.to::<J2000s>());
    println!("TT JD        : {:.9}", tt.to::<JD>());
    println!("TDB JD       : {:.9}", tdb.to::<JD>());
    println!("UT1 MJD      : {:.9}", ut1.to::<MJD>());
    println!("GPS seconds  : {:.3}", gps);
    println!("Leap second? : {}", utc.is_leap_second());

    assert!(
        (tai_from_gps.to::<J2000s>().raw() - tai.to::<J2000s>().raw()).abs() < Second::new(1e-9)
    );

    Ok(())
}
