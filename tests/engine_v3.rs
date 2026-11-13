//! BornPets v3 engine tests (plan 1).

use bornhack_aegg::game::bornpets_cfg::{apply_overrides, is_custom_set};
use bornhack_aegg::game::engine::needs::{Need, build_edges};
use bornhack_aegg::game::engine::thresholds::{
    BORNPETS_CFG_KEYS, Mode, Thresholds, cfg_template, validate,
};

fn slice(t: &bornhack_aegg::game::engine::needs::EdgeTable, n: Need) -> &[u8] {
    &t.edges[n as usize][..t.len[n as usize] as usize]
}

#[test]
fn classic_edges_match_prototype() {
    let t = build_edges(&Thresholds::CLASSIC);
    let want: [&[u8]; 9] = [
        &[15, 25, 38, 78, 85, 95],
        &[15, 25, 38, 83, 85, 90, 95],
        &[10, 33, 50, 70, 85],
        &[33, 40, 73],
        &[25, 58, 85],
        &[10, 15, 30, 38, 70],
        &[15, 40, 60],
        &[25, 43, 50, 60, 61, 70],
        &[20, 38, 63, 65, 80],
    ];
    for i in 0..9 {
        assert_eq!(&t.edges[i][..t.len[i] as usize], want[i], "need {i}");
    }
}

#[test]
fn casual_bands_are_five_wider() {
    assert_eq!(Thresholds::CASUAL.FULL_LO_PCT, 33);
    assert_eq!(Thresholds::CASUAL.FULL_HI_PCT, 83);
    assert_eq!(Thresholds::CASUAL.FIT_LO_PCT, 38);
    let t = build_edges(&Thresholds::CASUAL);
    for i in 0..9 {
        let s = &t.edges[i][..t.len[i] as usize];
        assert!(s.windows(2).all(|w| w[0] < w[1]), "need {i}: {s:?}");
    }
}

#[test]
fn cfg_override_moves_an_edge() {
    let mut t = Thresholds::CLASSIC;
    assert_eq!(apply_overrides(&mut t, b"FULL_HI_PCT=80\n"), 1);
    let e = build_edges(&t);
    let s = slice(&e, Need::Full);
    assert!(s.contains(&80));
    assert!(!s.contains(&78));
}

#[test]
fn bad_band_pair_falls_back_to_preset() {
    let mut t = Thresholds::CLASSIC;
    apply_overrides(&mut t, b"FULL_LO_PCT=90\n");
    let v = validate(&t, &Thresholds::CLASSIC);
    assert_eq!((v.FULL_LO_PCT, v.FULL_HI_PCT), (38, 78));

    let mut t = Thresholds::CLASSIC;
    apply_overrides(&mut t, b"WATER_HI_PCT=0\n");
    assert_eq!(t.WATER_HI_PCT, 1);
    let v = validate(&t, &Thresholds::CLASSIC);
    assert_eq!((v.WATER_LO_PCT, v.WATER_HI_PCT), (38, 83));
}

#[test]
fn custom_flag_only_when_values_differ() {
    let mut t = Thresholds::CLASSIC;
    let n = apply_overrides(&mut t, b"FULL_LO_PCT=38\nFULL_HI_PCT=78\n");
    assert_eq!(n, 2);
    assert_eq!(t, Thresholds::CLASSIC);
    assert!(!is_custom_set(&t, Mode::Classic));
    assert!(is_custom_set(&t, Mode::Casual));
    t.FULL_HI_PCT = 80;
    assert!(is_custom_set(&t, Mode::Classic));
}

#[test]
fn new_egg_starts_inside_every_band() {
    use bornhack_aegg::game::engine::needs::{
        dirt_hi, energy_lo, fit_lo, full_hi, full_lo, fun_hi, fun_lo, water_hi, water_lo,
        weight_hi, weight_lo,
    };
    use bornhack_aegg::game::engine::thresholds::HATCHING_TICKS;
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    let mut s = GameState::new_egg(42, PetKind::DEFAULT);
    s.update(HATCHING_TICKS() as u32);
    assert_eq!(s.phase, Phase::Active);

    let inside = |v: u16, lo: i32, hi: i32| (lo..=hi).contains(&(v as i32));
    assert!(
        inside(s.fullness, full_lo(), full_hi()),
        "fullness {}",
        s.fullness
    );
    assert!(
        inside(s.hydration, water_lo(), water_hi()),
        "hydration {}",
        s.hydration
    );
    assert!(s.energy as i32 >= energy_lo(), "energy {}", s.energy);
    assert!(inside(s.fun, fun_lo(), fun_hi()), "fun {}", s.fun);
    assert!(
        inside(s.weight, weight_lo(), weight_hi()),
        "weight {}",
        s.weight
    );
    assert!((s.dirt as i32) < dirt_hi(), "dirt {}", s.dirt);
    assert!(s.fitness as i32 >= fit_lo(), "fitness {}", s.fitness);
}

mod regime_rates {
    use bornhack_aegg::game::engine::needs::{MAX, P};
    use bornhack_aegg::game::engine::regime::{energy_cap, rates, sleep_quality_pm, thirst_pct};
    use bornhack_aegg::game::engine::{GameState, PetKind};

    fn egg() -> GameState {
        GameState::new_egg(1, PetKind(0))
    }

    #[test]
    fn awake_rates_in_band() {
        let mut s = egg();
        s.is_sleeping = false;
        assert_eq!(rates(&s), [-54, -45, -36, -40, 8, -36, -80, -4, -1]);
        assert_eq!(thirst_pct(&s), 100);
        assert_eq!(energy_cap(&s), MAX);
    }

    #[test]
    fn hung_over_sluggish_pet_drains_faster() {
        let mut s = egg();
        s.is_sleeping = false;
        s.fitness = (30 * P) as u16;
        s.hangover_min = 10;
        assert_eq!(rates(&s)[2], -36 * 1250 / 1000 * 125 / 100);
        assert_eq!(rates(&s)[2], -56);
    }

    #[test]
    fn unsettled_sleep_recovers_at_600() {
        let mut s = egg();
        s.is_sleeping = true;
        s.energy = (20 * P) as u16;
        assert_eq!(sleep_quality_pm(&s), 1000);
        assert_eq!(rates(&s)[2], 120);
        s.booze = (35 * P) as u16;
        assert_eq!(sleep_quality_pm(&s), 600);
        assert_eq!(rates(&s)[2], 72);
    }

    #[test]
    fn deprived_pet_stops_recovering_at_70() {
        let mut s = egg();
        s.is_sleeping = true;
        s.deprived = true;
        s.energy = (70 * P) as u16;
        assert_eq!(energy_cap(&s), 70 * P);
        assert_eq!(rates(&s)[2], 0);
    }

    fn awake() -> GameState {
        let mut s = egg();
        s.is_sleeping = false;
        s
    }

    fn pct(p: i32) -> u16 {
        (p * P) as u16
    }

    #[test]
    fn weight_rate_follows_band_and_belly() {
        let w = |weight: i32, full: i32| {
            let mut s = awake();
            s.weight = pct(weight);
            s.fullness = pct(full);
            rates(&s)[8]
        };
        assert_eq!(w(70, 50), -3); // heavy, belly in band
        assert_eq!(w(30, 50), 0); // light, belly in band
        assert_eq!(w(50, 50), -1); // in band
        assert_eq!(w(70, 20), -6); // heavy, empty belly
        assert_eq!(w(30, 90), 3); // light, stuffed belly
        assert_eq!(w(50, 90), 2); // in band, stuffed belly
    }

    #[test]
    fn heavy_pet_loses_fitness_faster() {
        let mut s = awake();
        s.fitness = pct(60);
        assert_eq!(rates(&s)[7], -4);
        s.weight = pct(70);
        assert_eq!(rates(&s)[7], -6);
        s.fitness = pct(40);
        assert_eq!(rates(&s)[7], -3); // -2 * 3 / 2
        s.fitness = pct(10);
        assert_eq!(rates(&s)[7], -1); // -1 * 3 / 2
    }

    #[test]
    fn booze_clears_slower_when_dry_or_dependent() {
        let mut s = awake();
        assert_eq!(rates(&s)[5], -36);
        s.hydration = pct(10);
        assert_eq!(rates(&s)[5], -24);
        s.dependent = true;
        assert_eq!(rates(&s)[5], -16);
        s.hydration = pct(60);
        assert_eq!(rates(&s)[5], -24);
    }

    #[test]
    fn thirst_steps_with_booze_band() {
        let mut s = awake();
        for (b, want) in [
            (0, 100),
            (14, 100),
            (15, 110),
            (39, 110),
            (40, 125),
            (69, 125),
            (70, 140),
            (100, 140),
        ] {
            s.booze = if b == 100 { 65535 } else { pct(b) };
            assert_eq!(thirst_pct(&s), want, "booze {b}");
        }
        s.booze = pct(70);
        assert_eq!(rates(&s)[1], -45 * 140 / 100);
    }
}

mod health_terms {
    use bornhack_aegg::game::engine::needs::{MAX, P};
    use bornhack_aegg::game::engine::regime::{health_ceiling, health_damage, mood_target};
    use bornhack_aegg::game::engine::thresholds::Thresholds;
    use bornhack_aegg::game::engine::{GameState, PetKind};

    fn awake() -> GameState {
        let mut s = GameState::new_egg(1, PetKind(0));
        s.is_sleeping = false;
        s
    }

    fn pct(p: i32) -> u16 {
        (p * P) as u16
    }

    #[test]
    fn target_is_full_when_all_in_band() {
        assert_eq!(mood_target(&awake()), MAX);
    }

    #[test]
    fn hungry_and_bored_penalties_add() {
        let mut s = awake();
        s.fullness = pct(20);
        s.fun = pct(10);
        assert_eq!(mood_target(&s), MAX - (18 * P * 3 / 2) - 23 * P);
    }

    #[test]
    fn ceiling_steps() {
        let mut s = awake();
        for (fit, want) in [(24, 55), (49, 65), (69, 80), (70, 100)] {
            s.fitness = pct(fit);
            assert_eq!(health_ceiling(&s), want * P, "fitness {fit}");
        }
        s.diabetic = true;
        s.insulin_min = 0;
        assert_eq!(health_ceiling(&s), 80 * P);
        s.insulin_min = 5;
        assert_eq!(health_ceiling(&s), 100 * P);
    }

    #[test]
    fn sleep_shortage_stops_damaging_at_30() {
        let mut s = awake();
        s.energy = pct(5);
        s.health = pct(31);
        assert_eq!(health_damage(&s), 8);
        s.health = pct(30);
        assert_eq!(health_damage(&s), 0);
    }

    #[test]
    fn casual_halves_only_damage_rates() {
        let (c, k) = (Thresholds::CLASSIC, Thresholds::CASUAL);
        assert_eq!(c.DMG_FULL_LOW_PM, 40);
        assert_eq!(k.DMG_FULL_LOW_PM, 20);
        let dmg = |t: &Thresholds| {
            [
                t.DMG_FULL_LOW_PM,
                t.DMG_FULL_HIGH_PM,
                t.DMG_WATER_LOW_PM,
                t.DMG_WATER_HIGH_PM,
                t.DMG_ENERGY_PM,
                t.DMG_BOOZE_PM,
                t.DMG_MOOD_PM,
                t.DMG_DIRT_PM,
                t.DMG_WEIGHT_HIGH_PM,
                t.DMG_WEIGHT_LOW_PM,
            ]
        };
        for (a, b) in dmg(&c).iter().zip(dmg(&k)) {
            assert_eq!(*a / 2, b);
        }
        assert_eq!(c.MOOD_STEP, k.MOOD_STEP);
        assert_eq!(c.REC_GOOD_PM, k.REC_GOOD_PM);
    }
}

mod advance_wake {
    use bornhack_aegg::game::engine::advance::{advance, wake};
    use bornhack_aegg::game::engine::needs::{MAX, P};
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    fn pct(p: i32) -> u16 {
        (p * P) as u16
    }

    /// Awake pet with every need mid-band, full mood and full health room.
    fn calm() -> GameState {
        let mut s = GameState::new_egg(1, PetKind(0));
        s.phase = Phase::Active;
        s.is_sleeping = false;
        s.fullness = pct(60);
        s.hydration = pct(65);
        s.energy = pct(80);
        s.fun = pct(50);
        s.dirt = pct(20);
        s.booze = pct(30);
        s.strain = pct(20);
        s.fitness = pct(55);
        s.weight = pct(50);
        s.mood = MAX as u16;
        s.health = pct(70);
        s
    }

    #[test]
    fn one_minute_awake_moves_needs_by_their_rates() {
        let mut s = calm();
        advance(&mut s, 1);
        assert_eq!(s.fullness, pct(60) - 54);
        assert_eq!(
            s.hydration,
            pct(65) - 45 * 110 / 100,
            "booze 30 %: thirst x1.1"
        );
        assert_eq!(s.energy, pct(80) - 36);
        assert_eq!(s.fun, pct(50) - 40);
        assert_eq!(s.dirt, pct(20) + 8);
        assert_eq!(s.booze, pct(30) - 36);
        assert_eq!(s.strain, pct(20) - 80);
        assert_eq!(s.fitness, pct(55) - 4);
        assert_eq!(s.weight, pct(50) - 1);
        assert_eq!(s.mood, MAX as u16);
        assert_eq!(
            s.health,
            pct(70) + 14,
            "below the ceiling and in a good mood"
        );
        assert_eq!(s.care_awake_min, 1);
        assert_eq!(s.care_good_min, 1);
    }

    #[test]
    fn sleep_from_30_percent_is_full_in_about_six_and_a_half_hours() {
        let mut s = calm();
        s.is_sleeping = true;
        s.energy = pct(30);
        s.booze = 0;
        let mut minutes = 0;
        while s.energy < MAX as u16 && minutes < 600 {
            advance(&mut s, 1);
            minutes += 1;
        }
        assert!((375..=390).contains(&minutes), "took {minutes} minutes");
        assert_eq!(s.energy, MAX as u16);
        assert_eq!(s.sleep_min, minutes as u16);
        assert!(s.good_sleep_min >= 300, "good sleep {}", s.good_sleep_min);
        assert!(s.fun >= pct(40), "fun floor while asleep");
    }

    #[test]
    fn oversleep_past_four_hours_makes_groggy() {
        let mut s = calm();
        s.is_sleeping = true;
        s.energy = MAX as u16;
        advance(&mut s, 250);
        assert_eq!(s.oversleep_min, 250);
        assert!(s.is_sleeping);
        let (energy, fit) = (s.energy, s.fitness);
        wake(&mut s, true);
        assert!(!s.is_sleeping);
        assert_eq!(s.groggy_min, 120);
        assert_eq!(s.energy, energy - pct(10));
        assert_eq!(s.fitness, fit - pct(2));
        assert_eq!(s.mood, MAX as u16, "full energy: not grumpy");
    }

