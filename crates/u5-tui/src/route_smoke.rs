//! Route-level smoke suite for local clean assets.
//!
//! These cases intentionally exercise public harness routes and sidecar-backed
//! transitions without asserting copyrighted text content.

use std::io;
use std::path::Path;

use u5_runtime::{
    Area, DungeonScene, FIRST_PLAYABLE_FRIGATE_TILE, FIRST_PLAYABLE_FULL_SHIP_HULL, PlayOptions,
    PlayState, PlayTarget, Scene, TileGraphicsDepth, TransportState, WindState, WorldPlane,
    load_tile_atlas,
};

use crate::{
    play_script_state_line, raster_diagnostic_line, raster_frame_kind, replay_play_script_commands,
};

const VIEWPORT_RADIUS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteSmokeExpectation {
    World(WorldPlane),
    Town(Scene),
    Dungeon(DungeonScene),
}

impl RouteSmokeExpectation {
    fn matches(self, state: &PlayState) -> bool {
        match (self, state.area) {
            (Self::World(expected), Area::World { plane }) => expected == plane,
            (Self::Town(expected), Area::Town { scene, .. }) => expected == scene,
            (Self::Dungeon(expected), Area::Dungeon { scene, .. }) => expected == scene,
            _ => false,
        }
    }

    fn label(self) -> String {
        match self {
            Self::World(plane) => plane.key().to_string(),
            Self::Town(scene) => scene.key(),
            Self::Dungeon(scene) => scene.key(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RouteSmokeCase {
    pub name: &'static str,
    pub options: PlayOptions,
    pub script: &'static [&'static str],
    pub expected: RouteSmokeExpectation,
    pub min_turn: u64,
    pub expected_frame_kind: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteSmokeReport {
    pub name: String,
    pub commands_run: usize,
    pub final_state_line: String,
    pub final_raster_line: String,
}

pub fn route_smoke_cases() -> Vec<RouteSmokeCase> {
    let castle = Scene::new(0x11).expect("castle scene is valid");
    let dungeon = DungeonScene::new(0x21).expect("dungeon scene is valid");

    let world = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        ..PlayOptions::default()
    };

    let underworld = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        ..PlayOptions::default()
    };

    let mut world_move = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        ..PlayOptions::default()
    };
    world_move.start = Some((62, 124));

    let world_to_castle = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        debug_enter: Some(PlayTarget::Town(castle)),
        ..PlayOptions::default()
    };

