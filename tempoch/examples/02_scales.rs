use qtty::Second;
use tempoch::{
    constats::J2000_JD_TT, J2000s, JulianDate, Time, TimeContext, JD, TAI, TCB, TCG, TDB, TT,
    UT1, UTC,
};

fn main() {
    let ctx = TimeContext::new();
    let tt = JulianDate::<TT>::try_new(J2000_JD_TT).unwrap().to_time();

    let tai: Time<TAI> = tt.to::<TAI>();
    let utc: Time<UTC> = tt.to::<UTC>();
    let ut1: Time<UT1> = tt.to_with::<UT1>(&ctx).unwrap();
    let tdb: Time<TDB> = tt.to::<TDB>();
    let tcg: Time<TCG> = tt.to::<TCG>();
    let tcb: Time<TCB> = tt.to::<TCB>();

    println!("TT  JD  : {:.9}", tt.to::<JD>().raw());
    println!("TAI JD  : {:.9}", tai.to::<JD>().raw());
    println!("UT1 JD  : {:.9}", ut1.to::<JD>().raw());
    println!("TDB JD  : {:.9}", tdb.to::<JD>().raw());
    println!("TCG JD  : {:.9}", tcg.to::<JD>().raw());
    println!("TCB JD  : {:.9}", tcb.to::<JD>().raw());
    println!("UTC     : {}", utc.to_chrono().unwrap());
    println!(
        "TT-TAI  : {:.6}",
        tt.to::<J2000s>().raw() - tai.to::<J2000s>().raw()
    );
    println!(
        "TT-UT1  : {:.6}",
        tt.to::<J2000s>().raw() - ut1.to::<J2000s>().raw()
    );
    assert!(
        (tt.to::<J2000s>().raw() - tai.to::<J2000s>().raw() - Second::new(32.184)).abs()
            < Second::new(1e-9)
    );
}