    #[test]
    fn grumpy_when_woken_tired_and_no_groggy_clears_oversleep() {
        let mut s = calm();
        s.is_sleeping = true;
        s.energy = pct(40);
        s.mood = pct(80);
        wake(&mut s, true);
        assert_eq!(s.mood, pct(70));
        let mut s = calm();
        s.is_sleeping = true;
        s.energy = pct(40);
        s.mood = pct(80);
        wake(&mut s, false);
        assert_eq!(s.mood, pct(80), "self-wake is not grumpy");
        let mut s = calm();
        s.is_sleeping = true;
        s.oversleep_min = 250;
        s.no_groggy = true;
        wake(&mut s, false);
        assert_eq!((s.oversleep_min, s.groggy_min, s.no_groggy), (0, 0, false));
    }

    #[test]
    fn well_rested_after_five_good_hours() {
        let mut s = calm();
        s.is_sleeping = true;
        s.good_sleep_min = 299;
        wake(&mut s, false);
        assert_eq!(s.rested_min, 0);
        s.is_sleeping = true;
        s.good_sleep_min = 300;
        wake(&mut s, false);
        assert_eq!(s.rested_min, 960);
    }

    fn night_wake(s: &mut GameState, minutes: u16) {
        s.is_sleeping = true;
        s.night = true;
        s.sleep_min = minutes;
        wake(s, false);
    }

    #[test]
    fn three_short_nights_deprive_two_good_nights_clear() {
        let mut s = calm();
        for n in 1..=3u8 {
            night_wake(&mut s, 300);
            assert_eq!(s.short_nights, n);
            assert_eq!(s.deprived, n == 3);
        }
        night_wake(&mut s, 365); // enough to break the streak, not a good night
        assert_eq!((s.short_nights, s.good_nights, s.deprived), (0, 0, true));
        night_wake(&mut s, 400);
        assert_eq!((s.good_nights, s.deprived), (1, true));
        night_wake(&mut s, 400);
        assert_eq!((s.good_nights, s.deprived), (2, false));
        // a short night resets the good streak
        night_wake(&mut s, 300);
        assert_eq!((s.short_nights, s.good_nights), (1, 0));
    }

    #[test]
    fn naps_do_not_count_as_nights() {
        let mut s = calm();
        s.is_sleeping = true;
        s.night = false;
        s.sleep_min = 60;
        wake(&mut s, false);
        assert_eq!((s.short_nights, s.good_nights), (0, 0));
    }

    #[test]
    fn hangover_starts_after_peak_and_lasts_480() {
        let mut s = calm();
        s.booze = pct(60);
        advance(&mut s, 1);
        assert!(s.booze_peak);
        assert_eq!(s.hangover_min, 0);
        s.booze = pct(15) + 20; // falls under 15 % this minute
        advance(&mut s, 1);
        assert!(!s.booze_peak);
        assert_eq!(s.hangover_min, 479, "480 set, one minute elapsed");
        advance(&mut s, 400);
        assert_eq!(s.hangover_min, 79);
        advance(&mut s, 79);
        assert_eq!(s.hangover_min, 0);
    }

    #[test]
    fn hangover_runs_down_twice_as_fast_asleep() {
        let mut s = calm();
        s.is_sleeping = true;
        s.hangover_min = 100;
        advance(&mut s, 10);
        assert_eq!(s.hangover_min, 80);
    }

    #[test]
    fn dependence_builds_while_drunk_and_decays_when_sober() {
        let mut s = calm();
        s.booze = pct(90);
        s.dependence_min = 1198 * 4 + 2;
        advance(&mut s, 1);
        assert!(!s.dependent);
        advance(&mut s, 1);
        assert_eq!(s.dependence_min, 1200 * 4 + 2);
        assert!(s.dependent);

        let mut s = calm();
        s.booze = 0;
        s.dependence_min = 400;
        advance(&mut s, 40);
        assert_eq!(
            s.dependence_min, 360,
            "quarter-minutes: minus 1 per sober minute"
        );
    }

    #[test]
    fn recovering_pet_relapses_after_400() {
        let mut s = calm();
        s.booze = pct(90);
        s.recovering_min = 5000;
        s.dependence_min = 400 * 4 - 4;
        advance(&mut s, 1);
        assert!(s.dependent);
    }

    #[test]
    fn rehab_course_cures_and_marks_recovering() {
        let mut s = calm();
        s.dependent = true;
        s.dependence_min = 700 * 4;
        s.rehab_min = 3;
        advance(&mut s, 2);
        assert!(s.dependent);
        assert_eq!(s.rehab_min, 1);
        advance(&mut s, 1);
        assert_eq!(s.rehab_min, 0);
        assert!(!s.dependent);
        assert_eq!(s.dependence_min, 0);
        assert_eq!(s.recovering_min, 10080);
        advance(&mut s, 1);
        assert_eq!(s.recovering_min, 10079);
    }

    #[test]
    fn diabetes_sets_in_after_4320_heavy_minutes() {
        let mut s = calm();
        s.weight = pct(85);
        s.diabetes_min = 4319 * 4;
        advance(&mut s, 1);
        assert_eq!(s.diabetes_min, 4320 * 4);
        assert!(s.diabetic);
        assert_eq!(s.reversal_min, 0);
    }

    #[test]
    fn diabetes_reverses_after_10080_lean_fit_minutes() {
        let mut s = calm();
        s.diabetic = true;
        s.weight = pct(60);
        s.fitness = pct(65);
        s.reversal_min = 10078;
        advance(&mut s, 1);
        assert!(s.diabetic);
        assert_eq!(s.reversal_min, 10079);
        advance(&mut s, 1);
        assert!(!s.diabetic);
        assert_eq!(s.diabetes_min, 0);

        let mut s = calm();
        s.diabetic = true;
        s.weight = pct(60);
        s.fitness = pct(58);
        s.reversal_min = 100;
        advance(&mut s, 5);
        assert_eq!(s.reversal_min, 100, "not fit enough: no progress");
    }

    #[test]
    fn collapse_at_zero_energy_sleeps_and_costs_health() {
        let mut s = calm();
        s.energy = 20;
        s.health = pct(60);
        s.sleep_min = 500;
        advance(&mut s, 1);
        assert!(s.is_sleeping);
        assert!(s.night);
        assert_eq!(s.sleep_min, 0);
        assert_eq!(s.good_sleep_min, 0);
        assert_eq!(s.health, pct(57) + 14, "lost 3 points, then +14 recovery");

        let mut s = calm();
        s.energy = 20;
        s.health = pct(30);
        advance(&mut s, 1);
        assert!(s.is_sleeping);
        assert!(s.health >= pct(30), "no collapse loss at or under 33 %");
    }

    #[test]
    fn long_sleep_wakes_itself_at_720() {
        let mut s = calm();
        s.is_sleeping = true;
        s.energy = pct(5);
        s.booze = 0;
        s.night = true;
        advance(&mut s, 719);
        assert!(s.is_sleeping);
        advance(&mut s, 1);
        assert!(!s.is_sleeping);
    }

    #[test]
    fn zero_health_counts_toward_leaving_and_recovers_at_half_speed() {
        let mut s = calm();
        s.health = 0;
        s.fullness = 0;
        s.hydration = 0;
        advance(&mut s, 100);
        assert_eq!(s.health, 0);
        assert_eq!(s.leaving_min, 200);
        assert_eq!(s.phase, Phase::Leaving);
        advance(&mut s, 619);
        assert_eq!(s.phase, Phase::Leaving);
        advance(&mut s, 1);
        assert_eq!(s.phase, Phase::Gone);
        assert!(s.realm_pending);

        let mut s = calm();
        s.leaving_min = 100;
        s.phase = Phase::Leaving;
        advance(&mut s, 40);
        assert_eq!(s.leaving_min, 60, "half-minutes: minus 1 per minute");
        advance(&mut s, 60);
        assert_eq!(s.leaving_min, 0);
        assert_eq!(s.phase, Phase::Active);
    }
}

mod split_invariance {
    use bornhack_aegg::game::engine::advance::advance;
    use bornhack_aegg::game::engine::needs::{MAX, P};
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    fn pct(p: i32) -> u16 {
        (p * P) as u16
    }

    fn calm() -> GameState {
        let mut s = GameState::new_egg(1, PetKind(0));
        s.phase = Phase::Active;
        s.is_sleeping = false;
        s.fullness = pct(60);
        s.hydration = pct(65);
        s.energy = pct(80);
        s.fun = pct(50);
        s.dirt = pct(20);
        s.booze = pct(30);
        s.strain = pct(20);
        s.fitness = pct(55);
        s.weight = pct(50);
        s.mood = MAX as u16;
        s.health = pct(70);
        s
    }

    type Snapshot = ([u16; 14], [u32; 4]);

    fn snap(s: &GameState) -> Snapshot {
        (
            [
                s.fullness,
                s.hydration,
                s.energy,
                s.fun,
                s.dirt,
                s.booze,
                s.strain,
                s.fitness,
                s.weight,
                s.mood,
                s.health,
                s.sleep_min,
                s.oversleep_min,
                s.reversal_min,
            ],
            [
                s.dependence_min,
                s.diabetes_min,
                s.leaving_min,
                s.care_awake_min,
            ],
        )
    }

    fn check(name: &str, s0: GameState) {
        const N: u32 = 20;
        let mut whole = s0.clone();
        advance(&mut whole, N);
        let mut ones = s0.clone();
        for _ in 0..N {
            advance(&mut ones, 1);
        }
        let mut halves = s0.clone();
        advance(&mut halves, 7);
        advance(&mut halves, N - 7);
        assert_eq!(snap(&ones), snap(&whole), "{name}: N x 1 vs one call");
        assert_eq!(snap(&halves), snap(&whole), "{name}: 7 + 13 vs one call");
    }

    #[test]
    fn advance_is_split_invariant() {
        check("awake in band", calm());

        let mut s = calm();
        s.is_sleeping = true;
        s.energy = pct(30);
        s.booze = 0;
        check("asleep", s);

        let mut s = calm();
        s.booze = pct(60);
        s.dependence_min = 400;
        check("drunk above booze_hi", s);

        let mut s = calm();
        s.booze = pct(10);
        s.dependence_min = 400;
        check("sobering", s);

        let mut s = calm();
        s.weight = pct(85);
        s.diabetes_min = 100;
        check("heavy", s);

        let mut s = calm();
        s.diabetes_min = 400;
        s.weight = pct(60);
        check("lean again", s);

        let mut s = calm();
        s.health = 0;
        s.fullness = 0;
        s.hydration = 0;
        s.leaving_min = 100;
        s.phase = Phase::Leaving;
        check("health 0", s);

        let mut s = calm();
        s.leaving_min = 100;
        s.phase = Phase::Leaving;
        check("healed, leaving recovers", s);
    }
}

mod carried_from_task_4 {
    use bornhack_aegg::game::bornpets_cfg::apply_overrides;
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::regime::health_recovery;
    use bornhack_aegg::game::engine::thresholds::Thresholds;
    use bornhack_aegg::game::engine::{GameState, PetKind};

    #[test]
    fn health_recovery_tiers_and_deprived_halving() {
        let mut s = GameState::new_egg(1, PetKind(0));
        for (mood, want) in [(100, 14), (60, 14), (59, 5), (40, 5), (39, 0), (0, 0)] {
            s.mood = (mood * P) as u16;
            assert_eq!(health_recovery(&s), want, "mood {mood}");
        }
        s.deprived = true;
        for (mood, want) in [(70, 7), (50, 2), (10, 0)] {
            s.mood = (mood * P) as u16;
            assert_eq!(health_recovery(&s), want, "deprived, mood {mood}");
        }
    }

    #[test]
    fn ceiling_settings_cannot_exceed_one_hundred_percent() {
        let mut t = Thresholds::CLASSIC;
        let cfg = b"CEILING_STEP0_PCT=500\nCEILING_STEP1_PCT=500\nCEILING_STEP2_PCT=500\nCEILING_STEP3_PCT=500\nCEILING_DIABETIC_PCT=500\n";
        assert_eq!(apply_overrides(&mut t, cfg), 5);
        assert_eq!(
            [
                t.CEILING_STEP0_PCT,
                t.CEILING_STEP1_PCT,
                t.CEILING_STEP2_PCT,
                t.CEILING_STEP3_PCT,
                t.CEILING_DIABETIC_PCT
            ],
            [100; 5]
        );
    }

    #[test]
    fn ceiling_is_clamped_to_the_level_range() {
        use bornhack_aegg::game::engine::needs::MAX;
        use bornhack_aegg::game::engine::regime::ceiling_with;
        let mut t = Thresholds::CLASSIC;
        t.CEILING_STEP3_PCT = 500;
        assert_eq!(ceiling_with(&t, 80 * P, false), MAX);
        let mut t = Thresholds::CLASSIC;
        t.CEILING_STEP0_PCT = 10;
        t.CEILING_DIABETIC_PCT = 50;
        assert_eq!(ceiling_with(&t, 0, true), 0);
        assert_eq!(ceiling_with(&Thresholds::CLASSIC, 0, true), 35 * P);
    }
}

mod update_minutes {
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    fn hatched() -> GameState {
        let mut s = GameState::new_egg(1, PetKind(0));
        s.phase = Phase::Active;
        s.hatching_countdown = 0;
        s.is_sleeping = false;
        s
    }

    #[test]
    fn many_small_updates_equal_one_big_update() {
        let mut a = hatched();
        let mut b = hatched();
        for t in 1..=8640 {
            a.update(t);
        }
        b.update(8640);
        assert_eq!(a.fullness, b.fullness);
        assert_eq!(a.energy, b.energy);
        assert_eq!(a.age_ticks, b.age_ticks);
        assert!(
            b.fullness < (60 * P) as u16,
            "pet did not age: {}",
            b.fullness
        );
    }

    #[test]
    fn sub_minute_time_is_carried_not_lost() {
        let mut s = hatched();
        s.update(5);
        assert_eq!(s.tick_rem, 5);
        let before = s.fullness;
        s.update(6);
        assert_eq!(s.tick_rem, 0);
        assert!(
            s.fullness < before,
            "one whole minute must have been applied"
        );
        assert_eq!(s.age_ticks, 6);
    }

    #[test]
    fn clock_going_backwards_is_zero_elapsed() {
        let mut s = hatched();
        s.update(100);
        let (f, rem, age) = (s.fullness, s.tick_rem, s.age_ticks);
        s.update(50);
        assert_eq!((s.fullness, s.tick_rem, s.age_ticks), (f, rem, age));
    }
}

