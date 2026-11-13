//! Golden-bytes tests for the persisted / on-air record encodings.
//!
//! These pin the exact byte layout of everything the firmware writes to flash
//! (game state, Unicorn Realm, friends list) or sends over the mesh (pet
//! beacon, battle result). The literals were captured from the encoders at
//! main 978eabf; they must never change, or saves written by an older
//! firmware stop loading.

use bornhack_aegg::game::battle::{BattleResultMsg, KIND_UNKNOWN};
use bornhack_aegg::game::engine::{
    Action, GameState, PET_RECORD_SIZE, PetKind, PetRealm, PetRecord, Phase, REALM_MAX_PETS,
    REALM_SAVE_SIZE, SAVE_FORMAT_VERSION, SAVE_SIZE,
};
use bornhack_aegg::game::friends::{
    FRIENDS_MAX, FRIENDS_SAVE_SIZE, FriendRecord, FriendsList, PetBeacon,
};

// ---------------------------------------------------------------------------
// GameState
// ---------------------------------------------------------------------------

const GAME_GOLDEN: [u8; 97] = [
    0x02, 0x70, 0x11, 0x73, 0x12, 0x76, 0x13, 0x79, 0x14, 0x7C, 0x15, 0x7F,
    0x16, 0x82, 0x17, 0x85, 0x18, 0x88, 0x19, 0x8C, 0x6A, 0x49, 0x2A, 0x8D,
    0x6B, 0x4A, 0x2B, 0x01, 0x94, 0x1D, 0x90, 0x6E, 0x4D, 0x2E, 0x9A, 0x1F,
    0x07, 0x41, 0xA3, 0x22, 0xA6, 0x23, 0xA9, 0x24, 0xAC, 0x25, 0xAF, 0x26,
    0xB2, 0x27, 0xB5, 0x28, 0xB8, 0x29, 0xBB, 0x2A, 0x9D, 0x7B, 0x5A, 0x3B,
    0x9E, 0x7C, 0x5B, 0x3C, 0x9F, 0x7D, 0x5C, 0x3D, 0xA0, 0x7E, 0x5D, 0x3E,
    0x01, 0x00, 0x01, 0x00, 0xA5, 0x83, 0x62, 0x43, 0xA6, 0x84, 0x63, 0x44,
    0x05, 0xDF, 0x36, 0xE2, 0x37, 0xE5, 0x38, 0xAB, 0x89, 0x68, 0x49, 0x01,
    0x01,
];

// Offsets into the save used by the normalisation test below.
const PHASE_OFS: usize = 1 + 18 + 8;
const ACTION_OFS: usize = PHASE_OFS + 1 + 2 + 4 + 2;

#[test]
#[ignore = "v1 rules; replaced by BornPets v2 plan 1 task 11"]
fn game_state_golden_bytes_round_trip() {
    assert_eq!(SAVE_SIZE, 97);
    assert_eq!(SAVE_FORMAT_VERSION, 2);
    let s = GameState::from_bytes(&GAME_GOLDEN).expect("golden save must decode");

    assert_eq!(s.fullness, 0x1170);
    assert_eq!(s.energy, 0x1273);
    assert_eq!(s.health, 0x1376);
    assert_eq!(s.mood, 0x1479);
    assert_eq!(s.weight, 0x157C);
    assert_eq!(s.booze, 0x167F);
    assert_eq!(s.vitality, 0x1782);
    assert_eq!(s.curiosity, 0x1885);
    assert_eq!(s.resilience, 0x1988);
    assert_eq!(s.last_update_tick, 0x2A49_6A8C);
    assert_eq!(s.age_ticks, 0x2B4A_6B8D);
    assert_eq!(s.phase, Phase::Active);
    assert_eq!(s.hatching_countdown, 0x1D94);
    assert_eq!(s.leaving_min, 0x2E4D_6E90);
    assert_eq!(s.generation, 0x1F9A);
    assert_eq!(s.active_action, Some(Action::Drink));
    assert_eq!(s.action_ticks_remaining, 0x41);
    // v1 field removed in v3: cooldown_feed
    // v1 field removed in v3: cooldown_heal
    // v1 field removed in v3: cooldown_relax
    // v1 field removed in v3: cooldown_play
    // v1 field removed in v3: cooldown_exercise
    // v1 field removed in v3: cooldown_medicate
    // v1 field removed in v3: cooldown_ozempic
    // v1 field removed in v3: cooldown_drink
    // v1 field removed in v3: cooldown_rehab
    assert!(s.is_sleeping);
    assert!(!s.hibernating);
    assert!(s.diabetic);
    assert!(!s.dependent);
    assert_eq!(s.hibernate_ticks, 0x4362_83A5);
    assert_eq!(s.pet_kind, PetKind::from_u8(5));
    assert_eq!(s.wins, 0x36DF);
    assert_eq!(s.losses, 0x37E2);
    assert_eq!(s.cooldown_battle, 0x38E5);
    assert_eq!(s.money, 0x4968_89AB);
    assert!(s.money_enabled);
    assert!(s.hard_mode);

    // Re-encoding reproduces every byte, including the private counters
    // (miserable_interval/tired_passive/overweight/drunk ticks, last_save_tick).
    assert_eq!(s.to_bytes(), GAME_GOLDEN);
}

