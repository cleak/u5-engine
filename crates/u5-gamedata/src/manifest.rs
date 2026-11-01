//! The game-data manifest: every file the engine reads from the data folder.
//!
//! Derived from the engine's actual opens (system-call traces of the route
//! smoke, the headless and Bevy frame suites, the terminal intro, character
//! creation, the throne-room report and the location audit) plus the
//! gameplay paths those runs do not reach (endgame panels, the character
//! creation screen, the Lord British path, signs, the Word-of-Power
//! lettering), and checked against the GOG.com release. The release also
//! ships the original executable, overlays, display drivers and the
//! four-colour (`.4`) and Hercules (`.HCS`) art; the engine never opens
//! those, so they are not listed and their absence is fine.
//!
//! Names are the canonical upper-case names the engine opens.

/// How the engine uses a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileRole {
    /// Static game data; the engine cannot run without it.
    Required,
    /// A save or save mirror. Optional (a new game creates them), copied
    /// into a managed folder when present, and never overwritten there.
    Save,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DataFile {
    pub name: &'static str,
    pub role: FileRole,
    /// A short description of what the file is for (our own wording).
    pub purpose: &'static str,
}

impl DataFile {
    pub const fn is_required(&self) -> bool {
        matches!(self.role, FileRole::Required)
    }

    pub const fn is_save(&self) -> bool {
        matches!(self.role, FileRole::Save)
    }
}

const fn req(name: &'static str, purpose: &'static str) -> DataFile {
    DataFile {
        name,
        role: FileRole::Required,
        purpose,
    }
}

const fn save(name: &'static str, purpose: &'static str) -> DataFile {
    DataFile {
        name,
        role: FileRole::Save,
        purpose,
    }
}

/// Every file the engine reads, required data first.
pub const MANIFEST: &[DataFile] = &[
    req("BRIT.CBT", "outdoor combat maps"),
    req("BRIT.DAT", "Britannia surface map"),
    req("BRITISH.BIT", "title-screen artwork"),
    req("BRITISH.PTH", "title-screen signature path"),
    req("CASTLE.DAT", "castle maps"),
    req("CASTLE.NPC", "castle residents"),
    req("CASTLE.TLK", "castle conversations"),
    req("CREATE.16", "character-creation artwork"),
    req("DATA.OVL", "shared game tables"),
    req("DNG1.16", "dungeon view artwork"),
    req("DNG2.16", "mine view artwork"),
    req("DNG3.16", "dungeon view artwork (variant)"),
    req("DUNGEON.CBT", "dungeon room maps"),
    req("DUNGEON.DAT", "dungeon maps"),
    req("DWELLING.DAT", "dwelling maps"),
    req("DWELLING.NPC", "dwelling residents"),
    req("DWELLING.TLK", "dwelling conversations"),
    req("END.DAT", "endgame sequence data"),
    req("END1.16", "endgame artwork"),
    req("END2.16", "endgame artwork"),
    req("ENDMSG.DAT", "endgame messages"),
    req("ENDSC.16", "endgame certificate artwork"),
    req("IBM.CH", "text font"),
    req("INIT.GAM", "new-game party template"),
    req("INIT.OOL", "new-game world objects"),
    req("ITEMS.16", "item artwork"),
    req("KARMA.DAT", "virtue questions data"),
    req("KEEP.DAT", "keep maps"),
    req("KEEP.NPC", "keep residents"),
    req("KEEP.TLK", "keep conversations"),
    req("LOOK2.DAT", "look descriptions"),
    req("MISCMAPS.DAT", "miscellaneous maps"),
    req("MISCMSG.DAT", "miscellaneous messages"),
    req("MON0.16", "dungeon monster artwork"),
    req("MON1.16", "dungeon monster artwork"),
    req("MON2.16", "dungeon monster artwork"),
    req("MON3.16", "dungeon monster artwork"),
    req("MON4.16", "dungeon monster artwork"),
    req("MON5.16", "dungeon monster artwork"),
    req("MON6.16", "dungeon monster artwork"),
    req("MON7.16", "dungeon monster artwork"),
    req("PROPORT.PCS", "proportional font"),
    req("QUESTION.DAT", "character-creation questions"),
    req("RUNES.CH", "runic font"),
    req("SHOPPE.DAT", "shop data"),
    req("SIGNS.DAT", "signs"),
    req("STARTSC.16", "start-screen artwork"),
    req("STORY1.16", "introduction artwork"),
    req("STORY2.16", "introduction artwork"),
    req("STORY3.16", "introduction artwork"),
    req("STORY4.16", "introduction artwork"),
    req("STORY5.16", "introduction artwork"),
    req("STORY6.16", "introduction artwork"),
    req("STORY.DAT", "introduction text"),
    req("TEXT.16", "title lettering"),
    req("TILES.16", "map tiles"),
    req("TITLE.BIT", "title artwork"),
    req("TOWNE.DAT", "town maps"),
    req("TOWNE.NPC", "town residents"),
    req("TOWNE.TLK", "town conversations"),
    req("ULTIMA.16", "title artwork"),
    req("UNDER.DAT", "Underworld map"),
    req("WD.BIT", "Word of Power lettering"),
    save("SAVED.GAM", "saved game"),
    save("SAVED.OOL", "saved world objects"),
    save("BRIT.OOL", "Britannia object mirror"),
    save("UNDER.OOL", "Underworld object mirror"),
];

/// Look a canonical (upper-case) name up in the manifest.
pub fn manifest_entry(name: &str) -> Option<&'static DataFile> {
    MANIFEST
        .iter()
        .find(|file| file.name.eq_ignore_ascii_case(name))
}

pub fn required_files() -> impl Iterator<Item = &'static DataFile> {
    MANIFEST.iter().filter(|file| file.is_required())
}

pub fn save_files() -> impl Iterator<Item = &'static DataFile> {
    MANIFEST.iter().filter(|file| file.is_save())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_are_unique_upper_case_8_3() {
        let mut seen = HashSet::new();
        for file in MANIFEST {
            assert!(seen.insert(file.name), "duplicate {}", file.name);
            assert_eq!(file.name, file.name.to_ascii_uppercase(), "{}", file.name);
            let (stem, ext) = file.name.split_once('.').expect("has an extension");
            assert!((1..=8).contains(&stem.len()) && (1..=3).contains(&ext.len()));
        }
    }

    #[test]
    fn counts() {
        assert_eq!(required_files().count(), 63);
        assert_eq!(save_files().count(), 4);
        assert!(manifest_entry("tiles.16").is_some());
        assert!(manifest_entry("ULTIMA.EXE").is_none());
    }
}
