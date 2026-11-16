//! The menu's wording table (Phase B design 3.2, 3.2.10; build plan M4-01,
//! D-M4.11).
//!
//! Every label, help line, dialog line and toast the menu shows is a
//! `&'static str` here, in the original's voice kept plain and clear: "Save
//! thy game...", never "thee" for the sake of it. Key names stay as they are
//! printed on keyboards (F12, Esc, Enter). Toasts that carry a slot name or
//! an OS reason are templates with one `{}` (D-M4.7); only a player-typed
//! slot name or the OS's own error text is ever put into them.
//!
//! The lengths are fixed by the panels (`geometry`): a narrow page's rows
//! hold [`NARROW_TEXT`] cells, a wide page's [`WIDE_TEXT`], a dialog's
//! [`DIALOG_TEXT`], a footer tab [`TAB_TEXT_NARROW`] or [`TAB_TEXT_WIDE`].
//! `tests::every_label_fits_its_page` holds them to that.
//!
//! No text here names a place, person or item of the game (1.4.7); a row
//! that can show what the player has not found starts with [`SPOILER`].

use super::geometry::{DIALOG_CELLS, NARROW_CELLS, WIDE_CELLS};

/// Text cells of a narrow page's row: the interior less one margin cell on
/// each side.
pub const NARROW_TEXT: usize = NARROW_CELLS as usize - 4;
/// Text cells of a wide page's row.
pub const WIDE_TEXT: usize = WIDE_CELLS as usize - 4;
/// Text cells of a dialog's line.
pub const DIALOG_TEXT: usize = DIALOG_CELLS as usize - 4;
/// Text cells of a tab on a narrow page's ring: the ring less the two corner
/// cells, one band cell inside each, and the two caps.
pub const TAB_TEXT_NARROW: usize = NARROW_CELLS as usize - 6;
/// Text cells of a tab on a wide page's ring.
pub const TAB_TEXT_WIDE: usize = WIDE_CELLS as usize - 6;

// --- The top menu (3.2.1) -------------------------------------------------

/// The top menu's title tab.
pub const MENU_TITLE: &str = "Menu";
/// Close the menu and go back to the prompt (Resume).
pub const RESUME: &str = "Return to the game";
/// Open the Save page.
pub const SAVE: &str = "Save thy game...";
/// Open the Load page.
pub const LOAD: &str = "Load a saved game...";
/// The journal door (M5), shown only when its owner pushes it.
pub const JOURNAL: &str = "Thy journal";
/// The full-map door (chapter 4), shown only when its owner pushes it.
pub const MAP: &str = "The map";
/// Open the Settings page.
pub const SETTINGS: &str = "Settings...";
/// The cheats door (chapter 7).
pub const CHEATS: &str = "Cheats...";
/// Restart at the title.
pub const QUIT_TO_TITLE: &str = "Return to the title";
/// Close the game.
pub const QUIT_TO_DESKTOP: &str = "Leave the game";

/// Footer: turns since the last save or load this session; `{}` is the
/// count, at most [`LAST_SAVE_MAX`], so the line fits a narrow page.
pub const LAST_SAVE_TURNS: &str = "{} turns unsaved.";
/// Footer: one turn since the last save or load.
pub const LAST_SAVE_ONE: &str = "1 turn unsaved.";
/// The largest count [`LAST_SAVE_TURNS`] shows.
pub const LAST_SAVE_MAX: u64 = 99_999;
/// Footer: the working save holds this very turn (just saved or loaded).
pub const LAST_SAVE_NOW: &str = "Thy game is saved.";
/// Footer: nothing saved or loaded yet this session.
pub const LAST_SAVE_NONE: &str = "Not saved this session.";
/// Footer: the world is Live behind the menu (U1).
pub const WAITING: &str = "The game waits.";
/// Help for Save when `save_command_available` is false.
pub const SAVE_UNAVAILABLE: &str = "Thou canst not save at this moment.";

// --- Footer key tabs --------------------------------------------------------

/// The key that closes the menu from any page.
pub const KEY_CLOSE: &str = "F12: close";
/// The key that goes back one page.
pub const KEY_BACK: &str = "Esc: back";
/// The Load page's keys, in its footer box.
pub const LOAD_KEYS: &str = "Enter: load  F2: rename  Del: delete";

// --- Save (3.2.2) -----------------------------------------------------------