mod solver {
    use bornhack_aegg::game::engine::advance::advance;
    use bornhack_aegg::game::engine::needs::{MAX, NEEDS, P, edges};
    use bornhack_aegg::game::engine::regime::rates;
    use bornhack_aegg::game::engine::solver::seg_len;
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    fn pct(p: i32) -> u16 {
        (p * P) as u16
    }

    /// Awake, sober, every need mid-band except fullness (40 %), no timers.
    fn calm() -> GameState {
        let mut s = GameState::new_egg(1, PetKind(0));
        s.phase = Phase::Active;
        s.is_sleeping = false;
        s.fullness = pct(40);
        s.hydration = pct(65);
        s.energy = pct(80);
        s.fun = pct(50);
        s.dirt = pct(20);
        s.booze = 0;
        s.strain = pct(20);
        s.fitness = pct(55);
        s.weight = pct(50);
        s.mood = MAX as u16;
        s.health = pct(70);
        s
    }

    #[test]
    fn segment_stops_one_unit_before_a_threshold() {
        let s = calm();
        assert_eq!(seg_len(&s, 10_000), (40 * 655 - 38 * 655 - 1) / 54);
    }

    #[test]
    fn timer_bounds_the_segment() {
        let mut s = calm();
        s.fullness = pct(60);
        s.injured_min = 7;
        assert_eq!(
            seg_len(&s, 10_000),
            6,
            "injured feeds mood: one minute early (R20)"
        );
    }

    #[test]
    fn pinned_level_is_ignored() {
        let mut s = calm();
        s.fullness = pct(60);
        s.is_sleeping = true;
        s.energy = MAX as u16;
        s.sleep_min = 700;
        assert_eq!(
            seg_len(&s, 10_000),
            19,
            "only the self-wake timer bounds it, one minute early"
        );
    }

    #[test]
    fn never_zero() {
        let mut s = calm();
        s.fullness = pct(38); // exactly on an edge, heading down
        assert_eq!(seg_len(&s, 10_000), 1);
        s.fullness = pct(38) + 1; // one unit above the edge
        assert_eq!(seg_len(&s, 10_000), 1);
        assert_eq!(seg_len(&calm(), 1), 1);
    }

    #[test]
    fn dependence_and_diabetes_timers_use_ceil_division() {
        let mut s = calm();
        s.fullness = pct(60);
        s.booze = pct(50);
        s.dependence_min = 1200 * 4 - 5; // 5 quarter-minutes left: 2 minutes, bounded one early
        assert_eq!(seg_len(&s, 10_000), 1);
        let mut s = calm();
        s.fullness = pct(60);
        s.weight = pct(85);
        s.diabetes_min = 4320 * 4 - 9; // 3 minutes, bounded one early
        assert_eq!(seg_len(&s, 10_000), 2);
    }

    #[test]
    fn leaving_timer_runs_in_half_minutes() {
        let mut s = calm();
        s.health = 0;
        s.leaving_min = 720 * 2 - 3; // 2 minutes
        assert_eq!(seg_len(&s, 10_000), 2);
    }

    fn sides(s: &GameState) -> Vec<i32> {
        let e = edges();
        let lv = [
            s.fullness,
            s.hydration,
            s.energy,
            s.fun,
            s.dirt,
            s.booze,
            s.strain,
            s.fitness,
            s.weight,
        ];
        let mut out = Vec::new();
        for n in 0..NEEDS {
            for k in 0..e.len[n] as usize {
                out.push((lv[n] as i32 - e.edges[n][k] as i32 * P).signum());
            }
        }
        out
    }

    fn timers(s: &GameState) -> [u16; 11] {
        [
            s.rested_min,
            s.injured_min,
            s.groggy_min,
            s.med_cd_min,
            s.recovering_min,
            s.junk_min,
            s.insulin_min,
            s.ozempic_min,
            s.ozempic_cd_min,
            s.rehab_min,
            s.hangover_min,
        ]
    }

    #[test]
    fn segments_never_cross_an_edge_or_a_timer() {
        let mut fresh = GameState::new_egg(7, PetKind(0));
        fresh.phase = Phase::Active;
        fresh.fitness = pct(65); // an egg's 60 % sits exactly on an edge: a one-minute segment
        let mut hungry = calm();
        hungry.fullness = pct(20);
        let mut drunk = calm();
        drunk.booze = pct(50);
        drunk.hangover_min = 90;
        let mut asleep = calm();
        asleep.is_sleeping = true;
        asleep.energy = MAX as u16;
        asleep.injured_min = 900;
        let mut heavy = calm();
        heavy.weight = pct(85);
        let mut dead = calm();
        dead.health = 0;
        for (name, s0) in [
            ("fresh", fresh),
            ("hungry", hungry),
            ("drunk", drunk),
            ("asleep", asleep),
            ("heavy", heavy),
            ("dead", dead),
        ] {
            let d = seg_len(&s0, 2000);
            assert!(
                (2..=2000).contains(&d),
                "{name}: segment of {d} minutes is not a real segment"
            );
            let (r0, side0, t0) = (rates(&s0), sides(&s0), timers(&s0));
            let mut s = s0.clone();
            for minute in 0..d {
                assert_eq!(
                    rates(&s),
                    r0,
                    "{name}: rates changed at minute {minute} of {d}"
                );
                assert_eq!(
                    sides(&s),
                    side0,
                    "{name}: edge crossed at minute {minute} of {d}"
                );
                for (a, b) in t0.iter().zip(timers(&s)) {
                    assert!(
                        *a == 0 || b > 0,
                        "{name}: timer expired at minute {minute} of {d}"
                    );
                }
                advance(&mut s, 1);
            }
        }
    }

    fn at_target(mut s: GameState) -> GameState {
        s.mood = bornhack_aegg::game::engine::regime::mood_target(&s) as u16;
        s
    }

    /// R20: one closed-form call over a segment equals the same minutes stepped
    /// one at a time, on the whole state.
    #[test]
    fn a_segment_equals_the_same_minutes_stepped() {
        let mut cases: Vec<(&str, GameState)> = Vec::new();
        let mut s = calm();
        s.fullness = pct(60);
        s.injured_min = 7;
        cases.push(("injured", at_target(s)));
        let mut s = calm();
        s.fullness = pct(60);
        s.diabetic = true;
        s.insulin_min = 5;
        cases.push(("insulin", s));
        let mut s = calm();
        s.fullness = pct(60);
        s.weight = pct(85);
        s.diabetes_min = 4320 * 4 - 9;
        cases.push(("diabetes onset", s));
        let mut s = calm();
        s.fullness = pct(60);
        s.groggy_min = 6;
        cases.push(("groggy", at_target(s)));
        let mut s = calm();
        s.fullness = pct(60);
        s.hangover_min = 9;
        cases.push(("hangover", at_target(s)));
        let mut s = calm();
        s.fullness = pct(60);
        s.rehab_min = 4;
        cases.push(("rehab expiry", at_target(s)));
        let mut s = calm();
        s.fullness = pct(60);
        s.dependent = true;
        s.rehab_min = 3;
        s.dependence_min = 500;
        cases.push(("rehab ends dependence", at_target(s)));
        let mut s = calm();
        s.fullness = pct(60);
        s.diabetic = true;
        s.weight = pct(50);
        s.fitness = pct(65);
        s.reversal_min = 10080 - 5;
        cases.push(("reversal near done", s));
        let mut s = calm();
        s.fullness = pct(60);
        s.booze = pct(50);
        s.dependence_min = 1200 * 4 - 9;
        cases.push(("dependence onset", s));
        let mut s = calm();
        s.is_sleeping = true;
        s.energy = pct(52);
        s.sleep_min = 715;
        cases.push(("self-wake", at_target(s)));
        let mut s = calm();
        s.is_sleeping = true;
        s.energy = MAX as u16;
        s.oversleep_min = 236;
        cases.push(("oversleep line", at_target(s)));
        let mut s = calm();
        s.is_sleeping = true;
        s.energy = MAX as u16;
        s.oversleep_min = 296;
        cases.push(("oversleep wake", at_target(s)));
        for (name, s0) in cases {
            let d = seg_len(&s0, 2000);
            let mut closed = s0.clone();
            advance(&mut closed, d);
            let mut stepped = s0.clone();
            for _ in 0..d {
                advance(&mut stepped, 1);
            }
            assert_eq!(
                closed, stepped,
                "{name}: segment of {d} minutes differs from stepping"
            );
        }
    }

    #[test]
    fn clamped_levels_do_not_force_one_minute_segments() {
        let mut s = calm();
        s.fullness = pct(60);
        s.dirt = 0; // rising from 0: the 0 line is behind it
        assert!(seg_len(&s, 10_000) > 1);
        let mut s = calm();
        s.fullness = pct(60);
        s.dirt = MAX as u16; // rising, pinned at MAX
        s.health = pct(70);
        assert!(seg_len(&s, 10_000) > 1);
        let mut s = calm();
        s.fullness = pct(60);
        s.hydration = MAX as u16; // falling from MAX: the MAX line is behind it
        assert!(seg_len(&s, 10_000) > 1);
    }

    #[test]
    fn max_zero_and_gone_pets_still_give_one() {
        assert_eq!(seg_len(&calm(), 0), 1);
        let mut s = calm();
        s.phase = Phase::Gone;
        assert_eq!(seg_len(&s, 0), 1);
        assert_eq!(seg_len(&s, 50), 50);
    }
}

mod zone_check {
    use bornhack_aegg::game::engine::advance::advance;
    use bornhack_aegg::game::engine::needs::{MAX, P};
    use bornhack_aegg::game::engine::regime::mood_target;
    use bornhack_aegg::game::engine::solver::sig;
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    fn pct(p: i32) -> u16 {
        (p * P) as u16
    }

    fn calm() -> GameState {
        let mut s = GameState::new_egg(1, PetKind(0));
        s.phase = Phase::Active;
        s.hatching_countdown = 0;
        s.is_sleeping = false;
        s.fullness = pct(60);
        s.hydration = pct(65);
        s.energy = pct(80);
        s.fun = pct(50);
        s.dirt = pct(20);
        s.booze = 0;
        s.strain = pct(20);
        s.fitness = pct(55);
        s.weight = pct(50);
        s.mood = MAX as u16;
        s.health = pct(70);
        s
    }

    /// `update` over `minutes` must equal `minutes` x `advance(1)` on the full
    /// state (ruling R21), apart from the clock fields only `update` touches.
    fn equivalent(name: &str, s0: &GameState, minutes: u32) {
        let mut fast = s0.clone();
        fast.update(fast.last_update_tick + minutes * 6);
        let mut slow = s0.clone();
        for _ in 0..minutes {
            advance(&mut slow, 1);
        }
        slow.age_ticks = fast.age_ticks;
        slow.last_update_tick = fast.last_update_tick;
        assert_eq!(fast, slow, "{name}: update over {minutes} min != stepped");
    }

    #[test]
    fn sig_packs_the_zones() {
        let mut s = calm();
        let mut seen = std::collections::HashSet::new();
        for mood in [100, 80, 79, 60, 59, 40, 39, 20, 19, 0] {
            s.mood = pct(mood);
            seen.insert(sig(&s));
        }
        assert_eq!(seen.len(), 5, "mood zones 80/60/40/20");
        let mut seen = std::collections::HashSet::new();
        for health in [0, 1, 24, 25, 29, 30, 32, 33, 44, 45, 49, 50, 60] {
            let mut s = calm();
            s.health = pct(health);
            s.mood = 0;
            seen.insert(sig(&s));
        }
        // 0 | 1..24 | 25..29 | 30..32 | 33..44 | 45..49 | 50..59 (below the ceiling 80 at fitness 55)
        assert_eq!(seen.len(), 7, "health zones");
        let (mut a, mut b) = (calm(), calm());
        a.is_sleeping = true;
        b.diabetic = true;
        assert_ne!(sig(&a), sig(&calm()));
        assert_ne!(sig(&b), sig(&calm()));
        let mut c = calm();
        c.health = 65535;
        assert_ne!(sig(&c), sig(&calm()), "above vs below the ceiling");
    }

    #[test]
    fn mood_meeting_a_falling_target_does_not_hide_a_line() {
        // Mood climbs at MOOD_STEP below a falling target, meets it, follows it
        // down: crosses a line and comes back with equal end signatures.
        let mut s = calm();
        s.fullness = pct(30);
        s.health = pct(35);
        s.mood = pct(45);
        equivalent("roundtrip", &s, 300);
        for m0 in [41, 43, 57, 59, 61, 21, 23, 37, 39] {
            for (full, health) in [(30, 35), (30, 20), (36, 40), (37, 45)] {
                let mut s = calm();
                s.fullness = pct(full);
                s.health = pct(health);
                s.mood = pct(m0);
                equivalent(&format!("full {full} health {health} mood {m0}"), &s, 300);
            }
        }
    }

    #[test]
    fn target_clamped_at_zero_is_a_zone() {
        // Penalty 73080 > MAX: the target is clamped at 0 and leaves the clamp
        // part-way through the span as health recovers.
        let mut s = calm();
        s.fullness = 63767;
        s.hydration = 65317;
        s.fun = 56219;
        s.energy = 34436;
        s.dirt = 53097;
        s.booze = 24026;
        s.weight = 58314;
        s.fitness = 12941;
        s.health = 27148;
        s.mood = 0;
        equivalent("target at 0", &s, 132);
    }

    #[test]
    fn calm_pet_over_half_a_day_costs_few_advances() {
        let s = calm();
        let mut f = s.clone();
        bornhack_aegg::game::engine::ADVANCE_CALLS.with(|c| c.set(0));
        f.update(720 * 6);
        let n = bornhack_aegg::game::engine::ADVANCE_CALLS.with(|c| c.get());
        println!("calm 720 min: {n} advance calls");
        assert!(n <= 200, "{n} advance calls");
    }

    #[test]
    fn mood_crossing_60_splits_the_segment() {
        let mut s = calm();
        s.fun = 0;
        s.mood = pct(90);
        equivalent("fun 0, mood 90", &s, 600);
    }

    #[test]
    fn health_zero_at_mood_target_is_exact() {
        let mut s = calm();
        s.health = 0;
        s.mood = mood_target(&s) as u16;
        equivalent("health 0 at target", &s, 40);
        equivalent("health 0 at target, long", &s, 600);
    }

