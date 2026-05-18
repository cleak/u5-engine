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
    INTRO_INLINE_DOORWAY_STEP, INTRO_STORY_STEP_COUNT, MISCMAPS_DAT_FILE,
    MISCMAPS_RTV_COMMAND_SECTION_OFFSET, MISCMAPS_RTV_STRIP_SECTION_BYTES,
    MISCMAPS_RTV_STRIP_SECTION_OFFSET, RTV_COMMAND_STREAM_BYTES, TileGraphicsDepth,
    U4TransferOverrides, commit_u4_transfer_save, intro_step_has_story6_secondary_pass,
    intro_step_transition_strips, intro_story_art_file_for_step,
    intro_story_art_placement_for_step, intro_story_step_waits_for_input,
    intro_story6_secondary_subimage, load_play_options_from_save, load_return_to_view_script,
    load_story_records, read_u4_transfer_source_from_party_sav, summarize_return_to_view_script,
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
            UnifiedMenuStep::Ignored => {
                println!("Choose J, C, T, U, A, R, or press Enter to repeat the last choice.");
            }
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
                    println!("No loadable saved game: {err}");
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
            println!("Acknowledgements");
            println!(
                "The clean specification identifies this as a self-contained menu screen; exact text pagination is not transcribed in the engine."
            );
            dispatch.complete_subflow(subflow, IntroSubflowResult::ReturnedToMenu);
            prompt_continue()?;
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
    println!("Ultima V Introduction");
    let Some(records) = load_story_records(game_dir)? else {
        println!("STORY.DAT is missing; returning to the intro menu.");
        return Ok(());
    };

    for step in 0..INTRO_STORY_STEP_COUNT {
        println!();
        println!("Story step {} of {}", step + 1, INTRO_STORY_STEP_COUNT);
        if let Some(file) = intro_story_art_file_for_step(step) {
            if let Some(placement) = intro_story_art_placement_for_step(step) {
                println!(
                    "Art {file} subimage {} at ({}, {}).",
                    placement.subimage, placement.top_left_x, placement.top_left_y
                );
            }
        }
        if let Some(strips) = intro_step_transition_strips(step) {
            println!(
                "Transition strips: #{}, ({}, {}); #{}, ({}, {}).",
                strips[0].0, strips[0].1, strips[0].2, strips[1].0, strips[1].1, strips[1].2
            );
        }
        if step == INTRO_INLINE_DOORWAY_STEP {
            println!("[Inline doorway transition text]");
        } else {
            let record_index = if step < INTRO_INLINE_DOORWAY_STEP {
                step
            } else {
                step - 1
            };
            if let Some(text) = records.record(record_index) {
                println!("{text}");
            }
        }
        if intro_step_has_story6_secondary_pass(step) {
            if let Some(subimage) = intro_story6_secondary_subimage(step) {
                println!("Secondary STORY6.16 subimage {subimage}.");
            }
        }
        if intro_story_step_waits_for_input(step) {
            prompt_continue()?;
        }
    }
    Ok(())
}

fn run_return_to_view_preview(game_dir: &Path) -> io::Result<()> {
    println!("Return to View");
    let path = game_dir.join(MISCMAPS_DAT_FILE);
    match fs::metadata(&path) {
        Ok(metadata) => {
            println!(
                "{} found ({} bytes). Return-to-View strips start at byte {}, span {} bytes; command stream starts at byte {} and spans {} bytes.",
                MISCMAPS_DAT_FILE,
                metadata.len(),
                MISCMAPS_RTV_STRIP_SECTION_OFFSET,
                MISCMAPS_RTV_STRIP_SECTION_BYTES,
                MISCMAPS_RTV_COMMAND_SECTION_OFFSET,
                RTV_COMMAND_STREAM_BYTES
            );
            match load_return_to_view_script(game_dir) {
                Ok(Some(script)) => println!("{}", summarize_return_to_view_script(&script)),
                Ok(None) => println!("{MISCMAPS_DAT_FILE} is missing; preview cannot run."),
                Err(err) => println!("Return-to-View script error: {err}"),
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            println!("{MISCMAPS_DAT_FILE} is missing; preview cannot run.");
        }
        Err(err) => return Err(err),
    }
    println!("The terminal harness reports the clean preview layout and returns to the menu.");
    Ok(())
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
