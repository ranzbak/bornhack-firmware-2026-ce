//! BornPets v3 demo mode tests (plan 3).

mod state {
    use bornhack_aegg::game::engine::demo::{BLOCK_AT, BLOCK_LEN, Demo};
    use bornhack_aegg::game::engine::doses::{Reason, Refusal};
    use bornhack_aegg::game::engine::hint::Remedy;
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::{
        FoodKind, GRADE_WIN, GameState, MiniGame, PetKind, Phase, SAVE_KEY, SAVE_SIZE,
    };

    fn active() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.phase = Phase::Active;
        s
    }

    fn full_demo() -> Demo {
        Demo {
            on: true,
            soft_armed: true,
            clock_armed: true,
            lazy: 11,
            clock_min: 777,
            whim_min: 650,
            lazy_min: 300,
            next_min: 29,
            pause_min: 30,
            animated: true,
            pet_action: true,
            demo_min: 3 * 60 + 59,
        }
    }

    #[test]
    fn plan2_save_loads_with_demo_off() {
        let mut b = active().to_bytes();
        for x in &mut b[BLOCK_AT..SAVE_SIZE] {
            *x = 0;
        }
        let s = GameState::from_bytes(&b).expect("loads");
        assert!(!s.demo.on);
        assert_eq!(s.demo, Demo::default());
        assert!(!s.is_demo());
    }

    #[test]
    fn demo_block_round_trips() {
        let mut s = active();
        s.demo = full_demo();
        let back = GameState::from_bytes(&s.to_bytes()).unwrap();
        assert_eq!(
            back.demo,
            Demo {
                demo_min: 180,
                ..full_demo()
            }
        );

        s.demo.demo_min = 40_000 * 60;
        let b = s.to_bytes();
        let d = Demo::read(b[BLOCK_AT..BLOCK_AT + BLOCK_LEN].try_into().unwrap());
        assert_eq!(d.demo_min, 32767 * 60);
        assert!(d.pet_action);
    }

    #[test]
    fn tail_after_demo_block_stays_zero() {
        let mut s = active();
        s.demo = Demo {
            on: true,
            soft_armed: true,
            clock_armed: true,
            lazy: 15,
            clock_min: u16::MAX,
            whim_min: u16::MAX,
            lazy_min: u16::MAX,
            next_min: u8::MAX,
            pause_min: u8::MAX,
            animated: true,
            pet_action: true,
            demo_min: u32::MAX,
        };
        let b = s.to_bytes();
        assert!(b[BLOCK_AT + BLOCK_LEN..].iter().all(|&x| x == 0));
        assert_eq!(b[0], 3);
        assert_eq!(b.len(), 177);
    }

    #[test]
    fn demo_never_writes_old_keys() {
        // Behaviour, not a grep: demo state lives only in bytes 149.., the
        // head is the same with demo on or off, and the key stays state_v3.
        assert_eq!(SAVE_KEY, "state_v3");
        let off = active();
        let mut on = off.clone();
        on.demo = full_demo();
        assert_eq!(on.to_bytes()[..BLOCK_AT], off.to_bytes()[..BLOCK_AT]);
        assert_ne!(on.to_bytes()[BLOCK_AT..], off.to_bytes()[BLOCK_AT..]);
    }

    #[test]
    fn demo_turns_on_only_for_a_pet_in_fair_shape() {
        let mut egg = GameState::new_egg(7, PetKind(0));
        assert_eq!(egg.set_demo(true), Err(Reason::Egg));

        let mut s = active();
        s.health = (39 * P) as u16;
        assert_eq!(s.set_demo(true), Err(Reason::Refused(Refusal::FixMeFirst)));
        let mut l = active();
        l.phase = Phase::Leaving;
        assert_eq!(l.set_demo(true), Err(Reason::Refused(Refusal::FixMeFirst)));

        let mut ok = active();
        ok.health = (60 * P) as u16;
        ok.demo.demo_min = 120;
        assert_eq!(ok.set_demo(true), Ok(()));
        assert!(ok.is_demo());
        assert_eq!(ok.demo.lazy_min, 0);
        assert!(ok.demo.whim_min > 0);
        assert_eq!(ok.set_demo(false), Ok(()));
        assert!(!ok.is_demo());
        assert_eq!(ok.demo.demo_min, 120);
        assert_eq!(ok.demo.whim_min, 0);
    }

    #[test]
    fn set_demo_edges() {
        use bornhack_aegg::game::engine::thresholds::{DEMO_ON_HEALTH_PCT, DEMO_WHIM_BASE_MIN};

        // A running action does not block the toggle.
        let mut busy = active();
        assert!(busy.feed(FoodKind::Apple));
        assert!(busy.active_action.is_some());
        assert_eq!(busy.set_demo(true), Ok(()));

        let mut h = active();
        h.hibernating = true;
        assert_eq!(h.set_demo(true), Err(Reason::Hibernating));
        let mut g = active();
        g.phase = Phase::Gone;
        assert_eq!(g.set_demo(true), Err(Reason::Gone));

        // Exactly the line is fair shape; one level under is not.
        let line = DEMO_ON_HEALTH_PCT() as i32 * P;
        let mut edge = active();
        edge.health = line as u16;
        assert_eq!(edge.set_demo(true), Ok(()));
        assert_eq!(edge.demo.whim_min, DEMO_WHIM_BASE_MIN());
        let mut under = active();
        under.health = (line - 1) as u16;
        assert_eq!(
            under.set_demo(true),
            Err(Reason::Refused(Refusal::FixMeFirst))
        );

        // Care mode and whose action runs survive both ways.
        let mut k = active();
        k.demo.animated = true;
        k.demo.pet_action = true;
        k.demo.soft_armed = true;
        assert_eq!(k.set_demo(true), Ok(()));
        assert!(k.demo.animated && k.demo.pet_action && !k.demo.soft_armed);
        assert_eq!(k.set_demo(false), Ok(()));
        assert!(k.demo.animated && k.demo.pet_action);
    }

    #[test]
    fn demo_work_reward_pays_nothing() {
        use bornhack_aegg::game::engine::{Action, doses};
        let mut o = active();
        o.money_enabled = true;
        o.money = 50;
        doses::apply(&mut o, Action::OnlyPets).unwrap();
        assert!(o.money > 50);

        let mut s = active();
        s.money_enabled = true;
        s.money = 50;
        s.demo.on = true;
        doses::apply(&mut s, Action::OnlyPets).unwrap();
        assert_eq!(s.money, 50);
    }

    #[test]
    fn demo_freezes_hax() {
        let mut s = active();
        s.money_enabled = true;
        s.money = 50;
        s.demo.on = true;
        assert!(s.feed(FoodKind::Pizza));
        assert_eq!(s.money, 50);
        s.award_inspiration(MiniGame::Nim, GRADE_WIN);
        assert_eq!(s.money, 50);
        s.record_battle(true);
        assert_eq!(s.money, 50);
        s.add_money(10);
        assert_eq!(s.money, 50);
        assert!(!s.stats(0).money_enabled);
        assert!(s.stats(0).demo);

        let mut o = active();
        o.money_enabled = true;
        o.money = 50;
        assert!(o.feed(FoodKind::Pizza));
        assert!(o.money < 50);
    }

    #[test]
    fn broke_demo_pet_is_not_told_cant_afford() {
        let mut s = active();
        s.money_enabled = true;
        s.money = 0;
        s.fullness = (10 * P) as u16;
        s.demo.on = true;
        assert_ne!(s.remedy_reason(Remedy::Feed), Some(Reason::CantAfford));
        s.demo.on = false;
        assert_eq!(s.remedy_reason(Remedy::Feed), Some(Reason::CantAfford));
    }

    #[test]
    fn self_raised_needs_a_quarter_of_the_life() {
        let mut s = active();
        s.age_ticks = 400 * 6;
        s.demo.demo_min = 100;
        assert!(!s.self_raised());
        s.demo.demo_min = 101;
        assert!(s.self_raised());
        assert!(s.stats(0).self_raised);
    }

    #[test]
    fn fix_me_first_fits() {
        let r = Reason::Refused(Refusal::FixMeFirst);
        assert_eq!(r.text().as_str(), "fix me first");
        assert!(r.text().len() <= 16);
        let (k, s) = r.pack();
        assert_eq!(Reason::unpack(k, s), r);
    }
}