    #[test]
    fn update_equals_stepping_near_every_line() {
        let mut cases: Vec<(&str, GameState)> = Vec::new();
        for (name, mood) in [
            ("mood 60+", 60),
            ("mood 40+", 40),
            ("mood 20+", 20),
            ("mood 80+", 80),
        ] {
            let mut s = calm();
            s.fun = 0;
            s.mood = pct(mood) + 30;
            cases.push((name, s));
        }
        let mut s = calm();
        s.energy = pct(20);
        s.health = pct(30) + 40;
        cases.push(("health 30+ with tired", s));
        let mut s = calm();
        s.health = pct(50) + 10;
        s.fitness = pct(30);
        cases.push(("health near mood floor", s));
        let mut s = calm();
        s.health = pct(65) - 20;
        s.fitness = pct(30);
        cases.push(("health just under ceiling", s));
        let mut s = calm();
        s.health = pct(80);
        s.fitness = pct(30);
        cases.push(("health above ceiling", s));
        let mut s = calm();
        s.hydration = 0;
        s.fullness = 0;
        s.health = pct(10);
        s.mood = pct(30);
        cases.push(("starving", s));
        for (name, s) in &cases {
            for minutes in [1, 7, 40, 300, 2000] {
                equivalent(name, s, minutes);
            }
        }
    }
}

mod zone_check_fast_target {
    use bornhack_aegg::game::engine::advance::advance;
    use bornhack_aegg::game::engine::needs::{MAX, P};
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    #[test]
    fn target_moving_faster_than_mood_step_stays_exact() {
        let mut s = GameState::new_egg(1, PetKind(0));
        s.phase = Phase::Active;
        s.hatching_countdown = 0;
        s.is_sleeping = false;
        let p = |x: i32| (x * P) as u16;
        // Several needs out of band at once, mood right on its target.
        s.fullness = p(37);
        s.hydration = p(40);
        s.fun = p(20);
        s.dirt = p(70);
        s.energy = p(45);
        s.health = p(45);
        s.mood = (MAX / 2) as u16;
        for minutes in [1, 3, 10, 60, 500] {
            let mut fast = s.clone();
            fast.update(minutes * 6);
            let mut slow = s.clone();
            for _ in 0..minutes {
                advance(&mut slow, 1);
            }
            slow.age_ticks = fast.age_ticks;
            slow.last_update_tick = fast.last_update_tick;
            assert_eq!(fast, slow, "{minutes} min");
        }
    }
}

mod rounding_margin {
    use bornhack_aegg::game::engine::advance::advance;
    use bornhack_aegg::game::engine::needs::MAX;
    use bornhack_aegg::game::engine::regime::mood_target;
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    /// Mood on, or one full step off, a target moving at about MOOD_STEP per
    /// minute: the 3/2 and 1/2 truncations make the target only nearly linear.
    #[test]
    fn update_is_exact_near_the_mood_step() {
        let mut x: u64 = 0x1234_5678_9abc_def1;
        let mut r = |n: u32| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % n as u64) as u32
        };
        let (mut cand, mut bad) = (0, 0);
        for _ in 0..200_000 {
            let mut s = GameState::new_egg(1, PetKind(0));
            s.phase = Phase::Active;
            s.hatching_countdown = 0;
            s.is_sleeping = r(4) == 0;
            s.fullness = r(65536) as u16;
            s.hydration = r(65536) as u16;
            s.fun = r(65536) as u16;
            s.energy = (r(60000) + 3000) as u16;
            s.dirt = r(65536) as u16;
            s.booze = if r(3) == 0 { r(40000) as u16 } else { 0 };
            s.weight = r(65536) as u16;
            s.fitness = r(65536) as u16;
            s.health = (r(65000) + 500) as u16;
            let t0 = mood_target(&s);
            let mut a = s.clone();
            a.mood = t0 as u16;
            advance(&mut a, 1);
            if ((mood_target(&a) - t0).abs() - 120).abs() > 3 {
                continue;
            }
            for moff in [0i32, 120, -120, 1, -1, 240, -240] {
                let mut s2 = s.clone();
                s2.mood = (t0 + moff).clamp(0, MAX) as u16;
                cand += 1;
                for n in [2u32, 3, 5, 9, 17, 40] {
                    let mut fast = s2.clone();
                    fast.update(n * 6);
                    let mut slow = s2.clone();
                    for _ in 0..n {
                        advance(&mut slow, 1);
                    }
                    slow.age_ticks = fast.age_ticks;
                    slow.last_update_tick = fast.last_update_tick;
                    if fast != slow {
                        bad += 1;
                        break;
                    }
                }
            }
        }
        assert!(cand > 500, "only {cand} candidates");
        assert_eq!(bad, 0, "{bad} of {cand} near-step states differ");
    }
}

mod doses_food_drink {
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::thresholds::{DRINK_DURATION, FEED_DURATION, HATCHING_TICKS};
    use bornhack_aegg::game::engine::{Action, DrinkKind, FoodKind, GameState, PetKind, Phase};

    fn pct(v: i32) -> i32 {
        v * P
    }

    fn pet() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.update(HATCHING_TICKS() as u32);
        assert_eq!(s.phase, Phase::Active);
        s.is_sleeping = false;
        s.tick_rem = 0;
        s.mood = pct(50) as u16;
        s.energy = pct(90) as u16;
        s.fullness = pct(50) as u16;
        s.hydration = pct(50) as u16;
        s.weight = pct(50) as u16;
        s.dirt = pct(10) as u16;
        s.fitness = pct(50) as u16;
        s.fun = pct(50) as u16;
        s
    }

    /// Let the running action's animation run out, exactly.
    fn finish(s: &mut GameState, ticks: u8) {
        let now = s.last_update_tick + ticks as u32;
        s.update(now);
        assert_eq!(s.active_action, None);
    }

    fn eat(s: &mut GameState, f: FoodKind) {
        assert!(s.feed(f));
        finish(s, FEED_DURATION());
    }

    fn sip(s: &mut GameState, d: DrinkKind) {
        assert!(s.drink(d));
        finish(s, DRINK_DURATION());
    }

    #[test]
    fn overeating_becomes_food_coma_and_weight() {
        let mut s = pet();
        s.fullness = pct(80) as u16;
        eat(&mut s, FoodKind::Pizza);
        // 80 + 40 = 120 % counts in full, the stored level is capped.
        assert_eq!(s.fullness, 65535);
        let excess = pct(120) - pct(78);
        assert_eq!(s.energy as i32, pct(90) - excess / 2);
        assert_eq!(s.weight as i32, pct(52) + excess / 4);
        assert_eq!(s.dirt as i32, pct(20));
        assert_eq!(s.mood as i32, pct(58));
        assert_eq!(s.junk_min, 180);
    }

    #[test]
    fn meal_inside_the_band_has_no_coma() {
        let mut s = pet();
        eat(&mut s, FoodKind::Salad);
        assert_eq!(s.fullness as i32, pct(70));
        assert_eq!(s.energy as i32, pct(90));
        assert_eq!(s.hydration as i32, pct(58));
        assert_eq!(s.fitness as i32, pct(51));
        assert_eq!(s.junk_min, 0, "healthy food is not a treat");
        assert_eq!(s.mood as i32, pct(50));
    }

    #[test]
    fn water_refused_above_85_and_never_past_90() {
        let mut s = pet();
        s.hydration = pct(86) as u16;
        assert!(!s.drink(DrinkKind::Water));
        assert_eq!(s.active_action, None);

        let mut s = pet();
        s.hydration = pct(85) as u16;
        sip(&mut s, DrinkKind::Water);
        assert_eq!(s.hydration as i32, pct(90), "only 5 taken");
        assert_eq!(s.fullness as i32, pct(50), "under 15 taken: no belly");

        let mut s = pet();
        s.hydration = pct(70) as u16;
        s.booze = pct(20) as u16;
        s.hangover_min = 100;
        s.groggy_min = 20;
        sip(&mut s, DrinkKind::Water);
        assert_eq!(s.hydration as i32, pct(90));
        assert_eq!(s.fullness as i32, pct(52));
        assert_eq!(s.booze as i32, pct(15));
        assert_eq!(s.hangover_min, 40);
        assert_eq!(s.groggy_min, 0);

        let mut s = pet();
        s.hydration = pct(30) as u16;
        sip(&mut s, DrinkKind::Water);
        assert_eq!(s.hydration as i32, pct(60), "at most 30 per glass");
    }

    #[test]
    fn second_treat_within_three_hours_gives_a_third() {
        let mut s = pet();
        eat(&mut s, FoodKind::Frikandel);
        assert_eq!(s.mood as i32, pct(56));
        eat(&mut s, FoodKind::Pizza);
        assert_eq!(s.mood as i32, pct(58), "8 / 3 = 2");
        assert_eq!(s.junk_min, 180);
    }

    #[test]
    fn treat_on_ozempic_gives_no_mood_but_starts_the_window() {
        let mut s = pet();
        s.ozempic_min = 100;
        eat(&mut s, FoodKind::Cake);
        assert_eq!(s.mood as i32, pct(50));
        assert_eq!(s.junk_min, 180);
        assert_eq!(s.weight as i32, pct(53));
    }

    #[test]
    fn beer_on_a_tired_pet_adds_22_booze() {
        let mut s = pet();
        s.energy = pct(20) as u16;
        sip(&mut s, DrinkKind::Beer);
        assert_eq!(s.booze as i32 / P, 22); // 15 * 3 / 2
        assert_eq!(s.hydration as i32, pct(70));
        assert_eq!(s.fullness as i32, pct(52));
        assert_eq!(s.weight as i32, pct(51));

        let mut s = pet();
        sip(&mut s, DrinkKind::Beer);
        assert_eq!(s.booze as i32, pct(15));
        assert_eq!(s.mood as i32, pct(62));
        assert_eq!(s.energy as i32, pct(95));
        assert_eq!(s.fun as i32, pct(55));
    }

    #[test]
    fn drunk_pet_gets_less_out_of_a_drink_and_a_dependent_one_half() {
        let mut s = pet();
        s.booze = pct(50) as u16;
        sip(&mut s, DrinkKind::Whiskey);
        assert_eq!(s.mood as i32, pct(50) + 20 * P * 50 / 100);

        let mut s = pet();
        s.dependent = true;
        sip(&mut s, DrinkKind::Wine);
        assert_eq!(s.mood as i32, pct(50) + 15 * P * 50 / 100);
    }

    #[test]
    fn beer_during_rehab_is_a_relapse() {
        let mut s = pet();
        s.rehab_min = 1000;
        sip(&mut s, DrinkKind::Beer);
        assert_eq!(s.rehab_min, 0);
        assert_eq!(s.mood as i32, pct(50 - 20 + 12));
    }

    #[test]
    fn cola_is_a_sugar_rush() {
        let mut s = pet();
        s.energy = pct(60) as u16;
        sip(&mut s, DrinkKind::Cola);
        assert_eq!(s.hydration as i32, pct(70));
        assert_eq!(s.fullness as i32, pct(52));
        assert_eq!(s.energy as i32, pct(68));
        assert_eq!(s.mood as i32, pct(53));
        assert_eq!(s.weight as i32, pct(51));
        assert_eq!(s.booze, 0);
    }

    #[test]
    fn care_action_wakes_a_sleeping_pet_grumpy_under_50() {
        let mut s = pet();
        s.is_sleeping = true;
        s.energy = pct(30) as u16;
        assert!(s.feed(FoodKind::Apple));
        assert!(!s.is_sleeping);
        assert_eq!(s.mood as i32, pct(40), "grumpy: -10");

        let mut s = pet();
        s.is_sleeping = true;
        s.energy = pct(60) as u16;
        assert!(s.feed(FoodKind::Apple));
        assert!(!s.is_sleeping);
        assert_eq!(s.mood as i32, pct(50), "rested enough: not grumpy");
    }

    #[test]
    fn refused_action_costs_no_hax() {
        let mut s = pet();
        s.is_sleeping = true;
        s.hydration = pct(90) as u16;
        s.money_enabled = true;
        s.money = 100;
        assert!(!s.drink(DrinkKind::Water));
        assert_eq!(s.money, 100);
        assert!(s.is_sleeping, "a refused action does not wake the pet");
        assert_eq!(s.active_action, None);
    }

    #[test]
    fn idle_remainder_is_not_spent_on_an_action() {
        let mut s = pet();
        s.tick_rem = 5;
        let (full0, water0) = (s.fullness, s.hydration);
        assert!(s.feed(FoodKind::Apple));
        assert_eq!(s.active_action, Some(Action::Feed));
        let now = s.last_update_tick + FEED_DURATION() as u32;
        s.update(now);
        assert_eq!(s.active_action, None, "dose applied on the last tick");
        assert_eq!(s.tick_rem, 5);
        assert_eq!(s.fullness as i32, full0 as i32 + pct(25), "no needs aged");
        assert_eq!(s.hydration as i32, water0 as i32 + pct(8));
    }

    #[test]
    fn ticks_after_the_action_age_the_pet_from_the_remainder() {
        let mut s = pet();
        s.tick_rem = 5;
        assert!(s.feed(FoodKind::Apple));
        let now = s.last_update_tick + FEED_DURATION() as u32 + 1;
        s.update(now);
        assert_eq!(s.tick_rem, 0, "5 + 1 = one whole minute");
    }

    #[test]
    fn remainder_waits_out_a_partial_update_of_the_action() {
        let mut s = pet();
        s.tick_rem = 5;
        let full0 = s.fullness;
        assert!(s.feed(FoodKind::Apple));
        let t0 = s.last_update_tick;
        s.update(t0 + 1);
        assert_eq!(s.active_action, Some(Action::Feed));
        assert_eq!(s.action_ticks_remaining, 1);
        assert_eq!(s.tick_rem, 5);
        assert_eq!(s.fullness, full0, "dose not yet applied");
        s.update(t0 + 2);
        assert_eq!(s.active_action, None);
        assert_eq!(s.tick_rem, 5);
        assert_eq!(s.fullness as i32, full0 as i32 + pct(25));
    }

    #[test]
    fn leftover_ticks_plus_remainder_age_exactly_one_minute() {
        let mut s = pet();
        s.tick_rem = 5;
        assert!(s.feed(FoodKind::Apple));
        let t0 = s.last_update_tick;
        // Dose only, then one more tick: 5 + 1 = one minute, aged once.
        let mut after_dose = s.clone();
        after_dose.update(t0 + FEED_DURATION() as u32);
        assert_eq!(after_dose.tick_rem, 5);
        // 2 ticks feed, 1 left over, in one call.
        s.update(t0 + FEED_DURATION() as u32 + 1);
        assert_eq!(s.tick_rem, 0);
        let mut one_minute = after_dose.clone();
        bornhack_aegg::game::engine::advance::advance(&mut one_minute, 1);
        assert_eq!(s.fullness, one_minute.fullness);
        assert_eq!(s.hydration, one_minute.hydration);
        assert_eq!(s.energy, one_minute.energy);
        assert_ne!(s.fullness, after_dose.fullness, "one minute did age");
    }

    #[test]
    fn tired_is_judged_before_the_glass() {
        let mut s = pet();
        s.energy = (pct(33) - 1) as u16; // just under ENERGY_LO, +5 would lift it over
        sip(&mut s, DrinkKind::Beer);
        assert_eq!(s.booze as i32 / P, 22);
    }

    #[test]
    fn unaffordable_action_on_a_sleeper_neither_wakes_nor_charges() {
        let mut s = pet();
        s.is_sleeping = true;
        s.money_enabled = true;
        s.money = 5;
        assert!(!s.feed(FoodKind::Salad));
        assert_eq!(s.money, 5);
        assert!(s.is_sleeping);
        assert_eq!(s.active_action, None);
    }
}

