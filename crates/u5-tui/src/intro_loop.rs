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
use u5_runtime::u4_transfer_session::u4_transfer_preview_from_u4_values;
use u5_runtime::{
    MISCMAPS_DAT_FILE, SAVED_GAM_FILENAME, STORY_DAT_FILE, TileGraphicsDepth, U4TransferOverrides,
    commit_u4_transfer_save, disk_io_error_message, load_play_options_from_save,
    read_u4_transfer_source_from_party_sav,
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
    let mut dispatch = UnifiedMenuDispatch::new();

    println!("Ultima V");
    println!("Terminal title/menu flow. Press any key to continue, or J for Journey Onward.");
    let Some(first_key) = read_menu_key()? else {
        return Ok(());
    };
    dispatch.dismiss_title();
    if first_key.eq_ignore_ascii_case(&b'J') {
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
    panic!(
        "terminal Return-to-View diagnostics are a forbidden fallback; Return-to-View requires the graphical preview pixel geometry and caption renderer before playback can advance; see cleak/u5-spec#54 and cleak/u5-spec#70"
    )
}

fn run_manual_u4_transfer(game_dir: &Path) -> io::Result<bool> {
    println!("Transfer from Ultima IV");
    println!(
        "Reading PARTY.SAV. This writes a U5 save from BRIT.GAM/BRIT.OOL and returns to the intro menu."
    );

    let source = match read_u4_transfer_source_from_party_sav(game_dir) {
        Ok(source) => source,
        Err(err) => {
            println!("Transfer source rejected: {err}");
            return Ok(false);
        }
    };
    let preview = u4_transfer_preview_from_u4_values(
        u4_transfer_display_name(&source.name),
        source.class_index,
        source.strength,
        source.dexterity,
        source.intelligence,
        0,
    );
    let mut overrides = U4TransferOverrides {
        name: None,
        male: None,
    };
    println!(
        "Preview: {} class {}, {}, STR {}, DEX {}, INT {}, XP {}.",
        preview.name,
        preview.class_index,
        if source.male { "male" } else { "female" },
        preview.strength,
        preview.dexterity,
        preview.intelligence,
        source.experience / 10
    );
    if !prompt_yes_no(&format!("Use imported name {}? (Y/N): ", preview.name))? {
        let Some(name) = prompt_nonblank_name("Replacement name: ")? else {
            return Ok(false);
        };
        overrides.name = Some(name);
    }
    if !prompt_yes_no(&format!(
        "Use imported gender {}? (Y/N): ",
        if source.male { "M" } else { "F" }
    ))? {
        let Some(male) = prompt_gender("Replacement gender M/F: ")? else {
            return Ok(false);
        };
        overrides.male = Some(male);
    }
    if !prompt_yes_no("Commit transfer save? (Y/N): ")? {
        return Ok(false);
    }

    let avatar = commit_u4_transfer_save(game_dir, &source, Some(&overrides))?;
    let end = avatar
        .name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(avatar.name.len());
    println!(
        "Transferred {}. Choose Journey Onward to load the new save.",
        String::from_utf8_lossy(&avatar.name[..end])
    );
    Ok(true)
}

fn u4_transfer_display_name(name: &[u8]) -> String {
    let end = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    String::from_utf8_lossy(&name[..end]).trim_end().to_string()
}

fn prompt_nonblank_name(prompt: &str) -> io::Result<Option<Vec<u8>>> {
    loop {
        let Some(value) = prompt_line(prompt)? else {
            return Ok(None);
        };
        if value.trim().is_empty() {
            println!("Name may not be blank.");
        } else {
            return Ok(Some(value.trim().as_bytes().to_vec()));
        }
    }
}

fn prompt_gender(prompt: &str) -> io::Result<Option<bool>> {
    loop {
        let Some(value) = prompt_line(prompt)? else {
            return Ok(None);
        };
        match value.bytes().next().map(|byte| byte.to_ascii_uppercase()) {
            Some(b'M') => return Ok(Some(true)),
            Some(b'F') => return Ok(Some(false)),
            _ => println!("Press M or F."),
        }
    }
}

fn prompt_line(prompt: &str) -> io::Result<Option<String>> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        return Ok(None);
    }
    Ok(Some(input.trim_end_matches(['\r', '\n']).to_string()))
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

fn prompt_yes_no(prompt: &str) -> io::Result<bool> {
    loop {
        let Some(value) = prompt_line(prompt)? else {
            return Ok(false);
        };
        match value.bytes().next().map(|byte| byte.to_ascii_uppercase()) {
            Some(b'Y') => return Ok(true),
            Some(b'N') | None => return Ok(false),
            _ => println!("Press Y or N."),
        }
    }
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
        assert!(message.contains("cleak/u5-spec#54"), "{message}");
        assert!(message.contains("cleak/u5-spec#70"), "{message}");
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
