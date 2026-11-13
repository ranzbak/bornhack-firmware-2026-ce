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
