//! Blackthorn cutscene helpers per `blackthorn.md` section 7.

/// `conversation.md §2.1` / `blackthorn.md §7a`: the roster dialog
/// marker that bypasses `.TLK` loading and enters the regime guard
/// demand handler.
pub const BLACKTHORN_GUARD_DEMAND_DIALOG_ID: u8 = 0xff;
pub const BLACKTHORN_GUARD_TRIBUTE_PER_LIVING_MEMBER: u16 = 10;
pub const BLACKTHORN_GUARD_PASSWORD_INPUT_MAX: usize = 14;
pub const BLACKTHORN_GUARD_PASSWORD_COMPARE_LEN: usize = 4;
pub const BLACKTHORN_GUARD_PASSWORD: &str = "IMPERA";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornGuardDemandPrompt {
    PalacePassword,
    MinocCharity,
    Tribute { amount: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornGuardDemandStart {
    Prompt(BlackthornGuardDemandPrompt),
    Refused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornGuardDemandResolution {
    AwaitingInput,
    PaidOrPassed { gold: u16 },
    Refused { gold: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveBlackthornGuardDemand {
    pub prompt: BlackthornGuardDemandPrompt,
    pub arrest: crate::TownArrestPrompt,
}

impl BlackthornGuardDemandPrompt {
    pub fn message(self) -> String {
        match self {
            Self::PalacePassword => "Black Badge bearer, give the password:".to_string(),
            Self::MinocCharity => "Give half thy gold to charity? (Y/N).".to_string(),
            // Measured against the original (`cleak/u5-spec#198`): a
            // Moonglow tribute guard prints
            // `A guard demands a 10 gp tribute to Blackthorn!`, a blank
            // row, then `Dost thou pay?` with the input cursor on the
            // next line - the same envelope shape as
            // [`crate::TLK_KEYWORD_PROMPT`]. `blackthorn.md §7a` says the
            // amount "is printed in the line" but publishes none of the
            // wording, and the engine had invented
            // `Pay {amount} gold tribute to Blackthorn? (Y/N).`
            //
            // §7a calls the demander "the guard" on all three branches,
            // so the fixed subject is safe here. The original composes it
            // from the NPC's Look description, which matters for the
            // sibling non-speaker line - see the same issue.
            Self::Tribute { amount } => {
                format!("A guard demands a {amount} gp tribute to Blackthorn!\n\nDost thou pay?")
            }
        }
    }
}

/// Classify the scene-keyed demand without mutating gameplay state.
/// The palace branch refuses immediately unless the Black Badge aura
/// is the currently active shared timed effect. The caller supplies
/// that predicate so this pure classifier does not own live save state.
pub const fn begin_blackthorn_guard_demand(
    scene_byte: u8,
    black_badge_aura_active: bool,
    living_party_members: u16,
) -> BlackthornGuardDemandStart {
    if scene_byte == crate::SCENE_LORD_BLACKTHORNS_CASTLE {
        if black_badge_aura_active {
            BlackthornGuardDemandStart::Prompt(BlackthornGuardDemandPrompt::PalacePassword)
        } else {
            BlackthornGuardDemandStart::Refused
        }
    } else if scene_byte == crate::SCENE_MINOC {
        BlackthornGuardDemandStart::Prompt(BlackthornGuardDemandPrompt::MinocCharity)
    } else {
        BlackthornGuardDemandStart::Prompt(BlackthornGuardDemandPrompt::Tribute {
            amount: living_party_members.saturating_mul(BLACKTHORN_GUARD_TRIBUTE_PER_LIVING_MEMBER),
        })
    }
}

/// Resolve one guard-demand prompt. Yes/no prompts ignore other input;
/// the palace password compares only the first four of at most fourteen
/// typed characters, case-insensitively. A refusal never changes gold.
pub fn resolve_blackthorn_guard_demand(
    prompt: BlackthornGuardDemandPrompt,
    input: &str,
    gold: u16,
) -> BlackthornGuardDemandResolution {
    match prompt {
        BlackthornGuardDemandPrompt::PalacePassword => {
            let typed = input
                .chars()
                .take(BLACKTHORN_GUARD_PASSWORD_INPUT_MAX)
                .collect::<String>();
            let typed_prefix = typed
                .chars()
                .take(BLACKTHORN_GUARD_PASSWORD_COMPARE_LEN)
                .collect::<String>();
            let expected_prefix = BLACKTHORN_GUARD_PASSWORD
                .chars()
                .take(BLACKTHORN_GUARD_PASSWORD_COMPARE_LEN)
                .collect::<String>();
            if typed_prefix.eq_ignore_ascii_case(&expected_prefix) {
                BlackthornGuardDemandResolution::PaidOrPassed { gold }
            } else {
                BlackthornGuardDemandResolution::Refused { gold }
            }
        }
        BlackthornGuardDemandPrompt::MinocCharity => match yes_no_demand_input(input) {
            Some(true) => BlackthornGuardDemandResolution::PaidOrPassed { gold: gold / 2 },
            Some(false) => BlackthornGuardDemandResolution::Refused { gold },
            None => BlackthornGuardDemandResolution::AwaitingInput,
        },
        BlackthornGuardDemandPrompt::Tribute { amount } => match yes_no_demand_input(input) {
            Some(true) if gold >= amount => BlackthornGuardDemandResolution::PaidOrPassed {
                gold: gold - amount,
            },
            Some(true) | Some(false) => BlackthornGuardDemandResolution::Refused { gold },
            None => BlackthornGuardDemandResolution::AwaitingInput,
        },
    }
}

fn yes_no_demand_input(input: &str) -> Option<bool> {
    match input.trim_start().chars().next()?.to_ascii_lowercase() {
        'y' => Some(true),
        'n' => Some(false),
        _ => None,
    }
}

/// `blackthorn.md` section 7: scene byte the rescue/refuge path hands control
/// to (`CASTLE:0`, Lord British's Castle, scene byte 17). Anchored
/// to [`crate::SCENE_LORD_BRITISHS_CASTLE`] so the rescue handoff
/// and the named scene constant share one source of truth.
pub const BLACKTHORN_RESCUE_HANDOFF_SCENE: u8 = crate::SCENE_LORD_BRITISHS_CASTLE;

/// `blackthorn.md` section 7: local position (X, Y) the rescue path hands the
/// party to inside the rescue handoff scene.
pub const BLACKTHORN_RESCUE_HANDOFF_X: u8 = 10;
pub const BLACKTHORN_RESCUE_HANDOFF_Y: u8 = 10;

/// `blackthorn.md` section 7: the rescue path raises the shared moral-standing
/// selector to at least this floor after printing the verdict.
pub const BLACKTHORN_RESCUE_STANDING_FLOOR: u8 = 75;

pub const BLACKTHORN_RESCUE_LEFT_GUARDIAN_CELL: (u8, u8) = (2, 7);
pub const BLACKTHORN_RESCUE_RIGHT_GUARDIAN_CELL: (u8, u8) = (8, 7);
pub const BLACKTHORN_RESCUE_SPECTRAL_CELL: (u8, u8) = (5, 2);
pub const BLACKTHORN_RESCUE_SOFTWARE_ENVELOPE_COUNT: u8 = 6;
pub const BLACKTHORN_RESCUE_FLASH_COUNT: u8 = 2;
pub const BLACKTHORN_RESCUE_FLASH_PRNG_DRAWS_PER_INVOCATION: u16 = 1_856;

/// Completed, blocking rescue tableau between the two viewport dissolves.
/// The runtime records direct-screen operations so frontends do not have to
/// infer them from the final Lord British's Castle handoff state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlackthornRescuePlayback {
    pub party_cell: (u8, u8),
    pub party_atlas_index: u16,
    pub software_envelope_count: u8,
    pub guardian_reveals: Vec<BlackthornCellRevealPlayback>,
    pub spectral_reveal: BlackthornCellRevealPlayback,
    pub redraw_count: u8,
    pub bios_waits: Vec<u8>,
    pub flash_count: u8,
    pub flash_prng_draws: u16,
    pub persistent_terrain: Vec<((u8, u8), u8)>,
    pub persistent_actors: Vec<((u8, u8), u8)>,
}

pub fn blackthorn_rescue_playback() -> BlackthornRescuePlayback {
    BlackthornRescuePlayback {
        party_cell: crate::BLACKTHORN_RESCUE_PARTY_CELL,
        party_atlas_index: BLACKTHORN_RESCUE_PARTY_ATLAS_INDEX,
        software_envelope_count: BLACKTHORN_RESCUE_SOFTWARE_ENVELOPE_COUNT,
        guardian_reveals: vec![
            blackthorn_cell_reveal_playback(
                BLACKTHORN_RESCUE_LEFT_GUARDIAN_CELL,
                u16::from(BLACKTHORN_RESCUE_LEFT_GUARDIAN_TILE),
            ),
            blackthorn_cell_reveal_playback(
                BLACKTHORN_RESCUE_RIGHT_GUARDIAN_CELL,
                u16::from(BLACKTHORN_RESCUE_RIGHT_GUARDIAN_TILE),
            ),
        ],
        spectral_reveal: blackthorn_cell_reveal_playback(
            BLACKTHORN_RESCUE_SPECTRAL_CELL,
            BLACKTHORN_RESCUE_SPECTRAL_ATLAS_INDEX,
        ),
        // Party tableau, two Guardian commits, and the spectral commit.
        redraw_count: 4,
        bios_waits: vec![4, 4],
        flash_count: BLACKTHORN_RESCUE_FLASH_COUNT,
        flash_prng_draws: BLACKTHORN_RESCUE_FLASH_PRNG_DRAWS_PER_INVOCATION
            * u16::from(BLACKTHORN_RESCUE_FLASH_COUNT),
        persistent_terrain: vec![
            (
                BLACKTHORN_RESCUE_LEFT_GUARDIAN_CELL,
                BLACKTHORN_RESCUE_LEFT_GUARDIAN_TILE,
            ),
            (
                BLACKTHORN_RESCUE_RIGHT_GUARDIAN_CELL,
                BLACKTHORN_RESCUE_RIGHT_GUARDIAN_TILE,
            ),
        ],
        persistent_actors: vec![
            (crate::BLACKTHORN_RESCUE_PARTY_CELL, crate::PLAYER_TILE),
            (
                BLACKTHORN_RESCUE_SPECTRAL_CELL,
                BLACKTHORN_RESCUE_SPECTRAL_ACTOR_BYTE,
            ),
        ],
    }
}

/// `blackthorn.md` section 6 cutscene-VM actor families. The audience and
/// failure beats reference these slots by index when emitting
/// movement descriptors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornCutsceneActor {
    /// Slot 0: Avatar / party-leader presentation actor.
    Avatar,
    /// Slot 1: second party member; dragged-away victim of the
    /// failed-challenge punishment beat.
    SecondPartyMember,
    /// Slot 6: left/acting guard.
    LeftGuard,
    /// Slot 7: right/secondary guard.
    RightGuard,
    /// Slot 8: seated Blackthorn and throne tableau.
    SeatedBlackthorn,
}

impl BlackthornCutsceneActor {
    /// `blackthorn.md` section 6: returns the cinematic actor slot index
    /// the script VM uses for this role.
    pub const fn slot_index(self) -> u8 {
        match self {
            Self::Avatar => 0,
            Self::SecondPartyMember => 1,
            Self::LeftGuard => 6,
            Self::RightGuard => 7,
            Self::SeatedBlackthorn => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlackthornCutsceneActorPlacement {
    pub actor: BlackthornCutsceneActor,
    pub type_byte: u8,
    pub tile: u8,
    pub x: usize,
    pub y: usize,
}

pub const BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER: u8 = 0xb7;
pub const BLACKTHORN_GUARD_ACTOR_BYTE: u8 = 0x70;
pub const BLACKTHORN_SUPPRESSED_ACTOR_BYTE: u8 = 0x16;
pub const BLACKTHORN_SEATED_ACTOR_BYTE: u8 = 0x78;
pub const BLACKTHORN_SEATED_ATLAS_INDEX: u16 = 0x178;
pub const BLACKTHORN_COBBLE_TILE: u8 = 0x44;
pub const BLACKTHORN_LOCKED_DOOR_TILE: u8 = 0xbb;
pub const BLACKTHORN_PENDULUM_TILE: u8 = 0x82;
pub const BLACKTHORN_HOURGLASS_TILE: u8 = 0xe9;
pub const BLACKTHORN_RESCUE_LEFT_GUARDIAN_TILE: u8 = 0x5e;
pub const BLACKTHORN_RESCUE_RIGHT_GUARDIAN_TILE: u8 = 0x5f;
pub const BLACKTHORN_RESCUE_SPECTRAL_ACTOR_BYTE: u8 = 0x74;
pub const BLACKTHORN_RESCUE_SPECTRAL_ATLAS_INDEX: u16 = 0x174;
pub const BLACKTHORN_RESCUE_PARTY_ATLAS_INDEX: u16 = 0x11c;

/// `blackthorn.md` section 6: clean semantic placements for the named
/// cutscene-VM actor slots. Actor bytes index the upper atlas bank;
/// `0x16` is the actor-storage draw-nothing sentinel, not atlas tile
/// `0x116`.
pub const BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS: [BlackthornCutsceneActorPlacement; 5] = [
    BlackthornCutsceneActorPlacement {
        actor: BlackthornCutsceneActor::Avatar,
        type_byte: crate::PLAYER_TILE,
        tile: crate::PLAYER_TILE,
        x: 5,
        y: 9,
    },
    BlackthornCutsceneActorPlacement {
        actor: BlackthornCutsceneActor::SecondPartyMember,
        type_byte: crate::PLAYER_TILE,
        tile: crate::PLAYER_TILE,
        x: 6,
        y: 9,
    },
    BlackthornCutsceneActorPlacement {
        actor: BlackthornCutsceneActor::LeftGuard,
        type_byte: BLACKTHORN_GUARD_ACTOR_BYTE,
        tile: BLACKTHORN_GUARD_ACTOR_BYTE,
        x: 4,
        y: 10,
    },
    BlackthornCutsceneActorPlacement {
        actor: BlackthornCutsceneActor::RightGuard,
        type_byte: BLACKTHORN_GUARD_ACTOR_BYTE,
        tile: BLACKTHORN_GUARD_ACTOR_BYTE,
        x: 6,
        y: 10,
    },
    BlackthornCutsceneActorPlacement {
        actor: BlackthornCutsceneActor::SeatedBlackthorn,
        type_byte: BLACKTHORN_SUPPRESSED_ACTOR_BYTE,
        tile: BLACKTHORN_SUPPRESSED_ACTOR_BYTE,
        x: 5,
        y: 5,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlackthornCutsceneActorState {
    pub x: usize,
    pub y: usize,
    pub actor_byte: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornCutscenePresentationEvent {
    WorldTick,
    BiosTick,
    Stinger,
    TerrainBuffered {
        x: usize,
        y: usize,
        tile: u8,
    },
    ActorMoved {
        actor: BlackthornCutsceneActor,
        x: usize,
        y: usize,
    },
    ActorCleared(BlackthornCutsceneActor),
    CellReveal {
        actor: BlackthornCutsceneActor,
        x: usize,
        y: usize,
        atlas_index: u16,
    },
    ActorByteAssigned {
        actor: BlackthornCutsceneActor,
        actor_byte: u8,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlackthornCellRevealPlayback {
    pub cell: (u8, u8),
    pub atlas_index: u16,
    pub pixel_order: Vec<(u8, u8)>,
    pub world_tick_after_operations: Vec<u16>,
}

pub fn blackthorn_cell_reveal_playback(
    cell: (u8, u8),
    atlas_index: u16,
) -> BlackthornCellRevealPlayback {
    BlackthornCellRevealPlayback {
        cell,
        atlas_index,
        pixel_order: crate::combat_terrain_reveal_pixel_order(),
        world_tick_after_operations: (1..=crate::COMBAT_TERRAIN_REVEAL_WORLD_TICKS)
            .map(|tick| u16::from(tick) * 8)
            .collect(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornCutsceneCommand {
    End,
    SetRepeat(u8),
    SetPairedMovement {
        actor: BlackthornCutsceneActor,
        direction: crate::Direction,
    },
    SetPerStepPause(bool),
    QuietRedrawPause(u8),
    WriteTerrain {
        x: usize,
        y: usize,
        tile: u8,
    },
    ExplicitRedraw,
    StingerPause,
    RevealActor {
        actor: BlackthornCutsceneActor,
        x: usize,
        y: usize,
        atlas_index: u16,
        actor_byte: u8,
    },
    ClearActor(BlackthornCutsceneActor),
    MoveActor {
        actor: BlackthornCutsceneActor,
        direction: crate::Direction,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornCutsceneBeat {
    PerQuestionIntermission,
    FailedChallengeReaction,
    AudienceThroneApproach,
    GuardReleaseRoute,
    ConditionalThroneCleanup,
}

pub const BLACKTHORN_CUTSCENE_PER_QUESTION_INTERMISSION: [BlackthornCutsceneCommand; 18] = [
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::West,
    },
    BlackthornCutsceneCommand::WriteTerrain {
        x: 0,
        y: 4,
        tile: BLACKTHORN_COBBLE_TILE,
    },
    BlackthornCutsceneCommand::ExplicitRedraw,
    BlackthornCutsceneCommand::SetRepeat(2),
    BlackthornCutsceneCommand::SetPairedMovement {
        actor: BlackthornCutsceneActor::Avatar,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::South,
    },
    BlackthornCutsceneCommand::WriteTerrain {
        x: 0,
        y: 4,
        tile: BLACKTHORN_LOCKED_DOOR_TILE,
    },
    BlackthornCutsceneCommand::StingerPause,
    BlackthornCutsceneCommand::SetRepeat(2),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::SeatedBlackthorn,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::ClearActor(BlackthornCutsceneActor::SeatedBlackthorn),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::South,
    },
    BlackthornCutsceneCommand::ClearActor(BlackthornCutsceneActor::LeftGuard),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::RightGuard,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::ClearActor(BlackthornCutsceneActor::RightGuard),
    BlackthornCutsceneCommand::SetRepeat(6),
    BlackthornCutsceneCommand::StingerPause,
    BlackthornCutsceneCommand::End,
];

pub const BLACKTHORN_CUTSCENE_FAILED_CHALLENGE_REACTION: [BlackthornCutsceneCommand; 21] = [
    BlackthornCutsceneCommand::QuietRedrawPause(22),
    BlackthornCutsceneCommand::SetRepeat(5),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::East,
    },
    BlackthornCutsceneCommand::SetRepeat(3),
    BlackthornCutsceneCommand::SetPairedMovement {
        actor: BlackthornCutsceneActor::SecondPartyMember,
        direction: crate::Direction::South,
    },
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::South,
    },
    BlackthornCutsceneCommand::QuietRedrawPause(3),
    BlackthornCutsceneCommand::WriteTerrain {
        x: 5,
        y: 7,
        tile: BLACKTHORN_PENDULUM_TILE,
    },
    BlackthornCutsceneCommand::ClearActor(BlackthornCutsceneActor::SecondPartyMember),
    BlackthornCutsceneCommand::ExplicitRedraw,
    BlackthornCutsceneCommand::SetRepeat(3),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::SetRepeat(5),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::West,
    },
    BlackthornCutsceneCommand::QuietRedrawPause(12),
    BlackthornCutsceneCommand::SetRepeat(3),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::RightGuard,
        direction: crate::Direction::West,
    },
    BlackthornCutsceneCommand::WriteTerrain {
        x: 5,
        y: 9,
        tile: BLACKTHORN_HOURGLASS_TILE,
    },
    BlackthornCutsceneCommand::SetRepeat(3),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::RightGuard,
        direction: crate::Direction::East,
    },
    BlackthornCutsceneCommand::End,
];

pub const BLACKTHORN_CUTSCENE_AUDIENCE_THRONE_APPROACH: [BlackthornCutsceneCommand; 10] = [
    BlackthornCutsceneCommand::StingerPause,
    BlackthornCutsceneCommand::SetPairedMovement {
        actor: BlackthornCutsceneActor::RightGuard,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::SetRepeat(3),
    BlackthornCutsceneCommand::SetPairedMovement {
        actor: BlackthornCutsceneActor::RightGuard,
        direction: crate::Direction::East,
    },
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::West,
    },
    BlackthornCutsceneCommand::QuietRedrawPause(8),
    BlackthornCutsceneCommand::RevealActor {
        actor: BlackthornCutsceneActor::SeatedBlackthorn,
        x: 5,
        y: 5,
        atlas_index: BLACKTHORN_SEATED_ATLAS_INDEX,
        actor_byte: BLACKTHORN_SEATED_ACTOR_BYTE,
    },
    BlackthornCutsceneCommand::QuietRedrawPause(8),
    BlackthornCutsceneCommand::End,
];

pub const BLACKTHORN_CUTSCENE_GUARD_RELEASE_ROUTE: [BlackthornCutsceneCommand; 6] = [
    BlackthornCutsceneCommand::QuietRedrawPause(11),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::West,
    },
    BlackthornCutsceneCommand::SetRepeat(4),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::LeftGuard,
        direction: crate::Direction::East,
    },
    BlackthornCutsceneCommand::End,
];

pub const BLACKTHORN_CUTSCENE_CONDITIONAL_THRONE_CLEANUP: [BlackthornCutsceneCommand; 7] = [
    BlackthornCutsceneCommand::QuietRedrawPause(4),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::SeatedBlackthorn,
        direction: crate::Direction::East,
    },
    BlackthornCutsceneCommand::SetRepeat(5),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::SeatedBlackthorn,
        direction: crate::Direction::South,
    },
    BlackthornCutsceneCommand::ClearActor(BlackthornCutsceneActor::SeatedBlackthorn),
    BlackthornCutsceneCommand::ExplicitRedraw,
    BlackthornCutsceneCommand::End,
];

pub const fn blackthorn_cutscene_beat_commands(
    beat: BlackthornCutsceneBeat,
) -> &'static [BlackthornCutsceneCommand] {
    match beat {
        BlackthornCutsceneBeat::PerQuestionIntermission => {
            &BLACKTHORN_CUTSCENE_PER_QUESTION_INTERMISSION
        }
        BlackthornCutsceneBeat::FailedChallengeReaction => {
            &BLACKTHORN_CUTSCENE_FAILED_CHALLENGE_REACTION
        }
        BlackthornCutsceneBeat::AudienceThroneApproach => {
            &BLACKTHORN_CUTSCENE_AUDIENCE_THRONE_APPROACH
        }
        BlackthornCutsceneBeat::GuardReleaseRoute => &BLACKTHORN_CUTSCENE_GUARD_RELEASE_ROUTE,
        BlackthornCutsceneBeat::ConditionalThroneCleanup => {
            &BLACKTHORN_CUTSCENE_CONDITIONAL_THRONE_CLEANUP
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlackthornCutsceneVm {
    pub actors: [Option<BlackthornCutsceneActorState>; BLACKTHORN_CUTSCENE_ACTOR_SLOT_COUNT],
    pub visible_actors:
        [Option<BlackthornCutsceneActorState>; BLACKTHORN_CUTSCENE_ACTOR_SLOT_COUNT],
    pub tile_buffer: Vec<u8>,
    pub visible_tile_buffer: Vec<u8>,
    pub presentation_events: Vec<BlackthornCutscenePresentationEvent>,
    pub cell_reveals: Vec<BlackthornCellRevealPlayback>,
    pub world_ticks: u16,
    pub bios_ticks: u16,
    pub stinger_count: u16,
    pub ended: bool,
    repeat_count: u8,
    paired_movement: Option<(BlackthornCutsceneActor, crate::Direction)>,
    per_step_pause: bool,
    cinematic_animation_enabled: bool,
}

pub const BLACKTHORN_CUTSCENE_ACTOR_SLOT_COUNT: usize = 9;

impl BlackthornCutsceneVm {
    pub fn new(tile_buffer: Vec<u8>) -> Self {
        let visible_tile_buffer = tile_buffer.clone();
        Self {
            actors: [None; BLACKTHORN_CUTSCENE_ACTOR_SLOT_COUNT],
            visible_actors: [None; BLACKTHORN_CUTSCENE_ACTOR_SLOT_COUNT],
            tile_buffer,
            visible_tile_buffer,
            presentation_events: Vec::new(),
            cell_reveals: Vec::new(),
            world_ticks: 0,
            bios_ticks: 0,
            stinger_count: 0,
            ended: false,
            repeat_count: 1,
            paired_movement: None,
            per_step_pause: true,
            cinematic_animation_enabled: true,
        }
    }

    pub fn with_audience_setup(tile_buffer: Vec<u8>) -> Self {
        let mut vm = Self::new(tile_buffer);
        for placement in BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS {
            vm.set_actor(
                placement.actor,
                BlackthornCutsceneActorState {
                    x: placement.x,
                    y: placement.y,
                    actor_byte: placement.tile,
                },
            );
        }
        vm.visible_actors = vm.actors;
        vm
    }

    pub fn actor(&self, actor: BlackthornCutsceneActor) -> Option<BlackthornCutsceneActorState> {
        self.actors
            .get(actor.slot_index() as usize)
            .and_then(|actor| *actor)
    }

    pub fn set_actor(
        &mut self,
        actor: BlackthornCutsceneActor,
        state: BlackthornCutsceneActorState,
    ) {
        self.actors[actor.slot_index() as usize] = Some(state);
    }

    pub fn tile(&self, x: usize, y: usize, width: usize) -> Option<u8> {
        let index = y.checked_mul(width)?.checked_add(x)?;
        self.tile_buffer.get(index).copied()
    }

    pub fn step(&mut self, command: BlackthornCutsceneCommand) {
        if self.ended {
            return;
        }
        match command {
            BlackthornCutsceneCommand::End => {
                self.ended = true;
            }
            BlackthornCutsceneCommand::SetRepeat(count) => {
                self.repeat_count = count.max(1);
            }
            BlackthornCutsceneCommand::SetPairedMovement { actor, direction } => {
                self.paired_movement = Some((actor, direction));
            }
            BlackthornCutsceneCommand::SetPerStepPause(enabled) => {
                self.per_step_pause = enabled;
            }
            BlackthornCutsceneCommand::QuietRedrawPause(ticks) => {
                self.quiet_redraw_pause(ticks);
                self.repeat_count = 1;
            }
            BlackthornCutsceneCommand::WriteTerrain { x, y, tile } => {
                if let Some(index) = blackthorn_cutscene_tile_index(x, y) {
                    if let Some(cell) = self.tile_buffer.get_mut(index) {
                        *cell = tile;
                        self.presentation_events.push(
                            BlackthornCutscenePresentationEvent::TerrainBuffered { x, y, tile },
                        );
                    }
                }
            }
            BlackthornCutsceneCommand::ExplicitRedraw => {
                self.redraw_world_tick();
            }
            BlackthornCutsceneCommand::StingerPause => {
                self.stinger_pause();
            }
            BlackthornCutsceneCommand::RevealActor {
                actor,
                x,
                y,
                atlas_index,
                actor_byte,
            } => {
                self.reveal_actor(actor, x, y, atlas_index, actor_byte);
            }
            BlackthornCutsceneCommand::ClearActor(actor) => {
                self.actors[actor.slot_index() as usize] = None;
                self.presentation_events
                    .push(BlackthornCutscenePresentationEvent::ActorCleared(actor));
            }
            BlackthornCutsceneCommand::MoveActor { actor, direction } => {
                let repeat_count = self.repeat_count;
                let paired = self.paired_movement.take();
                for _ in 0..repeat_count {
                    self.move_actor_one_step(actor, direction);
                    if let Some((second_actor, second_direction)) = paired {
                        self.move_actor_one_step(second_actor, second_direction);
                    }
                    if self.per_step_pause {
                        let saved_repeat = self.repeat_count;
                        self.repeat_count = 1;
                        self.stinger_pause();
                        self.repeat_count = saved_repeat;
                    }
                }
                self.repeat_count = 1;
            }
        }
    }

    pub fn run(&mut self, commands: &[BlackthornCutsceneCommand]) {
        for command in commands {
            self.step(*command);
            if self.ended {
                break;
            }
        }
    }

    fn move_actor_one_step(&mut self, actor: BlackthornCutsceneActor, direction: crate::Direction) {
        let Some(mut state) = self.actor(actor) else {
            return;
        };
        match direction {
            crate::Direction::North => state.y = state.y.saturating_sub(1),
            crate::Direction::East => state.x = state.x.saturating_add(1),
            crate::Direction::South => state.y = state.y.saturating_add(1),
            crate::Direction::West => state.x = state.x.saturating_sub(1),
            _ => return,
        }
        self.set_actor(actor, state);
        self.presentation_events
            .push(BlackthornCutscenePresentationEvent::ActorMoved {
                actor,
                x: state.x,
                y: state.y,
            });
    }

    fn redraw_world_tick(&mut self) {
        self.world_ticks = self.world_ticks.saturating_add(1);
        self.visible_tile_buffer.clone_from(&self.tile_buffer);
        self.visible_actors = self.actors;
        self.presentation_events
            .push(BlackthornCutscenePresentationEvent::WorldTick);
    }

    fn quiet_redraw_pause(&mut self, ticks: u8) {
        if !self.cinematic_animation_enabled {
            return;
        }
        for _ in 0..ticks {
            self.redraw_world_tick();
            self.bios_ticks = self.bios_ticks.saturating_add(1);
            self.presentation_events
                .push(BlackthornCutscenePresentationEvent::BiosTick);
        }
    }

    fn stinger_pause(&mut self) {
        let repeat = self.repeat_count;
        for _ in 0..repeat {
            self.stinger_count = self.stinger_count.saturating_add(1);
            self.presentation_events
                .push(BlackthornCutscenePresentationEvent::Stinger);
            self.quiet_redraw_pause(2);
        }
        self.repeat_count = 1;
    }

    fn reveal_actor(
        &mut self,
        actor: BlackthornCutsceneActor,
        x: usize,
        y: usize,
        atlas_index: u16,
        actor_byte: u8,
    ) {
        self.set_actor(
            actor,
            BlackthornCutsceneActorState {
                x,
                y,
                actor_byte: BLACKTHORN_SUPPRESSED_ACTOR_BYTE,
            },
        );
        self.visible_actors[actor.slot_index() as usize] = None;
        self.cell_reveals.push(blackthorn_cell_reveal_playback(
            (x as u8, y as u8),
            atlas_index,
        ));
        self.presentation_events
            .push(BlackthornCutscenePresentationEvent::CellReveal {
                actor,
                x,
                y,
                atlas_index,
            });
        self.world_ticks = self
            .world_ticks
            .saturating_add(u16::from(crate::COMBAT_TERRAIN_REVEAL_WORLD_TICKS));
        let state = BlackthornCutsceneActorState { x, y, actor_byte };
        self.set_actor(actor, state);
        self.visible_actors[actor.slot_index() as usize] = Some(state);
        self.presentation_events
            .push(BlackthornCutscenePresentationEvent::ActorByteAssigned { actor, actor_byte });
    }
}

pub fn blackthorn_cutscene_tile_index(x: usize, y: usize) -> Option<usize> {
    (x < crate::MISCMAPS_CUTSCENE_VISIBLE_COLUMNS && y < crate::MISCMAPS_CUTSCENE_ROWS)
        .then(|| y * crate::MISCMAPS_CUTSCENE_VISIBLE_COLUMNS + x)
}

/// `blackthorn.md §6`: classify a cutscene-VM actor slot byte.
/// Returns `None` for indices outside the published role table; the
/// script VM treats those as caller-private temporaries rather than
/// named actors.
pub const fn blackthorn_cutscene_actor(slot: u8) -> Option<BlackthornCutsceneActor> {
    Some(match slot {
        0 => BlackthornCutsceneActor::Avatar,
        1 => BlackthornCutsceneActor::SecondPartyMember,
        6 => BlackthornCutsceneActor::LeftGuard,
        7 => BlackthornCutsceneActor::RightGuard,
        8 => BlackthornCutsceneActor::SeatedBlackthorn,
        _ => return None,
    })
}

/// `blackthorn.md §3`: scene byte the audience cinematic hands the
/// party off to after the throne cleanup beat. The captive cell
/// sits inside Lord Blackthorn's Castle (scene byte 18). Anchored
/// to [`crate::SCENE_LORD_BLACKTHORNS_CASTLE`] so the captive-cell
/// scene reference and the named castle-scene anchor share one
/// source of truth.
pub const BLACKTHORN_CAPTIVE_CELL_SCENE: u8 = crate::SCENE_LORD_BLACKTHORNS_CASTLE;

/// `blackthorn.md §3`: local cell (X, Y) inside
/// `BLACKTHORN_CAPTIVE_CELL_SCENE` the audience hand-off seeds the
/// party at.
pub const BLACKTHORN_CAPTIVE_CELL_X: u8 = 10;
pub const BLACKTHORN_CAPTIVE_CELL_Y: u8 = 7;

/// `blackthorn.md §2` two player-visible Blackthorn cinematic
/// families. Both replace the ordinary map loop and hand control
/// back through an explicit scene/position transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornEntryFamily {
    /// Audience/capture: party is subdued, challenged, and routed to
    /// captivity or release. Traced direct entry path is the town
    /// post-action NPC event cleanup's arrest/unconscious branch.
    AudienceCapture,
    /// Rescue/refuge: darkness-and-thunder cinematic that restores
    /// the party and moves it to the refuge scene. Reachable from
    /// town, overworld, and dungeon modes.
    RescueRefuge,
}

/// `blackthorn.md §3` / `formats/location-dat.md §11`: the audience
/// capture presentation loads cutscene-map record 0 from `MISCMAPS.DAT`.
pub const BLACKTHORN_AUDIENCE_CUTSCENE_MAP_RECORD: usize = 0;

/// `blackthorn.md §5` failure-reaction victim slot. When a
/// punishable challenge branch fails, the failure beat names the
/// party's second visible member (zero-based slot index `1`) as the
/// dragged-away victim. Compatibility code should preserve the
/// fixed slot index rather than searching for a "first non-leader"
/// member.
pub const BLACKTHORN_FAILURE_VICTIM_SLOT: usize = 1;

/// `blackthorn.md §4`: Blackthorn challenge prompt input limit.
pub const BLACKTHORN_CHALLENGE_INPUT_LIMIT: usize = 14;
/// `blackthorn.md §4`: number of prompt ordinals the challenge loop
/// can ask. "It can ask up to four prompts", and "**The loop asks
/// about ONE shrine, up to four times.** ... The four prompt ordinals
/// change only the *wording*". The ordinal is therefore a wording
/// selector, never an answer selector.
pub const BLACKTHORN_CHALLENGE_PROMPT_COUNT: usize = 4;

/// `blackthorn.md §4`: case-insensitive substring match of the
/// player's typed answer against the expected mantra. The expected
/// word may appear anywhere in the typed buffer rather than being the
/// entire input.
/// `blackthorn.md §4` shrine virtue/mantra table, in shrine order.
///
/// "**The expected answer is the selected shrine's mantra, and it is
/// the same on all four prompts.** All eight virtue/mantra pairs are
/// live". The withdrawal box in the same section retires the earlier
/// readings that the answer lookup was "indexed by prompt ordinal
/// rather than by party slot" and that "this traced challenge loop
/// only iterates the first four ordinals": the answer is indexed by
/// shrine, and the four ordinals are four wordings of one question.
///
/// The shrine order is the one `blackthorn.md §3` step 2 scans when it
/// selects the interrogated shrine from the eight shrine ruin flags.
pub const BLACKTHORN_SHRINE_MANTRAS: [(&str, &str); 8] = [
    ("Honesty", "Ahm"),
    ("Compassion", "Mu"),
    ("Valour", "Ra"),
    ("Justice", "Beh"),
    ("Sacrifice", "Cah"),
    ("Honor", "Summ"),
    ("Spirituality", "Om"),
    ("Humility", "Lum"),
];

/// `blackthorn.md §3,§4` number of shrines the eight-slot scan selects
/// from, and therefore the length of [`BLACKTHORN_SHRINE_MANTRAS`].
pub const BLACKTHORN_SHRINE_COUNT: usize = BLACKTHORN_SHRINE_MANTRAS.len();

/// `blackthorn.md §4`: returns the (virtue, accepted-answer) pair for
/// a shrine index `0..=7`, or `None` for an out-of-range index.
pub const fn blackthorn_shrine_mantra(shrine_index: u8) -> Option<(&'static str, &'static str)> {
    if (shrine_index as usize) >= BLACKTHORN_SHRINE_COUNT {
        None
    } else {
        Some(BLACKTHORN_SHRINE_MANTRAS[shrine_index as usize])
    }
}

/// `blackthorn.md §4` escalating wording of the one repeated question.
/// "The four prompt ordinals change only the *wording*, which escalates
/// from a plain question, to a repeat, to an impatient demand, to a
/// shouted final demand." The wording text itself is data-owned
/// (`MISCMSG.DAT`), so only the escalation step is modelled here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornChallengeWording {
    /// Ordinal 0 — the plain question.
    Plain,
    /// Ordinal 1 — the repeat.
    Repeat,
    /// Ordinal 2 — the impatient demand.
    ImpatientDemand,
    /// Ordinal 3 — the shouted final demand.
    ShoutedFinalDemand,
}

/// `blackthorn.md §4`: the wording for one prompt ordinal `0..=3`.
/// Returns `None` past the fourth prompt, which is where the loop ends.
pub const fn blackthorn_challenge_wording(ordinal: u8) -> Option<BlackthornChallengeWording> {
    Some(match ordinal {
        0 => BlackthornChallengeWording::Plain,
        1 => BlackthornChallengeWording::Repeat,
        2 => BlackthornChallengeWording::ImpatientDemand,
        3 => BlackthornChallengeWording::ShoutedFinalDemand,
        _ => return None,
    })
}

pub fn blackthorn_challenge_answer_matches(typed: &str, expected_mantra: &str) -> bool {
    let typed_upper = typed.to_ascii_uppercase();
    let expected_upper = expected_mantra.to_ascii_uppercase();
    typed_upper.contains(&expected_upper)
}

/// `blackthorn.md §4`: prompt input accepts at most fourteen typed
/// characters before the case-insensitive substring comparison runs.
/// Trimming matches the play prompt path, which treats blank answers as
/// a prompt repeat rather than as a failed challenge answer.
pub fn blackthorn_challenge_limited_input(typed: &str) -> String {
    typed
        .trim()
        .chars()
        .take(BLACKTHORN_CHALLENGE_INPUT_LIMIT)
        .collect()
}

/// `formats/karma-dat.md §4`: Lord British-in-disguise camp event
/// verdict-record selector. Uses the same twenty-point band scale for
/// the lower range, selecting records `0..=3` for bands below 80; for
/// the top band (`80..=99`) it seeks directly to record 5. Record 4 is
/// not selected by this event.
pub const fn lord_british_camp_verdict_record(standing: u8) -> u8 {
    match standing {
        0..=19 => 0,
        20..=39 => 1,
        40..=59 => 2,
        60..=79 => 3,
        _ => 5,
    }
}

/// `formats/karma-dat.md §3` semantic tier label for a `KARMA.DAT`
/// record index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KarmaDatTier {
    /// Record 0 — addressed to an avatar who has strayed.
    Lowest,
    /// Record 1 — corrective speech.
    Low,
    /// Record 2 — middle "you have potential".
    Middle,
    /// Record 3 — high; praises the work but flags more remains.
    High,
    /// Record 4 — highest; declares the avatar's destiny.
    Highest,
    /// Record 5 — short near-variant of record 4 used by the Lord
    /// British camp event's top band.
    HighestCampVariant,
}

/// `formats/karma-dat.md §3`: classify a record index `0..=5` into
/// its semantic tier label. Returns `None` for indices outside the
/// six-record file.
pub const fn karma_dat_tier(record_index: usize) -> Option<KarmaDatTier> {
    Some(match record_index {
        0 => KarmaDatTier::Lowest,
        1 => KarmaDatTier::Low,
        2 => KarmaDatTier::Middle,
        3 => KarmaDatTier::High,
        4 => KarmaDatTier::Highest,
        5 => KarmaDatTier::HighestCampVariant,
        _ => return None,
    })
}

/// `blackthorn.md §7` / `formats/karma-dat.md §4` shared band width
/// for the `KARMA.DAT` twenty-point selector. Both the rescue/refuge
/// path and the Lord British-in-disguise camp verdict path divide
/// the one-byte standing input into bands of this width before
/// indexing the per-band record. Promote it so the band edges are
/// not encoded as bare literal pairs at each call site.
pub const KARMA_DAT_BAND_WIDTH: u8 = 20;

/// `blackthorn.md §7`: rescue/refuge `KARMA.DAT` verdict band selector.
/// Divides the one-byte standing input into five twenty-point bands and
/// returns the matching record index `0..=4`. The shipped sixth record
/// is not selected by this rescue/refuge table; values `>= 100` clamp
/// to the top band, since the moral-standing selector caps at 99.
pub const fn blackthorn_rescue_verdict_record(standing: u8) -> u8 {
    match standing {
        0..=19 => 0,
        20..=39 => 1,
        40..=59 => 2,
        60..=79 => 3,
        _ => 4,
    }
}

/// `karma.md §6` rescue/refuge post-print standing bump. After the
/// rescue path prints its selected verdict record, the moral-standing
/// selector is raised to at least [`BLACKTHORN_RESCUE_STANDING_FLOOR`].
/// Returns the input when it already meets or exceeds the floor.
pub const fn blackthorn_rescue_post_print_standing(standing: u8) -> u8 {
    if standing < BLACKTHORN_RESCUE_STANDING_FLOOR {
        BLACKTHORN_RESCUE_STANDING_FLOOR
    } else {
        standing
    }
}

#[cfg(test)]
mod guard_demand_tests {
    use super::*;

    #[test]
    fn scene_dispatch_requires_badge_aura_only_at_the_palace() {
        assert_eq!(
            begin_blackthorn_guard_demand(crate::SCENE_LORD_BLACKTHORNS_CASTLE, false, 3),
            BlackthornGuardDemandStart::Refused
        );
        assert_eq!(
            begin_blackthorn_guard_demand(crate::SCENE_LORD_BLACKTHORNS_CASTLE, true, 3),
            BlackthornGuardDemandStart::Prompt(BlackthornGuardDemandPrompt::PalacePassword)
        );
        assert_eq!(
            begin_blackthorn_guard_demand(crate::SCENE_MINOC, false, 3),
            BlackthornGuardDemandStart::Prompt(BlackthornGuardDemandPrompt::MinocCharity)
        );
        assert_eq!(
            begin_blackthorn_guard_demand(crate::SCENE_JHELOM, false, 3),
            BlackthornGuardDemandStart::Prompt(BlackthornGuardDemandPrompt::Tribute { amount: 30 })
        );
    }

    #[test]
    fn palace_password_uses_case_insensitive_four_character_prefix() {
        for accepted in ["IMPE", "impera", "ImPeachment"] {
            assert_eq!(
                resolve_blackthorn_guard_demand(
                    BlackthornGuardDemandPrompt::PalacePassword,
                    accepted,
                    99
                ),
                BlackthornGuardDemandResolution::PaidOrPassed { gold: 99 }
            );
        }
        assert_eq!(
            resolve_blackthorn_guard_demand(
                BlackthornGuardDemandPrompt::PalacePassword,
                "IMPA",
                99
            ),
            BlackthornGuardDemandResolution::Refused { gold: 99 }
        );
    }

    #[test]
    fn charity_and_tribute_mutate_only_gold_on_accepted_affordable_input() {
        assert_eq!(
            resolve_blackthorn_guard_demand(BlackthornGuardDemandPrompt::MinocCharity, "yes", 101),
            BlackthornGuardDemandResolution::PaidOrPassed { gold: 50 }
        );
        assert_eq!(
            resolve_blackthorn_guard_demand(
                BlackthornGuardDemandPrompt::Tribute { amount: 30 },
                "Y",
                30
            ),
            BlackthornGuardDemandResolution::PaidOrPassed { gold: 0 }
        );
        assert_eq!(
            resolve_blackthorn_guard_demand(
                BlackthornGuardDemandPrompt::Tribute { amount: 30 },
                "Y",
                29
            ),
            BlackthornGuardDemandResolution::Refused { gold: 29 }
        );
        assert_eq!(
            resolve_blackthorn_guard_demand(
                BlackthornGuardDemandPrompt::Tribute { amount: 30 },
                "later",
                99
            ),
            BlackthornGuardDemandResolution::AwaitingInput
        );
    }
}
