//! Game engine tests: hatching and fresh-egg basics. The v2 rule tests live
//! in `engine_v3.rs`.

use bornhack_aegg::game::engine::thresholds::*;
use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

#[test]
fn new_egg_starts_hatching() {
    let state = GameState::new_egg(42, PetKind::DEFAULT);
    assert_eq!(state.phase, Phase::Hatching);
    assert_eq!(state.hatching_countdown, HATCHING_TICKS());
    assert_eq!(state.fullness, 60 * 655);
    assert_eq!(state.energy, 80 * 655);
    assert!(state.health > 0); // health = 85 % - (MAX - vitality) / 8
}

#[test]
fn hatching_completes() {
    let mut state = GameState::new_egg(42, PetKind::DEFAULT);
    state.update(HATCHING_TICKS() as u32);
    assert_eq!(state.phase, Phase::Active);
    assert_eq!(state.hatching_countdown, 0);
}

#[test]
fn next_wake_tick_during_hatching() {
    let state = GameState::new_egg(42, PetKind::DEFAULT);
    let wake = state.next_wake_tick();
    assert_eq!(
        wake,
        HATCHING_TICKS() as u32,
        "should wake exactly when hatching completes"
    );
}
