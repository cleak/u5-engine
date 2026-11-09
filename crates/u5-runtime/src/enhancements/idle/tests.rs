//! The idle predicate: every excluded state refuses, the three exploration
//! prompts and a combat member's bare prompt count (design 1.3.7, 1.7).

use super::*;
use crate::test_fixtures::{
    configured_original_asset_dir, dungeon_state, open_dungeon_record, open_grid, open_world_grid,
    test_state, world_state,
};
use crate::z_stats::{
    DirectionPromptKind, DirectionPromptSession, YellSession, YesNoPromptKind, YesNoPromptSession,
};
use crate::{
    COMBAT_ACTOR_FLAG_SELECTABLE_80, CombatActorDescriptor, CombatTargetingCursorSession,
    CombatTerrainRevealPlayback, Direction, EndgameState, MapViewportDissolveSource,
    MiscmapsCutsceneMap, POTION_WHITE_INDEX, PlayOptions, StonegateTrapdoorPlayback,
    TransportState, VisibilitySweep, blackthorn::BlackthornRescuePhase, blackthorn_rescue_playback,
    potion_flash_playback, run_map_viewport_dissolve, text_hold::PendingTextHold,
};

/// A town at its command prompt with nothing pending.
fn idle_town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

/// A combat arena with one party member whose bare command prompt is open.
fn combat_member_prompt() -> PlayState {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.pending_combat_actor_slot = Some(0);
    state
}

#[test]
fn every_exploration_command_prompt_is_idle() {
    for (label, state) in [
        ("town", idle_town()),
        ("world", world_state(open_world_grid(), 10, 20)),
        ("dungeon", dungeon_state(open_dungeon_record(), 1, 1, 1)),
    ] {
        assert_eq!(state.idle_blocker(), None, "{label}");
        assert_eq!(
            state.idle_command_prompt(),
            Some(IdlePromptKind::Exploration),
            "{label}"
        );
        assert!(state.at_idle_command_prompt(), "{label}");
    }
}

