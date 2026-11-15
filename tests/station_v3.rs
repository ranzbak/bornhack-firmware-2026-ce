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
