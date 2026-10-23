//! Terminal intro/menu loop.
//!
//! This is a text-mode shell around the runtime intro-menu state
//! machine. It preserves the menu ownership rules from `intro.md`:
//! non-play subflows return to the menu, and only Journey Onward
//! launches gameplay.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use u5_runtime::intro_menu::{IntroSubflow, IntroSubflowResult};
use u5_runtime::menu_dispatch::{UnifiedMenuDispatch, UnifiedMenuStep};
use u5_runtime::{
    DisplayDriverFamily, JOURNEY_ONWARD_SHORTCUT_BANNER, LOAD_EMPTY_SAVE_LINE_1,
    LOAD_EMPTY_SAVE_LINE_2, LOAD_EMPTY_SAVE_LINE_3, MISCMAPS_DAT_FILE, PreFlourishOutcome,
    SAVED_GAM_FILENAME, STORY_DAT_FILE, TextWindowSystem, TileGraphicsDepth, disk_io_error_message,
    load_play_options_from_save, read_u4_transfer_source_from_party_sav,
    run_intro_pre_flourish_phase, save_image_has_active_avatar,
};

use crate::cli::run_interactive_create_character;
use crate::play_loop::run_play_loop;

enum IntroLoopControl {
    Continue,
    Launched,
}

pub fn run_intro_menu_loop(
    game_dir: &Path,
    raster_diagnostics: bool,
    raster_depth: TileGraphicsDepth,
) -> io::Result<()> {
    println!("Ultima V");
    println!("Terminal title/menu flow. Press any key to continue, or J for Journey Onward.");
    let Some(first_key) = read_menu_key()? else {
        return Ok(());
    };

    // `systems/intro.md §3` step 2: pre-flourish phase. The terminal
    // harness has no asynchronous key buffer, so the first stdin
    // byte above stands in for the spec's "key queued at boot"
    // poll. The phase loads IBM.CH/RUNES.CH into the resident
    // font-slot table, resets the primary text window to the full
    // 40x25 rectangle, selects descriptor index 0, and consults the
    // queued byte for the early J shortcut.
    let mut text_windows = TextWindowSystem::new();
    let (_font_slots, pre_flourish_outcome) = run_intro_pre_flourish_phase(
        game_dir,
        DisplayDriverFamily::Ega,
        &mut text_windows,
        Some(first_key),
    )?;

    let mut dispatch = UnifiedMenuDispatch::new();
    dispatch.dismiss_title();
    if matches!(
        pre_flourish_outcome,
        PreFlourishOutcome::JourneyOnwardShortcut
    ) {
        println!("{JOURNEY_ONWARD_SHORTCUT_BANNER}");
        if let IntroLoopControl::Launched = drive_intro_subflow(
            &mut dispatch,
            IntroSubflow::JourneyOnward,
            game_dir,
            raster_diagnostics,
            raster_depth,
        )? {
            return Ok(());
        }
    }

    loop {
        print_intro_menu();
        let Some(key) = read_menu_key()? else {
            return Ok(());
        };
        match dispatch.submit_menu_key(key) {
            UnifiedMenuStep::EnteredSubflow(subflow) => {
                if let IntroLoopControl::Launched = drive_intro_subflow(
                    &mut dispatch,
                    subflow,
                    game_dir,
                    raster_diagnostics,
                    raster_depth,
                )? {
                    return Ok(());
                }
            }
            UnifiedMenuStep::Ignored => {}
            UnifiedMenuStep::PresentMenu | UnifiedMenuStep::ReturnedToMenu => {}
            UnifiedMenuStep::LaunchGameplay => {
                let options = load_play_options_from_save(game_dir)?;
                run_play_loop(game_dir, options, raster_diagnostics, raster_depth, None)?;
                return Ok(());
            }
            UnifiedMenuStep::PresentTitle
            | UnifiedMenuStep::CodexAdvanced(_)
            | UnifiedMenuStep::CodexCompleted
            | UnifiedMenuStep::BlackthornAdvanced
            | UnifiedMenuStep::BlackthornEnded { .. }
            | UnifiedMenuStep::U4Stepped => {}
        }
    }
}