/// The Save page's title tab.
pub const SAVE_TITLE: &str = "Save thy game";
/// The first row: make a new slot.
pub const NEW_SLOT: &str = "A new slot...";
/// The name field's prompt.
pub const NAME_PROMPT: &str = "Name this save:";
/// The default name of an unnamed slot; `{}` is its number.
pub const DEFAULT_SLOT_NAME: &str = "Slot {}";
/// The quick slot's name (a fixed word, always first).
pub const QUICK_SLOT: &str = "quick";
/// The Save page's footer (D3.2).
pub const SAVE_FOOTER: &str = "This also writes the save that Journey Onward loads.";

// --- Load (3.2.3) -----------------------------------------------------------

/// The Load page's title tab.
pub const LOAD_TITLE: &str = "Load a game";
/// A slot whose files are missing, the wrong length or fail their digest.
pub const DAMAGED: &str = "Damaged";
/// Shown in the thumbnail box of a slot imported from a DOS save.
pub const IMPORTED: &str = "Imported";
/// The details line of a slot saved after cheats (chapter 7): at most
/// the details box's 16 text cells.
pub const SAVED_WITH_CHEATS: &str = "Cheats were used";
/// The one-cell cheats mark on the slot's row.
pub const CHEATS_MARK: &str = "C";
/// The details line with the turn count; `{}` is the turn.
pub const TURN: &str = "Turn {}";
/// An empty slot list.
pub const NO_SLOTS: &str = "No saves yet.";
/// The dialog while the host restores a slot (3.3.9).
pub const LOADING: &str = "Loading...";
/// The name field's prompt for a rename (F2).
pub const RENAME_PROMPT: &str = "A new name for this save:";

/// A slot's game date in the lists, `year/month/day` (3.2.2).
pub fn slot_date(clock: [u16; 5]) -> String {
    let [year, month, day, ..] = clock;
    format!("{year}/{month:02}/{day:02}")
}

/// A slot's game date and time in its details (3.2.3).
pub fn slot_when(clock: [u16; 5]) -> String {
    let [.., hour, minute] = clock;
    format!("{} {hour:02}:{minute:02}", slot_date(clock))
}

// --- Dialogs (3.2.2, 3.2.3, 3.2.5, 3.2.6) --------------------------------

/// Overwrite an existing slot; `{}` is its name.
pub const OVERWRITE: &str = "Write over \"{}\"?";
/// Delete a slot; `{}` is its name.
pub const DELETE: &str = "Delete \"{}\"?";
/// The second line of the delete dialog.
pub const DELETE_WARNING: &str = "It cannot be undone.";
/// Load a slot; `{}` is its name.
pub const LOAD_ASK: &str = "Load \"{}\"?";
/// Return to the title, asked when there is unsaved progress.
pub const QUIT_TO_TITLE_ASK: &str = "Return to the title?";
/// Leave the game, asked when there is unsaved progress.
pub const QUIT_TO_DESKTOP_ASK: &str = "Leave the game?";
/// Unsaved progress since a save or load this session; `{}` is the turns.
pub const UNSAVED_TURNS: &str = "{} turns since thy last save will be lost.";
/// [`UNSAVED_TURNS`] for one turn.
pub const UNSAVED_TURN: &str = "1 turn since thy last save will be lost.";
/// Unsaved progress with no save or load yet this session.
pub const UNSAVED_SESSION: &str = "Progress since this session began will be lost.";
/// Dialog buttons.
pub const YES: &str = "Yes";
/// See [`YES`].
pub const NO: &str = "No";
/// See [`YES`].
pub const LOAD_BUTTON: &str = "Load";
/// See [`YES`].
pub const CANCEL: &str = "Cancel";
/// See [`YES`].
pub const DELETE_BUTTON: &str = "Delete";

// --- The title layer (3.2.4) -----------------------------------------------

/// The title layer's title tab.
pub const TITLE_LAYER_TITLE: &str = "Ultima V";
/// Continue the newest slot; `{}` is "name, date".
pub const CONTINUE_SLOT: &str = "Continue  ({})";
/// The title layer's Continue row for slot `name` saved on `date`: the
/// name is cut (ending in "...") so the row fits a wide page.
pub fn continue_label(name: &str, date: &str) -> String {
    let fixed = CONTINUE_SLOT.len() - 2 + 2 + date.len();
    let room = WIDE_TEXT.saturating_sub(fixed);
    fill(CONTINUE_SLOT, &[&format!("{}, {date}", cut(name, room))])
}

