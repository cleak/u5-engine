//! Blackthorn cutscene helpers per `blackthorn.md` section 7.

use std::fs;
use std::io;
use std::path::Path;

/// Clean-engine companion save file for Blackthorn story state whose
/// exact original `SAVED.GAM` byte offsets are not yet public. The
/// main save image remains byte-preserving for unknown fields; this
/// sidecar carries only clean semantic state named by
/// `systems/blackthorn.md` section 8.
pub const BLACKTHORN_STORY_STATE_FILE: &str = "SAVED.BTH";
pub const BLACKTHORN_STORY_STATE_MAGIC: [u8; 4] = *b"BTH1";
pub const BLACKTHORN_STORY_STATE_LEN: usize = 9;
pub const BLACKTHORN_CAPTURE_CONTEXT_NONE: u8 = 0;

/// `blackthorn.md` section 8 durable capture/rescue state. Jailed or handled
/// party-member flags are represented as roster-slot bits so the state
/// survives mode changes and save/load without depending on current
/// marching order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlackthornStoryState {
    pub jailed_slots_mask: u16,
    pub captive_cell_counter: u8,
    pub rescue_progression: u8,
    pub capture_context: u8,
}

impl BlackthornStoryState {
    pub const fn is_party_slot_jailed(self, slot: u8) -> bool {
        if slot >= crate::SAVE_ROSTER_SLOT_COUNT as u8 {
            false
        } else {
            (self.jailed_slots_mask & (1u16 << slot)) != 0
        }
    }

    pub fn mark_party_slot_jailed(&mut self, slot: u8) -> bool {
        if slot >= crate::SAVE_ROSTER_SLOT_COUNT as u8 {
            return false;
        }
        let bit = 1u16 << slot;
        let was_clear = (self.jailed_slots_mask & bit) == 0;
        self.jailed_slots_mask |= bit;
        was_clear
    }

    pub fn clear_jailed_party_slots(&mut self) {
        self.jailed_slots_mask = 0;
    }

    pub fn jailed_party_slots(self) -> Vec<u8> {
        (0..crate::SAVE_ROSTER_SLOT_COUNT as u8)
            .filter(|slot| self.is_party_slot_jailed(*slot))
            .collect()
    }

    pub fn encoded(self) -> [u8; BLACKTHORN_STORY_STATE_LEN] {
        let mut bytes = [0; BLACKTHORN_STORY_STATE_LEN];
        bytes[0..4].copy_from_slice(&BLACKTHORN_STORY_STATE_MAGIC);
        bytes[4..6].copy_from_slice(&self.jailed_slots_mask.to_le_bytes());
        bytes[6] = self.captive_cell_counter;
        bytes[7] = self.rescue_progression;
        bytes[8] = self.capture_context;
        bytes
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != BLACKTHORN_STORY_STATE_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{BLACKTHORN_STORY_STATE_FILE} must be {BLACKTHORN_STORY_STATE_LEN} bytes, got {}",
                    bytes.len()
                ),
            ));
        }
        if bytes[0..4] != BLACKTHORN_STORY_STATE_MAGIC[..] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{BLACKTHORN_STORY_STATE_FILE} has an invalid signature"),
            ));
        }
        Ok(Self {
            jailed_slots_mask: u16::from_le_bytes([bytes[4], bytes[5]]),
            captive_cell_counter: bytes[6],
            rescue_progression: bytes[7],
            capture_context: bytes[8],
        })
    }
}

pub fn load_blackthorn_story_state(game_dir: &Path) -> io::Result<BlackthornStoryState> {
    match fs::read(game_dir.join(BLACKTHORN_STORY_STATE_FILE)) {
        Ok(bytes) => BlackthornStoryState::decode(&bytes),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(BlackthornStoryState::default()),
        Err(err) => Err(err),
    }
}