    let world_to_dungeon = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        debug_enter: Some(PlayTarget::Dungeon(dungeon)),
        ..PlayOptions::default()
    };

    let underworld_to_castle = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        debug_enter: Some(PlayTarget::Town(castle)),
        ..PlayOptions::default()
    };

    let ship_transport = TransportState::Ship {
        type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 2,
    };
    let ship_xit = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        transport: ship_transport,
        ..PlayOptions::default()
    };
    let ship_sail = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        transport: ship_transport,
        wind: WindState::East,
        wind_save_byte: WindState::East.save_byte(),
        ..PlayOptions::default()
    };

    let dungeon_options = PlayOptions {
        target: PlayTarget::Dungeon(dungeon),
        floor: 0,
        ..PlayOptions::default()
    };

    vec![
        RouteSmokeCase {
            name: "castle-pass-and-idle",
            options: PlayOptions::default(),
            script: &["empty", "idle:2"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-move-pass-idle",
            options: world_move,
            script: &["d", "empty", "idle:1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 2,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-look-pass",
            options: world.clone(),
            script: &["l6", "empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-save-refusal",
            options: world,
            script: &["Q", "N"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "underworld-pass-and-idle",
            options: underworld,
            script: &["empty", "idle:1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-z-stats-modal",
            options: PlayOptions::default(),
            script: &["Z", "empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-look-pass",
            options: PlayOptions::default(),
            script: &["l6", "empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-save-refusal",
            options: PlayOptions::default(),
            script: &["Q", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "debug-enter-castle",
            options: world_to_castle.clone(),
            script: &["e", "empty", "idle:1"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "debug-enter-castle-return-world",
            options: world_to_castle,
            script: &["e", "w", "idle:1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "debug-enter-castle-from-underworld",
            options: underworld_to_castle,
            script: &["e", "empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "ship-xit-launches-skiff",
            options: ship_xit,
            script: &["X", "empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 2,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "ship-hoist-and-sail-east",
            options: ship_sail,
            script: &["Y", "d", "empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 3,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "debug-enter-dungeon",
            options: world_to_dungeon,
            script: &["e", "Q", "N"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-exit-refusal",
            options: dungeon_options.clone(),
            script: &["Q", "N", "idle:1"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-turn-and-blocked-step",
            options: dungeon_options.clone(),
            script: &["w", "a", "d", "s"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 2,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-exit-confirm",
            options: dungeon_options,
            script: &["Q", "Y"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
    ]
}

pub fn run_route_smoke(game_dir: &Path, raster_depth: TileGraphicsDepth) -> io::Result<()> {
    let cases = route_smoke_cases();
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    println!("Route smoke: {} case(s).", cases.len());
    for case in &cases {
        let report = run_route_smoke_case(game_dir, &atlas, case)?;
        println!(
            "route-smoke {}: {} command(s), {}",
            report.name, report.commands_run, report.final_state_line
        );
        println!("{}", report.final_raster_line);
    }
    println!("Route smoke: all cases passed.");
    Ok(())
}

pub fn run_route_smoke_case(
    game_dir: &Path,
    atlas: &u5_runtime::TileAtlas,
    case: &RouteSmokeCase,
) -> io::Result<RouteSmokeReport> {
    let mut state = PlayState::load_scene(game_dir, case.options.clone())?;
    let commands = case
        .script
        .iter()
        .map(|command| (*command).to_string())
        .collect::<Vec<_>>();
    let mut commands_run = 0;

    let initial_raster = raster_diagnostic_line(&mut state, VIEWPORT_RADIUS, atlas)?;
    require_raster_available(case, &initial_raster)?;

    replay_play_script_commands(&mut state, game_dir, &commands, |state, _, _| {
        commands_run += 1;
        let raster = raster_diagnostic_line(state, VIEWPORT_RADIUS, atlas)?;
        require_raster_hash(case, &raster)
    })?;

    if !case.expected.matches(&state) {
        return Err(io::Error::other(format!(
            "route smoke `{}` ended in `{}`; expected {}",
            case.name,
            state.current_area_label(),
            case.expected.label()
        )));
    }
    if state.turn < case.min_turn {
        return Err(io::Error::other(format!(
            "route smoke `{}` ended at turn {}; expected at least {}",
            case.name, state.turn, case.min_turn
        )));
    }
    if raster_frame_kind(&state) != case.expected_frame_kind {
        return Err(io::Error::other(format!(
            "route smoke `{}` ended with `{}`; expected `{}`",
            case.name,
            raster_frame_kind(&state),
            case.expected_frame_kind
        )));
    }

    let final_raster_line = raster_diagnostic_line(&mut state, VIEWPORT_RADIUS, atlas)?;
    require_raster_hash(case, &final_raster_line)?;
    Ok(RouteSmokeReport {
        name: case.name.to_string(),
        commands_run,
        final_state_line: play_script_state_line(&state),
        final_raster_line,
    })
}

fn require_raster_hash(case: &RouteSmokeCase, raster: &str) -> io::Result<()> {
    if !raster.contains(case.expected_frame_kind) || !raster.contains(" hash ") {
        return Err(io::Error::other(format!(
            "route smoke `{}` produced weak raster diagnostic: {raster}",
            case.name
        )));
    }
    Ok(())
}

fn require_raster_available(case: &RouteSmokeCase, raster: &str) -> io::Result<()> {
    if !raster.contains(" hash ") {
        return Err(io::Error::other(format!(
            "route smoke `{}` produced weak initial raster diagnostic: {raster}",
            case.name
        )));
    }
    Ok(())
}
