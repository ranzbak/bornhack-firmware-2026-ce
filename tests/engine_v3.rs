//! BornPets v2 engine tests (plan 1).

use bornhack_aegg::game::bornpets_cfg::{apply_overrides, is_custom_set};
use bornhack_aegg::game::engine::needs::{Need, build_edges};
use bornhack_aegg::game::engine::thresholds::{Mode, Thresholds, validate};

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
    assert!(inside(s.fullness, full_lo(), full_hi()), "fullness {}", s.fullness);
    assert!(inside(s.hydration, water_lo(), water_hi()), "hydration {}", s.hydration);
    assert!(s.energy as i32 >= energy_lo(), "energy {}", s.energy);
    assert!(inside(s.fun, fun_lo(), fun_hi()), "fun {}", s.fun);
    assert!(inside(s.weight, weight_lo(), weight_hi()), "weight {}", s.weight);
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
        for (b, want) in [(0, 100), (14, 100), (15, 110), (39, 110), (40, 125), (69, 125), (70, 140), (100, 140)] {
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
                t.DMG_FULL_LOW_PM, t.DMG_FULL_HIGH_PM, t.DMG_WATER_LOW_PM, t.DMG_WATER_HIGH_PM,
                t.DMG_ENERGY_PM, t.DMG_BOOZE_PM, t.DMG_MOOD_PM, t.DMG_DIRT_PM,
                t.DMG_WEIGHT_HIGH_PM, t.DMG_WEIGHT_LOW_PM,
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
        assert_eq!(s.hydration, pct(65) - 45 * 110 / 100, "booze 30 %: thirst x1.1");
        assert_eq!(s.energy, pct(80) - 36);
        assert_eq!(s.fun, pct(50) - 40);
        assert_eq!(s.dirt, pct(20) + 8);
        assert_eq!(s.booze, pct(30) - 36);
        assert_eq!(s.strain, pct(20) - 80);
        assert_eq!(s.fitness, pct(55) - 4);
        assert_eq!(s.weight, pct(50) - 1);
        assert_eq!(s.mood, MAX as u16);
        assert_eq!(s.health, pct(70) + 14, "below the ceiling and in a good mood");
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
        assert_eq!(s.dependence_min, 360, "quarter-minutes: minus 1 per sober minute");
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
                s.fullness, s.hydration, s.energy, s.fun, s.dirt, s.booze, s.strain, s.fitness,
                s.weight, s.mood, s.health, s.sleep_min, s.oversleep_min, s.reversal_min,
            ],
            [s.dependence_min, s.diabetes_min, s.leaving_min, s.care_awake_min],
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
            [t.CEILING_STEP0_PCT, t.CEILING_STEP1_PCT, t.CEILING_STEP2_PCT, t.CEILING_STEP3_PCT, t.CEILING_DIABETIC_PCT],
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
        assert!(b.fullness < (60 * P) as u16, "pet did not age: {}", b.fullness);
    }

    #[test]
    fn sub_minute_time_is_carried_not_lost() {
        let mut s = hatched();
        s.update(5);
        assert_eq!(s.tick_rem, 5);
        let before = s.fullness;
        s.update(6);
        assert_eq!(s.tick_rem, 0);
        assert!(s.fullness < before, "one whole minute must have been applied");
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
        assert_eq!(seg_len(&s, 10_000), 6, "injured feeds mood: one minute early (R20)");
    }

    #[test]
    fn pinned_level_is_ignored() {
        let mut s = calm();
        s.fullness = pct(60);
        s.is_sleeping = true;
        s.energy = MAX as u16;
        s.sleep_min = 700;
        assert_eq!(seg_len(&s, 10_000), 19, "only the self-wake timer bounds it, one minute early");
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
            assert_eq!(closed, stepped, "{name}: segment of {d} minutes differs from stepping");
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
