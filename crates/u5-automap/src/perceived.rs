//! The perceived rule for dungeon cells (design 4.2.3).
//!
//! A remembered cell is drawn as the first-person view painted it, never
//! as the data says it is (systems/dungeon-mode.md §6.4, §6.6). This is
//! the spoiler guard of both maps: a secret door (`0xD?`) is drawn as the
//! wall the player saw, and the hidden pit `0x61`, its rising form `0x69`
//! and the bomb trap `0x62` are drawn as the passage the player saw.
//!
//! | Raw cell | Drawn as |
//! |---|---|
//! | `0xB?`, `0xD?` | wall |
//! | `0xC?` | flavour wall |
//! | `0xA?`, `0xE?`, `0xF?` | door |
//! | `0x1?` / `0x2?` / `0x3?` | ladder up / down / both |
//! | `0x4?` / `0x7?` | chest / open chest |
//! | `0x5?` | fountain |
//! | `0x6?`, low 3 bits 0 | pit |
//! | other `0x6?` | passage |
//! | `0x8?` | energy field |
//! | `0x0?`, `0x9?` | passage |
//!
//! A raw byte below class 9 with bit `0x08` set also carries the small
//! rising-marker badge, because the view paints a rising overlay there.
//! Bit `0x08` is never exploration state (formats/dungeon-dat.md §3).

/// What a dungeon cell looks like on the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DungeonLook {
    Passage,
    Wall,
    FlavourWall,
    Door,
    LadderUp,
    LadderDown,
    LadderBoth,
    Chest,
    OpenChest,
    Fountain,
    Pit,
    Field,
}

/// One dungeon cell as the player perceived it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PerceivedCell {
    pub look: DungeonLook,
    /// The view paints a rising overlay on this cell (raw class below 9
    /// with bit `0x08`).
    pub rising_marker: bool,
}

/// The perceived look of a raw dungeon byte (design 4.2.3).
pub const fn perceived_dungeon_cell(raw: u8) -> PerceivedCell {
    let class = raw >> 4;
    let look = match class {
        0x0 | 0x9 => DungeonLook::Passage,
        0x1 => DungeonLook::LadderUp,
        0x2 => DungeonLook::LadderDown,
        0x3 => DungeonLook::LadderBoth,
        0x4 => DungeonLook::Chest,
        0x5 => DungeonLook::Fountain,
        0x6 => {
            if raw & 0x07 == 0 {
                DungeonLook::Pit
            } else {
                DungeonLook::Passage
            }
        }
        0x7 => DungeonLook::OpenChest,
        0x8 => DungeonLook::Field,
        0xB | 0xD => DungeonLook::Wall,
        0xC => DungeonLook::FlavourWall,
        // 0xA, 0xE, 0xF.
        _ => DungeonLook::Door,
    };
    PerceivedCell {
        look,
        rising_marker: class < 9 && raw & 0x08 != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 4.2.3 table, written out independently of the match above.
    fn table_look(raw: u8) -> DungeonLook {
        let hi = raw & 0xF0;
        let lo3 = raw & 0x07;
        if hi == 0xB0 || hi == 0xD0 {
            DungeonLook::Wall
        } else if hi == 0xC0 {
            DungeonLook::FlavourWall
        } else if hi == 0xA0 || hi == 0xE0 || hi == 0xF0 {
            DungeonLook::Door
        } else if hi == 0x10 {
            DungeonLook::LadderUp
        } else if hi == 0x20 {
            DungeonLook::LadderDown
        } else if hi == 0x30 {
            DungeonLook::LadderBoth
        } else if hi == 0x40 {
            DungeonLook::Chest
        } else if hi == 0x70 {
            DungeonLook::OpenChest
        } else if hi == 0x50 {
            DungeonLook::Fountain
        } else if hi == 0x60 && lo3 == 0 {
            DungeonLook::Pit
        } else if hi == 0x80 {
            DungeonLook::Field
        } else {
            assert!(hi == 0x00 || hi == 0x90 || hi == 0x60, "{raw:#04x}");
            DungeonLook::Passage
        }
    }

    #[test]
    fn every_raw_byte_maps_to_its_table_class() {
        for raw in 0..=u8::MAX {
            let cell = perceived_dungeon_cell(raw);
            assert_eq!(cell.look, table_look(raw), "raw {raw:#04x}");
            let badge = raw < 0x90 && raw & 0x08 != 0;
            assert_eq!(cell.rising_marker, badge, "raw {raw:#04x}");
        }
    }

    #[test]
    fn secret_doors_look_like_wall_until_search_rewrites_them() {
        for raw in 0xD0..=0xDF {
            let cell = perceived_dungeon_cell(raw);
            assert_eq!(cell.look, DungeonLook::Wall, "raw {raw:#04x}");
            assert!(!cell.rising_marker);
            assert_eq!(cell, perceived_dungeon_cell(0xB0 | (raw & 0x0F)));
        }
        // Search writes 0xE0 or 0xE8: a door from then on.
        assert_eq!(perceived_dungeon_cell(0xE0).look, DungeonLook::Door);
        assert_eq!(perceived_dungeon_cell(0xE8).look, DungeonLook::Door);
    }

    #[test]
    fn hidden_pits_and_bomb_traps_look_like_passage() {
        let hidden = perceived_dungeon_cell(0x61);
        assert_eq!(hidden.look, DungeonLook::Passage);
        assert!(!hidden.rising_marker);

        let rising = perceived_dungeon_cell(0x69);
        assert_eq!(rising.look, DungeonLook::Passage);
        assert!(rising.rising_marker, "0x69 carries the rising badge");

        let bomb = perceived_dungeon_cell(0x62);
        assert_eq!(bomb.look, DungeonLook::Passage);
        assert!(!bomb.rising_marker);

        // Search turns 0x61 into the visible pit 0x60.
        assert_eq!(perceived_dungeon_cell(0x60).look, DungeonLook::Pit);
        assert_eq!(perceived_dungeon_cell(0x68).look, DungeonLook::Pit);
        assert!(perceived_dungeon_cell(0x68).rising_marker);
    }

    #[test]
    fn the_raw_and_cleared_door_forms_look_the_same() {
        // A non-current dungeon reloads its 0xF? cells, which draw as the
        // same door as their cleared 0xA? form (design 4.3.4).
        for low in 0..=0x0F {
            assert_eq!(perceived_dungeon_cell(0xF0 | low).look, DungeonLook::Door);
            assert_eq!(perceived_dungeon_cell(0xA0 | low).look, DungeonLook::Door);
        }
    }

    #[test]
    fn bit_0x08_never_badges_class_nine_and_above() {
        for raw in 0x90..=0xFF {
            if raw & 0x08 != 0 {
                assert!(!perceived_dungeon_cell(raw).rising_marker, "{raw:#04x}");
            }
        }
    }
}