/// `text` cut to `cells`, ending in "..." when it was longer.
pub fn cut(text: &str, cells: usize) -> String {
    if text.len() <= cells {
        text.to_string()
    } else {
        format!("{}...", &text[..cells.saturating_sub(3)])
    }
}

/// Continue without slots: the game's own Journey Onward.
pub const CONTINUE: &str = "Continue";
/// Leave from the title.
pub const QUIT: &str = "Leave the game";

// --- Settings (3.2.7) -------------------------------------------------------

/// The four tabs.
pub const TAB_AUDIO: &str = "Audio";
/// See [`TAB_AUDIO`].
pub const TAB_DISPLAY: &str = "Display";
/// See [`TAB_AUDIO`].
pub const TAB_GAME: &str = "Game";
/// See [`TAB_AUDIO`]. The wizard's Enhancements page; "Enhancements"
/// does not fit the strip with a band cell between the tabs.
pub const TAB_ENHANCEMENTS: &str = "Extras";
/// The tabs in order.
pub const SETTINGS_TABS: [&str; 4] = [TAB_AUDIO, TAB_DISPLAY, TAB_GAME, TAB_ENHANCEMENTS];

/// Audio rows.
pub const VOLUME: &str = "Volume";
/// See [`VOLUME`].
pub const MUTE: &str = "Mute";
/// See [`VOLUME`].
pub const MUTE_ON_FOCUS_LOSS: &str = "Mute in background";
/// The Audio tab's help.
pub const AUDIO_HELP: &str =
    "Volume sets the loudness only. The game's own Sound On and Off is on the Game tab.";

/// Game rows: the game's live values.
pub const SOUND: &str = "Sound (Ctrl-S)";
/// See [`SOUND`].
pub const BUFFER: &str = "Buffer (Ctrl-B)";
/// See [`SOUND`]. Also this key's row on the Extras tab, where it sits in
/// its group: at most [`SETTINGS_LABEL`] less one cell.
pub const START_SOUND_OFF: &str = "Sound off at start";
/// See [`START_SOUND_OFF`].
pub const START_BUFFER_ON: &str = "Buffer on at start";
/// See [`SOUND`]. Also this key's row on the Extras tab, a sub-option
/// there: at most [`SETTINGS_LABEL`] less three cells.
pub const CONFIRM_UNSAVED: &str = "Warn if unsaved";
/// See [`SOUND`].
pub const PACING: &str = "Pacing...";
/// The game's own On and Off, as the game prints them.
pub const ON: &str = "On";
/// See [`ON`].
pub const OFF: &str = "Off";

/// The warning line of the unsaved-progress question (3.2.6).
pub fn unsaved_line(unsaved: super::pages::Unsaved) -> String {
    match unsaved {
        super::pages::Unsaved::Turns(1) => UNSAVED_TURN.to_string(),
        super::pages::Unsaved::Turns(turns) => fill(UNSAVED_TURNS, &[&turns.to_string()]),
        super::pages::Unsaved::Session => UNSAVED_SESSION.to_string(),
    }
}
/// A reserved row.
pub const RESERVED: &str = "(later)";

/// The preset row.
pub const PRESET: &str = "Preset:";
/// A preset after a later change.
pub const CUSTOM: &str = "Custom";
/// The preset row's help.
pub const PRESET_HELP: &str =
    "Left and Right choose a preset. It sets every feature below; a later change shows Custom.";

/// The Sound row's help (3.2.7 "Game").
pub const SOUND_HELP: &str =
    "Enter types Ctrl-S at the prompt, as thou wouldst, and the game answers.";
/// The Buffer row's help.
pub const BUFFER_HELP: &str =
    "Enter types Ctrl-B at the prompt, as thou wouldst, and the game answers.";
/// Why Sound and Buffer are greyed before play (the title layer).
pub const GAME_IN_PLAY: &str = "These answer only while thou art playing.";
/// Why the reserved Pacing row is greyed.
pub const PACING_LATER: &str = "Pacing comes in a later build.";
/// A tab with no rows in this build.
pub const NOTHING_HERE: &str = "Nothing to set here yet.";
/// The footer line of a row whose change waits for the next prompt, when
/// its row has no room for the tag.
pub const NOTE_NEXT_PROMPT: &str = "Takes hold at the next prompt.";
/// See [`NOTE_NEXT_PROMPT`].
pub const NOTE_NEXT_LAUNCH: &str = "Takes hold at the next launch.";
/// The collapsed Cheats group's help. The cheats ship (M8), but the
/// group's rows on this tab are not built yet: they are set in
/// `config.toml` or on the command line, and their page is the door.
pub const CHEATS_GROUP_HELP: &str =
    "Cheats are set in config.toml or with --enable. Ctrl+F12 opens their page.";