#[test]
#[ignore = "v1 rules; replaced by BornPets v2 plan 1 task 11"]
fn game_state_rejects_wrong_version_and_length() {
    let mut bad = GAME_GOLDEN;
    bad[0] = SAVE_FORMAT_VERSION + 1;
    assert!(GameState::from_bytes(&bad).is_none());
    bad[0] = 1;
    assert!(GameState::from_bytes(&bad).is_none());
    bad[0] = 0;
    assert!(GameState::from_bytes(&bad).is_none());

    assert!(GameState::from_bytes(&[]).is_none());
    assert!(GameState::from_bytes(&GAME_GOLDEN[..SAVE_SIZE - 1]).is_none());
    let mut long = GAME_GOLDEN.to_vec();
    long.push(0);
    assert!(GameState::from_bytes(&long).is_none());
}

#[test]
#[ignore = "v1 rules; replaced by BornPets v2 plan 1 task 11"]
fn game_state_decode_normalises_out_of_range_bytes() {
    assert_eq!(GAME_GOLDEN[PHASE_OFS], 1);
    assert_eq!(GAME_GOLDEN[ACTION_OFS], 7);
    let flags_ofs = ACTION_OFS + 2 + 18 + 16;

    let mut b = GAME_GOLDEN;
    b[PHASE_OFS] = 0x77; // unknown phase -> Gone
    b[ACTION_OFS] = 2; // formerly Relax -> no action
    b[flags_ofs] = 0x7F; // is_sleeping: any non-zero is true
    b[flags_ofs + 1] = 0x02; // hibernating
    let s = GameState::from_bytes(&b).unwrap();
    assert_eq!(s.phase, Phase::Gone);
    assert_eq!(s.active_action, None);
    assert!(s.is_sleeping);
    assert!(s.hibernating);

    // None is written back as 0xFF.
    let mut b = GAME_GOLDEN;
    b[ACTION_OFS] = 0xFF;
    let s = GameState::from_bytes(&b).unwrap();
    assert_eq!(s.active_action, None);
    assert_eq!(s.to_bytes()[ACTION_OFS], 0xFF);

    // Phases 0 and 2 decode to Hatching / Leaving and re-encode identically.
    for (byte, phase) in [(0u8, Phase::Hatching), (2, Phase::Leaving)] {
        let mut b = GAME_GOLDEN;
        b[PHASE_OFS] = byte;
        let s = GameState::from_bytes(&b).unwrap();
        assert_eq!(s.phase, phase);
        assert_eq!(s.to_bytes()[PHASE_OFS], byte);
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn padded<const N: usize>(prefix: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    out[..prefix.len()].copy_from_slice(prefix);
    out
}

fn name12(s: &str) -> [u8; 12] {
    let mut n = [0u8; 12];
    n[..s.len()].copy_from_slice(s.as_bytes());
    n
}

// ---------------------------------------------------------------------------
// Unicorn Realm
// ---------------------------------------------------------------------------

// Three records; the remaining seven slots are all-zero.
const REALM_PREFIX: [u8; 79] = [
    0x03, 0x02, 0x01, 0x0D, 0x0C, 0x0B, 0x0A, 0x12, 0x11, 0x22, 0x21, 0x32, 0x31,
    0x07, 0x5A, 0x6F, 0x72, 0x70, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x04, 0x04, 0x03, 0x1D, 0x1C, 0x1B, 0x1A, 0x42, 0x41, 0x52, 0x51, 0x62, 0x61,
    0x09, 0x42, 0x6F, 0x62, 0x20, 0x74, 0x68, 0x65, 0x20, 0x50, 0x65, 0x74, 0x00,
    0x0B, 0x06, 0x05, 0x2D, 0x2C, 0x2B, 0x2A, 0x72, 0x71, 0x82, 0x81, 0x92, 0x91,
    0x03, 0x51, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01,
];

fn pet_record(
    generation: u16,
    age_ticks: u32,
    (vitality, curiosity, resilience): (u16, u16, u16),
    kind: u8,
    name: &str,
) -> PetRecord {
    PetRecord {
        generation,
        age_ticks,
        vitality,
        curiosity,
        resilience,
        pet_kind: PetKind::from_u8(kind),
        name: name12(name),
        name_len: name.len() as u8,
    }
}

fn realm_value() -> PetRealm {
    let mut r = PetRealm::new();
    r.pets[0] = pet_record(0x0102, 0x0A0B_0C0D, (0x1112, 0x2122, 0x3132), 7, "Zorp");
    r.pets[1] = pet_record(0x0304, 0x1A1B_1C1D, (0x4142, 0x5152, 0x6162), 9, "Bob the Pet");
    r.pets[2] = pet_record(0x0506, 0x2A2B_2C2D, (0x7172, 0x8182, 0x9192), 3, "Q");
    r.count = 3;
    r
}

#[test]
fn realm_golden_bytes_round_trip() {
    assert_eq!(PET_RECORD_SIZE, 26);
    assert_eq!(REALM_SAVE_SIZE, 261);
    let golden: [u8; REALM_SAVE_SIZE] = padded(&REALM_PREFIX);
    assert_eq!(realm_value().to_bytes(), golden);

    let r = PetRealm::from_bytes(&golden);
    assert_eq!(r.count, 3);
    let want = realm_value();
    for i in 0..3 {
        let (a, b) = (&r.pets[i], &want.pets[i]);
        assert_eq!(a.generation, b.generation);
        assert_eq!(a.age_ticks, b.age_ticks);
        assert_eq!(a.vitality, b.vitality);
        assert_eq!(a.curiosity, b.curiosity);
        assert_eq!(a.resilience, b.resilience);
        assert_eq!(a.pet_kind, b.pet_kind);
        assert_eq!(a.name, b.name);
        assert_eq!(a.name_len, b.name_len);
    }
    assert_eq!(r.to_bytes(), golden);
}

#[test]
fn realm_short_and_oversized_input() {
    // Empty -> fresh realm.
    assert_eq!(PetRealm::from_bytes(&[]).count, 0);
    // Count is clamped to REALM_MAX_PETS.
    let mut b: [u8; REALM_SAVE_SIZE] = padded(&REALM_PREFIX);
    b[0] = 200;
    assert_eq!(PetRealm::from_bytes(&b).count as usize, REALM_MAX_PETS);
    // A truncated buffer keeps the count, but a record that does not fit
    // completely stays at its default (zero) value; complete ones still decode.
    let cut = &REALM_PREFIX[..1 + 2 * PET_RECORD_SIZE + 5];
    let r = PetRealm::from_bytes(cut);
    assert_eq!(r.count, 3);
    assert_eq!(r.pets[1].generation, 0x0304);
    assert_eq!(r.pets[2].generation, 0);
    assert_eq!(r.pets[2].name_len, 0);
}

// ---------------------------------------------------------------------------
// Friends
// ---------------------------------------------------------------------------

// Two records; the remaining eighteen slots are all-zero.
const FRIENDS_PREFIX: [u8; 73] = [
    0x02, 0xA1, 0xB2, 0x04, 0x52, 0x65, 0x78, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x03, 0x04, 0x03, 0x02, 0x01, 0x08, 0x07, 0x06,
    0x05, 0x0C, 0x0B, 0x0A, 0x09, 0x29, 0x20, 0x17, 0x72, 0x02, 0x01, 0x04,
    0x03, 0xC3, 0xD4, 0x08, 0x46, 0x6C, 0x75, 0x66, 0x66, 0x79, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x06, 0x14, 0x13, 0x12, 0x11, 0x18, 0x17, 0x16,
    0x15, 0x1C, 0x1B, 0x1A, 0x19, 0x37, 0x42, 0x4D, 0x58, 0x12, 0x11, 0x14,
    0x13,
];

fn friend(
    device_id: [u8; 2],
    pet_kind: u8,
    name: &str,
    ticks: [u32; 3],
    combat: [u8; 4],
    wl: [u16; 2],
) -> FriendRecord {
    FriendRecord {
        device_id,
        pet_kind,
        name: name12(name),
        name_len: name.len() as u8,
        first_seen_tick: ticks[0],
        last_boost_tick: ticks[1],
        last_seen_tick: ticks[2],
        attack: combat[0],
        defense: combat[1],
        speed: combat[2],
        max_hp: combat[3],
        wins: wl[0],
        losses: wl[1],
    }
}

fn friends_value() -> FriendsList {
    let mut f = FriendsList::new();
    f.friends[0] = friend(
        [0xA1, 0xB2],
        4,
        "Rex",
        [0x0102_0304, 0x0506_0708, 0x090A_0B0C],
        [41, 32, 23, 114],
        [0x0102, 0x0304],
    );
    f.friends[1] = friend(
        [0xC3, 0xD4],
        8,
        "Fluffy",
        [0x1112_1314, 0x1516_1718, 0x191A_1B1C],
        [55, 66, 77, 88],
        [0x1112, 0x1314],
    );
    f.count = 2;
    f
}

#[test]
fn friends_golden_bytes_round_trip() {
    assert_eq!(FRIENDS_SAVE_SIZE, 721);
    let golden: [u8; FRIENDS_SAVE_SIZE] = padded(&FRIENDS_PREFIX);
    assert_eq!(friends_value().to_bytes(), golden);

    let f = FriendsList::from_bytes(&golden);
    assert_eq!(f.count, 2);
    let want = friends_value();
    for i in 0..2 {
        let (a, b) = (&f.friends[i], &want.friends[i]);
        assert_eq!(a.device_id, b.device_id);
        assert_eq!(a.pet_kind, b.pet_kind);
        assert_eq!(a.name, b.name);
        assert_eq!(a.name_len, b.name_len);
        assert_eq!(a.first_seen_tick, b.first_seen_tick);
        assert_eq!(a.last_boost_tick, b.last_boost_tick);
        assert_eq!(a.last_seen_tick, b.last_seen_tick);
        assert_eq!(a.attack, b.attack);
        assert_eq!(a.defense, b.defense);
        assert_eq!(a.speed, b.speed);
        assert_eq!(a.max_hp, b.max_hp);
        assert_eq!(a.wins, b.wins);
        assert_eq!(a.losses, b.losses);
    }
    assert_eq!(f.to_bytes(), golden);
}

#[test]
fn friends_short_and_oversized_input() {
    assert_eq!(FriendsList::from_bytes(&[]).count, 0);
    let mut b: [u8; FRIENDS_SAVE_SIZE] = padded(&FRIENDS_PREFIX);
    b[0] = 250;
    assert_eq!(FriendsList::from_bytes(&b).count as usize, FRIENDS_MAX);
    // Truncated mid second record: first decodes, second stays default.
    let f = FriendsList::from_bytes(&FRIENDS_PREFIX[..1 + 36 + 10]);
    assert_eq!(f.count, 2);
    assert_eq!(f.friends[0].device_id, [0xA1, 0xB2]);
    assert_eq!(f.friends[1].device_id, [0, 0]);
    assert_eq!(f.friends[1].wins, 0);
}

const BEACON_GOLDEN: [u8; 22] = [
    0xE5, 0xF6, 0x06, 0x0B, 0x0A, 0x50, 0x69, 0x78, 0x65, 0x6C, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x31, 0x42, 0x53, 0x64,
];

#[test]
fn pet_beacon_golden_bytes_round_trip() {
    let beacon = PetBeacon {
        device_id: [0xE5, 0xF6],
        pet_kind: 6,
        generation: 0x0A0B,
        name: name12("Pixel"),
        name_len: 5,
        attack: 0x31,
        defense: 0x42,
        speed: 0x53,
        max_hp: 0x64,
    };
    assert_eq!(beacon.to_bytes(), BEACON_GOLDEN);

    let b = PetBeacon::from_bytes(&BEACON_GOLDEN).unwrap();
    assert_eq!(b.device_id, [0xE5, 0xF6]);
    assert_eq!(b.pet_kind, 6);
    assert_eq!(b.generation, 0x0A0B);
    assert_eq!(b.name, name12("Pixel"));
    assert_eq!(b.name_len, 5);
    assert_eq!((b.attack, b.defense, b.speed, b.max_hp), (0x31, 0x42, 0x53, 0x64));
    assert_eq!(b.to_bytes(), BEACON_GOLDEN);

    // Too short -> rejected; longer is tolerated (trailing bytes ignored).
    assert!(PetBeacon::from_bytes(&BEACON_GOLDEN[..21]).is_none());
    assert!(PetBeacon::from_bytes(&[]).is_none());
    let mut long = BEACON_GOLDEN.to_vec();
    long.extend_from_slice(&[0xEE, 0xEE]);
    assert_eq!(PetBeacon::from_bytes(&long).unwrap().to_bytes(), BEACON_GOLDEN);
}

// ---------------------------------------------------------------------------
// Battle result (on-air)
// ---------------------------------------------------------------------------

const BATTLE_GOLDEN: [u8; 9] = [0xAB, 0xCD, 0x12, 0x34, 0x01, 0x57, 0x09, 0x05, 0x0C];

#[test]
fn battle_result_golden_bytes_round_trip() {
    let msg = BattleResultMsg {
        challenger_id: [0xAB, 0xCD],
        target_id: [0x12, 0x34],
        challenger_won: true,
        challenger_hp_pct: 0x57,
        target_hp_pct: 0x09,
        challenger_kind: 5,
        target_kind: 0x0C,
    };
    assert_eq!(msg.to_bytes(), BATTLE_GOLDEN);

    let back = BattleResultMsg::from_bytes(&BATTLE_GOLDEN).unwrap();
    assert_eq!(back.challenger_id, [0xAB, 0xCD]);
    assert_eq!(back.target_id, [0x12, 0x34]);
    assert!(back.challenger_won);
    assert_eq!(back.challenger_hp_pct, 0x57);
    assert_eq!(back.target_hp_pct, 0x09);
    assert_eq!(back.challenger_kind, 5);
    assert_eq!(back.target_kind, 0x0C);
    assert_eq!(back.to_bytes(), BATTLE_GOLDEN);
}

#[test]
fn battle_result_short_packet_tolerance() {
    // Legacy 7-byte packet: species unknown. 8 bytes is still "short".
    for len in [7usize, 8] {
        let back = BattleResultMsg::from_bytes(&BATTLE_GOLDEN[..len]).unwrap();
        assert_eq!(back.challenger_id, [0xAB, 0xCD]);
        assert_eq!(back.target_hp_pct, 0x09);
        assert_eq!(back.challenger_kind, KIND_UNKNOWN);
        assert_eq!(back.target_kind, KIND_UNKNOWN);
    }
    for len in [0usize, 1, 6] {
        assert!(BattleResultMsg::from_bytes(&BATTLE_GOLDEN[..len]).is_none());
    }
    // Trailing bytes ignored; any non-zero "won" byte means true.
    let mut long = BATTLE_GOLDEN.to_vec();
    long[4] = 0x02;
    long.push(0x99);
    let back = BattleResultMsg::from_bytes(&long).unwrap();
    assert!(back.challenger_won);
    assert_eq!(back.target_kind, 0x0C);
    long[4] = 0;
    assert!(!BattleResultMsg::from_bytes(&long).unwrap().challenger_won);
}
