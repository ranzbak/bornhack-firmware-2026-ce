//! Runtime pet roster.
//!
//! The roster comes entirely from the `PETS.CFG` manifest on the USB-MSC
//! partition — there are no built-in pets.  See [`crate::game::pets_cfg`].
//! A badge whose `PETS.CFG` is missing, empty or unreadable has an empty
//! roster and no pet can be selected until the file is restored.
//!
//! A pet is a pure cosmetic skin: a sprite-prefix byte plus a display name.
//! All game behaviour (stats, decay, traits, animations) is identical across
//! pets, so a pet needs only its `PPAAFF.PCX` sprites at the matching prefix
//! and one manifest line.  Frame counts are auto-discovered from the FAT
//! directory catalogue, so no per-pet code is required.

use core::sync::atomic::{AtomicBool, Ordering};

use super::engine::PetKind;

/// Maximum total pets in the roster (3 built-in + custom).
pub const MAX_PETS: usize = 12;
/// Maximum display-name length (bytes).
pub const NAME_CAP: usize = 16;
/// Sprite prefixes reserved for non-pet assets: `3` sponsor slideshow,
/// `4` menu icons.  Everything else `< MAX_PET_PREFIX` is a pet.
pub const RESERVED_PREFIXES: [u8; 2] = [3, 4];

/// Exclusive upper bound on a pet's sprite prefix.  The shipped roster uses
/// `0..=2` and `5..=8`; `3`/`4` are reserved.  Must stay in sync with
/// `sprite_loader::PP_MAX` so the frame catalogue covers every pet.  Kept
/// small deliberately — every extra prefix grows the catalogue table, and
/// the debug build is RAM-tight.
pub const MAX_PET_PREFIX: u8 = 9;

/// Whether `id` is usable as a pet's sprite prefix (in range, not reserved).
pub fn is_pet_prefix(id: u8) -> bool {
    id < MAX_PET_PREFIX && !RESERVED_PREFIXES.contains(&id)
}

/// One roster entry: sprite-prefix byte + display name.
#[derive(Clone)]
pub struct PetDef {
    pub id: u8,
    pub name: heapless::String<NAME_CAP>,
}

struct Registry {
    defs: heapless::Vec<PetDef, MAX_PETS>,
    ids: heapless::Vec<PetKind, MAX_PETS>,
}

/// Shown for an id that isn't in the roster — a save whose pet was dropped
/// from `PETS.CFG`, or any lookup before the manifest has been installed.
const UNKNOWN_NAME: &str = "Pet";

// Single global roster, populated once at boot.  `heapless` collections are
// const-constructible, so this needs no allocator or `StaticCell` (keeping
// the module dependency-free for the simulator build).  Mirrors the
// install-once pattern in `engine::thresholds`.
static mut REGISTRY: Registry = Registry {
    defs: heapless::Vec::new(),
    ids: heapless::Vec::new(),
};
static INSTALLED: AtomicBool = AtomicBool::new(false);

fn current() -> Option<&'static Registry> {
    if INSTALLED.load(Ordering::Acquire) {
        // SAFETY: `REGISTRY` is written exactly once inside `install` (before
        // the `INSTALLED` Release store) and only ever read afterwards.  Boot
        // is single-threaded, so no concurrent access occurs.
        Some(unsafe { &*core::ptr::addr_of!(REGISTRY) })
    } else {
        None
    }
}

/// Install the roster from the manifest `entries`, in file order.  Reserved /
/// out-of-range ids, duplicates and overflow past [`MAX_PETS`] are dropped; a
/// repeated id renames the earlier entry rather than listing the pet twice.
///
/// Call once at boot (see [`crate::game::pets_cfg::load_and_install`]).  A
/// second call is ignored.  There is no built-in roster: installing an empty
/// slice — which is what a missing or unreadable `PETS.CFG` produces — leaves
/// the badge with no selectable pets.
pub fn install(entries: &[PetDef]) {
    if INSTALLED.load(Ordering::Acquire) {
        return; // already installed — ignore
    }

    // SAFETY: exclusive access — install runs once at boot before any reader,
    // and the `INSTALLED` guard above prevents a second populate.
    let reg = unsafe { &mut *core::ptr::addr_of_mut!(REGISTRY) };

    for e in entries {
        if !is_pet_prefix(e.id) {
            continue;
        }
        if let Some(d) = reg.defs.iter_mut().find(|d| d.id == e.id) {
            d.name = e.name.clone(); // duplicate id — last name wins
            continue;
        }
        if reg.defs.push(e.clone()).is_err() {
            break; // roster full
        }
        let _ = reg.ids.push(PetKind(e.id));
    }

    INSTALLED.store(true, Ordering::Release);
}

/// Display name for a pet id, or [`UNKNOWN_NAME`] when the id isn't in the
/// installed roster (or nothing has been installed yet).
pub fn name_of(id: u8) -> &'static str {
    if let Some(reg) = current() {
        for d in &reg.defs {
            if d.id == id {
                return d.name.as_str();
            }
        }
    }
    UNKNOWN_NAME
}

/// Selectable pet kinds, in `PETS.CFG` order.  Empty before an install and
/// when the manifest declared no usable pet.
pub fn roster() -> &'static [PetKind] {
    match current() {
        Some(reg) => reg.ids.as_slice(),
        None => &[],
    }
}

/// Whether `id` resolves to a pet that actually exists in the roster.
pub fn is_known(id: u8) -> bool {
    roster().iter().any(|k| k.0 == id)
}