/// A row's apply point when it is not the next frame (3.4.1, D-M4.4).
pub const NEXT_PROMPT: &str = "next prompt";
/// See [`NEXT_PROMPT`].
pub const NEXT_LAUNCH: &str = "next launch";
/// A row the command line set for this run (3.3.6).
pub const SET_BY_COMMAND_LINE: &str = "(set by command line)";

/// The word a row that can show undiscovered things starts with (U17).
pub const SPOILER: &str = "Spoiler:";

/// The rows of 3.2.7's spoiler list that the catalogue has, besides every
/// `Tag::SpoilerRisk` row: a value of theirs shows what the player has not
/// found. Their labels and help start with [`SPOILER`]. The list's other
/// keys (the journal's tiers and clues, the automap's reveal options, the
/// quest landmarks, Teleport) join it as their rows arrive.
pub const SPOILER_KEYS: &[&str] = &["gem_map.glyphs"];

/// The Settings label of the catalogue row `key` (3.2.7): the catalogue's
/// own label made to fit the label column ([`SETTINGS_LABEL`] cells less
/// the row's indent; the Extras tab sets a group's rows in one cell and a
/// sub-option three). The catalogue label, which says more, is the row's
/// footer help when the row has no help line of its own. `None` for a key
/// with no row here.
pub fn setting_label(key: &str) -> Option<&'static str> {
    Some(match key {
        "enhancements.freeze_overlays" => "Pause behind menus",
        // [display], chapter 2.
        "display.scale_mode" => "Scaling",
        "display.pixel_aspect" => "Pixel shape",
        "display.fullscreen" => "Fullscreen",
        "display.fullscreen_key" => "Fullscreen on F11",
        "display.pillar_fill" => "Bars beside picture",
        "display.grow_left" => "Wider world view",
        "display.grow_left_light" => "Daylight reach",
        "display.crawl" => "Messages over map",
        "display.typed_colour" => "Cyan typed text",
        "display.ribbon_hp_colours" => "Health colours",
        "display.dither" => "Fade effects",
        "display.centre_party" => "Centre the party",
        "display.vsync" => "Wait for vsync",
        // [display], chapter 9.
        "display.brightness" => "Brightness",
        "display.brightness_scope" => "Brightness covers",
        "display.map_brightness" => "Map brightness",
        "display.vignette" => "Edge darkening",
        "display.sprite_composite" => "Sprites on terrain",
        "display.composite_halo" => "Sprite outlines",
        // [gem_map], chapter 10.
        "gem_map.enhanced" => "Enhanced gem map",
        "gem_map.labels" => "Names on the map",
        "gem_map.brightness_link" => "Match map dimming",
        "gem_map.map_brightness" => "Gem map dimming",
        "gem_map.sweep" => "Fade in and out",
        "gem_map.town_surroundings" => "Land around towns",
        "gem_map.dungeon_stack" => "Every level seen",
        "gem_map.wrap_rows" => "Wrapped edge",
        "gem_map.level_paging" => "Level paging",
        "gem_map.glyphs" => "Spoiler: symbols",
        // [journal], chapters 5 and 8 (M5, M6). Sub-options sit under
        // their switch: at most 17 cells.
        "journal.page" => "Quest journal",
        "journal.spoiler_tier" => "How much it tells",
        "journal.mantras" => "Mantras",
        "journal.side_panel" => "Journal panel",
        "journal.side_panel_side" => "Panel side",
        "journal.clues" => "Clue log",
        "journal.clue_text" => "What was said",
        "journal.clue_highlight" => "Mark asked topics",
        "journal.clue_highlight_style" => "Mark style",
        "journal.clue_scrollback" => "Talk scrollback",
        "journal.clue_toast" => "Clue log notice",
        "journal.notes" => "Own notes",
        "journal.clue_misses" => "Spoiler: Misses",
        "journal.quest_tracker" => "Quest trackers",
        "journal.tracker_evidence" => "Evidence",
        // [menu] and [audio], chapter 3.
        "menu.overlay" => "Menu overlay (F12)",
        "menu.title_layer" => "Title menu (F12)",
        "menu.named_slots" => "Named save slots",
        "menu.quicksave" => "Quick slot",
        "menu.confirm_unsaved" => CONFIRM_UNSAVED,
        "menu.quit_to_title" => "Return to title",
        "audio.volume" => VOLUME,
        "audio.muted" => MUTE,
        "audio.mute_on_focus_loss" => MUTE_ON_FOCUS_LOSS,
        "audio.start_sound_off" => START_SOUND_OFF,
        "audio.start_buffer_on" => START_BUFFER_ON,
        // [input], chapter 3 Phase 5.
        "input.gamepad" => "Gamepad",
        "input.mouse" => "Mouse",
        "input.extra_bindings" => "Extra key bindings",
        // [combat], chapter 6 (M7). Sub-options sit under "Auto-combat":
        // at most 17 cells.
        "combat.auto" => "Auto-combat",
        "combat.allies" => "Auto for allies",
        "combat.fast_forward" => "Faster fights",
        "combat.tactic" => "Tactic",
        "combat.targeting" => "Target",
        "combat.interrupt" => "A key during auto",
        "combat.step_ms" => "Key pause (ms)",
        "combat.pause_below_hp" => "Stop below HP %",
        "combat.action_cap" => "Most actions",
        "combat.victory" => "On victory",
        "combat.keep_between_fights" => "Keep next fight",
        "combat.pause_on_focus_loss" => "Pause if inactive",
        "combat.potions" => "Drink potions",
        "combat.mp_reserve" => "Keep magic",
        "combat.spells" => "Spells",
        // [cheats], chapter 7 (M8): the collapsed Cheats group's rows.
        "cheats.enabled" => "Allow cheats",
        "cheats.heal_all" => "Heal all",
        "cheats.cure_poison" => "Cure poison",
        "cheats.resurrect_all" => "Resurrect all",
        "cheats.max_gold" => "Max gold",
        "cheats.max_food" => "Max food",
        "cheats.refill_arrows" => "Refill arrows",
        "cheats.refill_quarrels" => "Quarrels too",
        "cheats.max_reagents" => "Max reagents",
        "cheats.repair_hull" => "Repair hull",
        "cheats.lock_hp" => "Lock health",
        "cheats.lock_mp" => "Lock mana",
        "cheats.lock_food" => "Lock food",
        "cheats.lock_gold" => "Lock gold",
        "cheats.lock_keys" => "Lock keys",
        "cheats.lock_gems" => "Lock gems",
        "cheats.lock_torches" => "Lock torches",
        "cheats.lock_arrows" => "Lock arrows",
        "cheats.lock_reagents" => "Lock reagents",
        "cheats.remember_locks" => "Remember locks",
        "cheats.stat_edit" => "Edit party",
        "cheats.teleport" => "Spoiler: Teleport",
        "cheats.state_watch" => "Watch game state",
        _ => return None,
    })
}

