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