mod doses_care {
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::thresholds::{
        EXERCISE_DURATION, HATCHING_TICKS, HEAL_DURATION, MEDICATE_DURATION, OZEMPIC_DURATION,
        PLAY_DURATION, REHAB_DURATION,
    };
    use bornhack_aegg::game::engine::{
        GameState, MiniGame, ONLYPETS_HOBBY_REWARD, PetKind, Phase, doses,
    };

    fn pct(v: i32) -> i32 {
        v * P
    }

    fn pet() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.update(HATCHING_TICKS() as u32);
        assert_eq!(s.phase, Phase::Active);
        s.is_sleeping = false;
        s.tick_rem = 0;
        s.money_enabled = false;
        s.mood = pct(50) as u16;
        s.energy = pct(90) as u16;
        s.fullness = pct(60) as u16;
        s.hydration = pct(60) as u16;
        s.weight = pct(50) as u16;
        s.dirt = pct(40) as u16;
        s.fitness = pct(50) as u16;
        s.fun = pct(50) as u16;
        s.strain = 0;
        s.booze = 0;
        s.health = pct(70) as u16;
        s
    }

    fn finish(s: &mut GameState, ticks: u8) {
        let now = s.last_update_tick + ticks as u32;
        s.update(now);
        assert_eq!(s.active_action, None);
    }

    fn exercise(s: &mut GameState) {
        assert!(s.exercise());
        finish(s, EXERCISE_DURATION());
    }

    #[test]
    fn play_is_a_dose() {
        let mut s = pet();
        s.groggy_min = 100;
        assert!(s.play());
        finish(&mut s, PLAY_DURATION());
        assert_eq!(s.fun as i32, pct(75));
        assert_eq!(s.energy as i32, pct(85));
        assert_eq!(s.fullness as i32, pct(57));
        assert_eq!(s.hydration as i32, pct(56));
        assert_eq!(s.fitness as i32, pct(52));
        assert_eq!(s.dirt as i32, pct(43));
        assert_eq!(s.groggy_min, 40);
    }

    #[test]
    fn tired_play_counts_double_and_injured_play_is_refused() {
        let mut s = pet();
        s.energy = pct(30) as u16;
        assert!(s.play());
        finish(&mut s, PLAY_DURATION());
        assert_eq!(s.fun as i32, pct(100));

        let mut s = pet();
        s.injured_min = 10;
        assert!(!s.play());
        assert!(!s.exercise());
        assert_eq!(s.active_action, None);
    }

    #[test]
    fn minigame_grade_scales_fun_and_doubles_when_tired() {
        let mut s = pet();
        doses::minigame(&mut s, 2);
        assert_eq!(s.fun as i32, pct(50 + 10 + 14));
        assert_eq!(s.energy as i32, pct(87));
        let mut s = pet();
        s.energy = pct(30) as u16;
        doses::minigame(&mut s, 1);
        assert_eq!(s.fun as i32, pct(50 + 34));
        assert_eq!(s.energy as i32, pct(27));
        let mut s = pet();
        s.award_inspiration(MiniGame::Nim, 2);
        assert!(s.fun as i32 > pct(50));
    }

    #[test]
    fn sport_while_hungry_injures_on_the_second_session() {
        let mut s = pet();
        s.fullness = pct(20) as u16; // under FULL_LO: strain times 3/2 = 52.5 %
        exercise(&mut s);
        assert_eq!(s.strain as i32, pct(35) * 150 / 100);
        assert_eq!(s.injured_min, 0);
        assert_eq!(s.fitness as i32, pct(62));
        let health = s.health;
        exercise(&mut s);
        assert!(s.strain as i32 > pct(60));
        assert_eq!(s.injured_min, 1440);
        assert_eq!(s.health as i32, health as i32 - pct(5));
        assert_eq!(s.fitness as i32, pct(62), "an injury earns no fitness");
        assert!(!s.exercise(), "injured: refused");
    }

    #[test]
    fn exercise_dose_numbers() {
        let mut s = pet();
        s.booze = pct(20) as u16;
        s.hangover_min = 200;
        s.groggy_min = 50;
        exercise(&mut s);
        assert_eq!(s.strain as i32, pct(35));
        assert_eq!(s.energy as i32, pct(80));
        assert_eq!(s.fullness as i32, pct(52));
        assert_eq!(s.hydration as i32, pct(48));
        assert_eq!(s.fun as i32, pct(60));
        assert_eq!(s.booze as i32, pct(15));
        assert_eq!(s.dirt as i32, pct(52));
        assert_eq!(s.weight as i32, pct(47));
        assert_eq!(s.hangover_min, 80);
        assert_eq!(s.groggy_min, 0);
        assert_eq!(s.fitness as i32, pct(62));
    }

    #[test]
    fn exercise_strain_multipliers_stack() {
        // Rested and fit: 35 * 7/10 * 85/100.
        let mut s = pet();
        s.rested_min = 100;
        s.fitness = pct(80) as u16;
        exercise(&mut s);
        assert_eq!(s.strain as i32, pct(35) * 70 / 100 * 85 / 100);

        // Hungry and overweight: 35 * 3/2 * 6/5.
        let mut s = pet();
        s.fullness = pct(20) as u16;
        s.weight = pct(90) as u16;
        exercise(&mut s);
        assert_eq!(s.strain as i32, pct(35) * 150 / 100 * 120 / 100);

        // Deprived: 5/4.
        let mut s = pet();
        s.deprived = true;
        exercise(&mut s);
        assert_eq!(s.strain as i32, pct(35) * 125 / 100);
    }

    #[test]
    fn wash_refused_when_clean_and_does_not_wake() {
        let mut s = pet();
        s.dirt = pct(20) as u16;
        s.is_sleeping = true;
        s.money_enabled = true;
        let money = s.money;
        assert!(!s.wash());
        assert!(s.is_sleeping);
        assert_eq!(s.money, money);
        assert_eq!(s.active_action, None);

        s.dirt = pct(80) as u16;
        assert!(s.wash());
        assert!(!s.is_sleeping, "a care action wakes the pet");
        finish(&mut s, PLAY_DURATION());
        assert_eq!(s.dirt as i32, pct(10));
    }

    #[test]
    fn relax_trades_fun_for_calm() {
        let mut s = pet();
        s.strain = pct(40) as u16;
        assert!(s.relax());
        finish(&mut s, PLAY_DURATION());
        assert_eq!(s.fun as i32, pct(25));
        assert_eq!(s.strain as i32, pct(25));
        assert_eq!(s.energy as i32, pct(93));
        assert_eq!(s.fitness as i32, pct(49));
    }

    #[test]
    fn medicine_on_an_injury_halves_it_and_adds_booze() {
        let mut s = pet();
        s.injured_min = 1440;
        assert!(s.heal());
        finish(&mut s, HEAL_DURATION());
        assert_eq!(s.injured_min, 720);
        assert_eq!(s.booze as i32, pct(10));
        assert_eq!(s.med_cd_min, 240);
        assert_eq!(s.health as i32, pct(70));
    }

    #[test]
    fn medicine_heals_or_numbs_and_waits_out_its_cooldown() {
        let mut s = pet();
        s.health = pct(50) as u16;
        assert!(s.heal());
        finish(&mut s, HEAL_DURATION());
        assert_eq!(s.health as i32, pct(60));
        assert_eq!(s.booze, 0);
        assert!(!s.heal(), "240 minute cooldown");

        let mut s = pet();
        s.health = pct(90) as u16;
        assert!(s.heal());
        finish(&mut s, HEAL_DURATION());
        assert_eq!(s.health as i32, pct(90));
        assert_eq!(s.booze as i32, pct(10));
    }

    #[test]
    fn medicate_gates_on_insulin_window() {
        let mut s = pet();
        assert!(!s.medicate(), "not diabetic");
        s.diabetic = true;
        s.insulin_min = 121;
        assert!(!s.medicate());
        s.insulin_min = 120;
        assert!(s.medicate());
        finish(&mut s, MEDICATE_DURATION());
        assert_eq!(s.insulin_min, 480);
        assert!(!s.is_diabetic_unmedicated());
        s.insulin_min = 0;
        assert!(s.is_diabetic_unmedicated());
        s.insulin_min = 480;
        assert!(!s.medicate(), "inside the window");
    }

    #[test]
    fn heal_at_exactly_80_percent_health_still_mends() {
        let mut s = pet();
        s.health = pct(80) as u16;
        assert!(s.heal());
        finish(&mut s, HEAL_DURATION());
        assert_eq!(s.health as i32, pct(90));
        assert_eq!(s.booze, 0);
    }

    #[test]
    fn sleep_at_exactly_50_percent_energy_is_a_nap() {
        let mut s = pet();
        s.energy = pct(50) as u16;
        assert!(s.sleep());
        assert!(!s.night);
    }

    #[test]
    fn ozempic_cooldown_refuses_second_dose() {
        let mut s = pet();
        s.weight = pct(80) as u16;
        assert!(s.ozempic());
        finish(&mut s, OZEMPIC_DURATION());
        assert_eq!(s.weight as i32, pct(65));
        assert_eq!(s.fitness as i32, pct(45));
        assert_eq!(s.ozempic_min, 1440);
        assert_eq!(s.ozempic_cd_min, 2880);
        assert!(!s.ozempic());
        assert_eq!(s.active_action, None);
    }

    #[test]
    fn rehab_only_when_dependent() {
        let mut s = pet();
        assert!(!s.rehab());
        s.dependent = true;
        assert!(s.rehab());
        finish(&mut s, REHAB_DURATION());
        assert_eq!(s.rehab_min, 4320);
        assert!(!s.rehab(), "already in rehab");
    }

    #[test]
    fn only_pets_on_a_sleeping_pet_is_refused_and_does_not_wake() {
        let mut s = pet();
        s.is_sleeping = true;
        assert!(!s.only_pets());
        assert!(s.is_sleeping);
        assert_eq!(s.active_action, None);
    }

    #[test]
    fn only_pets_pays_and_tires() {
        let mut s = pet();
        s.money_enabled = true;
        s.money = 0;
        assert!(s.only_pets());
        finish(&mut s, PLAY_DURATION());
        assert_eq!(s.money, ONLYPETS_HOBBY_REWARD);
        assert_eq!(s.energy as i32, pct(75));
        assert_eq!(s.fun as i32, pct(60));
        assert_eq!(s.mood as i32, pct(45));
        // No cooldown: it can go again.
        assert!(s.only_pets());
    }

    #[test]
    fn only_pets_refused_when_tired_or_injured() {
        let mut s = pet();
        s.energy = pct(30) as u16;
        assert!(!s.only_pets());
        let mut s = pet();
        s.injured_min = 5;
        assert!(!s.only_pets());
    }

    #[test]
    fn sleep_sets_night_from_energy_and_zeroes_the_counters() {
        let mut s = pet();
        s.energy = pct(40) as u16;
        s.sleep_min = 99;
        s.good_sleep_min = 99;
        assert!(s.sleep());
        assert!(s.is_sleeping && s.night);
        assert_eq!((s.sleep_min, s.good_sleep_min), (0, 0));
        assert!(!s.sleep(), "already asleep");
        assert!(s.wake());
        assert!(!s.is_sleeping);
        assert!(!s.wake(), "already awake");

        let mut s = pet();
        assert!(s.sleep());
        assert!(!s.night, "a nap");
    }

    #[test]
    fn friend_boost_gives_fun_not_mood() {
        let mut s = pet();
        s.friend_boost(true);
        assert_eq!(s.fun as i32, pct(65));
        s.friend_boost(false);
        assert_eq!(s.fun as i32, pct(70));
        assert_eq!(s.mood as i32, pct(50));
    }

    #[test]
    fn friend_doubles_when_tired() {
        for big in [true, false] {
            let gain = if big { 15 } else { 5 };
            let mut s = pet();
            s.energy = pct(20) as u16;
            s.fun = pct(30) as u16;
            s.friend_boost(big);
            assert_eq!(s.fun as i32, pct(30 + 2 * gain), "tired, big={big}");
            assert_eq!(s.energy as i32, pct(20), "no energy cost");
            let mut s = pet();
            s.energy = pct(33) as u16; // the Classic energy band edge
            s.fun = pct(30) as u16;
            s.friend_boost(big);
            assert_eq!(s.fun as i32, pct(30 + gain), "at the line is not tired");
        }
        let mut s = pet();
        s.is_sleeping = true;
        s.friend_boost(false);
        assert!(s.is_sleeping, "a friend does not wake the pet");
    }

    #[test]
    fn care_actions_on_a_sleeper_wake_it_but_refusals_do_not() {
        let mut s = pet();
        s.is_sleeping = true;
        s.injured_min = 1;
        assert!(!s.play());
        assert!(s.is_sleeping);
        s.injured_min = 0;
        assert!(s.play());
        assert!(!s.is_sleeping);
    }
}

mod cooldown_countdown {
    use bornhack_aegg::game::engine::thresholds::HATCHING_TICKS;
    use bornhack_aegg::game::engine::{GameState, PetKind};

    #[test]
    fn minigame_cooldown_counts_down_over_time() {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.update(HATCHING_TICKS() as u32);
        s.tick_rem = 0;
        s.cooldown_nim = 100;
        s.cooldown_battle = 50;
        s.cooldown_tictactoe = 30;
        let t0 = s.last_update_tick;
        s.update(t0 + 18); // 3 minutes
        assert_eq!(s.cooldown_nim, 82);
        assert_eq!(s.cooldown_battle, 32);
        assert_eq!(s.cooldown_tictactoe, 12);
        assert_eq!(s.cooldown_lightsout, 0);
        s.update(t0 + 120); // 20 minutes: all done
        assert_eq!(s.cooldown_nim, 0);
        assert_eq!(s.cooldown_battle, 0);
        assert_eq!(s.cooldown_tictactoe, 0);
    }
}

