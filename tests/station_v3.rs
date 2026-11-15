//! BornPets v3 NFC station tests (plan 4).

mod parse {
    use bornhack_aegg::game::engine::fix::{ALIASES, normalize, parse_text};

    #[test]
    fn the_shared_vectors_hold() {
        let data = include_str!("data/station_vectors.txt");
        let mut seen = 0;
        for (n, line) in data.lines().enumerate() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let at = format!("vectors line {}: {:?}", n + 1, line);
            let got = parse_text(f[1].as_bytes());
            match f[0] {
                "accept" => {
                    let fix = got.unwrap_or_else(|e| panic!("{at}: {e:?}")).expect(&at);
                    assert_eq!(fix.canonical().as_str(), f[2], "{at}");
                }
                "reject" => {
                    let r = got.expect_err(&at);
                    assert_eq!(r.code(), f[2], "{at}");
                }
                "ignore" => assert_eq!(got, Ok(None), "{at}"),
                other => panic!("{at}: unknown kind {other}"),
            }
            seen += 1;
        }
        assert!(seen >= 40, "only {seen} vectors");
    }

    #[test]
    fn every_preset_parses_and_is_understood() {
        let data = include_str!("../scripts/station_presets.txt");
        let mut n = 0;
        for line in data.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
            let (place, text) = line.split_once('|').expect(line);
            let fix = parse_text(text.as_bytes())
                .unwrap_or_else(|e| panic!("{place}: {e:?}"))
                .expect(place);
            assert!(fix.understood(), "{place}");
            if place != "Clinic" {
                assert_eq!(fix.canonical().as_str(), text, "{place}");
            }
            n += 1;
        }
        assert_eq!(n, 8);
    }

    #[test]
    fn every_alias_is_a_valid_fix() {
        for (phrase, fix) in ALIASES {
            let a = parse_text(phrase.as_bytes());
            let b = parse_text(fix.as_bytes());
            assert!(matches!(a, Ok(Some(_))), "{phrase}");
            assert_eq!(a, b, "{phrase}");
        }
    }

    #[test]
    fn normalize_keeps_the_limit() {
        assert!(normalize(b"").is_none());
        assert!(normalize(b" \t\n").is_none());
        assert_eq!(normalize(&[b'a'; 96]).unwrap().len(), 96);
        assert!(normalize(&[b'a'; 97]).is_none());
        assert_eq!(normalize(b"A\tB").unwrap().as_slice(), b"a b");
    }

    #[test]
    fn the_id_is_kept() {
        let f = parse_text(b"fix s=Gym1 fit+5").unwrap().unwrap();
        assert_eq!(f.id(), b"gym1");
    }
}

