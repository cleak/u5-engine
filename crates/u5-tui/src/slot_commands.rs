//! The slot commands without the overlay (Phase B design 3.2.9 "Slots
//! without the overlay", 3.3.5 "Imports and CLI", 3.4.5):
//!
//! - `--slots` lists the slots of the working folder;
//! - `--slot-save NAME` copies the current working save into a slot,
//!   without running Q (origin `cli`);
//! - `--slot-load NAME` restores a slot and then starts the windowed game
//!   with the boot `J`, the original early Journey Onward path.
//!
//! Each needs `menu.named_slots` and is refused under the parity lock, so
//! a harness never changes a working set by accident. The store itself is
//! the host's (`u5_host_desktop::slots`); this module is the glue that
//! knows what a save image means.

use std::io;
use std::path::Path;

use u5_host_desktop::{
    HostEnv, SlotDetails, SlotError, SlotInfo, SlotOrigin, SlotStore, SlotTarget, StoreRequest,
};
use u5_runtime::enhancements::{LockState, MenuSettings, Toggle};
use u5_runtime::{
    PlayState, SAVE_AVATAR_NAME_OFFSET, SAVE_CHARACTER_NAME_LEN, SAVED_GAM_FILENAME,
    load_play_options_from_save, read_save_image_file, save_image_has_active_avatar,
};

/// One slot command from the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotCommand {
    /// `--slots`.
    List,
    /// `--slot-save NAME`.
    Save(String),
    /// `--slot-load NAME`.
    Load(String),
}

impl SlotCommand {
    /// The flag as typed, for messages.
    pub fn flag(&self) -> &'static str {
        match self {
            Self::List => "--slots",
            Self::Save(_) => "--slot-save",
            Self::Load(_) => "--slot-load",
        }
    }
}

/// The refusal under the parity lock (design 3.4.5).
pub const SLOTS_PARITY_LOCKED: &str = "not available under the parity lock";

/// Whether the command may run: never under the parity lock, and only
/// with `menu.named_slots` on. The error is the line to print.
pub fn slot_command_gate(
    command: &SlotCommand,
    lock: LockState,
    menu: &MenuSettings,
) -> Result<(), String> {
    let flag = command.flag();
    if lock != LockState::Off {
        return Err(format!("{flag} is {SLOTS_PARITY_LOCKED}"));
    }
    if !menu.named_slots {
        if !Toggle::MenuNamedSlots.spec().is_shipped() {
            return Err(format!(
                "{flag} needs named save slots, which are not in this build yet"
            ));
        }
        return Err(format!(
            "{flag} needs named save slots: set menu.named_slots = true in config.toml, or add --named-slots"
        ));
    }
    Ok(())
}

/// The slot store of `working_dir`, under the per-user `saves` folder.
/// It is refused inside the working folder, the source folder `source`
/// and the runtime mirror root (3.3.5).
pub fn open_slot_store(working_dir: &Path, source: &Path) -> Result<SlotStore, SlotError> {
    let env = HostEnv::from_process();
    let saves = u5_host_desktop::saves_root(&env);
    let mirror = u5_host_desktop::runtime_root(&env);
    Ok(SlotStore::open(&saves, working_dir, &[source, &mirror])?.with_source_dir(source))
}

/// The restore's check (3.3.5 Restore step 3): `SAVED.GAM` reads back as
/// a 4,192-byte image with an active Avatar.
pub fn validate_working_set(dir: &Path) -> Result<(), String> {
    let image = read_save_image_file(&dir.join(SAVED_GAM_FILENAME), SAVED_GAM_FILENAME)
        .map_err(|error| u5_gamedata::friendly_io_error(&error))?;
    if !save_image_has_active_avatar(&image) {
        return Err("the save has no active game".to_string());
    }
    Ok(())
}

/// What an import shows, read from the working save without running Q:
/// the Avatar's name from the image, and the place and clock of the state
/// it loads into. The turn counter is not part of a save, so an import
/// records 0. A save that cannot be loaded keeps the name only.
pub fn working_set_details(dir: &Path) -> io::Result<SlotDetails> {
    let image = read_save_image_file(&dir.join(SAVED_GAM_FILENAME), SAVED_GAM_FILENAME)?;
    if !save_image_has_active_avatar(&image) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the working save has no active game to copy",
        ));
    }
    let name = &image[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + SAVE_CHARACTER_NAME_LEN];
    let end = name.iter().position(|b| *b == 0).unwrap_or(name.len());
    let avatar = String::from_utf8_lossy(&name[..end]).trim().to_string();
    let mut details = SlotDetails {
        avatar,
        ..SlotDetails::default()
    };
    if let Ok(state) = load_play_options_from_save(dir).and_then(|o| PlayState::load_scene(dir, o))
    {
        let clock = state.clock;
        details.place = state.area_status_label();
        details.clock = [
            clock.year,
            u16::from(clock.month),
            u16::from(clock.day),
            u16::from(clock.hour),
            u16::from(clock.minute),
        ];
    }
    Ok(details)
}