/// One setup per blocker, each on an idle town (or world, for the ship).
fn blocking_setups() -> Vec<(&'static str, IdleBlocker, fn(&mut PlayState))> {
    vec![
        ("text hold", IdleBlocker::TextHold, |s| {
            s.pending_text_hold = Some(PendingTextHold::default());
        }),
        ("staged narration", IdleBlocker::StagedNarration, |s| {
            s.staged_narration.push_secs(1.0, "x");
        }),
        ("buffered hold keys", IdleBlocker::BufferedHoldKeys, |s| {
            s.keys_buffered_during_hold.push_back(('a', String::new()));
        }),
        ("visibility sweep", IdleBlocker::VisibilitySweep, |s| {
            s.visibility_sweep = Some(VisibilitySweep {
                frames_remaining: 20,
                pause_bios_ticks_per_frame: 1,
                center_x: 5,
                center_y: 5,
                visible_cells: std::array::from_fn(|_| false),
            });
        }),
        ("Blackthorn rescue", IdleBlocker::Blackthorn, |s| {
            s.pending_blackthorn_rescue = Some(BlackthornRescuePhase::AwaitingAcknowledgement);
        }),
        ("Blackthorn audience", IdleBlocker::Blackthorn, |s| {
            s.blackthorn_audience_map = Some(MiscmapsCutsceneMap {
                record_index: 0,
                tiles: Vec::new(),
            });
        }),
        ("Blackthorn execution", IdleBlocker::Blackthorn, |s| {
            s.pending_blackthorn_merciful_death = true;
        }),
        ("audience exit beat", IdleBlocker::Blackthorn, |s| {
            s.pending_blackthorn_audience_exit = true;
        }),
        ("black viewport", IdleBlocker::Cinematic, |s| {
            s.cutscene_black_viewport = true;
        }),
        ("Codex reader", IdleBlocker::Cinematic, |s| {
            s.codex_presentation_steps = 1;
        }),
        ("swallow marker", IdleBlocker::Cinematic, |s| {
            s.party_marker_tile_override = Some(0);
        }),
        ("endgame", IdleBlocker::Endgame, |s| {
            s.endgame = Some(EndgameState::awaiting_first_confirmation());
        }),
        ("endgame entry redraw", IdleBlocker::Endgame, |s| {
            s.endgame_entry_redraw_pending = true;
        }),
        ("map dissolve", IdleBlocker::PresentationQueue, |s| {
            s.pending_map_viewport_dissolves
                .push(run_map_viewport_dissolve(
                    MapViewportDissolveSource::BlackthornRescueBlack,
                ));
        }),
        ("rescue tableau", IdleBlocker::PresentationQueue, |s| {
            s.pending_blackthorn_rescue_playbacks
                .push(blackthorn_rescue_playback());
        }),
        ("terrain reveal", IdleBlocker::PresentationQueue, |s| {
            s.pending_combat_terrain_reveals
                .push(CombatTerrainRevealPlayback {
                    actor_slot: 1,
                    arena_cell: (1, 1),
                    terrain_tile: 4,
                    pixel_order: Vec::new(),
                    world_tick_after_operations: Vec::new(),
                });
        }),
        ("potion flash", IdleBlocker::PresentationQueue, |s| {
            s.pending_potion_flash = potion_flash_playback(POTION_WHITE_INDEX);
        }),
        ("trapdoor", IdleBlocker::PresentationQueue, |s| {
            s.pending_stonegate_trapdoor_playback = Some(StonegateTrapdoorPlayback::complete(1));
        }),
        ("Yell line", IdleBlocker::LineSession, |s| {
            s.active_yell = Some(YellSession {
                buffer: String::new(),
            });
        }),
        ("yes/no prompt", IdleBlocker::ModalPrompt, |s| {
            s.active_yes_no_prompt = Some(YesNoPromptSession {
                kind: YesNoPromptKind::SaveGame,
            });
        }),
        ("direction prompt", IdleBlocker::ModalPrompt, |s| {
            s.active_direction_prompt = Some(DirectionPromptSession {
                kind: DirectionPromptKind::Attack,
            });
        }),
        ("acted charge owed", IdleBlocker::CommandInProgress, |s| {
            s.exploration_acted_charge_owed = true;
        }),
        ("modal wait turn", IdleBlocker::CommandInProgress, |s| {
            s.pending_modal_wait_turn = Some(false);
        }),
        ("party asleep", IdleBlocker::PartyNotReady, |s| {
            for member in &mut s.party {
                member.status = b'S';
            }
        }),
        ("party dead", IdleBlocker::PartyNotReady, |s| {
            for member in &mut s.party {
                member.status = b'D';
            }
        }),
        (
            "carried underfoot reaction",
            IdleBlocker::CarriedAction,
            |s| {
                s.pending_dungeon_underfoot_reaction = Some((1, 1, 1));
            },
        ),
        (
            "carried post-action helper",
            IdleBlocker::CarriedAction,
            |s| {
                s.pending_dungeon_post_action_helper = true;
            },
        ),
    ]
}

/// Design 1.3.7: refusal in each excluded state.
#[test]
fn every_excluded_state_refuses() {
    for (label, blocker, setup) in blocking_setups() {
        let mut state = idle_town();
        setup(&mut state);
        assert_eq!(state.idle_blocker(), Some(blocker), "{label}");
        assert_eq!(state.idle_command_prompt(), None, "{label}");
        assert!(!state.at_idle_command_prompt(), "{label}");
    }
}

#[test]
fn a_ship_under_sail_is_not_waiting() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.player.transport = TransportState::Ship {
        type_byte: 0,
        tile: 0,
        sails_hoisted: true,
        hull: 99,
        skiffs: 0,
    };
    assert_eq!(state.idle_blocker(), None, "hoisted but not steered");
    state.sail_cached_direction = Some(Direction::North);
    assert_eq!(state.idle_blocker(), Some(IdleBlocker::UnderSail));
}