fn drive_intro_subflow(
    dispatch: &mut UnifiedMenuDispatch,
    subflow: IntroSubflow,
    game_dir: &Path,
    raster_diagnostics: bool,
    raster_depth: TileGraphicsDepth,
) -> io::Result<IntroLoopControl> {
    match subflow {
        IntroSubflow::JourneyOnward => {
            println!("Journey Onward");
            // `save-load.md §4.2` step 2: an uninitialised save is the
            // empty-save guard, not a disk failure - "The intro prints a
            // three-line message ... waits for any keystroke, and returns
            // to the title menu". Playtest E06.
            if fs::read(game_dir.join(SAVED_GAM_FILENAME))
                .is_ok_and(|image| !save_image_has_active_avatar(&image))
            {
                println!("{LOAD_EMPTY_SAVE_LINE_1}");
                println!("{LOAD_EMPTY_SAVE_LINE_2}");
                println!("{LOAD_EMPTY_SAVE_LINE_3}");
                dispatch.complete_subflow(subflow, IntroSubflowResult::Cancelled);
                prompt_continue()?;
                return Ok(IntroLoopControl::Continue);
            }
            match load_play_options_from_save(game_dir) {
                Ok(options) => {
                    if matches!(
                        dispatch.complete_subflow(subflow, IntroSubflowResult::SaveReady),
                        UnifiedMenuStep::LaunchGameplay
                    ) {
                        run_play_loop(game_dir, options, raster_diagnostics, raster_depth, None)?;
                        return Ok(IntroLoopControl::Launched);
                    }
                }
                Err(err) => {
                    println!(
                        "{}",
                        disk_io_error_message(
                            u5_runtime::DiskIoHandlerPhase::ReadPrompt,
                            SAVED_GAM_FILENAME,
                            &err
                        )
                    );
                    dispatch.complete_subflow(subflow, IntroSubflowResult::Cancelled);
                    prompt_continue()?;
                }
            }
        }
        IntroSubflow::CharacterCreation => {
            let result = if run_interactive_create_character(game_dir)?.is_some() {
                IntroSubflowResult::SaveReady
            } else {
                IntroSubflowResult::Cancelled
            };
            dispatch.complete_subflow(subflow, result);
            prompt_continue()?;
        }
        IntroSubflow::UltimaIvTransfer => {
            let result = match refuse_unsupported_subflow(run_manual_u4_transfer(game_dir))? {
                Some(true) => IntroSubflowResult::SaveReady,
                Some(false) => IntroSubflowResult::Cancelled,
                None => {
                    dispatch.complete_subflow(subflow, IntroSubflowResult::Cancelled);
                    return Ok(IntroLoopControl::Continue);
                }
            };
            dispatch.complete_subflow(subflow, result);
            prompt_continue()?;
        }
        IntroSubflow::StorySlides => {
            let shown = refuse_unsupported_subflow(run_story_slides(game_dir))?.is_some();
            dispatch.complete_subflow(subflow, IntroSubflowResult::ReturnedToMenu);
            if shown {
                prompt_continue()?;
            }
        }
        IntroSubflow::Acknowledgements => {
            let _ = refuse_unsupported_subflow::<()>(Err(acknowledgements_need_graphical_surface()))?;
            dispatch.complete_subflow(subflow, IntroSubflowResult::ReturnedToMenu);
        }
        IntroSubflow::ReturnToView => {
            let shown = refuse_unsupported_subflow(run_return_to_view_preview(game_dir))?.is_some();
            dispatch.complete_subflow(subflow, IntroSubflowResult::ReturnedToMenu);
            if shown {
                prompt_continue()?;
            }
        }
    }
    Ok(IntroLoopControl::Continue)
}

