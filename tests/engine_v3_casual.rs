//! Routines under the Casual preset (R14). The threshold table is write-once
//! per process, so these tests live in a binary of their own, and every test
//! installs the preset through the same guarded call.

use bornhack_aegg::game::engine::sim::{self, Driver};
use bornhack_aegg::game::engine::thresholds::{Mode, active_mode};

fn install_casual() {
    sim::install_preset(Mode::Casual.preset(), Mode::Casual, false);
    assert_eq!(active_mode(), Mode::Casual);
}

#[test]
fn casual_routines_delta_matches_oracle() {
    install_casual();
    for (name, r) in [
        ("balanced", sim::balanced as fn(u32, u32, &_) -> _),
        ("lazy_treats", sim::lazy_treats),
        ("party", sim::party),
    ] {
        let o = sim::run_routine(&r, 30, Driver::Oracle);
        let d = sim::run_routine(&r, 30, Driver::Delta);
        println!("casual {name}: oracle {o:?}\n               delta  {d:?}");
        // R21: the delta engine is exact, so every field the oracle reports
        // is equal (the wake count exists only for the delta driver).
        assert_eq!(o.alive, d.alive, "{name}: alive");
        assert_eq!(o.phase, d.phase, "{name}: phase");
        assert_eq!(o.avg_mood_pct, d.avg_mood_pct, "{name}: avg mood");
        assert_eq!(o.end_health_pct, d.end_health_pct, "{name}: end health");
        assert_eq!(o.min_health_pct, d.min_health_pct, "{name}: min health");
        assert_eq!(o.end_fitness_pct, d.end_fitness_pct, "{name}: fitness");
        assert_eq!(o.end_weight_pct, d.end_weight_pct, "{name}: weight");
        assert_eq!(o.injuries, d.injuries, "{name}: injuries");
    }
}

#[test]
fn casual_is_gentler_than_classic_on_the_lazy_pet() {
    install_casual();
    let lazy = sim::run_routine(&sim::lazy_treats, 30, Driver::Delta);
    assert!(lazy.alive);
    // Classic: end health 55..=70 (design_targets). Halved damage and wider
    // bands must not make it worse.
    assert!(lazy.end_health_pct >= 55, "{lazy:?}");
}