/// Each classified drain's blocker holds the predicate false while that
/// drain has work. The setups are the states each drain acts on.
#[test]
fn every_guarded_drain_is_covered_by_its_blocker() {
    let setups = blocking_setups();
    let state_for = |method: &str| -> &'static str {
        match method {
            "take_pending_map_viewport_dissolves" => "map dissolve",
            "take_pending_blackthorn_rescue_playbacks" => "rescue tableau",
            "take_pending_combat_terrain_reveals" => "terrain reveal",
            "take_pending_potion_flash" => "potion flash",
            "take_pending_stonegate_trapdoor_playback" => "trapdoor",
            "advance_text_hold_to" => "text hold",
            "advance_staged_narration_to" => "staged narration",
            "advance_endgame_tableau_animation_to" => "endgame",
            "advance_blackthorn_rescue_animation_to" => "Blackthorn rescue",
            "advance_blackthorn_audience_composite_to" => "Blackthorn audience",
            other => panic!("{other} has no setup; add one with its blocker"),
        }
    };
    for drain in GUARDED_DRAINS {
        let label = state_for(drain.method);
        let (_, blocker, setup) = setups
            .iter()
            .find(|(name, _, _)| *name == label)
            .unwrap_or_else(|| panic!("no setup {label}"));
        assert_eq!(*blocker, drain.blocker, "{}", drain.method);
        let mut state = idle_town();
        setup(&mut state);
        assert_eq!(
            state.idle_blocker(),
            Some(drain.blocker),
            "{}",
            drain.method
        );
    }
    // The animators' own gates are the states above.
    let mut state = idle_town();
    state.pending_blackthorn_rescue = Some(BlackthornRescuePhase::AwaitingAcknowledgement);
    assert!(state.blackthorn_rescue_awaiting_acknowledgement());
}

/// Design 1.3.7: which combat prompts count.
#[test]
fn only_a_members_bare_command_prompt_counts_in_combat() {
    let state = combat_member_prompt();
    assert_eq!(state.idle_command_prompt(), Some(IdlePromptKind::Combat));
    assert!(
        !state.at_idle_command_prompt(),
        "combat is not the exploration prompt"
    );

    let not_bare: [(&str, IdleBlocker, fn(&mut PlayState)); 7] = [
        ("automatic walk", IdleBlocker::CombatWalk, |s| {
            s.pending_combat_actor_slot = None;
        }),
        ("owed hazard line", IdleBlocker::CombatWalk, |s| {
            s.combat_pending_hazard_lines.push((0, "x".to_string()));
        }),
        ("held divide line", IdleBlocker::CombatWalk, |s| {
            s.combat_pending_divide_line = Some("x".to_string());
        }),
        ("held faint lines", IdleBlocker::CombatWalk, |s| {
            s.combat_held_faint_lines = Some(Vec::new());
        }),
        ("targeting cursor", IdleBlocker::TargetingCursor, |s| {
            s.active_combat_targeting = Some(CombatTargetingCursorSession {
                actor_slot: 0,
                attempts: Vec::new(),
                index: 0,
                attacker: (5, 5),
                cursor: (5, 5),
                max_range: 1,
                melee_arm: true,
                foes_present_at_walk_start: true,
            });
        }),
        ("direction prompt", IdleBlocker::ModalPrompt, |s| {
            s.active_direction_prompt = Some(DirectionPromptSession {
                kind: DirectionPromptKind::Attack,
            });
        }),
        ("party down", IdleBlocker::PartyNotReady, |s| {
            for member in &mut s.party {
                member.status = b'D';
            }
        }),
    ];
    for (label, blocker, setup) in not_bare {
        let mut state = combat_member_prompt();
        setup(&mut state);
        assert_eq!(state.idle_blocker(), Some(blocker), "{label}");
        assert_eq!(state.idle_command_prompt(), None, "{label}");
    }

    // A fight's carried work belongs to its return, not to the prompt.
    let mut state = combat_member_prompt();
    state.pending_dungeon_underfoot_reaction = Some((1, 1, 1));
    assert_eq!(state.idle_command_prompt(), Some(IdlePromptKind::Combat));
}

#[test]
fn the_predicate_reads_and_never_writes() {
    for (label, _, setup) in blocking_setups() {
        let mut state = idle_town();
        setup(&mut state);
        let before = format!("{state:?}");
        let _ = state.idle_command_prompt();
        let _ = state.at_idle_command_prompt();
        assert_eq!(format!("{state:?}"), before, "{label}");
    }
}

/// On the shipped game: a loaded save is at its idle prompt once the
/// input helper's entry step has run.
#[test]
fn a_loaded_game_reaches_its_idle_prompt() {
    let Some(game_dir) = configured_original_asset_dir() else {
        return;
    };
    if !game_dir.join("SAVED.GAM").exists() {
        return;
    }
    let mut state = PlayState::load_scene(&game_dir, PlayOptions::default()).unwrap();
    let _ = state.run_overworld_input_helper_entry_step();
    assert_eq!(state.idle_blocker(), None);
    assert!(state.at_idle_command_prompt());
}