pub fn write_blackthorn_story_state(
    game_dir: &Path,
    state: BlackthornStoryState,
) -> io::Result<()> {
    fs::write(game_dir.join(BLACKTHORN_STORY_STATE_FILE), state.encoded())
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
    /// Slot 6: Blackthorn.
    Blackthorn,
    /// Slot 7: attendant or guard.
    Attendant,
    /// Slot 8: throne or throne-marker tile.
    Throne,
}

impl BlackthornCutsceneActor {
    /// `blackthorn.md` section 6: returns the cinematic actor slot index
    /// the script VM uses for this role.
    pub const fn slot_index(self) -> u8 {
        match self {
            Self::Avatar => 0,
            Self::SecondPartyMember => 1,
            Self::Blackthorn => 6,
            Self::Attendant => 7,
            Self::Throne => 8,
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
pub const BLACKTHORN_CUTSCENE_SECOND_PARTY_TYPE: u8 = 0xf1;
pub const BLACKTHORN_CUTSCENE_BLACKTHORN_TYPE: u8 = 0xf2;
pub const BLACKTHORN_CUTSCENE_ATTENDANT_TYPE: u8 = 0xf3;
pub const BLACKTHORN_CUTSCENE_THRONE_TYPE: u8 = 0xf4;

/// `blackthorn.md` section 6: clean semantic placements for the named
/// cutscene-VM actor slots. Exact tile art and byte-script pixel
/// parity remain visual work, so non-Avatar roles use hidden tiles
/// with distinct nonzero type tags instead of claiming final art ids.
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
        type_byte: BLACKTHORN_CUTSCENE_SECOND_PARTY_TYPE,
        tile: crate::NPC_HIDDEN_SPRITE_TILE,
        x: 6,
        y: 9,
    },
    BlackthornCutsceneActorPlacement {
        actor: BlackthornCutsceneActor::Blackthorn,
        type_byte: BLACKTHORN_CUTSCENE_BLACKTHORN_TYPE,
        tile: crate::NPC_HIDDEN_SPRITE_TILE,
        x: 5,
        y: 3,
    },
    BlackthornCutsceneActorPlacement {
        actor: BlackthornCutsceneActor::Attendant,
        type_byte: BLACKTHORN_CUTSCENE_ATTENDANT_TYPE,
        tile: crate::NPC_HIDDEN_SPRITE_TILE,
        x: 6,
        y: 3,
    },
    BlackthornCutsceneActorPlacement {
        actor: BlackthornCutsceneActor::Throne,
        type_byte: BLACKTHORN_CUTSCENE_THRONE_TYPE,
        tile: crate::NPC_HIDDEN_SPRITE_TILE,
        x: 5,
        y: 2,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlackthornCutsceneActorState {
    pub x: usize,
    pub y: usize,
    pub visible: bool,
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
    OutputByte(u8),
    WriteTile {
        x: usize,
        y: usize,
        tile: u8,
    },
    ClearScreen,
    TimedPause(u8),
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
    BlackthornRises,
    ConditionalThroneCleanup,
}

pub const BLACKTHORN_CUTSCENE_TEMP_TILE_A: u8 = 0x01;
pub const BLACKTHORN_CUTSCENE_TEMP_TILE_B: u8 = 0x02;
pub const BLACKTHORN_CUTSCENE_FORMAT_OUTPUT: u8 = 0x0d;

pub const BLACKTHORN_CUTSCENE_PER_QUESTION_INTERMISSION: [BlackthornCutsceneCommand; 10] = [
    BlackthornCutsceneCommand::WriteTile {
        x: 5,
        y: 5,
        tile: BLACKTHORN_CUTSCENE_TEMP_TILE_A,
    },
    BlackthornCutsceneCommand::SetRepeat(2),
    BlackthornCutsceneCommand::SetPairedMovement {
        actor: BlackthornCutsceneActor::Avatar,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::SetPerStepPause(true),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::Blackthorn,
        direction: crate::Direction::South,
    },
    BlackthornCutsceneCommand::SetRepeat(2),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::Throne,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::ClearActor(BlackthornCutsceneActor::Throne),
    BlackthornCutsceneCommand::ClearActor(BlackthornCutsceneActor::Blackthorn),
    BlackthornCutsceneCommand::ClearActor(BlackthornCutsceneActor::Attendant),
];

pub const BLACKTHORN_CUTSCENE_FAILED_CHALLENGE_REACTION: [BlackthornCutsceneCommand; 12] = [
    BlackthornCutsceneCommand::OutputByte(BLACKTHORN_CUTSCENE_FORMAT_OUTPUT),
    BlackthornCutsceneCommand::SetRepeat(3),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::Blackthorn,
        direction: crate::Direction::South,
    },
    BlackthornCutsceneCommand::SetRepeat(3),
    BlackthornCutsceneCommand::SetPairedMovement {
        actor: BlackthornCutsceneActor::SecondPartyMember,
        direction: crate::Direction::East,
    },
    BlackthornCutsceneCommand::SetPerStepPause(true),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::Blackthorn,
        direction: crate::Direction::East,
    },
    BlackthornCutsceneCommand::ClearActor(BlackthornCutsceneActor::SecondPartyMember),
    BlackthornCutsceneCommand::SetRepeat(3),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::Blackthorn,
        direction: crate::Direction::West,
    },
    BlackthornCutsceneCommand::WriteTile {
        x: 4,
        y: 8,
        tile: BLACKTHORN_CUTSCENE_TEMP_TILE_A,
    },
    BlackthornCutsceneCommand::WriteTile {
        x: 5,
        y: 8,
        tile: BLACKTHORN_CUTSCENE_TEMP_TILE_B,
    },
];

