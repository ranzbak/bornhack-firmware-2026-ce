//! Read `PETS.CFG` from the USB-MSC FAT12 partition and register the pets it
//! declares.  This file is the *only* source of the roster — no pet is baked
//! into the firmware, so pets are added, renamed and removed without a
//! reflash, and a badge with no `PETS.CFG` has no selectable pet at all.
//!
//! Format — one `PREFIX=NAME` per line, `#` introduces a comment.  `PREFIX`
//! is the pet's sprite-prefix byte in decimal; `3` and `4` are reserved
//! (sponsors, menu icons) and ignored, anything else below
//! [`pet_registry::MAX_PET_PREFIX`] is a pet slot.  The pet's sprites are the
//! matching `PPAAFF.PCX` files (e.g. `050100.PCX` for pet 5's idle-frame 0).
//! Names longer than [`NAME_CAP`] chars are truncated.  Rows are listed in
//! selection-screen order.
//!
//! ```text
//! # the shipped roster — each line needs matching PPxxxx.PCX sprites
//! 0=Bartholomeus
//! 1=Cat
//! 5=Panda
//! ```
//!
//! Call [`load_and_install`] exactly once at boot, after
//! `fat12::format_if_needed()` and before the game reads the roster.

use crate::game::pet_registry::{self, MAX_PETS, NAME_CAP, PetDef};

/// Only the FAT12 read path needs this; host builds (simulator, unit tests)
/// pull in just [`parse`].
#[cfg(feature = "embassy-core")]
const READ_BUF_LEN: usize = 1024;

/// Load `PETS.CFG` (if present) and install the resulting roster.  Always
/// calls [`pet_registry::install`], so the registry is marked installed even
/// when the file is missing or unreadable — in that case with an empty list,
/// leaving the badge with no selectable pet.
#[cfg(feature = "embassy-core")]
pub async fn load_and_install() {
    use crate::fw::fat12;

    let mut pets: heapless::Vec<PetDef, MAX_PETS> = heapless::Vec::new();

    if let Some(name) = fat12::to_8_3("PETS.CFG")
        && let Ok(file) = fat12::find_file(&name).await
    {
        let mut buf = [0u8; READ_BUF_LEN];
        if let Ok(n) = fat12::read_file(&file, 0, &mut buf).await {
            let parsed = parse(&buf[..n], &mut pets);
            defmt::info!("PETS.CFG: {} pet(s) parsed", parsed);
        }
    }

    if pets.is_empty() {
        defmt::warn!("PETS.CFG: no pets declared — none will be selectable");
    }
    pet_registry::install(&pets);
}

/// Parse `data` as one `PREFIX=NAME` per line into `out`.  Returns the number
/// of rows accepted.  Reserved prefixes and malformed rows are skipped;
/// over-long names are truncated at [`NAME_CAP`].  Visible for host tests.
pub fn parse(data: &[u8], out: &mut heapless::Vec<PetDef, MAX_PETS>) -> u32 {
    let mut added = 0u32;

    for (key, value) in crate::cfg_text::kv_lines(data, true) {
        let Some(id) = crate::cfg_text::parse_dec_u32(key).and_then(|v| u8::try_from(v).ok()) else {
            continue;
        };
        if !pet_registry::is_pet_prefix(id) {
            continue; // reserved (3 sponsors, 4 menu) or out of range
        }

        let mut name: heapless::String<NAME_CAP> = heapless::String::new();
        for &b in value {
            // ASCII only; drop anything else so names stay renderable.
            if !b.is_ascii() || name.push(b as char).is_err() {
                break; // truncate at cap / first non-ascii
            }
        }
        if name.is_empty() {
            continue;
        }

        if out.push(PetDef { id, name }).is_err() {
            break; // roster full
        }
        added += 1;
    }

    added
}

#[cfg(all(test, not(feature = "embassy-core")))]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_rows_and_skips_reserved() {
        let mut out = heapless::Vec::new();
        // 3 is reserved (sponsors); 5 and 6 are valid.
        let n = parse(b"# pets\n3=Nope\n5=Dragon\n6 = Ghost \n", &mut out);
        assert_eq!(n, 2);
        assert_eq!(out[0].id, 5);
        assert_eq!(out[0].name.as_str(), "Dragon");
        assert_eq!(out[1].id, 6);
        assert_eq!(out[1].name.as_str(), "Ghost");
    }

    #[test]
    fn truncates_long_names_and_ignores_malformed() {
        let mut out = heapless::Vec::new();
        let n = parse(
            b"5=ThisNameIsWayTooLongToFit\n=noident\n7=\nxx=bad\n",
            &mut out,
        );
        assert_eq!(n, 1);
        assert_eq!(out[0].name.as_str().len(), NAME_CAP);
    }
}