/// `intro.md §6.2`: "The labels are the **full names**. The abbreviated
/// forms ... and `Return to View` that earlier revisions of this table
/// published do not exist in the shipped data; they were invented." Rows
/// `0..=5` map to `J`, `C`, `T`, `U`, `A`, `R` (playtest J13).
pub(crate) const INTRO_MENU_ROWS: [(char, &str); 6] = [
    ('J', "Journey Onward"),
    ('C', "Create New Character"),
    ('T', "Transfer from Ultima IV"),
    ('U', "Ultima V Introduction"),
    ('A', "Acknowledgements"),
    ('R', "Return to the View"),
];

/// The terminal menu's rows: the hotkey letter, then the published label.
pub(crate) fn intro_menu_text_rows() -> Vec<String> {
    INTRO_MENU_ROWS
        .iter()
        .map(|(key, label)| format!("  {key}  {label}"))
        .collect()
}

fn print_intro_menu() {
    println!();
    println!("Intro Menu");
    for row in intro_menu_text_rows() {
        println!("{row}");
    }
    print!("Selection: ");
    let _ = io::stdout().flush();
}

fn run_story_slides(game_dir: &Path) -> io::Result<()> {
    let path = game_dir.join(STORY_DAT_FILE);
    match fs::metadata(&path) {
        Ok(_) => require_terminal_story_renderer_contract(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "intro story requires STORY.DAT; silently returning to the menu is a forbidden fallback",
        )),
        Err(err) => Err(err),
    }
}

fn run_return_to_view_preview(game_dir: &Path) -> io::Result<()> {
    let path = game_dir.join(MISCMAPS_DAT_FILE);
    match fs::metadata(&path) {
        Ok(_) => require_terminal_return_to_view_renderer_contract(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Return-to-View requires MISCMAPS.DAT; returning to the menu is a forbidden fallback",
        )),
        Err(err) => Err(err),
    }
}

/// A menu entry the terminal harness cannot present: an
/// [`io::ErrorKind::Unsupported`] refusal. [`drive_intro_subflow`] reports it
/// and returns to `Selection:`; a choice the menu itself offers must not
/// end the process (playtest G14).
fn unsupported_in_terminal(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, message.to_string())
}

/// Report an [`unsupported_in_terminal`] refusal on stderr and yield
/// `None`; any other error still propagates.
fn refuse_unsupported_subflow<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(err) if err.kind() == io::ErrorKind::Unsupported => {
            eprintln!("{err}");
            Ok(None)
        }
        Err(err) => Err(err),
    }
}

/// `intro.md §11`: the acknowledgements are artwork drawn into the
/// `STARTSC` bitmap; see [`u5_runtime::require_graphical_acknowledgements_surface`].
fn acknowledgements_need_graphical_surface() -> io::Error {
    unsupported_in_terminal(
        "intro acknowledgements are the §11 credits artwork screen and need the graphical intro renderer; the terminal harness has no pixel surface, and substituting placeholder credits is a forbidden fallback; see cleak/u5-spec#72",
    )
}

fn require_terminal_story_renderer_contract() -> io::Result<()> {
    Err(unsupported_in_terminal(
        "terminal intro story diagnostics are a forbidden fallback; story slides require the graphical intro renderer, published proportional width table, and inline doorway text before playback can advance; see cleak/u5-spec#69 and cleak/u5-spec#70",
    ))
}

fn require_terminal_return_to_view_renderer_contract() -> io::Result<()> {
    // `cleak/u5-spec#54` is published and the preview is implemented in
    // the graphical shell: nineteen 16x16 cells across by four down at
    // (8, 128). The terminal harness has no pixel surface to blit that
    // strip onto, so a text transcript of it stays a forbidden fallback.
    Err(unsupported_in_terminal(
        "terminal Return-to-View diagnostics are a forbidden fallback; the preview is a 304x64 tile strip and must run in the graphical shell; see cleak/u5-spec#54",
    ))
}

fn require_terminal_u4_transfer_renderer_contract() -> io::Result<bool> {
    Err(unsupported_in_terminal(
        "terminal Ultima IV transfer preview is a forbidden fallback; U4 transfer requires the graphical roster/status preview, prompt window, redraw timing, and confirmation input contract before a transfer can advance; see cleak/u5-spec#73",
    ))
}