/// `--slot-save NAME`: into the newest slot of that name, or a new one.
pub fn save_to_slot(store: &SlotStore, name: &str) -> Result<SlotInfo, String> {
    let details = working_set_details(store.working_dir())
        .map_err(|error| u5_gamedata::friendly_io_error(&error))?;
    let target = match store.find(name) {
        Ok(slot) => SlotTarget::Existing(slot.id),
        Err(SlotError::NotFound(_)) => SlotTarget::New(name.to_string()),
        Err(error) => return Err(error.to_string()),
    };
    store
        .store(&StoreRequest {
            target,
            origin: SlotOrigin::Cli,
            engine: env!("CARGO_PKG_VERSION").to_string(),
            details,
            thumbnail_png: None,
        })
        .map_err(|error| format!("Could not write slot {name}: {error}"))
}

/// `--slot-load NAME`: the exact restore with its check. A failure leaves
/// the game unchanged.
pub fn load_from_slot(store: &SlotStore, name: &str) -> Result<SlotInfo, String> {
    let slot = store.find(name).map_err(|error| error.to_string())?;
    store
        .restore(&slot.id, &validate_working_set)
        .map_err(|error| {
            format!(
                "Could not load {}: {error}. Your game is unchanged.",
                slot.display_name()
            )
        })?;
    Ok(slot)
}

/// `--slots`: one line per slot, the quick slot first and then newest
/// first, or a note when there are none.
pub fn slot_listing(slots: &[SlotInfo]) -> String {
    if slots.is_empty() {
        return "No save slots yet.\n".to_string();
    }
    let mut out = String::new();
    for slot in slots {
        out.push_str(&slot_line(slot));
        out.push('\n');
    }
    out
}