mod tunables {
    use bornhack_aegg::game::engine::thresholds::{BORNPETS_CFG_KEYS, Thresholds, validate};

    fn with(key: &str, v: u32) -> Thresholds {
        let mut t = Thresholds::CLASSIC;
        let set = BORNPETS_CFG_KEYS.iter().find(|(k, _)| *k == key).unwrap().1;
        set(&mut t, v);
        t
    }

    #[test]
    fn presets_validate_unchanged() {
        assert_eq!(
            validate(&Thresholds::CLASSIC, &Thresholds::CLASSIC),
            Thresholds::CLASSIC
        );
        assert_eq!(
            validate(&Thresholds::CASUAL, &Thresholds::CASUAL),
            Thresholds::CASUAL
        );
    }

    #[test]
    fn huge_values_are_clamped_to_their_saved_size() {
        let t = with("DEMO_LAZY_MAX_PCT", 99);
        assert_eq!(t.DEMO_LAZY_MAX_PCT, 15);
        assert_eq!(with("DEMO_PAUSE_MIN", 9999).DEMO_PAUSE_MIN, 255);
        assert_eq!(with("DEMO_BED_AT_MIN", 5000).DEMO_BED_AT_MIN, 1439);
        assert_eq!(with("DEMO_UP_SPREAD_MIN", 5000).DEMO_UP_SPREAD_MIN, 600);
        let r = validate(&with("DEMO_REACT_BASE_MIN", 60000), &Thresholds::CLASSIC);
        assert!(r.DEMO_REACT_BASE_MIN + r.DEMO_REACT_SPREAD_MIN <= 255);
        let w = validate(&with("DEMO_WHIM_SPREAD_MIN", 65535), &Thresholds::CLASSIC);
        assert!(w.DEMO_WHIM_BASE_MIN as u32 + w.DEMO_WHIM_SPREAD_MIN as u32 <= 65535);
        assert_eq!(with("DEMO_DRY_SPREAD_MIN", 0).DEMO_DRY_SPREAD_MIN, 1);
    }