fn run_manual_u4_transfer(game_dir: &Path) -> io::Result<bool> {
    match read_u4_transfer_source_from_party_sav(game_dir) {
        Ok(_) => require_terminal_u4_transfer_renderer_contract(),
        Err(err) => {
            println!("Transfer source rejected: {err}");
            Ok(false)
        }
    }
}

fn read_menu_key() -> io::Result<Option<u8>> {
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        return Ok(None);
    }
    Ok(Some(input.bytes().next().unwrap_or(b'\n')))
}

fn prompt_continue() -> io::Result<()> {
    print!("Press Enter to continue.");
    io::stdout().flush()?;
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `intro.md §6.2`: row 5 is `Return to the View`; `Return to View` was
    /// an invented abbreviation (playtest J13).
    #[test]
    fn playtest_j13_terminal_menu_uses_the_full_published_labels() {
        let rows = intro_menu_text_rows();
        assert_eq!(rows[5], "  R  Return to the View");
        assert!(rows.iter().all(|row| !row.ends_with("Return to View")));
        assert_eq!(rows[1], "  C  Create New Character");
    }

    fn temp_intro_dir(name: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("u5-tui-intro-{name}-{nonce}"))
    }

    #[test]
    fn terminal_story_requires_story_dat_instead_of_returning_to_menu() {
        let dir = temp_intro_dir("missing-story");
        fs::create_dir_all(&dir).unwrap();

        let err = run_story_slides(&dir).expect_err("missing STORY.DAT must fail loudly");

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("forbidden fallback"), "{}", err);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn terminal_story_refuses_text_diagnostic_fallback_when_story_dat_exists() {
        let dir = temp_intro_dir("present-story");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(STORY_DAT_FILE), b"placeholder").unwrap();

        let err = run_story_slides(&dir).expect_err("terminal story diagnostics must refuse");
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
        let message = err.to_string();
        assert!(
            message.contains("terminal intro story diagnostics are a forbidden fallback"),
            "{message}"
        );
        assert!(message.contains("cleak/u5-spec#69"), "{message}");
        assert!(message.contains("cleak/u5-spec#70"), "{message}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn terminal_return_to_view_requires_miscmaps_instead_of_returning_to_menu() {
        let dir = temp_intro_dir("missing-miscmaps");
        fs::create_dir_all(&dir).unwrap();

        let err =
            run_return_to_view_preview(&dir).expect_err("missing MISCMAPS.DAT must fail loudly");

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("forbidden fallback"), "{}", err);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn terminal_return_to_view_refuses_text_diagnostic_fallback_when_miscmaps_exists() {
        let dir = temp_intro_dir("present-miscmaps");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(MISCMAPS_DAT_FILE), [0u8]).unwrap();

        let err = run_return_to_view_preview(&dir)
            .expect_err("terminal Return-to-View diagnostics must refuse");
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
        let message = err.to_string();
        assert!(
            message.contains("terminal Return-to-View diagnostics are a forbidden fallback"),
            "{message}"
        );
        assert!(message.contains("304x64 tile strip"), "{message}");
        assert!(message.contains("cleak/u5-spec#54"), "{message}");
        let _ = fs::remove_dir_all(dir);
    }

    /// Playtest G14: a refusal returns to the menu instead of ending the
    /// process; other errors still propagate.
    #[test]
    fn unsupported_menu_entries_are_refused_without_ending_the_process() {
        assert_eq!(
            refuse_unsupported_subflow::<()>(Err(acknowledgements_need_graphical_surface()))
                .unwrap(),
            None
        );
        assert_eq!(
            refuse_unsupported_subflow::<()>(require_terminal_story_renderer_contract()).unwrap(),
            None
        );
        assert_eq!(refuse_unsupported_subflow(Ok(7)).unwrap(), Some(7));
        assert!(
            refuse_unsupported_subflow::<()>(Err(io::Error::from(io::ErrorKind::NotFound)))
                .is_err()
        );
    }
}
