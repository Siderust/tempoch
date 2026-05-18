//! Serde round-trip examples for `Time<S>` and `Period<S>`.

use tempoch::{
    tagged::{TaggedPeriod, TaggedTime},
    J2000Seconds, J2000s, Period, Time, TT,
};

fn main() {
    let tt = J2000Seconds::<TT>::new(42.5).to_j2000s();
    let window = Period::<TT>::new(
        J2000Seconds::<TT>::new(61_000.0).to_j2000s(),
        J2000Seconds::<TT>::new(61_001.0).to_j2000s(),
    );

    let tt_json = serde_json::to_string(&tt).unwrap();
    let window_json = serde_json::to_string(&window).unwrap();
    let tagged_tt_json = serde_json::to_string(&TaggedTime(tt)).unwrap();
    let tagged_window_json = serde_json::to_string(&TaggedPeriod(window)).unwrap();

    let tt_back: Time<TT> = serde_json::from_str(&tt_json).unwrap();
    let window_back: Period<TT> = serde_json::from_str(&window_json).unwrap();
    let tagged_tt_back: Time<TT> = serde_json::from_str::<TaggedTime<TT>>(&tagged_tt_json)
        .unwrap()
        .into();
    let tagged_window_back: Period<TT> =
        serde_json::from_str::<TaggedPeriod<TT>>(&tagged_window_json)
            .unwrap()
            .into();

    println!("Time JSON   : {tt_json}");
    println!("Period JSON : {window_json}");
    println!("Tagged Time JSON   : {tagged_tt_json}");
    println!("Tagged Period JSON : {tagged_window_json}");
    println!("TT round-trip     : {:.1}", tt_back.to::<J2000s>());
    println!(
        "Window round-trip : {:.1} → {:.1}",
        window_back.start.to::<J2000s>(),
        window_back.end.to::<J2000s>()
    );
    println!(
        "Tagged TT round-trip     : {:.1}",
        tagged_tt_back.to::<J2000s>()
    );
    println!(
        "Tagged Window round-trip : {:.1} → {:.1}",
        tagged_window_back.start.to::<J2000s>(),
        tagged_window_back.end.to::<J2000s>()
    );
}