    #[test]
    fn bad_orders_and_sums_fall_back() {
        let mut t = Thresholds::CLASSIC;
        t.DEMO_PICK_APPLE_PCT = 90;
        t.DEMO_PICK_COLA_PCT = 90;
        t.DEMO_HELP_HEALTH_PCT = 20;
        t.DEMO_RICH_FOOD_PCT = 70;
        assert_eq!(validate(&t, &Thresholds::CLASSIC), Thresholds::CLASSIC);
    }
}

mod floors {
    use bornhack_aegg::game::engine::advance::advance;
    use bornhack_aegg::game::engine::needs::{MAX, P};
    use bornhack_aegg::game::engine::solver;
    use bornhack_aegg::game::engine::thresholds as t;
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase, TICKS_PER_MINUTE};

    const DAY: u32 = 24 * 60;

    fn pct(p: i32) -> u16 {
        (p * P) as u16
    }

    fn calm(demo: bool) -> GameState {
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
        s.demo.on = demo;
        s
    }

    fn starving(demo: bool) -> GameState {
        let mut s = calm(demo);
        s.fullness = 0;
        s.hydration = 0;
        s.health = pct(30);
        s
    }

    fn run(s: &mut GameState, minutes: u32) {
        s.update(s.last_update_tick + minutes * TICKS_PER_MINUTE);
    }

    fn fit_floor() -> u16 {
        pct(t::DEMO_FIT_FLOOR_PCT() as i32)
    }

    #[test]
    fn fitness_floor_stops_the_fall_without_raising() {
        let mut s = calm(true);
        s.fitness = pct(50);
        run(&mut s, 10 * DAY);
        assert_eq!(s.fitness, fit_floor());

        let mut s = calm(true);
        s.fitness = pct(30);
        run(&mut s, DAY);
        assert_eq!(s.fitness, pct(30), "below the floor: not raised, not lower");

        let mut s = calm(false);
        s.fitness = pct(50);
        run(&mut s, 10 * DAY);
        assert!(s.fitness < fit_floor(), "demo off: no floor");
    }

    #[test]
    fn health_floor_stops_damage() {
        let mut s = starving(true);
        run(&mut s, 3 * DAY);
        assert_eq!(s.health, pct(t::DEMO_HEALTH_FLOOR_PCT() as i32));
    }

    #[test]
    fn demo_pet_never_leaves() {
        let mut s = starving(true);
        run(&mut s, 3 * DAY);
        assert_eq!(s.phase, Phase::Active);
        assert_eq!(s.leaving_min, 0);

        let mut s = starving(false);
        run(&mut s, 3 * DAY);
        assert!(matches!(s.phase, Phase::Leaving | Phase::Gone));
    }

    #[test]
    fn demo_never_becomes_dependent() {
        for demo in [true, false] {
            let mut s = calm(demo);
            for _ in 0..(2 * DAY / 30) {
                s.booze = pct(80);
                run(&mut s, 30);
            }
            assert_eq!(s.dependent, !demo, "demo {demo}");
        }
        let mut s = calm(true);
        s.booze = pct(80);
        s.dependence_min = u32::MAX / 2;
        run(&mut s, 60);
        assert!(!s.dependent, "a counter past the line is not onset");
    }

    #[test]
    fn demo_time_is_not_care_time() {
        let mut s = calm(true);
        s.care_awake_min = 5;
        s.care_good_min = 3;
        let before = s.demo.demo_min;
        run(&mut s, 600);
        assert_eq!((s.care_awake_min, s.care_good_min), (5, 3));
        assert_eq!(s.demo.demo_min, before + 600);

        let mut s = calm(false);
        run(&mut s, 600);
        assert!(s.care_awake_min > 0);
        assert_eq!(s.demo.demo_min, 0);
    }

    #[test]
    fn floors_are_segment_lines() {
        let mut s = calm(true);
        s.fitness = pct(46);
        let len = solver::seg_len(&s, 100_000);
        let mut oracle = s.clone();
        let mut to_floor = 0;
        while oracle.fitness > fit_floor() {
            advance(&mut oracle, 1);
            to_floor += 1;
        }
        assert!(to_floor > 2, "the fall is slow enough to test");
        assert!(len <= to_floor - 1, "stops before the floor minute");
        let mut seg = s.clone();
        advance(&mut seg, len);
        assert!(seg.fitness > fit_floor());
    }

    #[test]
    fn health_floor_is_a_sig_line() {
        let mut a = starving(true);
        let mut b = a.clone();
        a.health = pct(t::DEMO_HEALTH_FLOOR_PCT() as i32) + 1;
        b.health = pct(t::DEMO_HEALTH_FLOOR_PCT() as i32);
        assert_ne!(solver::sig(&a), solver::sig(&b));
        a.health = pct(t::DEMO_HELP_HEALTH_PCT() as i32) + 1;
        b.health = pct(t::DEMO_HELP_HEALTH_PCT() as i32);
        assert_ne!(solver::sig(&a), solver::sig(&b));
    }

    #[test]
    fn pinned_on_the_floor_takes_long_segments() {
        // Fitness is the only need pinned; the others move, and bound the
        // segment at 40 minutes. Without the skip a pet resting on the floor
        // gets 1-minute segments (the floor line is "already reached").
        let mut s = calm(true);
        s.booze = 0;
        s.fitness = fit_floor();
        let pinned = solver::seg_len(&s, 100_000);
        assert!(pinned > 30, "pinned segment {pinned}");
        s.fitness = fit_floor() + pct(1);
        assert!(solver::seg_len(&s, 100_000) > 30);
    }

    #[test]
    fn floors_hold_across_splits() {
        for start in [starving(true), calm(true), {
            let mut s = calm(true);
            s.fitness = pct(47);
            s
        }] {
            let mut whole = start.clone();
            run(&mut whole, 3 * DAY);
            let mut steps = start.clone();
            for _ in 0..(3 * DAY / 37) {
                run(&mut steps, 37);
            }
            run(&mut steps, 3 * DAY - 37 * (3 * DAY / 37));
            let mut single = start.clone();
            for _ in 0..(6 * 60) {
                run(&mut single, 1);
            }
            let mut whole6 = start.clone();
            run(&mut whole6, 6 * 60);
            assert_eq!(
                whole.to_bytes(),
                steps.to_bytes(),
                "3 days, 37-minute steps"
            );
            assert_eq!(
                whole6.to_bytes(),
                single.to_bytes(),
                "6 hours, 1-minute steps"
            );
            assert!(whole.fitness >= fit_floor().min(start.fitness));
            assert!(whole.health >= pct(t::DEMO_HEALTH_FLOOR_PCT() as i32).min(start.health));
        }
    }

    #[test]
    fn relax_dose_does_not_break_the_floor() {
        for (demo, holds) in [(true, true), (false, false)] {
            let mut s = calm(demo);
            s.fitness = fit_floor();
            assert!(s.relax());
            run(&mut s, 30);
            assert_eq!(s.fitness >= fit_floor(), holds, "demo {demo}");
        }
    }
}