/// The menu's own help for the row `key`, where the catalogue's would not
/// do: a spoiler row's, which starts with [`SPOILER`] and says what it
/// shows.
pub fn setting_help(key: &str) -> Option<&'static str> {
    match key {
        "gem_map.glyphs" => Some(
            "Spoiler: gem-faithful draws secret doors and hidden pits as soon as they are in sight.",
        ),
        "journal.clue_misses" => {
            Some("Spoiler: keeps the words nobody answered, which shows what is not a topic.")
        }
        "cheats.teleport" => {
            Some("Spoiler: lists every place in the game, including ones not found yet.")
        }
        _ => None,
    }
}
/// The collapsed cheats group's header (3.2.7).
pub const CHEATS_GROUP: &str = "Cheats (changes thy game)";

// --- Toasts (3.2.10, D-M4.7) ------------------------------------------------

/// F12 away from the command prompt.
pub const OPENS_AT_PROMPT: &str = "The menu opens at the command prompt.";
/// A slot save is under way: the menu typed `Q` and waits for the game.
pub const SAVING: &str = "Saving...";
/// A slot save finished; `{}` is the slot's name.
pub const SAVED_TO: &str = "Saved to \"{}\".";
/// The Q save failed.
pub const SAVE_FAILED: &str = "Save failed; the slot was not changed.";
/// The working save was written but the slot copy failed; `{}` the name,
/// `{}` the reason.
pub const SLOT_WRITE_FAILED: &str = "Saved the game, but could not write \"{}\": {}.";
/// The slot copy of a save to a new, unnamed slot failed; `{}` the
/// reason.
pub const SLOT_WRITE_FAILED_NEW: &str = "Saved the game, but could not write the new slot: {}.";
/// Another copy holds the slot lock.
pub const SLOTS_BUSY: &str = "The saves are busy (another copy of the game?).";
/// A restore failed and was rolled back; `{}` the name, `{}` the reason.
pub const LOAD_FAILED: &str = "Could not load \"{}\": {}. Thy game is unchanged.";
/// A rename (F2) went through; `{}` is the new name.
pub const RENAMED: &str = "Renamed to \"{}\".";
/// A rename failed; `{}` the name, `{}` the reason.
pub const RENAME_FAILED: &str = "Could not rename \"{}\": {}.";
/// A slot was deleted; `{}` is its name.
pub const DELETED: &str = "Deleted \"{}\".";
/// A delete failed; `{}` the name, `{}` the reason.
pub const DELETE_FAILED: &str = "Could not delete \"{}\": {}.";
/// The host could not list the slots; `{}` the reason.
pub const LIST_FAILED: &str = "Could not read the saves: {}.";
/// Return to the title in a window that cannot start the game again (a
/// shell with no host to ask); never in the shipped shells.
pub const CANNOT_RESTART: &str = "This window cannot start the game again.";

