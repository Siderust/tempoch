//! Serde round-trip examples for `Time<S>` and `Period<S>`.

use qtty::Second;
use tempoch::{Period, Time, TT};

fn main() {
    let tt = Time::<TT>::from_j2000_seconds(Second::new(42.5)).unwrap();
    let window = Period::<TT>::new(61_000.0, 61_001.0);

    let tt_json = serde_json::to_string(&tt).unwrap();
    let window_json = serde_json::to_string(&window).unwrap();

    let tt_back: Time<TT> = serde_json::from_str(&tt_json).unwrap();
    let window_back: Period<TT> = serde_json::from_str(&window_json).unwrap();

    println!("Time JSON   : {tt_json}");
    println!("Period JSON : {window_json}");
    println!("TT round-trip     : {:.1}", tt_back.j2000_seconds().value());
    println!(
        "Window round-trip : {:.1} → {:.1}",
        window_back.start.j2000_seconds().value(),
        window_back.end.j2000_seconds().value()
    );
}