pub const BLACKTHORN_CUTSCENE_AUDIENCE_THRONE_APPROACH: [BlackthornCutsceneCommand; 8] = [
    BlackthornCutsceneCommand::TimedPause(2),
    BlackthornCutsceneCommand::SetRepeat(2),
    BlackthornCutsceneCommand::SetPairedMovement {
        actor: BlackthornCutsceneActor::Attendant,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::SetPerStepPause(true),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::Blackthorn,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::SetRepeat(2),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::Blackthorn,
        direction: crate::Direction::West,
    },
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::Attendant,
        direction: crate::Direction::East,
    },
];

pub const BLACKTHORN_CUTSCENE_BLACKTHORN_RISES: [BlackthornCutsceneCommand; 4] = [
    BlackthornCutsceneCommand::OutputByte(BLACKTHORN_CUTSCENE_FORMAT_OUTPUT),
    BlackthornCutsceneCommand::SetPerStepPause(true),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::Blackthorn,
        direction: crate::Direction::North,
    },
    BlackthornCutsceneCommand::End,
];

pub const BLACKTHORN_CUTSCENE_CONDITIONAL_THRONE_CLEANUP: [BlackthornCutsceneCommand; 6] = [
    BlackthornCutsceneCommand::OutputByte(BLACKTHORN_CUTSCENE_FORMAT_OUTPUT),
    BlackthornCutsceneCommand::SetRepeat(3),
    BlackthornCutsceneCommand::MoveActor {
        actor: BlackthornCutsceneActor::Throne,
        direction: crate::Direction::East,
    },
    BlackthornCutsceneCommand::ClearActor(BlackthornCutsceneActor::Throne),
    BlackthornCutsceneCommand::ClearScreen,
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
        BlackthornCutsceneBeat::BlackthornRises => &BLACKTHORN_CUTSCENE_BLACKTHORN_RISES,
        BlackthornCutsceneBeat::ConditionalThroneCleanup => {
            &BLACKTHORN_CUTSCENE_CONDITIONAL_THRONE_CLEANUP
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlackthornCutsceneVm {
    pub actors: [Option<BlackthornCutsceneActorState>; BLACKTHORN_CUTSCENE_ACTOR_SLOT_COUNT],
    pub tile_buffer: Vec<u8>,
    pub output_bytes: Vec<u8>,
    pub pause_ticks: u16,
    pub screen_cleared: bool,
    pub ended: bool,
    repeat_count: u8,
    paired_movement: Option<(BlackthornCutsceneActor, crate::Direction)>,
    per_step_pause: bool,
}

pub const BLACKTHORN_CUTSCENE_ACTOR_SLOT_COUNT: usize = 9;

impl BlackthornCutsceneVm {
    pub fn new(tile_buffer: Vec<u8>) -> Self {
        Self {
            actors: [None; BLACKTHORN_CUTSCENE_ACTOR_SLOT_COUNT],
            tile_buffer,
            output_bytes: Vec::new(),
            pause_ticks: 0,
            screen_cleared: false,
            ended: false,
            repeat_count: 1,
            paired_movement: None,
            per_step_pause: false,
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
                    visible: true,
                },
            );
        }
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
            BlackthornCutsceneCommand::OutputByte(byte) => {
                self.output_bytes.push(byte);
            }
            BlackthornCutsceneCommand::WriteTile { x, y, tile } => {
                if let Some(index) = blackthorn_cutscene_tile_index(x, y) {
                    if let Some(cell) = self.tile_buffer.get_mut(index) {
                        *cell = tile;
                    }
                }
            }
            BlackthornCutsceneCommand::ClearScreen => {
                self.screen_cleared = true;
                self.tile_buffer.fill(0);
            }
            BlackthornCutsceneCommand::TimedPause(ticks) => {
                self.pause_ticks = self
                    .pause_ticks
                    .saturating_add(u16::from(ticks) * u16::from(self.repeat_count));
                self.repeat_count = 1;
            }
            BlackthornCutsceneCommand::ClearActor(actor) => {
                self.actors[actor.slot_index() as usize] = None;
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
                        self.pause_ticks = self.pause_ticks.saturating_add(1);
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
        6 => BlackthornCutsceneActor::Blackthorn,
        7 => BlackthornCutsceneActor::Attendant,
        8 => BlackthornCutsceneActor::Throne,
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
/// `blackthorn.md §4`: number of fixed prompt ordinals the challenge
/// loop iterates (the first four virtue/mantra pairs).
pub const BLACKTHORN_CHALLENGE_PROMPT_COUNT: usize = 4;

/// `blackthorn.md §4`: case-insensitive substring match of the
/// player's typed answer against the expected mantra. The expected
/// word may appear anywhere in the typed buffer rather than being the
/// entire input.
/// `blackthorn.md §4` per-prompt accepted-answer table. The
/// challenge loop iterates the first four virtue/mantra ordinals;
/// the prompt word and the expected answer are paired in order.
/// Index `0` is the Honesty/Ahm pair and index `3` is the
/// Justice/Beh pair.
pub const BLACKTHORN_CHALLENGE_PROMPT_TABLE: [(&str, &str); 4] = [
    ("Honesty", "Ahm"),
    ("Compassion", "Mu"),
    ("Valour", "Ra"),
    ("Justice", "Beh"),
];

/// `blackthorn.md §4`: returns the (prompt-word, expected-answer)
/// pair for ordinals `0..=3`. Returns `None` for ordinals outside
/// the live four-prompt range; the resident tables carry later
/// virtue/mantra pairs but the traced challenge loop only iterates
/// these four.
pub const fn blackthorn_challenge_prompt(ordinal: u8) -> Option<(&'static str, &'static str)> {
    if (ordinal as usize) >= BLACKTHORN_CHALLENGE_PROMPT_TABLE.len() {
        None
    } else {
        Some(BLACKTHORN_CHALLENGE_PROMPT_TABLE[ordinal as usize])
    }
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