mod status_wake {
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::status::{StatusWord, status};
    use bornhack_aegg::game::engine::thresholds::{HATCHING_TICKS, MAX_SLEEP_TICKS};
    use bornhack_aegg::game::engine::{
        Action, DisplayAnim, GameState, MiniGame, PetKind, Phase, doses,
    };

    fn pct(v: i32) -> u16 {
        (v * P) as u16
    }

    fn pet() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.update(HATCHING_TICKS() as u32);
        assert_eq!(s.phase, Phase::Active);
        s.is_sleeping = false;
        s.tick_rem = 0;
        s.money_enabled = false;
        s
    }

    /// What a wake-up would have to show: any change here is a visible event.
    fn obs(s: &GameState) -> (StatusWord, bool, DisplayAnim, bool, Phase, Option<Action>) {
        let (w, d) = status(s);
        (
            w,
            d,
            s.display_anim(),
            s.is_sleeping,
            s.phase,
            s.active_action,
        )
    }

    /// First minute (1..=120) at which `obs` differs, stepping a clone minute by minute.
    fn first_change_min(s: &GameState) -> Option<u32> {
        let mut c = s.clone();
        let o = obs(&c);
        let t0 = c.last_update_tick;
        (1..=120u32).find(|m| {
            c.update(t0 + m * 6);
            obs(&c) != o
        })
    }

    fn cases() -> Vec<(&'static str, GameState)> {
        let mut v = vec![("fresh", pet())];
        let mut s = pet();
        s.fullness = pct(40);
        v.push(("fullness 40", s));
        let mut s = pet();
        s.mood = pct(79);
        v.push(("mood 79 rising", s));
        let mut s = pet();
        s.mood = pct(81);
        s.fullness = pct(20);
        v.push(("mood 81 falling, hungry", s));
        let mut s = pet();
        s.mood = pct(100);
        s.dirt = pct(80);
        s.fullness = pct(36);
        v.push(("two terms crossing", s));
        let mut s = pet();
        s.is_sleeping = true;
        s.energy = pct(20);
        s.night = true;
        v.push(("sleeping tired", s));
        let mut s = pet();
        s.health = pct(33);
        s.dirt = pct(70);
        v.push(("health near danger", s));
        let mut s = pet();
        s.injured_min = 30;
        s.groggy_min = 10;
        v.push(("injured then groggy", s));
        v
    }

    #[test]
    fn wake_is_never_later_than_first_status_change() {
        for (name, s) in cases() {
            let o = status(&s);
            let mut c = s.clone();
            let t0 = c.last_update_tick;
            let m = (1..=120u32).find(|m| {
                c.update(t0 + m * 6);
                status(&c) != o
            });
            let wake = s.next_wake_tick();
            match m {
                Some(m) => assert!(
                    wake <= t0 + m * 6,
                    "{name}: wake {wake} after status change at +{m} min"
                ),
                None => assert!(wake <= t0 + 720, "{name}: wake {wake}"),
            }
            assert!(wake > t0, "{name}: wake must be in the future");
        }
    }

    #[test]
    fn wake_is_never_later_than_first_display_change() {
        for (name, s) in cases() {
            let wake = s.next_wake_tick();
            let t0 = s.last_update_tick;
            match first_change_min(&s) {
                Some(m) => assert!(
                    wake <= t0 + m * 6,
                    "{name}: wake {wake} after change at +{m} min"
                ),
                None => assert!(wake <= t0 + 720, "{name}: wake {wake}"),
            }
        }
    }

    #[test]
    fn wake_is_exact_when_the_first_change_is_a_status_flip() {
        // Not just early: waking early costs battery.
        let mut s = pet();
        s.fullness = pct(40);
        let t0 = s.last_update_tick;
        let m = first_change_min(&s).expect("fullness 40 turns hungry within 2 h");
        assert_eq!(s.next_wake_tick(), t0 + m * 6);
    }

    #[test]
    fn wake_lands_on_the_first_visible_change() {
        // Waking early costs battery: the wake is the minute of the change,
        // including a change caused by two terms crossing inside a segment.
        for (name, s) in cases() {
            let t0 = s.last_update_tick;
            let want = first_change_min(&s).map_or(t0 + 720, |m| t0 + m * 6);
            assert_eq!(s.next_wake_tick(), want, "{name}");
        }
    }

    #[test]
    fn wake_counts_the_sub_minute_remainder() {
        let mut s = pet();
        s.fullness = pct(40);
        let a = s.next_wake_tick();
        s.tick_rem = 4;
        assert_eq!(s.next_wake_tick(), a - 4);
    }

    #[test]
    fn wake_is_capped_at_720_ticks() {
        assert_eq!(MAX_SLEEP_TICKS(), 720);
        for (name, s) in cases() {
            assert!(s.next_wake_tick() <= s.last_update_tick + 720, "{name}");
        }
        // A sleeper at rest with every need mid-band has nothing to show for 2 h.
        let mut s = pet();
        s.is_sleeping = true;
        s.night = true;
        s.energy = pct(20);
        s.fullness = pct(60);
        s.hydration = pct(60);
        s.fun = pct(55);
        s.mood = pct(100);
        s.dirt = pct(10);
        assert_eq!(first_change_min(&s), None);
        assert_eq!(s.next_wake_tick(), s.last_update_tick + 720);
    }

    #[test]
    fn wake_of_gone_hibernating_hatching_and_action() {
        let mut s = pet();
        s.hibernating = true;
        assert_eq!(s.next_wake_tick(), u32::MAX);
        s.hibernating = false;
        s.phase = Phase::Gone;
        assert_eq!(s.next_wake_tick(), u32::MAX);

        let egg = GameState::new_egg(7, PetKind(0));
        assert_eq!(egg.next_wake_tick(), egg.hatching_countdown as u32);

        let mut s = pet();
        assert!(s.play());
        assert_eq!(
            s.next_wake_tick(),
            s.last_update_tick + s.action_ticks_remaining as u32
        );
    }

    #[test]
    fn status_picks_the_largest_penalty() {
        assert_eq!(status(&pet()), (StatusWord::Content, false));

        // Hungry on its own (12 points), but dirt at 95 % (37 points) is the bigger problem.
        let mut s = pet();
        s.fullness = pct(30);
        assert_eq!(status(&s), (StatusWord::Hungry, false));
        s.dirt = pct(95);
        assert_eq!(status(&s), (StatusWord::Grubby, true));

        let mut s = pet();
        s.fullness = pct(10);
        assert_eq!(status(&s), (StatusWord::Hungry, true));

        let mut s = pet();
        s.fullness = pct(97);
        assert_eq!(status(&s).0, StatusWord::Stuffed);

        let mut s = pet();
        s.hydration = pct(20);
        assert_eq!(status(&s).0, StatusWord::Thirsty);
        s.hydration = pct(99);
        assert_eq!(status(&s).0, StatusWord::Waterlogged);

        let mut s = pet();
        s.energy = pct(15);
        assert_eq!(status(&s).0, StatusWord::Tired);
        s.is_sleeping = true; // the tired term only counts while awake
        assert_eq!(status(&s).0, StatusWord::Content);

        let mut s = pet();
        s.fun = pct(5);
        assert_eq!(status(&s).0, StatusWord::Bored);
        s.fun = pct(100);
        assert_eq!(status(&s).0, StatusWord::Wired);

        let mut s = pet();
        s.booze = pct(50);
        assert_eq!(status(&s), (StatusWord::Sloppy, false));
        s.booze = pct(80);
        assert_eq!(status(&s), (StatusWord::Sloppy, true));

        let mut s = pet();
        s.weight = pct(95);
        assert_eq!(status(&s).0, StatusWord::Overweight);
        s.weight = pct(5);
        assert_eq!(status(&s).0, StatusWord::Underweight);

        let mut s = pet();
        s.fitness = pct(5);
        assert_eq!(status(&s).0, StatusWord::Unfit);

        let mut s = pet();
        s.hangover_min = 30;
        assert_eq!(status(&s).0, StatusWord::HungOver);
        s.groggy_min = 30;
        assert_eq!(status(&s).0, StatusWord::HungOver); // 15 beats 10
        s.hangover_min = 0;
        assert_eq!(status(&s).0, StatusWord::Groggy);
        s.injured_min = 30;
        assert_eq!(status(&s).0, StatusWord::Injured); // 20 beats 10

        let mut s = pet();
        s.health = pct(20);
        assert_eq!(status(&s), (StatusWord::Poorly, true));
        s.health = pct(45);
        assert_eq!(status(&s), (StatusWord::Poorly, false));
    }

    #[test]
    fn content_needs_three_points_of_penalty_to_give_way() {
        let mut s = pet();
        // gap 2 * 3/2 = exactly 3 points: still content
        s.fullness = pct(36);
        assert_eq!(status(&s).0, StatusWord::Content);
        s.fullness = pct(35);
        assert_eq!(status(&s).0, StatusWord::Hungry);
    }

    #[test]
    fn display_anim_follows_the_status() {
        let mut s = pet();
        s.mood = pct(90);
        assert_eq!(s.display_anim(), DisplayAnim::Happy);
        s.mood = pct(70);
        assert_eq!(s.display_anim(), DisplayAnim::Idle);

        let mut s = pet();
        s.fullness = pct(30);
        assert_eq!(s.display_anim(), DisplayAnim::WarningHungry);
        s.fullness = pct(10);
        assert_eq!(s.display_anim(), DisplayAnim::CriticalHungry);
        let mut s = pet();
        s.hydration = pct(20);
        assert_eq!(s.display_anim(), DisplayAnim::WarningHungry);

        let mut s = pet();
        s.energy = pct(15);
        assert_eq!(s.display_anim(), DisplayAnim::WarningTired);
        s.hangover_min = 30;
        s.energy = pct(90);
        assert_eq!(s.display_anim(), DisplayAnim::WarningTired);
        s.hangover_min = 0;
        s.groggy_min = 30;
        assert_eq!(s.display_anim(), DisplayAnim::WarningTired);

        let mut s = pet();
        s.health = pct(45);
        assert_eq!(s.display_anim(), DisplayAnim::WarningSick);
        s.health = pct(20);
        assert_eq!(s.display_anim(), DisplayAnim::CriticalSick);
        let mut s = pet();
        s.injured_min = 30;
        assert_eq!(s.display_anim(), DisplayAnim::WarningSick);

        for set in [
            (|s: &mut GameState| s.fun = pct(5)) as fn(&mut GameState),
            |s| s.fun = pct(100),
            |s| s.booze = pct(50),
            |s| s.dirt = pct(70),
            |s| s.fullness = pct(97),
            |s| s.hydration = pct(99),
            |s| s.weight = pct(95),
            |s| s.weight = pct(5),
            |s| s.fitness = pct(5),
        ] {
            let mut s = pet();
            set(&mut s);
            assert_eq!(
                s.display_anim(),
                DisplayAnim::WarningMiserable,
                "{:?}",
                status(&s)
            );
        }

        // Actions and sleep still outrank any status.
        let mut s = pet();
        s.fullness = pct(10);
        s.is_sleeping = true;
        assert_eq!(s.display_anim(), DisplayAnim::Sleeping);
    }

    #[test]
    fn stats_report_percentages_and_flags() {
        let mut s = pet();
        s.fullness = pct(50);
        s.energy = pct(40);
        s.health = pct(90);
        s.mood = pct(70);
        s.fitness = pct(45);
        s.booze = pct(20);
        s.hydration = pct(60);
        s.fun = pct(55);
        s.dirt = pct(10);
        s.weight = pct(52);
        let now = s.last_update_tick;
        let st = s.stats(now);
        assert_eq!(
            (
                st.hunger, st.tired, st.healthy, st.happy, st.weight, st.drunk
            ),
            (50, 40, 90, 70, 45, 80)
        );
        assert_eq!(
            (st.water, st.fun, st.clean, st.booze, st.weight_pct, st.fit),
            (60, 55, 90, 20, 52, 45)
        );
        assert_eq!((st.status, st.danger), (StatusWord::Content, false));
        assert!(st.can_feed && st.can_play && st.can_sleep && !st.can_wake);
    }

    #[test]
    fn stats_can_flags_follow_the_refusal_rule() {
        let mut s = pet();
        s.injured_min = 30;
        s.med_cd_min = 100;
        s.ozempic_cd_min = 200;
        s.insulin_min = 300;
        s.rehab_min = 400;
        s.dirt = pct(10);
        let now = s.last_update_tick;
        let st = s.stats(now);
        assert!(!st.can_play && !st.can_exercise && !st.can_only_pets);
        assert!(!st.can_heal && !st.can_ozempic && !st.can_medicate && !st.can_rehab);
        assert!(st.can_feed);
        assert_eq!(
            (
                st.med_cd_min,
                st.ozempic_cd_min,
                st.insulin_min,
                st.rehab_min,
                st.injured_min
            ),
            (100, 200, 300, 400, 30)
        );

        // every menu row is enabled iff the starter accepts it
        let mut s = pet();
        s.diabetic = true;
        s.insulin_min = 60;
        s.dependent = true;
        s.dirt = pct(50);
        let now = s.last_update_tick;
        let st = s.stats(now);
        assert_eq!(st.can_medicate, s.clone().medicate());
        assert_eq!(st.can_rehab, s.clone().rehab());
        assert_eq!(st.can_heal, s.clone().heal());
        assert_eq!(st.can_ozempic, s.clone().ozempic());
        assert_eq!(st.can_exercise, s.clone().exercise());
        assert_eq!(st.can_only_pets, s.clone().only_pets());
        assert_eq!(
            st.can_feed,
            s.clone().feed(bornhack_aegg::game::engine::FoodKind::Apple)
        );
        let mut tired = pet();
        tired.energy = pct(10);
        let now = tired.last_update_tick;
        assert!(!tired.stats(now).can_only_pets);
        assert!(!tired.clone().only_pets());
    }

    #[test]
    fn stats_of_a_sleeper_allow_waking_and_care() {
        let mut s = pet();
        s.is_sleeping = true;
        let now = s.last_update_tick;
        let st = s.stats(now);
        assert!(st.can_wake && !st.can_sleep && st.is_sleeping);
        assert_eq!(
            st.can_feed,
            s.clone().feed(bornhack_aegg::game::engine::FoodKind::Apple)
        );
    }

    #[test]
    fn minigame_grade_scales_the_fun_dose() {
        let fun_after = |grade: u8| {
            let mut s = pet();
            s.fun = pct(30);
            s.award_inspiration(MiniGame::Nim, grade);
            s.fun
        };
        assert!(fun_after(1) > fun_after(0));
        assert!(fun_after(2) > fun_after(1));
        let mut a = pet();
        a.fun = pct(30);
        let mut b = a.clone();
        a.award_inspiration(MiniGame::Nim, 2);
        doses::minigame(&mut b, 2);
        assert_eq!(a.fun, b.fun);
    }
}