mod apply {
    use bornhack_aegg::game::engine::fix::{Fix, Level, Op, Target, parse_text, safe_range};
    use bornhack_aegg::game::engine::needs::{self, MAX, P};
    use bornhack_aegg::game::engine::regime::{health_ceiling, health_damage};
    use bornhack_aegg::game::engine::thresholds::{
        BORNPETS_CFG_KEYS, DEMO_PAUSE_MIN, HATCHING_TICKS, RECOVERING_MIN, STATION_MOOD_PCT,
        Thresholds, validate,
    };
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    fn pet() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.update(HATCHING_TICKS() as u32);
        assert_eq!(s.phase, Phase::Active);
        s.is_sleeping = false;
        s.tick_rem = 0;
        s.money_enabled = false;
        s
    }

    fn pct(p: i32) -> u16 {
        (p * P) as u16
    }

    fn fix_of(t: &[u8]) -> Fix {
        parse_text(t).unwrap().unwrap()
    }

    const PRESETS: [&[u8]; 8] = [
        b"fix s=food1 full>60 water>60",
        b"fix s=pharm1 health>100 booze<20 clear=hangover,injury,groggy",
        b"fix s=nap1 energy>100 clear=groggy,deprived",
        b"fix s=spa1 dirt<10 fun<60 strain<10 clear=groggy",
        b"fix s=bar1 booze<10 water>60 clear=hangover",
        b"fix s=water1 water>70",
        b"fix s=gym1 fit+5 strain<20 cd=240",
        b"fix s=clinic1 clear=dependence,diabetes booze<5 fit>50 cd=1440",
    ];

    /// Seeded LCG, the generator of `status_wake::wake_is_exact_on_random_states`.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self, bound: u64) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 33) % (bound + 1)
        }
    }

    fn random_state(r: &mut Rng) -> GameState {
        let mut s = pet();
        let mut level = |r: &mut Rng| pct(r.next(100) as i32);
        s.fullness = level(r);
        s.hydration = level(r);
        s.energy = level(r);
        s.fun = level(r);
        s.dirt = level(r);
        s.booze = level(r);
        s.strain = level(r);
        s.fitness = level(r);
        s.weight = level(r);
        s.health = level(r);
        s.mood = level(r);
        s.is_sleeping = r.next(1) == 1;
        s.deprived = r.next(1) == 1;
        s.diabetic = r.next(1) == 1;
        s.dependent = r.next(1) == 1;
        s.rested_min = r.next(200) as u16;
        s.hangover_min = r.next(200) as u16;
        s.injured_min = r.next(200) as u16;
        s.groggy_min = r.next(200) as u16;
        s.insulin_min = r.next(200) as u16;
        s.rehab_min = r.next(200) as u16;
        s
    }

    fn run(s: &mut GameState, text: &[u8]) {
        s.station_fix(&fix_of(text));
    }

    #[test]
    fn the_edges_are_the_damage_lines() {
        let mut s = pet();
        s.fullness = pct(100);
        s.hydration = pct(100);
        run(&mut s, b"fix s=t full<0 water<0");
        assert_eq!((s.fullness, s.hydration), (pct(15), pct(15)));
        let mut s = pet();
        s.fullness = pct(0);
        s.hydration = pct(0);
        run(&mut s, b"fix s=t full>100 water>100");
        assert_eq!((s.fullness, s.hydration), (pct(95), pct(95)));
        let mut s = pet();
        s.fullness = pct(100);
        s.hydration = pct(100);
        run(&mut s, b"fix s=t full=0 water=0");
        assert_eq!((s.fullness, s.hydration), (pct(15), pct(15)));
        let mut s = pet();
        s.energy = pct(50);
        run(&mut s, b"fix s=t energy=0");
        assert_eq!(s.energy, pct(10));
        let mut s = pet();
        s.dirt = pct(10);
        run(&mut s, b"fix s=t dirt=100");
        assert_eq!(s.dirt, pct(85));
        let mut s = pet();
        s.booze = 0;
        run(&mut s, b"fix s=t booze=100");
        assert_eq!(s.booze as i32, needs::booze_danger());
        let mut s = pet();
        s.strain = 0;
        run(&mut s, b"fix s=t strain+100");
        assert_eq!(s.strain as i32, needs::strain_inj());
        let mut s = pet();
        s.weight = pct(50);
        run(&mut s, b"fix s=t weight=0");
        assert_eq!(s.weight, pct(20));
        let mut s = pet();
        s.weight = pct(50);
        run(&mut s, b"fix s=t weight=100");
        assert_eq!(s.weight, pct(80));
        let mut s = pet();
        s.health = pct(50);
        run(&mut s, b"fix s=t health=0");
        assert_eq!(s.health, pct(30));
        let mut s = pet();
        s.health = pct(10);
        run(&mut s, b"fix s=t health>100");
        assert_eq!(s.health as i32, health_ceiling(&s));
        let mut s = pet();
        s.fitness = pct(30);
        run(&mut s, b"fix s=t fit=90");
        assert_eq!(s.fitness, pct(60));
        let mut s = pet();
        s.fun = pct(40);
        run(&mut s, b"fix s=t fun=0");
        assert_eq!(s.fun, 0);
    }

    #[test]
    fn fit_is_capped_but_never_lowered() {
        for (start, text, want) in [
            (58, &b"fix s=t fit+5"[..], 60),
            (70, b"fix s=t fit+5", 70),
            (30, b"fix s=t fit>50", 50),
            (80, b"fix s=t fit>50", 80),
        ] {
            let mut s = pet();
            s.fitness = pct(start);
            run(&mut s, text);
            assert_eq!(s.fitness, pct(want), "{start} {:?}", core::str::from_utf8(text));
        }
    }

    #[test]
    fn a_pet_outside_the_range_is_only_helped() {
        for (text, set, want) in [
            (&b"fix s=t full>60"[..], 98, 98),
            (b"fix s=t dirt<10", 95, 10),
            (b"fix s=t booze<20", 90, 20),
            (b"fix s=t weight<70", 90, 70),
        ] {
            let mut s = pet();
            s.fullness = pct(set);
            s.dirt = pct(set);
            s.booze = pct(set);
            s.weight = pct(set);
            run(&mut s, text);
            let got = match &text[8..] {
                t if t.starts_with(b"full") => s.fullness,
                t if t.starts_with(b"dirt") => s.dirt,
                t if t.starts_with(b"booze") => s.booze,
                _ => s.weight,
            };
            assert_eq!(got, pct(want), "{:?}", core::str::from_utf8(text));
        }
    }

    #[test]
    fn the_clamp_never_works_against_the_item() {
        for target in Target::ALL {
            for op in [Op::Set, Op::AtLeast, Op::AtMost, Op::Add, Op::Sub] {
                for p in [0u8, 1, 50, 99, 100] {
                    for start in [0, 10, 50, 90, 100] {
                        let mut s = pet();
                        s.fullness = pct(start);
                        s.hydration = pct(start);
                        s.energy = pct(start);
                        s.fun = pct(start);
                        s.dirt = pct(start);
                        s.booze = pct(start);
                        s.strain = pct(start);
                        s.fitness = pct(start);
                        s.weight = pct(start);
                        s.health = pct(start);
                        let mut f = Fix::new(b"t");
                        f.levels.push(Level { target, op, pct: p }).unwrap();
                        let read = |s: &GameState| match target {
                            Target::Full => s.fullness,
                            Target::Water => s.hydration,
                            Target::Energy => s.energy,
                            Target::Fun => s.fun,
                            Target::Dirt => s.dirt,
                            Target::Booze => s.booze,
                            Target::Strain => s.strain,
                            Target::Fit => s.fitness,
                            Target::Weight => s.weight,
                            Target::Health => s.health,
                        } as i32;
                        let (cur, v) = (read(&s), if p == 100 { MAX } else { p as i32 * P });
                        let (lo, hi) = safe_range(&s, target);
                        assert!(lo <= hi, "{target:?}");
                        s.station_fix(&f);
                        let new = read(&s);
                        let at = format!("{target:?} {op:?} {p} from {start}");
                        match op {
                            Op::AtLeast | Op::Add => assert!(new >= cur, "{at}"),
                            Op::AtMost | Op::Sub => assert!(new <= cur, "{at}"),
                            Op::Set => assert!(new >= cur.min(v) && new <= cur.max(v), "{at}"),
                        }
                        assert!(new >= lo.min(cur) && new <= hi.max(cur), "{at}: left the range");
                    }
                }
            }
        }
    }

    #[test]
    fn no_fix_raises_health_damage() {
        let mut r = Rng(1);
        for i in 0..3000 {
            let mut s = random_state(&mut r);
            let mut f = Fix::new(b"t");
            for _ in 0..r.next(5) {
                let target = Target::ALL[r.next(9) as usize];
                let op = [Op::Set, Op::AtLeast, Op::AtMost, Op::Add, Op::Sub][r.next(4) as usize];
                let _ = f.levels.push(Level { target, op, pct: r.next(100) as u8 });
            }
            f.clear = r.next(63) as u8;
            let before = s.clone();
            let money = s.money;
            let _ = s.take_causes();
            s.station_fix(&f);
            let mut probe = s.clone();
            probe.health = before.health;
            assert!(health_damage(&probe) <= health_damage(&before), "state {i}: {f:?}");
            assert!(s.mood >= before.mood, "state {i}");
            assert_eq!(s.money, money);
            assert_eq!(s.phase, before.phase);
            assert_eq!(s.is_sleeping, before.is_sleeping);
            assert_eq!(s.hibernating, before.hibernating);
            assert!(s.take_causes().is_empty(), "state {i}");
        }
    }

    #[test]
    fn conditions_clear_and_chronic_ones_like_their_cures() {
        let mut s = pet();
        s.dependent = true;
        s.dependence_min = 500;
        s.rehab_min = 30;
        s.diabetic = true;
        s.diabetes_min = 500;
        s.reversal_min = 20;
        run(&mut s, PRESETS[7]);
        assert!(!s.dependent && s.dependence_min == 0 && s.rehab_min == 0);
        assert_eq!(s.recovering_min, RECOVERING_MIN());
        assert!(!s.diabetic && s.diabetes_min == 0 && s.reversal_min == 0);
    }

    #[test]
    fn health_reads_the_ceiling_after_the_clears() {
        let mut s = pet();
        s.fitness = pct(80);
        s.diabetic = true;
        s.insulin_min = 0;
        s.health = pct(10);
        let cut = health_ceiling(&s);
        run(&mut s, b"fix s=t health>100 clear=diabetes");
        assert!(!s.diabetic);
        let ceiling = health_ceiling(&s);
        assert!(ceiling > cut);
        assert_eq!(s.health as i32, ceiling);
    }

    #[test]
    fn an_energy_fix_forgives_the_next_oversleep() {
        for (text, want) in [
            (&b"fix s=t energy>100"[..], true),
            (b"fix s=t energy+5", true),
            (b"fix s=t energy=50", true),
            (b"fix s=t energy<50", false),
            (b"fix s=t energy-5", false),
            (b"fix s=t fun>50", false),
        ] {
            let mut s = pet();
            s.no_groggy = false;
            run(&mut s, text);
            assert_eq!(s.no_groggy, want, "{:?}", core::str::from_utf8(text));
        }
    }

    #[test]
    fn every_fix_lifts_mood_by_the_bump() {
        let bump = STATION_MOOD_PCT() as i32 * P;
        let mut s = pet();
        s.mood = pct(50);
        run(&mut s, PRESETS[5]);
        assert_eq!(s.mood as i32, pct(50) as i32 + bump);
        s.mood = MAX as u16;
        run(&mut s, PRESETS[5]);
        assert_eq!(s.mood as i32, MAX);
    }

    #[test]
    fn asleep_and_hibernating_pets_stay_so() {
        let mut s = pet();
        s.is_sleeping = true;
        s.energy = pct(5);
        run(&mut s, PRESETS[2]);
        assert!(s.is_sleeping);
        assert_eq!(s.energy as i32, MAX);

        let mut s = pet();
        s.hibernating = true;
        s.fullness = pct(10);
        s.hydration = pct(10);
        run(&mut s, PRESETS[0]);
        assert!(s.hibernating);
        assert_eq!((s.fullness, s.hydration), (pct(60), pct(60)));
        let t = s.last_update_tick;
        let frozen = (s.fullness, s.hydration, s.mood);
        s.update(t + 600);
        assert_eq!((s.fullness, s.hydration, s.mood), frozen);
    }

    #[test]
    fn the_demo_pet_pauses_and_thanks() {
        let mut s = pet();
        s.demo.on = true;
        assert!(s.station_fix(&fix_of(PRESETS[5])));
        assert_eq!(s.demo.pause_min, DEMO_PAUSE_MIN());
        let mut s = pet();
        s.demo.on = true;
        s.is_sleeping = true;
        assert!(!s.station_fix(&fix_of(PRESETS[5])));
        assert_eq!(s.demo.pause_min, DEMO_PAUSE_MIN());
        let mut s = pet();
        s.demo.on = true;
        s.hibernating = true;
        assert!(!s.station_fix(&fix_of(PRESETS[5])));
        let mut s = pet();
        s.demo.pause_min = 0;
        assert!(!s.station_fix(&fix_of(PRESETS[5])));
        assert_eq!(s.demo.pause_min, 0);
    }

    #[test]
    fn hard_mode_pays_nothing() {
        let mut s = pet();
        s.hard_mode = true;
        s.money_enabled = true;
        s.money = 50;
        run(&mut s, PRESETS[1]);
        assert_eq!((s.money, s.money_enabled, s.hard_mode), (50, true, true));
    }

    #[test]
    fn several_needs_at_one_event() {
        let mut s = pet();
        s.dirt = pct(80);
        s.fun = pct(90);
        s.strain = pct(50);
        s.groggy_min = 40;
        let t = s.last_update_tick;
        run(&mut s, PRESETS[3]);
        assert_eq!(s.dirt, pct(10));
        assert_eq!(s.fun, pct(60));
        assert_eq!(s.strain, pct(10));
        assert_eq!(s.groggy_min, 0);
        assert_eq!(s.last_update_tick, t);
    }

    #[test]
    fn a_fix_keeps_the_wake_exact() {
        fn first_change_min(s: &GameState) -> Option<u32> {
            let obs = |s: &GameState| format!("{:?}", s.wake_view_for_test());
            let mut c = s.clone();
            let o = obs(&c);
            let t0 = c.last_update_tick;
            (1..=120u32).find(|m| {
                c.update(t0 + m * 6);
                obs(&c) != o
            })
        }
        let mut r = Rng(1);
        for text in PRESETS {
            let f = fix_of(text);
            for i in 0..300 {
                let mut s = random_state(&mut r);
                s.station_fix(&f);
                let t0 = s.last_update_tick;
                let expect = first_change_min(&s).map_or(t0 + 120 * 6, |m| t0 + m * 6);
                assert_eq!(
                    s.next_wake_tick(),
                    expect,
                    "{:?} state {i}",
                    core::str::from_utf8(text)
                );
            }
        }
    }

    #[test]
    fn station_tunables_have_keys_and_validate() {
        for key in [
            "STATION_MOOD_PCT",
            "STATION_FIT_CAP_PCT",
            "STATION_CD_MIN",
            "STATION_CD_MAX_MIN",
        ] {
            assert!(BORNPETS_CFG_KEYS.iter().any(|(k, _)| *k == key), "{key}");
        }
        let mut t = Thresholds::CLASSIC;
        for (key, set) in BORNPETS_CFG_KEYS {
            match *key {
                "STATION_CD_MIN" => set(&mut t, 10),
                "STATION_CD_MAX_MIN" => set(&mut t, 5),
                _ => {}
            }
        }
        let v = validate(&t, &Thresholds::CLASSIC);
        assert_eq!((v.STATION_CD_MIN, v.STATION_CD_MAX_MIN), (5, 1440));
    }
}
