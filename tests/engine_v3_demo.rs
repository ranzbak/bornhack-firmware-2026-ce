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