mod fix_round_1 {
    use bornhack_aegg::game::bornpets_cfg::apply_overrides;
    use bornhack_aegg::game::engine::doses::{Reason, Refusal, fmt_wait};
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::status::{StatusWord, status};
    use bornhack_aegg::game::engine::thresholds::{HATCHING_TICKS, Thresholds};
    use bornhack_aegg::game::engine::{
        Action, DisplayAnim, DrinkKind, GameState, MiniGame, PetKind, Phase,
    };

    fn pct(v: i32) -> u16 {
        (v * P) as u16
    }

    fn pet() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.update(HATCHING_TICKS() as u32);
        assert_eq!(s.phase, Phase::Active);
        s.is_sleeping = false;
        s.tick_rem = 0;
        s.money_enabled = false;
        s
    }

    #[test]
    fn waits_are_readable() {
        assert_eq!(fmt_wait(0).as_str(), "0s");
        assert_eq!(fmt_wait(59).as_str(), "59s");
        assert_eq!(fmt_wait(59 * 60).as_str(), "59m");
        assert_eq!(fmt_wait(2 * 3600).as_str(), "2h00");
        assert_eq!(fmt_wait(2 * 3600 + 5 * 60).as_str(), "2h05");
        assert_eq!(fmt_wait(72 * 3600).as_str(), "72h00");
    }

    #[test]
    fn every_reason_has_one_short_text() {
        let t = |r: Reason| r.text();
        assert_eq!(
            t(Reason::Refused(Refusal::NotThirsty)).as_str(),
            "not thirsty"
        );
        assert_eq!(
            t(Reason::Refused(Refusal::AlreadyClean)).as_str(),
            "already clean"
        );
        assert_eq!(t(Reason::Refused(Refusal::Injured)).as_str(), "injured");
        assert_eq!(t(Reason::Refused(Refusal::Asleep)).as_str(), "asleep");
        assert_eq!(t(Reason::Refused(Refusal::NoNeed)).as_str(), "no need");
        assert_eq!(t(Reason::Refused(Refusal::Tired)).as_str(), "too tired");
        assert_eq!(t(Reason::Wait(2 * 3600 + 5 * 60)).as_str(), "wait 2h05");
        assert_eq!(t(Reason::CantAfford).as_str(), "can't afford");
    }

    #[test]
    fn a_tired_pets_only_pets_row_says_too_tired() {
        let mut s = pet();
        s.energy = pct(10);
        let why = s.refusal_for(Action::OnlyPets, None).expect("refused");
        assert_eq!(why, Reason::Refused(Refusal::Tired));
        assert_eq!(why.text().as_str(), "too tired");
        assert!(!s.only_pets());
        // not tired: accepted
        assert_eq!(pet().refusal_for(Action::OnlyPets, None), None);
    }

    #[test]
    fn refusal_for_gives_the_reason_the_starter_would() {
        let mut s = pet();
        s.dirt = pct(10);
        assert_eq!(
            s.refusal_for(Action::Wash, None),
            Some(Reason::Refused(Refusal::AlreadyClean))
        );
        s.hydration = pct(90);
        assert_eq!(
            s.refusal_for(Action::Drink, Some(DrinkKind::Water)),
            Some(Reason::Refused(Refusal::NotThirsty))
        );
        assert_eq!(s.refusal_for(Action::Drink, Some(DrinkKind::Cola)), None);
        s.injured_min = 30;
        assert_eq!(
            s.refusal_for(Action::Play, None),
            Some(Reason::Refused(Refusal::Injured))
        );
        s.is_sleeping = true;
        assert_eq!(
            s.refusal_for(Action::OnlyPets, None),
            Some(Reason::Refused(Refusal::Asleep))
        );
        // not diabetic: no need, no wait
        assert_eq!(
            s.refusal_for(Action::Medicate, None),
            Some(Reason::Refused(Refusal::NoNeed))
        );
        let mut s = pet();
        s.play();
        assert_eq!(s.refusal_for(Action::Feed, None), Some(Reason::Busy));
    }

    #[test]
    fn medical_waits_show_the_real_remaining_time() {
        let mut s = pet();
        s.med_cd_min = 125;
        assert_eq!(
            s.refusal_for(Action::Heal, None),
            Some(Reason::Wait(125 * 60))
        );
        s.ozempic_cd_min = 2880;
        assert_eq!(
            s.refusal_for(Action::Ozempic, None),
            Some(Reason::Wait(2880 * 60))
        );
        s.dependent = true;
        s.rehab_min = 4320;
        assert_eq!(
            s.refusal_for(Action::Rehab, None),
            Some(Reason::Wait(4320 * 60))
        );
        assert_eq!(Reason::Wait(4320 * 60).text().as_str(), "wait 72h00");
        s.diabetic = true;
        s.insulin_min = 120 + 45;
        assert_eq!(
            s.refusal_for(Action::Medicate, None),
            Some(Reason::Wait(45 * 60))
        );
    }

    #[test]
    fn game_rows_wait_out_their_cooldown() {
        let mut s = pet();
        assert_eq!(s.game_refusal(Some(MiniGame::Nim)), None);
        s.cooldown_nim = 18;
        assert_eq!(s.game_refusal(Some(MiniGame::Nim)), Some(Reason::Wait(180)));
        assert_eq!(s.game_refusal(Some(MiniGame::TicTacToe)), None);
        s.cooldown_battle = 6;
        assert_eq!(s.game_refusal(None), Some(Reason::Wait(60)));
        s.is_sleeping = true;
        assert_eq!(
            s.game_refusal(Some(MiniGame::TicTacToe)),
            Some(Reason::Refused(Refusal::Asleep))
        );
    }

    #[test]
    fn only_danger_and_real_problems_buzz() {
        for w in [
            StatusWord::Hungry,
            StatusWord::Thirsty,
            StatusWord::Tired,
            StatusWord::Groggy,
            StatusWord::HungOver,
            StatusWord::Injured,
            StatusWord::Poorly,
        ] {
            assert!(w.alerts(), "{w:?}");
        }
        for w in [
            StatusWord::Content,
            StatusWord::Bored,
            StatusWord::Wired,
            StatusWord::Sloppy,
            StatusWord::Grubby,
            StatusWord::Stuffed,
            StatusWord::Waterlogged,
            StatusWord::Overweight,
            StatusWord::Underweight,
            StatusWord::Unfit,
        ] {
            assert!(!w.alerts(), "{w:?}");
        }
    }

    #[test]
    fn wake_cap_is_clamped_and_follows_the_preset() {
        let mut t = Thresholds::CLASSIC;
        apply_overrides(&mut t, b"MAX_SLEEP_TICKS=0\n");
        assert_eq!(t.MAX_SLEEP_TICKS, 30);
        apply_overrides(&mut t, b"MAX_SLEEP_TICKS=999999\n");
        assert_eq!(t.MAX_SLEEP_TICKS, 1440);
        apply_overrides(&mut t, b"MAX_SLEEP_TICKS=360\n");
        assert_eq!(t.MAX_SLEEP_TICKS, 360);
        assert_eq!(Thresholds::CLASSIC.MAX_SLEEP_TICKS, 720);
        assert_eq!(Thresholds::CASUAL.MAX_SLEEP_TICKS, 720);
    }

    #[test]
    fn tired_in_danger_is_critical_tired() {
        let mut s = pet();
        s.energy = 0;
        s.health = pct(29);
        assert_eq!(status(&s), (StatusWord::Tired, true));
        assert_eq!(s.display_anim(), DisplayAnim::CriticalTired);
    }

    #[test]
    fn stats_heal_flag_follows_the_cooldown() {
        let mut s = pet();
        s.med_cd_min = 30;
        let now = s.last_update_tick;
        let st = s.stats(now);
        assert!(!st.can_heal);
        assert_eq!(st.med_cd_min, 30);
    }
}

mod oracle {
    use bornhack_aegg::game::engine::GameState;
    use bornhack_aegg::game::engine::sim::{self, Driver, Outcome, Plan};

    type Routine = fn(u32, u32, &GameState) -> Plan;

    const ALL: [(&str, Routine); 9] = [
        ("balanced", sim::balanced),
        ("sporty", sim::sporty),
        ("lazy_treats", sim::lazy_treats),
        ("party", sim::party),
        ("short_sleeper", sim::short_sleeper),
        ("spam_water", sim::spam_water),
        ("spam_wash", sim::spam_wash),
        ("once_a_day", sim::once_a_day),
        ("absent", sim::absent),
    ];

    fn line(name: &str, o: &Outcome, d: &Outcome) {
        println!(
            "{name:14} oracle alive={} mood={} end_h={} min_h={} fit={} wt={} inj={} | delta alive={} mood={} end_h={} min_h={} fit={} wt={} inj={} wakes/day={:.1}",
            o.alive,
            o.avg_mood_pct,
            o.end_health_pct,
            o.min_health_pct,
            o.end_fitness_pct,
            o.end_weight_pct,
            o.injuries,
            d.alive,
            d.avg_mood_pct,
            d.end_health_pct,
            d.min_health_pct,
            d.end_fitness_pct,
            d.end_weight_pct,
            d.injuries,
            d.engine_wakes_per_day,
        );
    }

    fn diff(a: u8, b: u8) -> i32 {
        (a as i32 - b as i32).abs()
    }

    #[test]
    fn delta_matches_oracle_all_routines() {
        for (name, r) in ALL {
            let o = sim::run_routine(&r, 30, Driver::Oracle);
            let d = sim::run_routine(&r, 30, Driver::Delta);
            line(name, &o, &d);
            assert_eq!(o.alive, d.alive, "{name}: alive");
            if !(o.alive && d.alive) {
                continue;
            }
            assert!(
                diff(o.avg_mood_pct, d.avg_mood_pct) <= 1,
                "{name}: avg mood"
            );
            if name != "party" {
                assert!(
                    diff(o.end_health_pct, d.end_health_pct) <= 2,
                    "{name}: end health"
                );
            }
            assert!(
                diff(o.end_fitness_pct, d.end_fitness_pct) <= 2,
                "{name}: fitness"
            );
            assert_eq!(o.injuries, d.injuries, "{name}: injuries");
        }
    }

    fn fresh_sleeper() -> GameState {
        use bornhack_aegg::game::engine::PetKind;
        use bornhack_aegg::game::engine::thresholds::HATCHING_TICKS;
        let mut fresh = GameState::new_egg(42, PetKind::DEFAULT);
        fresh.update(HATCHING_TICKS() as u32);
        assert!(fresh.sleep());
        fresh
    }

    /// Step `s` one minute at a time up to tick `end`.
    fn step_to(s: &mut GameState, end: u32) {
        while s.last_update_tick < end {
            s.update((s.last_update_tick + 6).min(end));
        }
    }

    #[test]
    fn fourteen_day_gap_matches_oracle() {
        let fresh = fresh_sleeper();
        // Before death the whole state is identical.
        for hours in [6u32, 12, 24] {
            let end = fresh.last_update_tick + hours * 360;
            let mut delta = fresh.clone();
            delta.update(end);
            let mut oracle = fresh.clone();
            step_to(&mut oracle, end);
            assert_eq!(delta, oracle, "{hours} h");
        }
        let end = fresh.last_update_tick + 14 * sim::TICKS_PER_DAY;
        let mut delta = fresh.clone();
        delta.update(end);
        let mut oracle = fresh.clone();
        step_to(&mut oracle, end);
        println!(
            "14 days: delta {:?} age {} h={} / oracle {:?} age {} h={}",
            delta.phase,
            delta.age_ticks,
            delta.health / 655,
            oracle.phase,
            oracle.age_ticks,
            oracle.health / 655
        );
        assert_eq!(delta.phase, oracle.phase);
        assert_eq!(delta.phase, bornhack_aegg::game::engine::Phase::Gone);
        // The moment of Gone: the age the pet reached.
        assert_eq!(delta.age_ticks, oracle.age_ticks);
        assert!((delta.health as i32 / 655 - oracle.health as i32 / 655).abs() <= 2);
    }

    #[test]
    fn gone_mid_gap_records_true_age() {
        let fresh = fresh_sleeper();
        let end = fresh.last_update_tick + 14 * sim::TICKS_PER_DAY;
        let mut one = fresh.clone();
        one.update(end);
        let mut stepped = fresh.clone();
        step_to(&mut stepped, end);
        assert_eq!(one.phase, bornhack_aegg::game::engine::Phase::Gone);
        assert_eq!(one.age_ticks, stepped.age_ticks);
        assert!(
            one.age_ticks < 4 * sim::TICKS_PER_DAY,
            "age {}",
            one.age_ticks
        );
        // A pet that survives the gap is credited all of it.
        let mut alive = GameState::new_egg(1, bornhack_aegg::game::engine::PetKind::DEFAULT);
        alive.update(6);
        let before = alive.age_ticks;
        alive.update(alive.last_update_tick + 100);
        assert_eq!(alive.age_ticks, before + 100);
    }

    #[test]
    fn party_plan_keeps_every_step_for_a_diabetic() {
        use bornhack_aegg::game::engine::{DrinkKind, FoodKind};
        use sim::Act;
        let mut s = GameState::new_egg(1, bornhack_aegg::game::engine::PetKind::DEFAULT);
        s.diabetic = true;
        s.insulin_min = 0;
        s.dirt = 60 * 655;
        s.hydration = 10 * 655;
        let p = sim::party(0, 11 * 60, &s);
        let water = Act::Drink(DrinkKind::Water);
        assert_eq!(p.len(), 7, "{:?}", p.as_slice());
        assert_eq!(p[0], Act::Insulin);
        assert_eq!(p.iter().filter(|a| **a == water).count(), 3);
        assert!(p.contains(&Act::Eat(FoodKind::Frikandel)));
    }

    #[test]
    fn design_targets() {
        let run = |r: Routine| sim::run_routine(&r, 30, Driver::Delta);
        let balanced = run(sim::balanced);
        assert!(
            balanced.alive && balanced.avg_mood_pct >= 90,
            "balanced {balanced:?}"
        );
        assert!(run(sim::sporty).alive, "sporty");
        let lazy = run(sim::lazy_treats);
        assert!(lazy.alive, "lazy");
        assert!(
            (55..=70).contains(&lazy.end_health_pct),
            "lazy health {}",
            lazy.end_health_pct
        );
        let short = run(sim::short_sleeper);
        assert!(
            short.alive && short.min_health_pct >= 30,
            "short sleeper {short:?}"
        );
        assert!(run(sim::spam_water).alive, "spam water");
        assert!(run(sim::spam_wash).alive, "spam wash");
        assert!(!run(sim::once_a_day).alive, "once a day");
        let absent = run(sim::absent);
        assert!(!absent.alive && absent.days < 3.0, "absent {absent:?}");
    }