/// One `--slots` line: name, place, game date and time in a neutral
/// format, turn, and the marks (imported, saved with cheats, damaged).
pub fn slot_line(slot: &SlotInfo) -> String {
    let name = slot.display_name();
    if let Some(reason) = &slot.damaged {
        return format!("{name:<24}  Damaged: {reason}");
    }
    let meta = &slot.meta;
    let [year, month, day, hour, minute] = meta.clock;
    let mut line = format!(
        "{name:<24}  {:<20}  {year}/{month:02}/{day:02} {hour:02}:{minute:02}",
        meta.place
    );
    if meta.turn > 0 {
        line.push_str(&format!("  turn {}", meta.turn));
    }
    if matches!(meta.origin, SlotOrigin::Import | SlotOrigin::Cli) {
        line.push_str("  Imported");
    }
    if meta.modified {
        line.push_str("  C (saved with cheats)");
    }
    line.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use u5_runtime::enhancements::LockReason;

    fn slots_on() -> MenuSettings {
        MenuSettings {
            named_slots: true,
            ..MenuSettings::default()
        }
    }

    /// The lock refuses every slot command with the design's words, even
    /// with slots on.
    #[test]
    fn the_parity_lock_refuses_every_slot_command() {
        for command in [
            SlotCommand::List,
            SlotCommand::Save("A".into()),
            SlotCommand::Load("A".into()),
        ] {
            for reason in [LockReason::CliFlag, LockReason::EnvVar, LockReason::QaEnv] {
                let error = slot_command_gate(&command, LockState::Locked(reason), &slots_on())
                    .unwrap_err();
                assert_eq!(
                    error,
                    format!("{} is not available under the parity lock", command.flag())
                );
            }
            assert!(slot_command_gate(&command, LockState::Off, &slots_on()).is_ok());
            let off = slot_command_gate(&command, LockState::Off, &MenuSettings::default());
            assert!(off.unwrap_err().contains("named save slots"));
        }
    }

    #[test]
    fn listing_lines_use_neutral_dates_and_marks() {
        let mut slot = SlotInfo {
            id: u5_host_desktop::SlotId::quick(),
            meta: u5_host_desktop::SlotMeta {
                format: 1,
                name: "quick".into(),
                place: "britannia".into(),
                clock: [139, 4, 7, 9, 5],
                turn: 18234,
                origin: SlotOrigin::Quick,
                ..Default::default()
            },
            damaged: None,
            has_thumbnail: true,
        };
        assert_eq!(
            slot_line(&slot),
            format!(
                "{:<24}  {:<20}  139/04/07 09:05  turn 18234",
                "quick", "britannia"
            )
        );
        slot.meta.origin = SlotOrigin::Cli;
        slot.meta.turn = 0;
        slot.meta.modified = true;
        assert!(slot_line(&slot).ends_with("09:05  Imported  C (saved with cheats)"));
        slot.damaged = Some("SAVED.GAM is missing".into());
        assert!(slot_line(&slot).ends_with("Damaged: SAVED.GAM is missing"));
        assert_eq!(slot_listing(&[]), "No save slots yet.\n");
    }

    fn scratch(label: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-tui-slots-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The restore's check refuses a short image and an empty save.
    #[test]
    fn the_restore_check_wants_a_whole_image_with_an_avatar() {
        let dir = scratch("validate");
        std::fs::write(dir.join(SAVED_GAM_FILENAME), [1u8; 10]).unwrap();
        assert!(validate_working_set(&dir).is_err());
        let mut image = vec![0u8; u5_runtime::SAVED_GAM_LEN];
        std::fs::write(dir.join(SAVED_GAM_FILENAME), &image).unwrap();
        assert!(validate_working_set(&dir).is_err());
        image[SAVE_AVATAR_NAME_OFFSET] = b'A';
        std::fs::write(dir.join(SAVED_GAM_FILENAME), &image).unwrap();
        assert!(validate_working_set(&dir).is_ok());
    }

    /// `--slot-save` then `--slot-load` through the glue, on a synthetic
    /// working folder (no game data): the restore is exact, a slot that
    /// fails the check is rolled back, and the store's folder is outside
    /// the working and source folders.
    #[test]
    fn save_then_load_through_the_glue() {
        let base = scratch("glue");
        let working = base.join("game");
        let source = base.join("source");
        std::fs::create_dir_all(&working).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        let mut image = vec![0u8; u5_runtime::SAVED_GAM_LEN];
        image[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + 4].copy_from_slice(b"Ann\0");
        std::fs::write(working.join(SAVED_GAM_FILENAME), &image).unwrap();
        let store = SlotStore::open(&base.join("saves"), &working, &[&source])
            .unwrap()
            .with_source_dir(&source);
        let saved = save_to_slot(&store, "Despise").unwrap();
        assert_eq!(saved.meta.avatar, "Ann");
        assert_eq!(saved.meta.origin, SlotOrigin::Cli);
        assert!(!saved.has_thumbnail);
        // A second save of the same name overwrites it.
        let again = save_to_slot(&store, "despise").unwrap();
        assert_eq!(again.id, saved.id);

        std::fs::write(working.join("SAVED.WPS"), [9u8; 4]).unwrap();
        image[SAVE_AVATAR_NAME_OFFSET + 1] = b'X';
        std::fs::write(working.join(SAVED_GAM_FILENAME), &image).unwrap();
        load_from_slot(&store, "Despise").unwrap();
        assert!(!working.join("SAVED.WPS").exists());
        assert_eq!(
            std::fs::read(working.join(SAVED_GAM_FILENAME)).unwrap()[SAVE_AVATAR_NAME_OFFSET + 1],
            b'n'
        );
        let error = load_from_slot(&store, "Nowhere").unwrap_err();
        assert_eq!(error, "There is no slot named Nowhere");
        assert!(std::fs::read_dir(&source).unwrap().next().is_none());
    }

    /// The three flags parse as their own mode and combine only with
    /// settings flags and a game folder.
    #[test]
    fn the_command_line_takes_one_slot_command() {
        let parse = |args: &[&str]| crate::cli::parse_cli_args(args.iter().map(|a| a.to_string()));
        let list = parse(&["--slots"]).unwrap();
        assert_eq!(list.slot_command, Some(SlotCommand::List));
        assert!(list.has_mode() && !list.is_qa_mode() && list.needs_game_data());
        let save = parse(&["--slot-save", "Minoc ship", "--preset", "classic", "/g"]).unwrap();
        assert_eq!(
            save.slot_command,
            Some(SlotCommand::Save("Minoc ship".into()))
        );
        assert_eq!(
            parse(&["--slot-load", "quick"]).unwrap().slot_command,
            Some(SlotCommand::Load("quick".into()))
        );
        for bad in [
            &["--slot-save"][..],
            &["--slot-load", " "],
            &["--slots", "--slot-load", "quick"],
            &["--slots", "--play"],
            &["--slot-load", "quick", "--boot-key", "J"],
            &["--slots", "--settings"],
            &["--slots", "--route-smoke"],
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
        // The parity lock is a refusal at run time, with its own line.
        assert!(parse(&["--slots", "--parity-lock"]).unwrap().parity_lock);
    }

    /// The host's sidecar folder and save length are the engine's.
    #[test]
    fn host_constants_are_the_engine_ones() {
        assert_eq!(
            u5_host_desktop::SIDECAR_DIR_NAME,
            u5_runtime::enhancements::SIDECAR_DIR
        );
        assert_eq!(
            u5_host_desktop::SAVE_IMAGE_LEN,
            u5_runtime::SAVED_GAM_LEN as u64
        );
    }
}