/// `template` with each `{}` replaced by the next of `args`, in order. A
/// template with fewer holes than `args` ignores the rest.
pub fn fill(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut args = args.iter();
    let mut rest = template;
    while let Some(at) = rest.find("{}") {
        out.push_str(&rest[..at]);
        out.push_str(args.next().copied().unwrap_or(""));
        rest = &rest[at + 2..];
    }
    out.push_str(rest);
    out
}

/// `text` broken into lines of at most `width` cells at spaces; a word
/// longer than `width` is cut.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ').filter(|word| !word.is_empty()) {
        let mut word = word;
        while word.len() > width {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            lines.push(word[..width].to_string());
            word = &word[width..];
        }
        if line.is_empty() {
            line.push_str(word);
        } else if line.len() + 1 + word.len() <= width {
            line.push(' ');
            line.push_str(word);
        } else {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Every row label of the narrow pages (the top menu, Save, the title
/// layer), for the fit test.
pub const NARROW_LABELS: &[&str] = &[
    RESUME,
    SAVE,
    LOAD,
    JOURNAL,
    MAP,
    SETTINGS,
    CHEATS,
    QUIT_TO_TITLE,
    QUIT_TO_DESKTOP,
    NEW_SLOT,
    NAME_PROMPT,
    LAST_SAVE_NOW,
    LAST_SAVE_ONE,
    NO_SLOTS,
];

/// Every row label of the wide pages' label column (Settings, the title
/// layer's rows), for the fit test: [`SETTINGS_LABEL`] cells.
pub const SETTINGS_ROW_LABELS: &[&str] = &[
    VOLUME,
    MUTE,
    MUTE_ON_FOCUS_LOSS,
    SOUND,
    BUFFER,
    START_SOUND_OFF,
    START_BUFFER_ON,
    CONFIRM_UNSAVED,
    PACING,
];

/// Cells of a Settings row's label column.
pub const SETTINGS_LABEL: usize = 20;

/// Every title tab, for the fit test: narrow pages, then wide ones.
pub const NARROW_TABS: &[&str] = &[MENU_TITLE, SAVE_TITLE, KEY_CLOSE, KEY_BACK];
/// See [`NARROW_TABS`].
pub const WIDE_TABS: &[&str] = &[LOAD_TITLE, TITLE_LAYER_TITLE, KEY_CLOSE, KEY_BACK];

/// Every dialog line, filled with the longest slot name a player can type
/// where it has a name, for the fit test (after wrapping to
/// [`DIALOG_TEXT`]).
pub const DIALOG_TEMPLATES: &[&str] = &[
    OVERWRITE,
    DELETE,
    DELETE_WARNING,
    LOAD_ASK,
    QUIT_TO_TITLE_ASK,
    QUIT_TO_DESKTOP_ASK,
    UNSAVED_TURNS,
    UNSAVED_TURN,
    UNSAVED_SESSION,
    NAME_PROMPT,
    RENAME_PROMPT,
    LOADING,
];

/// The longest slot name (3.2.2).
pub const SLOT_NAME_MAX: usize = 24;