    #[test]
    fn wake_budget_balanced_day() {
        let r = sim::run_full(&sim::balanced, 30, Driver::Delta, sim::DEFAULT_SEED);
        let o = r.outcome;
        println!(
            "balanced: {:.2} engine wakes/day; next_wake_tick: {} calls, avg {:.1} / max {} ADVANCE_CALLS",
            o.engine_wakes_per_day,
            r.wake_queries,
            r.wake_calls_total as f64 / r.wake_queries.max(1) as f64,
            r.wake_calls_max
        );
        assert!(o.alive);
        assert!(
            o.engine_wakes_per_day <= 20.0,
            "{} wakes/day",
            o.engine_wakes_per_day
        );
    }
}

#[test]
fn template_parses_to_classic_with_no_edits() {
    let template = cfg_template();
    assert_eq!(
        template,
        include_str!("../assets/to-badge/BPETS_V3.CFG"),
        "asset file out of date: regenerate it from cfg_template()"
    );
    let mut values = Thresholds::CLASSIC;
    assert_eq!(apply_overrides(&mut values, template.as_bytes()), 0);
    assert_eq!(values, Thresholds::CLASSIC);

    for (key, _) in BORNPETS_CFG_KEYS {
        let n = template
            .lines()
            .filter(|l| {
                l.strip_prefix("# ")
                    .is_some_and(|r| r.starts_with(&format!("{key}=")))
            })
            .count();
        assert_eq!(n, 1, "key {key} appears {n} times");
    }

    // Uncomment every key line the way a player does: strip only the
    // leading "# ".  The "# Casual n" note stays and must not matter.
    let uncommented: String = template
        .lines()
        .skip(1)
        .map(|l| format!("{}\n", l.strip_prefix("# ").unwrap()))
        .collect();
    assert!(uncommented.contains("# Casual "));
    // Start from Casual so every row is a real edit.
    let mut values = Thresholds::CASUAL;
    apply_overrides(&mut values, uncommented.as_bytes());
    assert_eq!(values, Thresholds::CLASSIC);
}

#[test]
fn template_first_line_and_casual_notes() {
    let template = cfg_template();
    assert!(template.ends_with('\n') && !template.contains('\r'));
    assert_eq!(
        template.lines().next().unwrap(),
        "# BornPets v3 balance. Uncomment a line to change it. The old firmware keeps using BORNPETS.CFG."
    );
    assert!(template.contains("# MOOD_STEP=") && template.contains("# Casual "));
}

#[test]
fn no_v3_key_reuses_a_v1_name() {
    const V1: &[&str] = &[
        "HUNGER_RATE",
        "TIRED_RATE",
        "SLEEP_HUNGER_COST",
        "SICK_RATE",
        "SICK_CONDITION_RATE",
        "SICK_CONDITION_MISERABLE_RATE",
        "MISERABLE_INTERVAL_BASE",
        "MISERABLE_INTERVAL_MIN",
        "FEED_HUNGER_RELIEF",
        "HEAL_SICK_RELIEF",
        "PLAY_HUNGER_COST",
        "PLAY_TIRED_COST",
        "MINIGAME_HUNGER_COST",
        "MINIGAME_COOLDOWN",
        "HATCHING_TICKS",
        "MAX_SLEEP_TICKS",
        "WEIGHT_RATE",
        "FEED_WEIGHT_GAIN",
        "EXERCISE_WEIGHT_RELIEF",
        "DIABETES_ONSET_TICKS",
        "MEDICATE_COOLDOWN",
    ];
    // These keep their v1 meaning and unit.
    const SAME_MEANING: &[&str] = &["HATCHING_TICKS", "MINIGAME_COOLDOWN", "MAX_SLEEP_TICKS"];
    for (key, _) in BORNPETS_CFG_KEYS {
        if V1.contains(key) {
            assert!(SAME_MEANING.contains(key), "{key} reuses a v1 name");
        }
    }
}

#[test]
fn every_cfg_key_sets_its_own_field() {
    for (key, _) in BORNPETS_CFG_KEYS {
        let before = Thresholds::CLASSIC;
        let classic = before.value_of(key).unwrap();
        // classic+1 may be clamped (e.g. a percent at its top); then use -1.
        let mut hit = None;
        for sentinel in [classic + 1, classic.wrapping_sub(1)] {
            let mut t = before;
            apply_overrides(&mut t, format!("{key}={sentinel}\n").as_bytes());
            if t.value_of(key) == Some(sentinel) {
                hit = Some(t);
                break;
            }
        }
        let t = hit.unwrap_or_else(|| panic!("{key}: setter clamps both classic+-1"));
        for (other, _) in BORNPETS_CFG_KEYS {
            if other != key {
                assert_eq!(
                    t.value_of(other),
                    before.value_of(other),
                    "setting {key} changed {other}"
                );
            }
        }
    }
}

mod debug_menu {
    use bornhack_aegg::game::engine::doses::{self, Reason};
    use bornhack_aegg::game::engine::needs::{P, pct};
    use bornhack_aegg::game::engine::status::{StatusWord, status};
    use bornhack_aegg::game::engine::thresholds::HATCHING_TICKS;
    use bornhack_aegg::game::engine::{
        Action, DebugInfo, DebugPoke, GameState, PetKind, Phase, regime,
    };

    const NEED_POKES: [(DebugPoke, StatusWord, bool); 13] = [
        (DebugPoke::Hungry, StatusWord::Hungry, true),
        (DebugPoke::Thirsty, StatusWord::Thirsty, true),
        (DebugPoke::Stuffed, StatusWord::Stuffed, false),
        (DebugPoke::Tired, StatusWord::Tired, false),
        (DebugPoke::Bored, StatusWord::Bored, false),
        (DebugPoke::Dirty, StatusWord::Grubby, true),
        (DebugPoke::Fat, StatusWord::Overweight, false),
        (DebugPoke::Thin, StatusWord::Underweight, false),
        (DebugPoke::Unfit, StatusWord::Unfit, false),
        (DebugPoke::Drunk, StatusWord::Sloppy, true),
        (DebugPoke::HungOver, StatusWord::HungOver, false),
        (DebugPoke::Groggy, StatusWord::Groggy, false),
        (DebugPoke::Injured, StatusWord::Injured, false),
    ];

    fn pet() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.update(HATCHING_TICKS() as u32);
        assert_eq!(s.phase, Phase::Active);
        s.is_sleeping = false;
        s.tick_rem = 0;
        s.money_enabled = false;
        s
    }

    fn comfy() -> GameState {
        let mut s = pet();
        s.debug_poke(DebugPoke::Comfort).unwrap();
        s
    }

    #[test]
    fn each_poke_shows_its_status_word() {
        for (poke, word, danger) in NEED_POKES {
            let mut s = comfy();
            s.debug_poke(poke).unwrap();
            assert_eq!(status(&s), (word, danger), "{poke:?}");
        }
    }

    #[test]
    fn pokes_never_start_actions_or_charge() {
        for (poke, _, _) in NEED_POKES.into_iter().chain([
            (DebugPoke::Comfort, StatusWord::Content, false),
            (DebugPoke::Diabetic, StatusWord::Content, false),
            (DebugPoke::Dependent, StatusWord::Content, false),
            (DebugPoke::ClearConditions, StatusWord::Content, false),
            (DebugPoke::ClearCooldowns, StatusWord::Content, false),
        ]) {
            let mut s = pet();
            s.money_enabled = true;
            s.money = 123;
            s.mark_saved();
            assert!(!s.needs_save());
            s.debug_poke(poke).unwrap();
            assert_eq!(s.active_action, None, "{poke:?}");
            assert_eq!(s.money, 123, "{poke:?}");
            assert!(s.needs_save(), "{poke:?} must request a save");
        }
        // A running action is left running.
        let mut s = pet();
        assert!(s.play());
        s.debug_poke(DebugPoke::Hungry).unwrap();
        assert_eq!(s.active_action, Some(Action::Play));
    }

    #[test]
    fn comfort_is_content() {
        let mut s = pet();
        s.debug_poke(DebugPoke::Hungry).unwrap();
        s.debug_poke(DebugPoke::Comfort).unwrap();
        assert_eq!(status(&s), (StatusWord::Content, false));
        assert_eq!(pct(s.mood as i32), 70);
        assert_eq!(pct(s.health as i32), 70);
        // And the pet stays content when time passes.
        let t = s.last_update_tick;
        s.update(t + 60);
        assert_eq!(status(&s).0, StatusWord::Content);
    }

    #[test]
    fn skip_minutes_equals_update() {
        for m in [10u32, 60, 1440] {
            let mut a = pet();
            a.debug_poke(DebugPoke::Hungry).unwrap();
            let mut b = a.clone();
            let last = a.last_update_tick;
            a.debug_skip_minutes(m);
            b.update(last + m * 6);
            assert_eq!(a.last_update_tick, last, "skip must not move the tick base");
            b.last_update_tick = last;
            assert_eq!(a, b, "skip {m}");
        }
    }

    #[test]
    fn skip_does_not_freeze_the_next_real_update() {
        let mut s = pet();
        let last = s.last_update_tick;
        s.debug_skip_minutes(60);
        let age = s.age_ticks;
        s.update(last + 6);
        assert_eq!(s.age_ticks, age + 6);
    }

    #[test]
    fn clear_conditions_and_cooldowns() {
        let mut s = comfy();
        s.injured_min = 900;
        s.hangover_min = 300;
        s.groggy_min = 100;
        s.dependent = true;
        s.dependence_min = 77;
        s.diabetic = true;
        s.diabetes_min = 99;
        s.rehab_min = 50;
        s.reversal_min = 40;
        s.med_cd_min = 500;
        s.ozempic_cd_min = 500;
        s.insulin_min = 5000;
        s.cooldown_tictactoe = 5;
        s.cooldown_lightsout = 5;
        s.cooldown_blackhole = 5;
        s.cooldown_nim = 5;
        s.cooldown_bornjeweled = 5;
        s.cooldown_battle = 5;
        assert!(s.refusal_for(Action::Heal, None).is_some());
        assert!(s.refusal_for(Action::Ozempic, None).is_some());
        assert!(s.refusal_for(Action::Medicate, None).is_some());

        s.debug_poke(DebugPoke::ClearCooldowns).unwrap();
        assert_eq!(s.med_cd_min, 0);
        assert_eq!(s.ozempic_cd_min, 0);
        assert_eq!(s.insulin_min, 0);
        for g in [
            s.cooldown_tictactoe,
            s.cooldown_lightsout,
            s.cooldown_blackhole,
            s.cooldown_nim,
            s.cooldown_bornjeweled,
            s.cooldown_battle,
        ] {
            assert_eq!(g, 0);
        }
        assert_eq!(s.refusal_for(Action::Heal, None), None);
        assert_eq!(s.refusal_for(Action::Ozempic, None), None);
        assert_eq!(s.refusal_for(Action::Medicate, None), None);

        s.debug_poke(DebugPoke::ClearConditions).unwrap();
        assert_eq!(
            (
                s.injured_min,
                s.hangover_min,
                s.groggy_min,
                s.rehab_min,
                s.reversal_min
            ),
            (0, 0, 0, 0, 0)
        );
        assert!(!s.dependent && !s.diabetic);
        assert_eq!((s.dependence_min, s.diabetes_min), (0, 0));
    }

    #[test]
    fn diabetic_and_dependent_set_the_flags() {
        let mut s = comfy();
        s.debug_poke(DebugPoke::Diabetic).unwrap();
        s.debug_poke(DebugPoke::Dependent).unwrap();
        assert!(s.diabetic && s.dependent);
    }

    #[test]
    fn pokes_refuse_a_pet_that_cannot_take_them() {
        let mut hatching = GameState::new_egg(7, PetKind(0));
        assert_eq!(hatching.phase, Phase::Hatching);
        let before = hatching.clone();
        assert!(hatching.debug_poke(DebugPoke::Hungry).is_err());
        assert_eq!(hatching, before);

        let mut gone = pet();
        gone.phase = Phase::Gone;
        assert!(gone.debug_poke(DebugPoke::Hungry).is_err());

        let mut asleep_in_ice = pet();
        asleep_in_ice.hibernating = true;
        assert_eq!(
            asleep_in_ice.debug_poke(DebugPoke::Comfort),
            Err(Reason::Hibernating)
        );
        // The reason is the one every refused action shows.
        assert_eq!(
            asleep_in_ice.refusal_for(Action::Wash, None),
            Some(Reason::Hibernating)
        );
    }

    #[test]
    fn debug_info_matches_engine() {
        for poke in [
            DebugPoke::Comfort,
            DebugPoke::Hungry,
            DebugPoke::Drunk,
            DebugPoke::Injured,
        ] {
            let mut s = comfy();
            s.debug_poke(poke).unwrap();
            let i = s.debug_info();
            let (word, danger) = status(&s);
            assert_eq!((i.status, i.danger), (word, danger));
            assert_eq!(i.mood_target as i32, pct(regime::mood_target(&s)));
            assert_eq!(i.mood as i32, pct(s.mood as i32));
            assert_eq!(i.health as i32, pct(s.health as i32));
            let wake = s.next_wake_tick();
            let want = (wake != u32::MAX).then(|| (wake - s.last_update_tick).div_ceil(6));
            assert_eq!(i.wake_in_min, want);
            assert_eq!((i.asleep, i.night), (s.is_sleeping, s.night));
        }
        let _ = (P, doses::fmt_wait(0));
    }

    #[test]
    fn debug_info_line_is_short() {
        let mut i = DebugInfo {
            status: StatusWord::Hungry,
            danger: true,
            mood: 62,
            mood_target: 70,
            health: 80,
            wake_in_min: Some(23),
            asleep: false,
            night: false,
        };
        assert_eq!(i.line().as_str(), "Hungry!\nm62/70 h80 wake 23m");
        i.wake_in_min = Some(125);
        i.danger = false;
        i.status = StatusWord::Underweight;
        i.asleep = true;
        assert_eq!(i.line().as_str(), "Underweight zz\nm62/70 h80 wake 2h05");
        i.wake_in_min = None;
        assert!(i.line().as_str().ends_with("wake -"));
        // 7 px font on a 152 px screen: 21 columns per line.
        for l in i.line().as_str().lines() {
            assert!(l.len() <= 21, "{l}");
        }
    }
}
