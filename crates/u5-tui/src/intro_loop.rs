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
    DisplayDriverFamily, JOURNEY_ONWARD_SHORTCUT_BANNER, MISCMAPS_DAT_FILE, PreFlourishOutcome,
    SAVED_GAM_FILENAME, STORY_DAT_FILE, TextWindowSystem, TileGraphicsDepth, disk_io_error_message,
    load_play_options_from_save, read_u4_transfer_source_from_party_sav,
    run_intro_pre_flourish_phase,
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
            let result = if run_manual_u4_transfer(game_dir)? {
                IntroSubflowResult::SaveReady
            } else {
                IntroSubflowResult::Cancelled
            };
            dispatch.complete_subflow(subflow, result);
            prompt_continue()?;
        }
        IntroSubflow::StorySlides => {
            run_story_slides(game_dir)?;
            dispatch.complete_subflow(subflow, IntroSubflowResult::ReturnedToMenu);
            prompt_continue()?;
        }
        IntroSubflow::Acknowledgements => {
            u5_runtime::require_acknowledgements_contract();
        }
        IntroSubflow::ReturnToView => {
            run_return_to_view_preview(game_dir)?;
            dispatch.complete_subflow(subflow, IntroSubflowResult::ReturnedToMenu);
            prompt_continue()?;
        }
    }
    Ok(IntroLoopControl::Continue)
}

fn print_intro_menu() {
    println!();
    println!("Intro Menu");
    println!("  J  Journey Onward");
    println!("  C  Create New Character");
    println!("  T  Transfer from Ultima IV");
    println!("  U  Ultima V Introduction");
    println!("  A  Acknowledgements");
    println!("  R  Return to View");
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

fn require_terminal_story_renderer_contract() -> ! {
    panic!(
        "terminal intro story diagnostics are a forbidden fallback; story slides require the graphical intro renderer, published proportional width table, and inline doorway text before playback can advance; see cleak/u5-spec#69 and cleak/u5-spec#70"
    )
}

fn require_terminal_return_to_view_renderer_contract() -> ! {
    // `cleak/u5-spec#54` is published and the preview is implemented in
    // the graphical shell: nineteen 16x16 cells across by four down at
    // (8, 128). The terminal harness has no pixel surface to blit that
    // strip onto, so a text transcript of it stays a forbidden fallback.
    panic!(
        "terminal Return-to-View diagnostics are a forbidden fallback; the preview is a 304x64 tile strip and must run in the graphical shell; see cleak/u5-spec#54"
    )
}

fn require_terminal_u4_transfer_renderer_contract() -> ! {
    panic!(
        "terminal Ultima IV transfer preview is a forbidden fallback; U4 transfer requires the graphical roster/status preview, prompt window, redraw timing, and confirmation input contract before a transfer can advance; see cleak/u5-spec#73"
    )
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

        let result = std::panic::catch_unwind(|| {
            let _ = run_story_slides(&dir);
        });

        let message = panic_message(result.expect_err("terminal story diagnostics must panic"));
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

        let result = std::panic::catch_unwind(|| {
            let _ = run_return_to_view_preview(&dir);
        });

        let message =
            panic_message(result.expect_err("terminal Return-to-View diagnostics must panic"));
        assert!(
            message.contains("terminal Return-to-View diagnostics are a forbidden fallback"),
            "{message}"
        );
        assert!(message.contains("304x64 tile strip"), "{message}");
        assert!(message.contains("cleak/u5-spec#54"), "{message}");
        let _ = fs::remove_dir_all(dir);
    }

    fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| {
                payload
                    .downcast_ref::<&str>()
                    .map(|message| message.to_string())
            })
            .expect("panic payload must be a string")
    }
}
