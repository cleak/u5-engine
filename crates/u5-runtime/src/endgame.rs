//! Terminal endgame state helpers.

use crate::*;

/// `catalogs/quest-graph.md §5` Eternal-Flame principle the
/// destruction of one Shadowlord's shard must use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EternalFlame {
    /// Falsehood's opposed flame.
    Truth,
    /// Hatred's opposed flame.
    Love,
    /// Cowardice's opposed flame.
    Courage,
}

impl EternalFlame {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Truth => "Flame of Truth",
            Self::Love => "Flame of Love",
            Self::Courage => "Flame of Courage",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        let key = key.to_ascii_lowercase();
        let key = key
            .strip_prefix("flame_of_")
            .or_else(|| key.strip_prefix("flame-of-"))
            .or_else(|| key.strip_prefix("flameof"))
            .or_else(|| key.strip_prefix("flame"))
            .unwrap_or(&key);
        match key.trim_matches(['_', '-']) {
            "truth" => Some(Self::Truth),
            "love" => Some(Self::Love),
            "courage" => Some(Self::Courage),
            _ => None,
        }
    }
}

/// `catalogs/quest-graph.md §5`: the Eternal-Flame paired with each
/// Shadowlord (by zero-based slot 0=Falsehood / 1=Hatred / 2=Cowardice).
pub const fn eternal_flame_for_shadowlord(slot: usize) -> Option<EternalFlame> {
    Some(match slot {
        0 => EternalFlame::Truth,
        1 => EternalFlame::Love,
        2 => EternalFlame::Courage,
        _ => return None,
    })
}

/// `catalogs/quest-graph.md §5` typed Shadowlord identity. The three
/// hideout slots are tied to a fixed virtue principle: slot 0 is
/// Falsehood (Fauline), slot 1 is Hatred (Astaroth), slot 2 is
/// Cowardice (Nosfentor). The principle, the name, and the opposed
/// Eternal-Flame are stable across the quest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowlordPrinciple {
    /// Slot 0 — Fauline, opposed by the Flame of Truth.
    Falsehood,
    /// Slot 1 — Astaroth, opposed by the Flame of Love.
    Hatred,
    /// Slot 2 — Nosfentor, opposed by the Flame of Courage.
    Cowardice,
}

impl ShadowlordPrinciple {
    /// Slot order published in `catalogs/quest-graph.md §5`.
    pub const ALL: [Self; 3] = [Self::Falsehood, Self::Hatred, Self::Cowardice];

    /// `catalogs/quest-graph.md §5`: hideout slot index `0..=2` the
    /// shipped roster assigns to this principle.
    pub const fn slot(self) -> usize {
        match self {
            Self::Falsehood => 0,
            Self::Hatred => 1,
            Self::Cowardice => 2,
        }
    }

    /// `catalogs/quest-graph.md §5`: opposed Eternal Flame.
    pub const fn eternal_flame(self) -> EternalFlame {
        match self {
            Self::Falsehood => EternalFlame::Truth,
            Self::Hatred => EternalFlame::Love,
            Self::Cowardice => EternalFlame::Courage,
        }
    }
}

/// `catalogs/quest-graph.md §5`: classify a hideout slot index into
/// the typed Shadowlord principle, or `None` for indices outside
/// the published `0..=2` set.
pub const fn shadowlord_principle_for_slot(slot: usize) -> Option<ShadowlordPrinciple> {
    Some(match slot {
        0 => ShadowlordPrinciple::Falsehood,
        1 => ShadowlordPrinciple::Hatred,
        2 => ShadowlordPrinciple::Cowardice,
        _ => return None,
    })
}

/// `catalogs/quest-graph.md §2`: the four main-quest requirements
/// the web-shaped progression converges on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainQuestRequirement {
    /// 1. Recover the royal artifacts: Crown, Sceptre, and Amulet.
    RoyalArtifacts,
    /// 2. Learn and use the eight dungeon Words of Power.
    DungeonWords,
    /// 3. Recover and destroy the three evil shards by using the
    ///    Eternal Flames and the Shadowlords' names.
    ShardsAndShadowlords,
    /// 4. Preserve and use the hidden sandalwood-box object that
    ///    enables Lord British's return.
    SandalwoodBox,
}

impl MainQuestRequirement {
    /// `catalogs/quest-graph.md §2` ordered list of the four
    /// requirements as they appear in the spec.
    pub const ALL: [Self; 4] = [
        Self::RoyalArtifacts,
        Self::DungeonWords,
        Self::ShardsAndShadowlords,
        Self::SandalwoodBox,
    ];
}

/// `catalogs/quest-graph.md §7`: shipped Sandalwood Box pickup —
/// non-speaking object slot 31 in `CASTLE:0` at local (X=18, Y=12, Z=2)
/// with object tag `0x0E` (the pickup runs through the shared
/// item-add path, sets the save-backed box flag).
/// `catalogs/quest-graph.md §7`: the Sandalwood Box ships in
/// Lord British's Castle (`CASTLE:0`, scene byte 17). Anchored
/// to [`crate::SCENE_LORD_BRITISHS_CASTLE`] so the box's scene
/// and the named scene constant share one source of truth.
pub const SANDALWOOD_BOX_PICKUP_SCENE: u8 = crate::SCENE_LORD_BRITISHS_CASTLE;
pub const SANDALWOOD_BOX_PICKUP_X: u8 = 18;
pub const SANDALWOOD_BOX_PICKUP_Y: u8 = 12;
pub const SANDALWOOD_BOX_PICKUP_Z: u8 = 2;
/// `catalogs/quest-graph.md §7` shipped Sandalwood Box pickup —
/// the box lives in the last active-object slot of the
/// 32-record CASTLE:0 active-object table. Anchored to
/// [`crate::OOL_SLOTS`] - 1 so the highest active-object slot
/// index has one source of truth.
pub const SANDALWOOD_BOX_PICKUP_OBJECT_SLOT: usize = crate::OOL_SLOTS - 1;
pub const SANDALWOOD_BOX_PICKUP_TAG: u8 = 0x0E;

/// `endgame.md §9` certificate body components. The spec lists the
/// fields the overlay's line accumulator composes (ordinal day,
/// ordinal month number, year in words, the party leader's name, a
/// short royal salvation statement, and a centered Codex-style
/// closing title drawn through the sign/tile-glyph path) but
/// deliberately does not reproduce the fixed wording, the separators,
/// or the closing title. The engine therefore carries the published
/// data-derived fields only; composing the surrounding English prose
/// in Rust would ship invented text. See `cleak/u5-spec#82`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndgameCertificate {
    /// `§9`: current saved day rendered as an ordinal word.
    pub day_ordinal: String,
    /// `§9`: current saved month *number* rendered as an ordinal word.
    pub month_ordinal: String,
    /// `§9.2`: the saved year in words, split - `year / 100` cardinal
    /// followed by the literal `Hundred` on one row, `year % 100`
    /// cardinal on the next.
    pub year_hundreds_words: String,
    pub year_remainder_words: String,
    /// `§9`: the party leader's name.
    pub leader_name: String,
}

/// `endgame.md §9` final report panel. Separate from the certificate
/// body: after the body the scroll clears or advances to this panel,
/// which prints the elapsed campaign time and then the line asking the
/// player to report the completed quest to Origin. The elapsed-time
/// arithmetic and its singular/plural, zero-omitting formatting are
/// published; the Origin line's wording is not. See `cleak/u5-spec#82`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndgameFinalReport {
    pub years: u16,
    pub months: u8,
    pub days: u8,
}

impl EndgameFinalReport {
    /// `endgame.md §9` elapsed-time rendering: numeric years, months
    /// and days, zero-value units omitted, singular/plural labels,
    /// separators only between printed nonzero units.
    pub fn elapsed_label(&self) -> String {
        elapsed_time_label(self.years, self.months, self.days)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndgameState {
    pub first_confirmation: Option<bool>,
    pub final_confirmation: Option<bool>,
    pub outcome: Option<EndgameOutcome>,
    pub certificate: Option<EndgameCertificate>,
    pub final_report: Option<EndgameFinalReport>,
    pub cinematic: crate::endgame_cinematic::EndgameCinematic,
    pub messages: Option<EndgameMessages>,
    pub final_narrative: Option<EndNarrative>,
    /// `endgame.md §4`: one pending restoration-announcement beat per
    /// party member the tableau setup pass raised from Dead. Each beat
    /// emits the exact revival line and is interleaved with that member's
    /// placement and walk-in, in party-slot order.
    pub entry_restoration_beats: u8,
    /// `endgame.md §4`/`§7`: Lord British enters first, walking from
    /// `(5,8)` to `(5,3)` before party-slot placement begins.
    pub entry_lord_british_pending: bool,
    /// `audio.md §8.7`'s "fixed 40-tick pause after Lord British appears",
    /// still owed before his first step. See
    /// [`ENDGAME_TABLEAU_LORD_BRITISH_ARRIVAL_HOLD_BIOS_TICKS`].
    pub entry_lord_british_pause_owed: bool,
    /// `endgame.md §4`: next party slot owned by the entry setup loop.
    /// The loop fully restores (when needed), places, and walks one member
    /// before advancing to the next. [`ENDGAME_ENTRY_COMPLETE_SLOT`] means
    /// the greeting/prompt has been published and no entry work remains.
    pub entry_party_slot: u8,
    /// `endgame.md §5.1` publishes a blocking key read between rite
    /// pages. A capture shows the dialogue paged the same way: the stock
    /// game prints Lord British's greeting and stops, and the first box
    /// question does not appear until a key is pressed. Sampling one run
    /// at five, fifteen and thirty seconds gives an identical window all
    /// three times (`cleak/u5-engine#10`). True while that page is owed.
    pub greeting_page_pending: bool,
    /// `endgame.md §7`: display-owned state for the victory tableau's
    /// Orb hold, shared-moongate rise/hold/sink sequence, and actor exits.
    pub victory_tableau_phase: EndgameVictoryTableauPhase,
    /// `endgame.md §5.2`'s hold between the absorption line and the
    /// tableau, still owed. See [`ENDGAME_ABSORPTION_HOLD_BIOS_TICKS`].
    pub entry_absorption_hold_owed: bool,
    /// `combat.md` Section 11 (u5-spec#311 Q5): the Doom absorption
    /// narration the walked-in contact owes the first pumped entry frame -
    /// `\n`, the dispatched slot's name and ` is absorbed!\n`. `None` for the
    /// direct entries, which have no absorption.
    pub entry_absorption_line: Option<String>,
    /// `endgame.md §5.2`'s hold between the last party member settling and
    /// Lord British's greeting, still owed. See
    /// [`ENDGAME_GREETING_HOLD_BIOS_TICKS`].
    pub entry_greeting_hold_owed: bool,
    /// `endgame.md §5.1`/`§6`: the waiting branch's display-driven beats.
    pub refusal_phase: EndgameRefusalPhase,
    /// `endgame.md §7` steps 4-6: the victory branch's box hand-over, beat
    /// by beat, before record `3` prints.
    pub victory_staging: EndgameVictoryStagingPhase,
    /// `endgame.md §6` step 3: the index into
    /// [`ENDGAME_TABLEAU_JITTER_SLOTS`] of the terminal loop's next wander
    /// call. Each call is its own paced iteration, so the loop is pumped one
    /// call per display frame.
    pub terminal_jitter_next_call: u8,
    /// The BIOS tick the tableau sprite animator last ran on
    /// ([`PlayState::advance_endgame_tableau_animation_to`]); `None` until
    /// the first look anchors it.
    pub tableau_animation_tick: Option<u64>,
}

/// `endgame.md §5.1` and `§6`: the waiting branch, beat by beat. The
/// lead-in prints with the answer; the forty-tick pause, record 10, the
/// leader's one-row nudge and the three-actor walk follow as display
/// frames, and only then does the terminal jitter loop run.
///
/// Measured 2026-10-16 (`doom-final-room`, `-refusal`, `-waiting`): the
/// original prints `"I see...`, holds about four timelapse frames, prints
/// record 10, and only then walks Lord British to the bed and the leader to
/// the chair over the next three; this engine printed all of it and teleported
/// the actors on the answering key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EndgameRefusalPhase {
    /// Not on the waiting branch, or nothing owed yet.
    #[default]
    Inactive,
    /// The forty-tick pause after the lead-in is owed.
    LeadInPauseOwed,
    /// Record 10 prints on the next frame.
    RecordOwed,
    /// `§6` step 1: slot 0's Y decrement and its sting beat.
    NudgeOwed,
    /// `§6` step 2: the three scripted walks. Each round is "one call per
    /// actor in exactly that order (slot 2, slot 31, slot 0), and every
    /// actual one-cell step carries its own two-iteration, sting,
    /// three-iteration beat", so one display frame runs calls until one
    /// actor has actually stepped: `next_call` indexes
    /// [`ENDGAME_TABLEAU_REFUSAL_WALK_ORDER`] and `moved_this_round`
    /// remembers whether the round has stepped anyone yet.
    Walking { next_call: u8, moved_this_round: bool },
    /// `§6` step 3: the terminal jitter loop.
    Jitter,
}

/// `endgame.md §7` steps 4-6 (issue #298 Q41): the victory branch's box
/// hand-over, between the effective Yes and record `3`. "Step slot 0 from its
/// settled position to (5, 4), then back to (5, 5) ... Eight paced iterations
/// precede the first step. After four more paced iterations, create the
/// **sandalwood box** in slot 6 with actor byte `0x0E` at (5, 4)", and only
/// then "Print the seven rite messages with the pacing of section 5.1 (record
/// 3, forty paced iterations, `He says:`, then the key-paced pages)".
///
/// The engine had stepped slot 0 up and back and placed the box inside the
/// answering key, and printed record `3` with it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EndgameVictoryStagingPhase {
    /// Not on the victory branch, or the hand-over has finished.
    #[default]
    Inactive,
    /// The eight paced iterations before slot 0's first step are owed.
    LeadPauseOwed,
    /// Slot 0 steps toward `(5, 4)`, one cell (and one sting beat) a frame.
    WalkUp,
    /// Slot 0 steps back toward `(5, 5)`.
    WalkBack,
    /// The four paced iterations before the box appears are owed.
    BoxPauseOwed,
    /// The box is placed and record `3` prints on the next frame.
    BoxOwed,
}

/// `endgame.md §7` step 4: "Eight paced iterations precede the first step."
pub const ENDGAME_VICTORY_HANDOVER_LEAD_TICKS: u16 = 8;
/// `endgame.md §7` step 5: "After four more paced iterations, create the
/// **sandalwood box**".
pub const ENDGAME_VICTORY_BOX_PLACEMENT_TICKS: u16 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndgameOutcome {
    Victory,
    MissingBoxOrRefused,
}

/// `endgame.md §7`: automatic presentation phases after the seven rite
/// messages. Only [`Self::OrbAwaitingAcknowledgement`] waits for input;
/// every later phase is advanced by the display-frame pump.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EndgameVictoryTableauPhase {
    #[default]
    Inactive,
    OrbAwaitingAcknowledgement,
    GateRise {
        next_phase: u8,
    },
    GateFullHold {
        ticks_remaining: u8,
    },
    ActorExit,
    GateSink {
        next_phase: u8,
    },
    RestoreFloor,
    Complete,
}

pub const ENDGAME_TABLEAU_WIDTH: usize = 11;
pub const ENDGAME_TABLEAU_HEIGHT: usize = 11;
pub const ENDGAME_TABLEAU_PARTY_START: (usize, usize) = (5, 9);
pub const ENDGAME_TABLEAU_BOX_SLOT: usize = 6;
pub const ENDGAME_TABLEAU_LORD_BRITISH_SLOT: usize = OOL_SLOTS - 1;
pub const ENDGAME_TABLEAU_BOX_ACTOR_BYTE: u8 = 0x0e;
pub const ENDGAME_TABLEAU_ORB_ACTOR_BYTE: u8 = 0x08;
pub const ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE: u8 = 0x7c;
pub const ENDGAME_TABLEAU_PHASE: u8 = 0;
pub const ENDGAME_GATE_CELL: (usize, usize) = (5, 4);
pub const ENDGAME_GATE_FULL_HOLD_TICKS: u8 = 4;
pub const ENDGAME_TABLEAU_LORD_BRITISH_THRONE_TARGET: (usize, usize) = (5, 3);
const ENDGAME_ENTRY_COMPLETE_SLOT: u8 = u8::MAX;
/// Every party member has settled and the greeting is waiting out
/// [`ENDGAME_GREETING_HOLD_BIOS_TICKS`].
pub(crate) const ENDGAME_ENTRY_GREETING_SLOT: u8 = u8::MAX - 1;
/// `formats/location-dat.md section 11`: endgame loads MISCMAPS cutscene-map
/// record 3 as the authored 11x11 terminal tableau scene.
pub const ENDGAME_TABLEAU_CUTSCENE_MAP_RECORD: usize = 3;

/// `endgame.md §3` step 4 and `display-driver-abi.md §10`: at the moment
/// the absorption line prints, the endgame issues the display driver's
/// whole-tileset colour remap. It "packs the whole prepared tileset from
/// planar rows to packed four-bit pixels, then rewrites twenty-two tiles
/// pixel by pixel through one sixteen-entry map, then unpacks". It is
/// one-shot and is never undone in the terminal state.
///
/// The map, by colour index: `0`, `7`, `8`, `14` and `15` stay; `1` and
/// `5` swap; `2` and `4` swap; `3` becomes `4`; `6` becomes `2`; `9`,
/// `10` and `11` become `12`; `12` becomes `10`; `13` becomes `9`.
pub const ENDGAME_TILESET_NIBBLE_MAP: [u8; 16] =
    [0, 5, 4, 4, 2, 1, 2, 7, 8, 12, 12, 12, 10, 9, 14, 15];

/// The twenty-two tiles `display-driver-abi.md §10` lists for the remap:
/// "floor, chairs, tables, beds, the fire fixtures, the sandalwood box and
/// the occupied-chair frames". Walls, ground outside the set, and actor
/// sprites are untouched, "which is why a captured final room shows green
/// floor and furniture inside an unchanged stone border".
pub const ENDGAME_TILESET_REMAP_TILES: [u8; 22] = [
    0x08, 0x0e, 0x1a, 0x38, 0x39, 0x3a, 0x3b, 0x44, 0x5c, 0x5d, 0x90, 0x92, 0x94, 0x96, 0x9b, 0xab,
    0xac, 0xaf, 0xb0, 0xb1, 0xbf, 0xdc,
];

/// Whether a tile id is one of the twenty-two the endgame remap rewrites.
pub fn endgame_tileset_remap_applies(tile: usize) -> bool {
    u8::try_from(tile).is_ok_and(|tile| ENDGAME_TILESET_REMAP_TILES.contains(&tile))
}

/// Apply [`ENDGAME_TILESET_NIBBLE_MAP`] to a buffer of palette indices.
pub fn remap_endgame_tileset_pixels(pixels: &mut [u8]) {
    for pixel in pixels {
        if let Some(mapped) = ENDGAME_TILESET_NIBBLE_MAP.get(usize::from(*pixel)) {
            *pixel = *mapped;
        }
    }
}

pub const ENDGAME_TABLEAU_WALKABLE_TILE: u8 = 0x44;

const ENDGAME_TABLEAU_PARTY_TARGETS: [(usize, usize); SAVE_PARTY_SIZE_MAX as usize] =
    [(5, 5), (4, 6), (6, 6), (3, 7), (5, 7), (7, 7)];
const ENDGAME_TABLEAU_LORD_BRITISH_POS: (usize, usize) = (5, 4);
const ENDGAME_TABLEAU_LORD_BRITISH_START: (usize, usize) = (5, 8);
const ENDGAME_TABLEAU_REFUSAL_SLOT0_TARGET: (usize, usize) = (8, 4);
const ENDGAME_TABLEAU_REFUSAL_SLOT2_TARGET: (usize, usize) = (8, 6);

/// `endgame.md §6` step 2 (issue #298 Q42): one walk round is one call per
/// actor "in exactly that order (slot 2, slot 31, slot 0)", each with its
/// target.
const ENDGAME_TABLEAU_REFUSAL_WALK_ORDER: [(usize, (usize, usize)); 3] = [
    (2, ENDGAME_TABLEAU_REFUSAL_SLOT2_TARGET),
    (ENDGAME_TABLEAU_LORD_BRITISH_SLOT, ENDGAME_TABLEAU_REFUSAL_LORD_BRITISH_TARGET),
    (0, ENDGAME_TABLEAU_REFUSAL_SLOT0_TARGET),
];
const ENDGAME_TABLEAU_REFUSAL_LORD_BRITISH_TARGET: (usize, usize) = (4, 1);
const ENDGAME_TABLEAU_VICTORY_EXIT_TARGET: (usize, usize) = (5, 4);
const ENDGAME_TABLEAU_SETTLE_STEP_CAP: usize = ENDGAME_TABLEAU_WIDTH * ENDGAME_TABLEAU_HEIGHT * 2;
pub const ENDGAME_TABLEAU_JITTER_SLOTS: [usize; 4] = [1, 3, 4, 5];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndgameTableauActorRole {
    PartyMember(u8),
    /// `endgame.md §4`: slot 6, actor byte `0x0E` - a lidded chest,
    /// the sandalwood box. Swapped in place for the Orb spark and then
    /// cleared during the victory rite. `cleak/u5-spec#82` retracted
    /// the earlier reading that called this slot Lord British.
    SandalwoodBox,
    /// `endgame.md §4`: slot 31, actor byte `0x7C` - a crowned seated
    /// figure between two banners. This, not slot 6, is Lord British;
    /// the earlier "scene marker" label was the other half of the same
    /// inversion.
    LordBritish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndgameTableauActorPlacement {
    pub role: EndgameTableauActorRole,
    pub active_object_slot: usize,
    pub start: (usize, usize),
    pub target: (usize, usize),
    pub tile: u8,
    pub type_byte: u8,
}

impl EndgameState {
    pub fn awaiting_first_confirmation() -> Self {
        Self::awaiting_first_confirmation_with_messages(None)
    }

    pub fn awaiting_first_confirmation_with_messages(messages: Option<EndgameMessages>) -> Self {
        Self {
            first_confirmation: None,
            final_confirmation: None,
            outcome: None,
            certificate: None,
            final_report: None,
            cinematic: crate::endgame_cinematic::EndgameCinematic::default(),
            messages,
            final_narrative: None,
            entry_restoration_beats: 0,
            entry_lord_british_pending: false,
            entry_lord_british_pause_owed: false,
            entry_party_slot: ENDGAME_ENTRY_COMPLETE_SLOT,
            victory_tableau_phase: EndgameVictoryTableauPhase::Inactive,
            greeting_page_pending: false,
            entry_absorption_hold_owed: false,
            entry_absorption_line: None,
            entry_greeting_hold_owed: false,
            refusal_phase: EndgameRefusalPhase::Inactive,
            victory_staging: EndgameVictoryStagingPhase::Inactive,
            terminal_jitter_next_call: 0,
            tableau_animation_tick: None,
        }
    }

    pub fn awaiting_final_confirmation(first_confirmation: bool) -> Self {
        Self::awaiting_final_confirmation_with_messages(first_confirmation, None)
    }

    pub fn awaiting_final_confirmation_with_messages(
        first_confirmation: bool,
        messages: Option<EndgameMessages>,
    ) -> Self {
        Self {
            first_confirmation: Some(first_confirmation),
            final_confirmation: None,
            outcome: None,
            certificate: None,
            final_report: None,
            cinematic: crate::endgame_cinematic::EndgameCinematic::default(),
            messages,
            final_narrative: None,
            entry_restoration_beats: 0,
            entry_lord_british_pending: false,
            entry_lord_british_pause_owed: false,
            entry_party_slot: ENDGAME_ENTRY_COMPLETE_SLOT,
            victory_tableau_phase: EndgameVictoryTableauPhase::Inactive,
            greeting_page_pending: false,
            entry_absorption_hold_owed: false,
            entry_absorption_line: None,
            entry_greeting_hold_owed: false,
            refusal_phase: EndgameRefusalPhase::Inactive,
            victory_staging: EndgameVictoryStagingPhase::Inactive,
            terminal_jitter_next_call: 0,
            tableau_animation_tick: None,
        }
    }

    pub fn terminal(
        first_confirmation: bool,
        final_confirmation: bool,
        has_sandalwood_box: bool,
        certificate: EndgameCertificate,
        final_report: EndgameFinalReport,
        messages: Option<EndgameMessages>,
        final_narrative: Option<EndNarrative>,
    ) -> Self {
        let outcome = endgame_outcome(final_confirmation, has_sandalwood_box);
        let cinematic = if outcome == EndgameOutcome::Victory {
            let rite_count = messages
                .as_ref()
                .map(|messages| messages.rite_messages().len().min(u8::MAX as usize) as u8)
                .unwrap_or(0);
            crate::endgame_cinematic::EndgameCinematic::start_with_rite_messages(rite_count)
        } else {
            crate::endgame_cinematic::EndgameCinematic::default()
        };
        Self {
            first_confirmation: Some(first_confirmation),
            final_confirmation: Some(final_confirmation),
            outcome: Some(outcome),
            certificate: (outcome == EndgameOutcome::Victory).then_some(certificate),
            final_report: (outcome == EndgameOutcome::Victory).then_some(final_report),
            cinematic,
            messages,
            final_narrative: (outcome == EndgameOutcome::Victory)
                .then_some(final_narrative)
                .flatten(),
            entry_restoration_beats: 0,
            entry_lord_british_pending: false,
            entry_lord_british_pause_owed: false,
            entry_party_slot: ENDGAME_ENTRY_COMPLETE_SLOT,
            victory_tableau_phase: EndgameVictoryTableauPhase::Inactive,
            greeting_page_pending: false,
            entry_absorption_hold_owed: false,
            entry_absorption_line: None,
            entry_greeting_hold_owed: false,
            refusal_phase: EndgameRefusalPhase::Inactive,
            victory_staging: EndgameVictoryStagingPhase::Inactive,
            terminal_jitter_next_call: 0,
            tableau_animation_tick: None,
        }
    }

    /// Advance the post-victory cinematic by one keystroke. Returns
    /// the new step's banner label for caller-side display.
    pub fn advance_cinematic(&mut self) -> &'static str {
        self.cinematic.advance();
        self.cinematic.banner_label()
    }

    /// Advance a frame-owned cinematic display operation that does not
    /// consume a keypress. Returns `true` when an operation was pending.
    pub fn advance_cinematic_frame_operation(&mut self) -> bool {
        self.cinematic.advance_fade_to_black()
    }

    /// `true` when the post-victory cinematic has presented every
    /// screen and the engine should remain on the terminal final panel.
    pub fn cinematic_is_finished(&self) -> bool {
        self.cinematic.is_finished()
    }

    pub fn is_terminal(&self) -> bool {
        self.outcome.is_some()
    }

    /// `endgame.md §5` steps 1-2, composed from the `ENDMSG.DAT` records
    /// rather than around them.
    ///
    /// Record 0 ends mid-sentence, on `"Well met,` and a line feed: the
    /// party leader's name belongs on the row after it, and the capture
    /// behind `cleak/u5-engine#10` shows the original closing that
    /// sentence with `!"` before the blank row and record 1. Record 1
    /// already ends `You reply: `, so the prompt needs no suffix - the
    /// engine's `(Y/N)` was an invention, and the leading
    /// `{leader_name}: ` was the name being prefixed to the greeting
    /// instead of substituted into it.
    pub fn first_prompt_text(&self, leader_name: &str) -> String {
        if let Some(messages) = &self.messages {
            if let (Some(greeting), Some(prompt)) =
                (messages.initial_greeting(), messages.first_box_prompt())
            {
                // The greeting is its own page: the original stops here
                // and waits for a key before the question
                // (`greeting_page_pending`).
                if self.greeting_page_pending {
                    return format!("{greeting}{leader_name}!\"");
                }
                return format!("{greeting}{leader_name}!\"\n\n{prompt}");
            }
        }
        "Endgame: Lord British asks whether thou hast brought his box. (Y/N)".to_string()
    }

    /// The first box question, printed once the greeting page has been
    /// advanced.
    pub fn first_box_question_text(&self) -> Option<String> {
        self.messages
            .as_ref()
            .and_then(|messages| messages.first_box_prompt())
            // One line feed closes the greeting's row and the second
            // leaves the blank row the stock window keeps between the
            // two records (`text-output.md §10.4`).
            .map(|prompt| format!("\n\n{prompt}"))
    }

    pub fn second_prompt_text(&self, first_answer: bool) -> String {
        if let Some(messages) = &self.messages {
            if let Some(prompt) = messages.second_box_prompt() {
                // The echo and the blank line §5.1 puts after it are both
                // committed by the caller, so the record follows directly.
                // Measured 2026-10-16 (`doom-final-room-refusal`, beats
                // `answered-no` and `after-key`, reading `scroll`): the
                // original keeps one blank row between `You reply: No` and
                // this record, where a leading feed here spent a second.
                let _ = first_answer;
                return prompt.to_string();
            }
        }
        "Endgame: Lord British asks again for the sandalwood box. (Y/N)".to_string()
    }

    /// `endgame.md §5`: the waiting branch prints the fixed `"I see...`
    /// lead-in, pauses forty world-animation ticks, and only then renders
    /// `ENDMSG.DAT` record 10. The engine printed record 10 alone, which is
    /// the missing line `cleak/u5-engine#12` recorded in the stock capture.
    pub fn refusal_text(&self) -> String {
        format!(
            "{}{}",
            crate::endgame_cinematic::ENDGAME_WAITING_LEAD_IN,
            self.refusal_record_text()
        )
    }

    /// `ENDMSG.DAT` record 10, the waiting exchange printed after the
    /// lead-in's forty-tick pause.
    pub fn refusal_record_text(&self) -> String {
        self.messages
            .as_ref()
            .and_then(|messages| messages.refusal_branch())
            .map(str::to_string)
            .unwrap_or_else(|| {
                "Endgame: the sandalwood box handoff failed; the ending tableau is terminal."
                    .to_string()
            })
    }

    /// The current `endgame.md §8` narrative window's `END.DAT` text with
    /// the `{` first-line indent and `_` soft hyphen kept in place, for the
    /// proportional page renderer that consumes them (`formats/end-dat.md
    /// §3`). `None` outside a narrative window. Fixed-cell surfaces print
    /// [`Self::current_cinematic_text`] instead, which has them removed.
    pub fn current_narrative_layout_text(&self) -> Option<String> {
        let crate::endgame_cinematic::EndgameCinematicStep::NarrativeWindow(index) =
            self.cinematic.step
        else {
            return None;
        };
        self.final_narrative
            .as_ref()?
            .layout_window_by_number(index.saturating_add(1))
    }

    /// Text the current cinematic beat puts on the endgame surface.
    ///
    /// `banner_label()` is a debug/pacing aid for tests; it is never a
    /// display fallback. A missing `ENDMSG.DAT` rite record or a
    /// missing `END.DAT` narrative window is an asset/contract failure
    /// (`endgame.md §7`, `§8`) and fails loudly instead of printing a
    /// label at the player. Beats that carry no prose of their own
    /// (the throne tableau and the late full-screen rectangle
    /// operation) return an empty string.
    pub fn current_cinematic_text(&self) -> String {
        use crate::endgame_cinematic::EndgameCinematicStep as Step;
        match self.cinematic.step {
            Step::RiteMessage(index) => self
                .messages
                .as_ref()
                .and_then(|messages| messages.rite_messages().get(index as usize))
                .map(|message| {
                    if index == 1 && self.cinematic.rite_lead_in_visible {
                        // `§5.1`: "exactly one blank row before it and one
                        // after". Record 3 leaves its row open, so the first
                        // feed closes that row and the second is the blank.
                        format!(
                            "\n\n{}\n\n{}",
                            crate::endgame_cinematic::ENDGAME_RITE_LEAD_IN,
                            message
                        )
                    } else {
                        message.clone()
                    }
                })
                .unwrap_or_else(|| {
                    panic!(
                        "endgame victory rite beat {index} has no ENDMSG.DAT record; displaying the step's debug banner label instead is a forbidden fallback (endgame.md §7)"
                    )
                }),
            Step::NarrativeWindow(index) => self
                .final_narrative
                .as_ref()
                .and_then(|narrative| {
                    // `formats/end-dat.md §3` / `text-output.md §8.2`: `{`
                    // and `_` draw nothing, so the message slot - which
                    // every fixed-cell consumer prints as-is - carries the
                    // visible glyphs only (playtest F58). The proportional
                    // page renderer takes the marked text from
                    // [`Self::current_narrative_layout_text`].
                    narrative.window_by_number(index.saturating_add(1))
                })
                .unwrap_or_else(|| {
                    panic!(
                        "endgame final narrative window {} has no END.DAT text; displaying the step's debug banner label instead is a forbidden fallback (endgame.md §8)",
                        index.saturating_add(1)
                    )
                }),
            // `§9`: the certificate screen is one composed page, not a
            // line of message text - the caller renders it from
            // `endgame_certificate_lines`. Its message slot stays empty
            // so nothing is echoed into the message window under it.
            Step::Certificate | Step::FinalReport | Step::Finished => String::new(),
            Step::Inactive | Step::ThroneTableau | Step::FadeToBlack => String::new(),
        }
    }
}

/// `endgame.md §9.2`: one printed row of the certificate screen.
///
/// The certificate is printed onto the visible page over the parchment
/// backdrop with centred output on; the body runs in inverse video
/// (that, not any palette work, is what makes dark lettering on the
/// light parchment), the closing title switches to the runic font slot
/// for exactly two lines, and the elapsed-time report switches inverse
/// video back off - which is what makes it legible where it lands, on
/// the cleared black page below the parchment's bottom edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndgameCertificateLine {
    /// Row within the full-screen text window. The cursor starts on
    /// row 1 and nothing wraps or scrolls, so rows are fixed.
    pub row: u8,
    pub text: String,
    pub inverse: bool,
    /// `§9.3`: the closing title goes through the ordinary fixed-cell
    /// character path with the runic font slot selected, not through a
    /// sign or tile-glyph composition.
    pub runic: bool,
}

impl EndgameCertificateLine {
    /// `endgame.md §9.2` / `text-output.md §5` centred output:
    /// `(40 - characters_in_line) / 2`, truncating.
    ///
    /// `cleak/u5-spec#82` retracted the earlier `(39 - length) / 2`
    /// form and its claimed systematic four-pixel left bias. The two
    /// agree only for odd-length lines; for even-length lines the 39
    /// form places the text a whole cell too far left. Even-length
    /// lines are centred exactly; odd-length lines land half a cell
    /// left of true centre, and that is plain truncation.
    pub fn column(&self) -> u8 {
        let width = crate::TEXT_SCREEN_COLUMNS as usize;
        // §9.3: the two closing-title digraphs each occupy one stored
        // fixed-font cell, so centring must count the encoded form rather
        // than its decoded Latin authoring text.
        let length = if self.runic {
            runic_line_encoding(&self.text).chars().count()
        } else {
            self.text.chars().count()
        }
        .min(width);
        ((width - length) / 2) as u8
    }
}

/// `endgame.md §9.4`: the certificate parchment, the shipped
/// end-screen archive's single 260x168 record, drawn at `(40, 0)` so
/// it occupies `(40, 0)..(299, 167)` - character rows 0 through 20.
/// The body and closing title land on it; the three report rows fall
/// on the cleared black page just below its bottom edge.
pub const ENDGAME_CERTIFICATE_PARCHMENT_X: usize = 40;
pub const ENDGAME_CERTIFICATE_PARCHMENT_Y: usize = 0;

/// `endgame.md §9.1`: the certificate's first printed row. The cursor
/// is set to column 0 of row 1 of the full-screen window.
pub const ENDGAME_CERTIFICATE_FIRST_ROW: u8 = 1;

/// `endgame.md §9.1`-`§9.5`: the whole certificate screen, as printed
/// rows.
///
/// The wording is the published transcription (`cleak/u5-spec#82`
/// ask 4, re-derived): it is assembled from resident fragments rather
/// than read from a data file, which is why it can be transcribed at
/// all when `END.DAT` and `ENDMSG.DAT` prose cannot.
///
/// Blank rows follow from the encoding rather than being placed: a
/// line feed is a combined CR+LF and the printer emits one per newline
/// byte, so a run of `k` newlines leaves `k - 1` blank rows. The three
/// body gaps are each a pair of newlines (one blank row apiece) and
/// the closing title ends with four (three blank rows). The earlier
/// two / two / one / four counts were retracted.
pub fn endgame_certificate_lines(
    certificate: &EndgameCertificate,
    report: EndgameFinalReport,
) -> Vec<EndgameCertificateLine> {
    let body = [
        "Be it known that on".to_string(),
        format!("the {} Day of", certificate.day_ordinal),
        format!("the {} Month", certificate.month_ordinal),
        "of the Year".to_string(),
        format!("{} Hundred", certificate.year_hundreds_words),
        certificate.year_remainder_words.clone(),
    ];
    let salvation = [
        "saved the life",
        "of our sovereign",
        "Lord British, thereby",
        "saving our people",
        "and our land.",
    ];

    let mut lines = Vec::with_capacity(16);
    let mut row = ENDGAME_CERTIFICATE_FIRST_ROW;
    let push = |lines: &mut Vec<EndgameCertificateLine>,
                row: &mut u8,
                text: String,
                inverse: bool,
                runic: bool| {
        lines.push(EndgameCertificateLine {
            row: *row,
            text,
            inverse,
            runic,
        });
        *row += 1;
    };

    for line in body {
        push(&mut lines, &mut row, line, true, false);
    }
    row += 1; // §9.2 row 7
    push(
        &mut lines,
        &mut row,
        format!("{} the Avatar", certificate.leader_name),
        true,
        false,
    );
    row += 1; // §9.2 row 9
    for line in salvation {
        push(&mut lines, &mut row, line.to_string(), true, false);
    }
    row += 1; // §9.2 row 15
    for line in ENDGAME_CERTIFICATE_CLOSING_TITLE {
        push(&mut lines, &mut row, line.to_string(), true, true);
    }
    row += 3; // §9.2 rows 18-20, the three blank rows after the title
    // §9.5: inverse video off for the report, which lands off the
    // parchment on the black page.
    push(
        &mut lines,
        &mut row,
        "Report now, thy Quest compleat in".to_string(),
        false,
        false,
    );
    push(&mut lines, &mut row, report.elapsed_label(), false, false);
    push(
        &mut lines,
        &mut row,
        "to Lord British at Origin Systems!".to_string(),
        false,
        false,
    );

    lines
}

/// `endgame.md §9.3` closing title, stored in decoded Latin.
pub const ENDGAME_CERTIFICATE_CLOSING_TITLE: [&str; 2] = ["THE QUEST OF THE AVATAR", "IS FOREVER"];

/// `endgame.md §9.3` / `text-output.md §7`: the runic word separator.
///
/// The original stores the closing title in the rune digraph encoding,
/// where the at-sign is the word space. That much is published, so the
/// engine re-encodes spaces rather than passing an ASCII blank through
/// the runic slot.
pub const RUNIC_WORD_SPACE: char = '@';
/// `endgame.md §9.3`: one-cell stored code for decoded `TH`.
pub const RUNIC_TH_DIGRAPH: char = '[';
/// `endgame.md §9.3`: one-cell stored code for decoded `ST`.
pub const RUNIC_ST_DIGRAPH: char = '_';

/// Re-encode a decoded Latin line for the runic font slot.
///
/// `endgame.md §9.3` / `cleak/u5-spec#127`: scan decoded uppercase Latin
/// left-to-right, collapse `TH` and `ST` to their exact one-cell stored
/// characters, map spaces to `@`, and pass every other character through.
pub fn runic_line_encoding(line: &str) -> String {
    let mut chars = line.chars().peekable();
    let mut encoded = String::with_capacity(line.len());
    while let Some(ch) = chars.next() {
        match (ch, chars.peek().copied()) {
            ('T', Some('H')) => {
                chars.next();
                encoded.push(RUNIC_TH_DIGRAPH);
            }
            ('S', Some('T')) => {
                chars.next();
                encoded.push(RUNIC_ST_DIGRAPH);
            }
            (' ', _) => encoded.push(RUNIC_WORD_SPACE),
            _ => encoded.push(ch),
        }
    }
    encoded
}

/// `endgame.md §7.1` full-screen fade to black, run against the real
/// display surface (`cleak/u5-spec#53`).
///
/// This is the published caller-side idiom, in order:
///
/// 1. point the render target at the hidden surface;
/// 2. fill the inclusive rectangle with palette index 0 (the fill is
///    render-target aware and really does fill the hidden surface -
///    #53 withdrew the earlier "front-buffer only" reading);
/// 3. point the render target back at the visible page;
/// 4. dissolve the same rectangle from the hidden surface to the
///    visible page.
///
/// Filling first is what makes this a dissolve *out* to a flat colour;
/// composing first would make it a dissolve *in* to new art. Skipping
/// step 2 would dissolve stale offscreen content onto the screen.
///
/// Both driver calls are blocking and self-paced. Nothing here samples
/// input or runs a title tick, so the beat cannot be interrupted.
pub fn run_endgame_fade_to_black(
    surface: &mut crate::display_driver::EgaDisplaySurface,
) -> std::io::Result<()> {
    let bounds = crate::endgame_cinematic::ENDGAME_FADE_TO_BLACK_RECT;
    let (x0, y0, x1, y1) = bounds;
    let rect = crate::display_driver::normalize_clamp_pixel_rect(
        i32::from(x0),
        i32::from(y0),
        i32::from(x1),
        i32::from(y1),
    )
    .expect("endgame fade-to-black rectangle is the whole surface");

    surface.set_render_target(crate::display_driver::DisplayRenderTarget::Back);
    surface.fill_back_rect(rect, ENDGAME_FADE_TO_BLACK_COLOR);
    surface.release_back_buffer();
    surface.set_render_target(crate::display_driver::DisplayRenderTarget::Front);

    // The dissolve itself is the shared driver primitive
    // (`display-driver-abi.md §9.6`), not a transfer of the endgame's own:
    // this is one of the six published call sites, and it has to visit
    // pixels in the same order the intro's dissolves do.
    crate::RectangleDissolve::new(bounds)?
        .run_to_completion(|x, y| surface.copy_back_pixel_to_front(usize::from(x), usize::from(y)));
    Ok(())
}

/// `display-driver-abi.md §9.6`: by the time the endgame fade runs, the abort
/// gate has long been cleared - the victory rite drew `ENDMSG.DAT` text
/// through the driver's fixed-cell glyph entry - so this dissolve is silent,
/// cannot be interrupted, and consumes no keystroke.
pub fn endgame_fade_to_black_abort_gate() -> crate::DissolveAbortGate {
    let mut gate = crate::DissolveAbortGate::on_driver_load();
    gate.note_fixed_cell_glyph_drawn();
    gate
}

/// `endgame.md §7.1`: the fade fills with palette index 0.
pub const ENDGAME_FADE_TO_BLACK_COLOR: u8 = 0;

/// `endgame.md §4`: returns `true` when a party slot needs the
/// dead-member restoration pass before the throne-room tableau. The
/// pass announces the restoration, flips the slot back to a Good
/// status, and restores current HP from the stored maximum. Only
/// Dead status triggers it; Ashes remains outside the restoration
/// per the documented per-slot loop, and other statuses keep
/// whatever HP/status they brought into the endgame.
pub const fn endgame_needs_tableau_restoration(status: CharacterStatus) -> bool {
    matches!(status, CharacterStatus::Dead)
}

pub fn endgame_outcome(final_confirmation: bool, has_sandalwood_box: bool) -> EndgameOutcome {
    if final_confirmation && has_sandalwood_box {
        EndgameOutcome::Victory
    } else {
        EndgameOutcome::MissingBoxOrRefused
    }
}

/// `endgame.md §5.1`: "The accepted answer is echoed into the message
/// window as the literal `Yes` or `No` followed by a blank line, and only
/// then does the next record print."
///
/// The engine echoed a lowercase `yes`/`no` and I reported the wording as
/// unpublished on `cleak/u5-spec#207`. It is published, in the same
/// subsection that publishes the record roles; I had read §5's flow and
/// not §5.1's detail.
fn yes_no_word(answer: bool) -> &'static str {
    if answer { "Yes" } else { "No" }
}

/// `endgame.md §9` certificate elapsed-time baseline. The certificate
/// computes campaign elapsed time by subtracting this fixed date from
/// the saved world clock under the thirteen-month / twenty-eight-day
/// calendar model. Year 139, month 4, day 5 corresponds to the
/// campaign-start date the certificate calls the "beginning of the
/// quest".
pub const ENDGAME_CAMPAIGN_START_YEAR: u16 = crate::CHARGEN_STARTING_YEAR;
pub const ENDGAME_CAMPAIGN_START_MONTH: u8 = crate::CHARGEN_STARTING_MONTH;
pub const ENDGAME_CAMPAIGN_START_DAY: u8 = crate::CHARGEN_STARTING_DAY;

pub fn endgame_elapsed_campaign_time(clock: GameClock) -> (u16, u8, u8) {
    let mut years = clock.year.saturating_sub(ENDGAME_CAMPAIGN_START_YEAR);
    let mut months = clock.month as i16 - ENDGAME_CAMPAIGN_START_MONTH as i16;
    let mut days = clock.day as i16 - ENDGAME_CAMPAIGN_START_DAY as i16;

    if days < 0 {
        days += DAYS_PER_MONTH as i16;
        months -= 1;
    }
    if months < 0 {
        months += MONTHS_PER_YEAR as i16;
        years = years.saturating_sub(1);
    }

    (years, months as u8, days as u8)
}

/// `endgame.md §9`: gather the certificate body's data-derived fields.
/// The ordinal day, ordinal month number and year-in-words helpers are
/// published; the prose that binds them is not (see
/// [`require_published_endgame_certificate_prose`] and
/// `cleak/u5-spec#82`).
pub fn endgame_certificate_fields(leader_name: &str, clock: GameClock) -> EndgameCertificate {
    EndgameCertificate {
        day_ordinal: endgame_ordinal_word(clock.day).unwrap_or_else(|| clock.day.to_string()),
        month_ordinal: endgame_ordinal_word(clock.month).unwrap_or_else(|| clock.month.to_string()),
        year_hundreds_words: endgame_cardinal_word(clock.year / 100),
        year_remainder_words: endgame_cardinal_word(clock.year % 100),
        leader_name: leader_name.to_string(),
    }
}

/// `endgame.md §9`: the separate final report panel's elapsed campaign
/// time, measured from the fixed campaign-start baseline.
pub fn endgame_final_report(clock: GameClock) -> EndgameFinalReport {
    let (years, months, days) = endgame_elapsed_campaign_time(clock);
    EndgameFinalReport {
        years,
        months,
        days,
    }
}

pub fn endgame_ordinal_word(value: u8) -> Option<String> {
    let word = match value {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        5 => "fifth",
        6 => "sixth",
        7 => "seventh",
        8 => "eighth",
        9 => "ninth",
        10 => "tenth",
        11 => "eleventh",
        12 => "twelfth",
        13 => "thirteenth",
        14 => "fourteenth",
        15 => "fifteenth",
        16 => "sixteenth",
        17 => "seventeenth",
        18 => "eighteenth",
        19 => "nineteenth",
        20 => "twentieth",
        21 => "twenty-first",
        22 => "twenty-second",
        23 => "twenty-third",
        24 => "twenty-fourth",
        25 => "twenty-fifth",
        26 => "twenty-sixth",
        27 => "twenty-seventh",
        28 => "twenty-eighth",
        _ => return None,
    };
    Some(word.to_string())
}

pub fn endgame_cardinal_word(value: u16) -> String {
    match value {
        0 => "zero".to_string(),
        1..=19 => small_cardinal_word(value as u8).unwrap().to_string(),
        20..=99 => two_digit_cardinal_word(value as u8),
        100..=999 => {
            let hundreds = value / 100;
            let remainder = value % 100;
            if remainder == 0 {
                format!("{} hundred", endgame_cardinal_word(hundreds))
            } else {
                format!(
                    "{} hundred {}",
                    endgame_cardinal_word(hundreds),
                    endgame_cardinal_word(remainder)
                )
            }
        }
        1000..=9999 => {
            let thousands = value / 1000;
            let remainder = value % 1000;
            if remainder == 0 {
                format!("{} thousand", endgame_cardinal_word(thousands))
            } else {
                format!(
                    "{} thousand {}",
                    endgame_cardinal_word(thousands),
                    endgame_cardinal_word(remainder)
                )
            }
        }
        _ => value.to_string(),
    }
}

fn small_cardinal_word(value: u8) -> Option<&'static str> {
    Some(match value {
        0 => "zero",
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        10 => "ten",
        11 => "eleven",
        12 => "twelve",
        13 => "thirteen",
        14 => "fourteen",
        15 => "fifteen",
        16 => "sixteen",
        17 => "seventeen",
        18 => "eighteen",
        19 => "nineteen",
        _ => return None,
    })
}

fn two_digit_cardinal_word(value: u8) -> String {
    let tens = match value / 10 {
        2 => "twenty",
        3 => "thirty",
        4 => "forty",
        5 => "fifty",
        6 => "sixty",
        7 => "seventy",
        8 => "eighty",
        9 => "ninety",
        _ => return small_cardinal_word(value).unwrap_or("zero").to_string(),
    };
    match value % 10 {
        0 => tens.to_string(),
        ones => format!("{tens}-{}", small_cardinal_word(ones).unwrap()),
    }
}

pub fn elapsed_time_label(years: u16, months: u8, days: u8) -> String {
    let mut parts = Vec::new();
    if years != 0 {
        parts.push(format!("{years} {}", plural("year", years)));
    }
    if months != 0 {
        parts.push(format!("{months} {}", plural("month", u16::from(months))));
    }
    if days != 0 {
        parts.push(format!("{days} {}", plural("day", u16::from(days))));
    }
    // `endgame.md §9.4`: "A zero component is skipped entirely". With all
    // three components zero the rule leaves the row empty, and this returns
    // that rather than substituting a `0 days` the spec does not publish.
    // Only a same-day victory can reach it, so no ordinary playthrough
    // shows the blank row; cleak/u5-spec#205 asks what the original prints.
    parts.join(", ")
}

fn plural(label: &'static str, amount: u16) -> &'static str {
    if amount == 1 {
        label
    } else {
        match label {
            "year" => "years",
            "month" => "months",
            "day" => "days",
            _ => label,
        }
    }
}

/// One-call grid step toward a target per `endgame.md` §7: examine the slot's
/// current `(x, y)` and choose the axis with the greater remaining distance.
/// Ties prefer the X axis. Returns the new `(x, y)`. When already on the
/// target, returns the input unchanged.
pub fn endgame_step_toward_target(
    current: (isize, isize),
    target: (isize, isize),
) -> (isize, isize) {
    let dx = target.0 - current.0;
    let dy = target.1 - current.1;
    if dx == 0 && dy == 0 {
        return current;
    }
    if dx.unsigned_abs() >= dy.unsigned_abs() {
        if dx == 0 {
            (current.0, current.1 + dy.signum())
        } else {
            (current.0 + dx.signum(), current.1)
        }
    } else if dy == 0 {
        (current.0 + dx.signum(), current.1)
    } else {
        (current.0, current.1 + dy.signum())
    }
}

/// `endgame.md §4` class -> tableau type/tile byte table, returned
/// verbatim as published.
///
/// cleak/u5-spec#82: the *index space* of these bytes is still an open
/// question. Read as top-down world tile ids they are not actor
/// sprites — `0x44` is exactly the authored walkable floor byte of the
/// MISCMAPS cutscene record the tableau uses, and `catalogs/
/// tile-catalog.md` puts `0x0E`/`0x4C` in the terrain/furniture bands.
/// Until the spec says which bank (world atlas, a per-scene
/// ENDSC/END1/END2 sprite bank, or an active-object type-to-tile
/// mapping) these index, the engine keeps the published bytes as-is
/// rather than guessing a translation.
/// `endgame.md §4` class -> tableau actor byte.
///
/// Lookup is by the position of the class letter within the nine-letter
/// class-order string, not by arithmetic on the letter code. The table
/// has nine entries but only four distinct values, so Mage, Bard and
/// Fighter get their own sprite and every other class - the Avatar
/// included - draws the Avatar sprite.
///
/// These are actor bytes, not tile ids: see [`crate::actor_tile_for_byte`].
/// `0x40` is tile 320 (Mage), `0x44` is 324 (Bard), `0x48` is 328
/// (Fighter) and `0x4C` is 332 (Avatar).
pub fn endgame_tableau_tile_for_class_byte(class_byte: u8) -> u8 {
    const ACTOR_BYTES: [u8; ENDGAME_CLASS_ORDER.len()] =
        [0x4C, 0x40, 0x44, 0x48, 0x4C, 0x4C, 0x4C, 0x4C, 0x4C];
    let upper = class_byte.to_ascii_uppercase();
    match ENDGAME_CLASS_ORDER
        .iter()
        .position(|candidate| *candidate == upper)
    {
        Some(index) => ACTOR_BYTES[index],
        None => ENDGAME_TABLEAU_AVATAR_ACTOR_BYTE,
    }
}

/// `endgame.md §4`: the nine-letter class-order string the tableau
/// class table is indexed by.
pub const ENDGAME_CLASS_ORDER: [u8; 9] = *b"AMBFDTPRS";

/// `endgame.md §4`: the sprite every class outside Mage/Bard/Fighter
/// draws, the Avatar.
pub const ENDGAME_TABLEAU_AVATAR_ACTOR_BYTE: u8 = 0x4C;

pub const fn endgame_tableau_party_tile(tile: u8) -> bool {
    matches!(tile, 0x40 | 0x44 | 0x48 | 0x4c)
}

pub fn endgame_tableau_party_placement(
    party_slot: usize,
    class_byte: u8,
) -> Option<EndgameTableauActorPlacement> {
    let target = ENDGAME_TABLEAU_PARTY_TARGETS.get(party_slot).copied()?;
    let tile = endgame_tableau_tile_for_class_byte(class_byte);
    Some(EndgameTableauActorPlacement {
        role: EndgameTableauActorRole::PartyMember(party_slot as u8),
        active_object_slot: party_slot,
        start: ENDGAME_TABLEAU_PARTY_START,
        target,
        tile,
        type_byte: tile,
    })
}

pub fn endgame_tableau_box_placement() -> EndgameTableauActorPlacement {
    EndgameTableauActorPlacement {
        role: EndgameTableauActorRole::SandalwoodBox,
        active_object_slot: ENDGAME_TABLEAU_BOX_SLOT,
        start: ENDGAME_TABLEAU_LORD_BRITISH_POS,
        target: ENDGAME_TABLEAU_LORD_BRITISH_POS,
        tile: ENDGAME_TABLEAU_BOX_ACTOR_BYTE,
        type_byte: ENDGAME_TABLEAU_BOX_ACTOR_BYTE,
    }
}

pub fn endgame_tableau_lord_british_placement(
    target: (usize, usize),
) -> EndgameTableauActorPlacement {
    EndgameTableauActorPlacement {
        role: EndgameTableauActorRole::LordBritish,
        active_object_slot: ENDGAME_TABLEAU_LORD_BRITISH_SLOT,
        start: ENDGAME_TABLEAU_LORD_BRITISH_START,
        target,
        tile: ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE,
        type_byte: ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE,
    }
}

pub fn endgame_tableau_actor_placements(
    party: &[PartyMember],
) -> Vec<EndgameTableauActorPlacement> {
    let mut placements = party
        .iter()
        .take(SAVE_PARTY_SIZE_MAX as usize)
        .enumerate()
        .filter_map(|(slot, member)| endgame_tableau_party_placement(slot, member.class_byte))
        .collect::<Vec<_>>();
    placements.push(endgame_tableau_lord_british_placement(
        ENDGAME_TABLEAU_LORD_BRITISH_THRONE_TARGET,
    ));
    placements
}

/// `audio.md §8.7` step 3's tick pauses either side of the tableau
/// movement sting: "two shared world-animation ticks, the short two-part
/// sting from Section 5.3, then three more".
pub const ENDGAME_TABLEAU_STING_LEADING_TICKS: usize = 2;
/// See [`ENDGAME_TABLEAU_STING_LEADING_TICKS`].
pub const ENDGAME_TABLEAU_STING_TRAILING_TICKS: usize = 3;

/// The same five ticks as wall clock. `audio.md §8.7` gives the spacing
/// only as a comparison with its own capture - "roughly 275 ms spacing
/// and occasional roughly 55 ms extension ... consistent with five tick
/// pauses per step and the extra member-placement tick", explicitly "not
/// a new wall-clock measurement or a universal fixed interval" - so the
/// five ticks are taken at their published count and paid at one BIOS
/// tick each, which is 274.5 ms.
pub const ENDGAME_TABLEAU_STING_HOLD_BIOS_TICKS: u16 =
    (ENDGAME_TABLEAU_STING_LEADING_TICKS + ENDGAME_TABLEAU_STING_TRAILING_TICKS) as u16;

/// `audio.md §8.7`: "plus ... the fixed 40-tick pause after Lord British
/// appears" - paid once, with him on stage at his starting cell and before
/// his first step. `endgame.md §5.2`/`§7` step 1 (issue #298 Q41): "Let the
/// display settle" is "forty paced iterations, after the untimed entry
/// work", the first of which draws the tableau.
pub const ENDGAME_TABLEAU_LORD_BRITISH_ARRIVAL_HOLD_BIOS_TICKS: u16 = 40;

/// `audio.md §8.7`: "Each member also has a separate one-tick placement
/// pause before walking."
pub const ENDGAME_TABLEAU_PLACEMENT_HOLD_BIOS_TICKS: u16 = 1;

/// The hold between the absorption line and the tableau: the stand-in for
/// `endgame.md §5.2`'s untimed entry block.
///
/// Published (`endgame.md §5.2`, issue #298 Q41): after the absorption snap
/// comes "untimed, CPU-bound work" - the absorbed actor's removal and a bare
/// world tick that draws the room without it, the combat exit, the overlay
/// load, the whole-tileset colour remap that dominates it, the file reads
/// and Lord British's placement - with "no tick wait or key poll", and only
/// then "**forty paced iterations**; the first of them is what first draws
/// the tableau", then the first step's two pre-sting iterations. "A frontend
/// that stands a fixed hold in for the untimed block should treat it as a
/// host-dependent cost: at the 3000-cycle setting it is about 1.8 s, roughly
/// thirty-three BIOS ticks, and the forty-two paced iterations follow it."
/// The whole snap-to-first-sting interval measured 4.08 s there. So this is
/// thirty-three ticks, with the arena still on screen
/// ([`PlayState::endgame_absorption_arena`]); the forty iterations are
/// [`ENDGAME_TABLEAU_LORD_BRITISH_ARRIVAL_HOLD_BIOS_TICKS`] and the two are
/// [`ENDGAME_TABLEAU_STING_LEADING_TICKS`]. (Previously a measured value:
/// over forty `doom-endgame-audio` captures the original's snap preceded the
/// first sting by 4.08 s where this engine took 2.27 s.)
pub const ENDGAME_ABSORPTION_HOLD_BIOS_TICKS: u16 = 33;

/// The hold between the last party member settling and Lord British's
/// greeting.
///
/// Published (`endgame.md §5.2`, issue #298 Q41): "The last party member's
/// final step ends with its own tail of **three paced iterations** ... Then
/// come **forty paced iterations**, and the greeting prints" - forty-three
/// after the last sting, measured 2.38 s and 2.39 s at a fixed 3000-cycle
/// emulator setting. The three are that step's
/// [`ENDGAME_TABLEAU_STING_TRAILING_TICKS`]; this is the forty. (Previously
/// a measured value read off the `doom-final-room` timelapses; the reported
/// 1.9 s lower bound did not reproduce.)
pub const ENDGAME_GREETING_HOLD_BIOS_TICKS: u16 = 40;

/// The actor byte the scene compositor actually stamps for one tableau
/// actor, or `None` when nothing is drawn.
///
/// `endgame.md §4`'s actor-byte table names what each slot *holds*; the
/// compositor that places it is the shared one of `visibility.md §8`, and
/// its default helper is terrain-aware for every byte at or above `0x40`
/// (`catalogs/tile-catalog.md §3.1`). The tableau's terrain is the scene
/// grid at the actor's own cell, which in the endgame's combat-range scene
/// is the viewport cell directly. So Lord British (`0x7C`) at the waiting
/// branch's `(4, 1)` lands on the bed and is stamped as its occupied form,
/// and the leader at `(8, 4)` lands on a chair and is stamped seated.
/// Measured 2026-10-16 (`doom-final-room`, beat `answered`;
/// `doom-final-room-waiting`, beats `waiting`, `tableau1`, `tableau2`): the
/// original's bed shows a sleeper with no standing king beside it and its
/// chair a seated figure, where this engine drew both actors standing.
///
/// The selecting rows of the default helper (`visibility.md §8`'s five
/// variant rows) are drawn with variant 0: the tableau's only selecting cell
/// is the `0x90` chair at `(8, 6)`, which only slot 2 of a party of three or
/// more reaches, and this renderer does not consume the shared stream.
pub fn endgame_tableau_actor_drawn_byte(grid: &[u8], object: ActiveObject) -> Option<u8> {
    if object.x >= ENDGAME_TABLEAU_WIDTH || object.y >= ENDGAME_TABLEAU_HEIGHT {
        return None;
    }
    let terrain = |x: usize, y: usize| grid.get(y * TOWN_GRID_SIDE + x).copied();
    let current = terrain(object.x, object.y).unwrap_or(0);
    let previous = object
        .y
        .checked_sub(1)
        .and_then(|y| terrain(object.x, y));
    let next = (object.y + 1 < ENDGAME_TABLEAU_HEIGHT)
        .then(|| terrain(object.x, object.y + 1))
        .flatten();
    let byte = match crate::active_object_default_composite(
        object.tile,
        current,
        previous,
        next,
        object.y,
        0,
    ) {
        crate::ActiveObjectCompositeResult::Suppress
        | crate::ActiveObjectCompositeResult::Direct(_) => return None,
        crate::ActiveObjectCompositeResult::Companion(byte)
        | crate::ActiveObjectCompositeResult::PreviousRowDirectAndCompanion {
            tile: byte, ..
        } => byte,
    };
    (byte != crate::ACTOR_TILE_TRANSPARENT_BYTE).then_some(byte)
}

pub fn endgame_tableau_cell_walkable_fallback(x: usize, y: usize) -> bool {
    x > 0
        && x + 1 < ENDGAME_TABLEAU_WIDTH
        && (3..ENDGAME_TABLEAU_HEIGHT).contains(&y)
        && !(y == 3 && !(3..=7).contains(&x))
}

pub const fn endgame_tableau_cell_tile_walkable(tile: u8) -> bool {
    tile == ENDGAME_TABLEAU_WALKABLE_TILE
}

pub fn endgame_tableau_cell_walkable_in_grid(grid: &[u8], x: usize, y: usize) -> bool {
    if x >= ENDGAME_TABLEAU_WIDTH || y >= ENDGAME_TABLEAU_HEIGHT {
        return false;
    }
    grid.get(y * TOWN_GRID_SIDE + x)
        .copied()
        .is_some_and(endgame_tableau_cell_tile_walkable)
}

pub fn endgame_tableau_role_for_slot(
    slot: usize,
    object: ActiveObject,
) -> Option<EndgameTableauActorRole> {
    if object.is_empty() {
        return None;
    }
    if slot < SAVE_PARTY_SIZE_MAX as usize
        && endgame_tableau_sprite_frame_of(object)
        && endgame_tableau_party_tile(object.type_byte)
    {
        Some(EndgameTableauActorRole::PartyMember(slot as u8))
    } else if slot == ENDGAME_TABLEAU_BOX_SLOT
        && object.type_byte == object.tile
        && matches!(
            object.type_byte,
            ENDGAME_TABLEAU_BOX_ACTOR_BYTE | ENDGAME_TABLEAU_ORB_ACTOR_BYTE
        )
    {
        Some(EndgameTableauActorRole::SandalwoodBox)
    } else if slot == ENDGAME_TABLEAU_LORD_BRITISH_SLOT
        && object.type_byte == ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE
        && endgame_tableau_sprite_frame_of(object)
    {
        Some(EndgameTableauActorRole::LordBritish)
    } else {
        None
    }
}

/// Whether `object`'s displayed frame is one of the four frames of its own
/// class: the class byte itself (frame 1) or one of the three after it.
///
/// `endgame.md §4` (issue #298 Q43, `RETRACTIONS.md` R559): the tableau's
/// party sprites and Lord British are ordinary animated active objects, so
/// their frame byte cycles through the class's four-frame run while the slot
/// keeps its class byte - Lord British through tiles 380 to 383.
pub const fn endgame_tableau_sprite_frame_of(object: ActiveObject) -> bool {
    object.tile & !0x03 == object.type_byte
}

/// The snap step's pause, `active-objects.md §8`: "for every class but `0x5C`
/// the delay is set to six and the position is held".
pub const ENDGAME_TABLEAU_SNAP_HOLD_TICKS: u8 = 6;

/// The most world ticks one look at the clock catches up
/// ([`PlayState::advance_endgame_tableau_animation_to`]), so a host stall
/// does not replay as a burst of frames.
pub const ENDGAME_TABLEAU_ANIMATION_CATCH_UP_TICKS: u64 = 8;

/// One decision point of `active-objects.md §8` **script A** (f2, f3, f4,
/// snap, restart) for an endgame tableau sprite, after the eligibility coin
/// has been accepted. The script position lives in the phase byte's high
/// nibble; `snap_accepted` is the snap step's one draw, "accepted one time in
/// four", and is consulted only at that step.
///
/// "values one to four select that frame of the class's four-frame run and
/// advance the position"; on an accepted snap "the displayed frame is set to
/// the slot's own type byte; ... the delay is set to six and the position is
/// held, so the pass ends and the snap step is retried after the pause.
/// Otherwise the position advances and the pass continues with the next
/// step", and restart "go[es] to step 0 and continue[s]".
pub fn endgame_tableau_script_a_decision(
    object: &mut ActiveObject,
    mut snap_accepted: impl FnMut() -> bool,
) {
    const SNAP_STEP: u8 = 3;
    let base = object.type_byte;
    let mut position = object.phase >> 4;
    loop {
        match position {
            0..=2 => {
                object.tile = base + position + 1;
                position += 1;
                break;
            }
            SNAP_STEP => {
                if snap_accepted() {
                    object.tile = base;
                    object.phase = (SNAP_STEP << 4) | ENDGAME_TABLEAU_SNAP_HOLD_TICKS;
                    return;
                }
                position += 1;
            }
            _ => position = 0,
        }
    }
    object.phase = position << 4;
}

pub fn endgame_tableau_object_from_placement(
    placement: EndgameTableauActorPlacement,
) -> ActiveObject {
    let mut object = ActiveObject::empty();
    write_endgame_tableau_placement(&mut object, placement);
    object
}

fn write_endgame_tableau_placement(
    object: &mut ActiveObject,
    placement: EndgameTableauActorPlacement,
) {
    object.type_byte = placement.type_byte;
    object.tile = placement.tile;
    object.x = placement.start.0;
    object.y = placement.start.1;
    object.phase = ENDGAME_TABLEAU_PHASE;
}

impl PlayState {
    pub fn enter_endgame(&mut self) -> MoveOutcome {
        self.enter_endgame_with_messages(None)
    }

    pub fn enter_endgame_from_game_dir(
        &mut self,
        game_dir: Option<&std::path::Path>,
    ) -> std::io::Result<MoveOutcome> {
        let messages = game_dir.map(require_endgame_messages).transpose()?;
        let tableau_map = game_dir
            .map(|dir| require_miscmaps_cutscene_map(dir, ENDGAME_TABLEAU_CUTSCENE_MAP_RECORD))
            .transpose()?;
        Ok(self.enter_endgame_with_resources(messages, tableau_map))
    }

    pub fn enter_endgame_with_messages(
        &mut self,
        messages: Option<EndgameMessages>,
    ) -> MoveOutcome {
        self.enter_endgame_with_resources(messages, None)
    }

    pub fn enter_endgame_with_resources(
        &mut self,
        messages: Option<EndgameMessages>,
        tableau_map: Option<MiscmapsCutsceneMap>,
    ) -> MoveOutcome {
        self.combat_active = false;
        self.pending_combat_actor_slot = None;
        self.pending_combat_terrain_trigger_slot = None;
        if let Some(map) = tableau_map {
            self.install_endgame_tableau_map(&map);
        }
        // endgame.md §4: clear the live table up front, but let the display-
        // driven entry loop restore, place, and walk party slots one at a time.
        // Lord British is the one initial scripted actor already present.
        self.begin_endgame_tableau_entry();
        // endgame.md §4/§7: the tableau actors start at (5,9) and the
        // movement helper steps them one cell per call with one display
        // tick after each movement, so the walk-in is a sequence of
        // rendered frames rather than an instantaneous placement. The
        // frames are pumped by `advance_endgame_entry_presentation`;
        // any input arriving before the walk-in finishes drains it via
        // `finish_endgame_entry_presentation`.
        let mut endgame = EndgameState::awaiting_first_confirmation_with_messages(messages);
        endgame.entry_restoration_beats = self
            .party
            .iter()
            .filter(|member| {
                character_status_for_byte(member.status)
                    .is_some_and(endgame_needs_tableau_restoration)
            })
            .count()
            .min(u8::MAX as usize) as u8;
        endgame.entry_party_slot = 0;
        endgame.entry_lord_british_pending = true;
        endgame.entry_lord_british_pause_owed = true;
        self.endgame = Some(endgame);
        self.message.clear();
        MoveOutcome::EndgameEntered
    }

    /// Display-driven endgame pump: the beats that advance without a
    /// keystroke. Returns `true` while a frame is still owed.
    ///
    /// `endgame.md §4`/`§7` entry presentation first (restoration beats
    /// and the one-cell tableau walk-in), then `§7.1`'s fade to black.
    /// The fade lands immediately before the first `END.DAT` window, so
    /// finishing it publishes that window's text here rather than
    /// waiting for the next keystroke - the beat samples no input.
    pub fn advance_endgame_display_frame(&mut self) -> bool {
        if self.advance_endgame_entry_presentation() {
            return true;
        }
        if self.advance_endgame_victory_staging_frame() {
            return true;
        }
        if self.advance_endgame_victory_tableau_frame() {
            self.message = self
                .endgame
                .as_ref()
                .map(|endgame| endgame.current_cinematic_text())
                .unwrap_or_default();
            return true;
        }
        // `endgame.md §6` step 3: the refusal branch's terminal loop keeps
        // jittering slots 1, 3, 4 and 5 for as long as the player watches.
        // It is a loop, not a per-keystroke step, so it is pumped here
        // with the rest of the display-driven beats. `cleak/u5-engine#12`.
        if self.endgame.as_ref().is_some_and(|endgame| {
            matches!(endgame.outcome, Some(EndgameOutcome::MissingBoxOrRefused))
        }) {
            if self.advance_endgame_refusal_staging_frame() {
                return true;
            }
            self.advance_endgame_terminal_tableau_jitter_call();
            return true;
        }
        let lead_in_was_visible = self
            .endgame
            .as_ref()
            .is_some_and(|endgame| endgame.cinematic.rite_lead_in_visible);
        let advanced_rite_pause = self
            .endgame
            .as_mut()
            .is_some_and(|endgame| endgame.cinematic.advance_rite_pause_tick(true));
        if advanced_rite_pause {
            self.animation.tick_static_tiles();
            // Only the pause's last tick prints (`He says:` and the second
            // page); the ticks before it leave record 3 where it is.
            if !lead_in_was_visible
                && self
                    .endgame
                    .as_ref()
                    .is_some_and(|endgame| endgame.cinematic.rite_lead_in_visible)
            {
                self.publish_endgame_cinematic_beat();
            }
            return true;
        }
        let advanced = self
            .endgame
            .as_mut()
            .is_some_and(|endgame| endgame.advance_cinematic_frame_operation());
        if advanced {
            self.publish_endgame_cinematic_beat();
        }
        advanced
    }

    /// `endgame.md §4`/`§7` entry presentation pump. Returns `true`
    /// while a frame is still owed. Party slots are processed in order; a
    /// Dead member first gets one exact announcement/restoration beat, then
    /// that member is placed and walked to the target one cell per frame
    /// before the next slot starts. The caller renders after each `true`.
    pub fn advance_endgame_entry_presentation(&mut self) -> bool {
        // `endgame.md §3` step 2's redraw rebuilds the *screen*, not the
        // message window: a paired capture of a walked-in absorption
        // keeps the step's own `>North` echo above the absorption line
        // and runs on into Lord British from there. The line is still
        // deferred to this first pumped frame, because the step commits
        // that echo after the contact handler returns and emitting at
        // the contact would print the two the wrong way round.
        if self.endgame_entry_redraw_pending {
            self.endgame_entry_redraw_pending = false;
            // `text-output.md §10.4`: the step that walked onto the field
            // completed its own command row, so this print's leading line
            // feed lands on a fresh row and leaves one blank row between
            // the echo and the absorption. `cleak/u5-engine#11`.
            // u5-spec#311 Q5: the line carries its own leading and
            // trailing feeds (`combat_absorbed_message`).
            if let Some(line) = self
                .endgame
                .as_mut()
                .and_then(|endgame| endgame.entry_absorption_line.take())
            {
                self.emit_message_line(line);
            }
            // `audio.md §8.7` step 2: "The accepted absorption effect prints
            // its actor/narration, **then** plays the action snap: forty
            // frequency updates from 1200 through 1980 Hz in 20 Hz
            // increments, one calibrated unit per update, followed by a
            // speaker stop." That is `SoundEffect::ActionSnap` exactly -
            // `action_snap()` is `glissando(40, 1, 2000, 1200)`.
            //
            // The whole endgame emitted two cues before this, both of them
            // later beats, so the absorption itself was silent. Measured
            // 2026-09-13 (`qa/tools/audio_compare.py` on
            // `qa/paired/doom-endgame-audio.tsv`): the original sounds for
            // 6.5 s of the nine-second capture after the absorbing step and
            // this engine sounded for none of it.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            // `endgame.md §5.1`: "The absorption line printed by the
            // final-room effect ends with its own line feed, which closes its
            // row." The line carries that feed, and the row state is stated
            // here as well, so it holds whatever the slot printed: otherwise the greeting's leading feed is spent closing this row
            // once the slot is flushed, and the blank row between the two
            // prints vanishes on the greeting's page key - measured
            // 2026-10-16 (`doom-final-room-refusal`, beat `greeting-page`,
            // reading `scroll`).
            self.close_message_row();
            if let Some(endgame) = self.endgame.as_mut() {
                endgame.entry_absorption_hold_owed = true;
            }
        }
        if let Some(endgame) = self.endgame.as_mut()
            && endgame.entry_absorption_hold_owed
        {
            endgame.entry_absorption_hold_owed = false;
            self.staged_narration
                .push_ticks(ENDGAME_ABSORPTION_HOLD_BIOS_TICKS, String::new());
            return true;
        }
        // The hold has run out (or there was none): from here on the pump
        // draws the tableau setup, so the arena the hold showed is released.
        self.endgame_absorption_arena = None;
        if let Some(endgame) = self.endgame.as_mut()
            && endgame.entry_greeting_hold_owed
        {
            endgame.entry_greeting_hold_owed = false;
            self.staged_narration
                .push_ticks_chained(ENDGAME_GREETING_HOLD_BIOS_TICKS);
            return true;
        }
        if self
            .endgame
            .as_ref()
            .is_some_and(|endgame| endgame.entry_party_slot == ENDGAME_ENTRY_GREETING_SLOT)
        {
            self.finish_endgame_entry_prompt();
            return true;
        }
        let Some(endgame) = self.endgame.as_ref() else {
            return false;
        };
        // Once the confirmation has resolved, the victory / refusal
        // scripts own the tableau (endgame.md §6/§7); the entry pump
        // must not keep pulling actors back to their setup targets.
        if endgame.is_terminal() {
            return false;
        }
        self.advance_endgame_entry_tableau_step()
    }

    /// `endgame.md §4`: the setup loop walks the slots in order and
    /// steps each one to its target before moving on, so exactly one
    /// actor moves per pumped frame.
    fn advance_endgame_entry_tableau_step(&mut self) -> bool {
        loop {
            if self
                .endgame
                .as_ref()
                .is_some_and(|endgame| endgame.entry_lord_british_pending)
            {
                // `audio.md §8.7`: "the fixed 40-tick pause after Lord
                // British appears" - once, on stage at his starting cell and
                // before his first step. Measured 2026-10-16 over
                // `doom-endgame-audio` (two runs, one-member party): the
                // original goes silent for about 3.8-3.9 s after the
                // absorption snap and then plays Lord British's five stings,
                // the member's placement tick and the member's four stings
                // as one unbroken train. This engine had paid the pause
                // after his walk instead, splitting the train in two.
                if let Some(endgame) = self.endgame.as_mut()
                    && endgame.entry_lord_british_pause_owed
                {
                    endgame.entry_lord_british_pause_owed = false;
                    self.staged_narration.push_ticks(
                        ENDGAME_TABLEAU_LORD_BRITISH_ARRIVAL_HOLD_BIOS_TICKS,
                        String::new(),
                    );
                    return true;
                }
                if self.step_endgame_tableau_slot_once_with_sting(
                    ENDGAME_TABLEAU_LORD_BRITISH_SLOT,
                    ENDGAME_TABLEAU_LORD_BRITISH_THRONE_TARGET,
                ) {
                    let arrived = self
                        .active_objects
                        .get(ENDGAME_TABLEAU_LORD_BRITISH_SLOT)
                        .is_some_and(|object| {
                            (object.x, object.y) == ENDGAME_TABLEAU_LORD_BRITISH_THRONE_TARGET
                        });
                    if arrived && let Some(endgame) = self.endgame.as_mut() {
                        endgame.entry_lord_british_pending = false;
                    }
                    return true;
                }
                if let Some(endgame) = self.endgame.as_mut() {
                    endgame.entry_lord_british_pending = false;
                }
            }
            let Some(slot) = self
                .endgame
                .as_ref()
                .map(|endgame| endgame.entry_party_slot)
            else {
                return false;
            };
            if slot == ENDGAME_ENTRY_COMPLETE_SLOT {
                return false;
            }
            let slot = usize::from(slot);
            let party_len = self.party.len().min(SAVE_PARTY_SIZE_MAX as usize);
            if slot >= party_len {
                self.finish_endgame_entry_prompt();
                return false;
            }

            if self.restore_endgame_party_member_for_tableau(slot) {
                if let Some(endgame) = self.endgame.as_mut() {
                    endgame.entry_restoration_beats =
                        endgame.entry_restoration_beats.saturating_sub(1);
                }
                let name = self
                    .party_names
                    .get(slot)
                    .and_then(|name| party_name_to_string(name))
                    .unwrap_or_else(|| {
                        panic!(
                            "endgame restoration slot {slot} has no printable in-party name; valid saves must supply every travelling-party name"
                        )
                    });
                self.message.push_str(&format!("\n{name} lives!\n"));
                // audio.md §8.7: after the restoration announcement and the
                // one gameplay-rectangle fill, and before the full stats-panel
                // redraw, run envelope (1, 5000, 40000, 1, 8800). It is a
                // single blocking flourish per restored member, so it belongs
                // on this one restoration beat and not on the placement or
                // walk-in beats that follow.
                self.emit_sound_effect(SoundEffect::EndgameRestoration);
                // The ordinary status/tableau compositor performs the
                // post-flourish redraw.
                self.animation.tick_static_tiles();
                return true;
            }

            let placement = endgame_tableau_party_placement(slot, self.party[slot].class_byte)
                .expect("entry slot is inside the six-member tableau band");
            let installed = self
                .active_objects
                .get(slot)
                .copied()
                .is_some_and(|object| {
                    endgame_tableau_role_for_slot(slot, object) == Some(placement.role)
                });
            if !installed && let Some(object) = self.active_objects.get_mut(slot) {
                write_endgame_tableau_placement(object, placement);
                // `audio.md §8.7`: "Each member also has a separate
                // one-tick placement pause before walking."
                self.staged_narration
                    .push_ticks_chained(ENDGAME_TABLEAU_PLACEMENT_HOLD_BIOS_TICKS);
                return true;
            }

            let at_target = self
                .active_objects
                .get(slot)
                .is_some_and(|object| (object.x, object.y) == placement.target);
            if at_target {
                if let Some(endgame) = self.endgame.as_mut() {
                    endgame.entry_party_slot = endgame.entry_party_slot.saturating_add(1);
                }
                continue;
            }

            let moved = self.step_endgame_tableau_slot_once_with_sting(slot, placement.target);
            let now_at_target = self
                .active_objects
                .get(slot)
                .is_some_and(|object| (object.x, object.y) == placement.target);
            if now_at_target {
                if let Some(endgame) = self.endgame.as_mut() {
                    endgame.entry_party_slot = endgame.entry_party_slot.saturating_add(1);
                    if usize::from(endgame.entry_party_slot) >= party_len {
                        endgame.entry_party_slot = ENDGAME_ENTRY_COMPLETE_SLOT;
                    }
                }
                if let Some(endgame) = self.endgame.as_mut()
                    && endgame.entry_party_slot == ENDGAME_ENTRY_COMPLETE_SLOT
                {
                    // The greeting waits out `§5.2`'s forty paced
                    // iterations first ([`ENDGAME_GREETING_HOLD_BIOS_TICKS`]).
                    endgame.entry_party_slot = ENDGAME_ENTRY_GREETING_SLOT;
                    endgame.entry_greeting_hold_owed = true;
                }
            }
            return moved;
        }
    }

    fn finish_endgame_entry_prompt(&mut self) {
        if let Some(endgame) = self.endgame.as_mut() {
            endgame.entry_party_slot = ENDGAME_ENTRY_COMPLETE_SLOT;
        }
        self.append_endgame_first_prompt();
    }

    /// The row of a live endgame confirmation that stays open for the
    /// player's answer: the last line of whatever prompt is on screen.
    ///
    /// `endgame.md §5`'s prompts come from `ENDMSG.DAT` and end
    /// `You reply: `, so the answer continues that row rather than
    /// opening a new one.
    pub fn endgame_prompt_last_line(&self) -> Option<String> {
        let endgame = self.endgame.as_ref()?;
        if endgame.is_terminal() {
            return None;
        }
        self.message
            .rsplit('\n')
            .find(|line| !line.trim().is_empty())
            .map(str::to_string)
    }

    /// The row of that prompt that is still *open* - waiting with the
    /// cursor on it rather than having closed itself.
    ///
    /// The authored trailing space is what keeps the cursor there:
    /// `You reply: ` ends in one, and `text-output.md` §10.6 puts the
    /// cursor in the cell after it. A line without one has closed its own
    /// row, and the key that follows is read on a fresh row below the
    /// turn-closing blank.
    ///
    /// The distinction did not exist: every line the endgame had on screen
    /// counted, which made the greeting a prompt it is not. Measured
    /// 2026-09-18 (`doom-final-room`, `-refusal` and `-waiting`, beat
    /// `settled`, all reading `cursor`): the original closes `"Well met,
    /// Avatar!"` and waits at column 0 two rows down, where this engine
    /// held the cursor against the closing quote.
    pub fn endgame_open_prompt_line(&self) -> Option<String> {
        self.endgame_prompt_last_line()
            .filter(|line| line.ends_with(' '))
    }

    fn append_endgame_first_prompt(&mut self) {
        // §5's greeting opens its own emission. A slot still holding a
        // line that is already in the transcript - the absorption line,
        // on the route that walks into the field - would otherwise be
        // composed onto and printed a second time beneath the committed
        // copy.
        if !self.message.is_empty() && self.message == self.message_flushed {
            self.message.clear();
            self.message_flushed.clear();
        }
        if let Some(endgame) = self.endgame.as_mut() {
            endgame.greeting_page_pending = true;
        }
        let prompt = self
            .endgame
            .as_ref()
            .expect("endgame entry prompt requires endgame state")
            .first_prompt_text(&self.party_leader_name());
        self.message.push_str(&prompt);
    }

    /// Advance `endgame.md §5`'s greeting page. Returns `true` when a
    /// key was owed and has now been spent printing the first box
    /// question.
    pub fn advance_endgame_greeting_page(&mut self) -> bool {
        let pending = self
            .endgame
            .as_ref()
            .is_some_and(|endgame| endgame.greeting_page_pending);
        if !pending {
            return false;
        }
        if let Some(endgame) = self.endgame.as_mut() {
            endgame.greeting_page_pending = false;
        }
        if let Some(question) = self
            .endgame
            .as_ref()
            .and_then(EndgameState::first_box_question_text)
        {
            self.message.push_str(&question);
        }
        true
    }

    /// Drain any owed entry-presentation frames at once. Used when
    /// input arrives (or a headless caller needs the settled tableau)
    /// before the walk-in has finished rendering.
    pub fn finish_endgame_entry_presentation(&mut self) -> usize {
        let mut frames = 0;
        while self.advance_endgame_entry_presentation() {
            frames += 1;
            if frames > ENDGAME_TABLEAU_SETTLE_STEP_CAP * OOL_SLOTS {
                break;
            }
        }
        frames
    }

    /// `endgame.md §4` (issue #298 Q43, `RETRACTIONS.md` R559): one world
    /// tick of the shared per-slot animator over the tableau sprites.
    ///
    /// "The shared per-slot animator of `systems/active-objects.md` Section 8
    /// runs inside every world tick of the endgame ... and the endgame does
    /// not suspend it. Lord British (`0x7C`) and the four party sprites ...
    /// all select script A". The box (`0x0E`) and the Orb (`0x08`) "lie below
    /// the animator's class floor and stay static", so only the party and
    /// Lord British roles are walked, slot zero to slot thirty-one.
    ///
    /// Each slot takes `active-objects.md §8`'s per-tick rule: a non-zero
    /// delay nibble is decremented and nothing else; at a decision point the
    /// coin ("an inclusive `0..255` draw accepted at 128 or above") gates one
    /// step of [`endgame_tableau_script_a_decision`]. Returns whether any
    /// displayed frame changed.
    pub fn animate_endgame_tableau_sprites(&mut self) -> bool {
        let mut changed = false;
        for slot in 0..self.active_objects.len() {
            let mut object = self.active_objects[slot];
            if !matches!(
                endgame_tableau_role_for_slot(slot, object),
                Some(EndgameTableauActorRole::PartyMember(_) | EndgameTableauActorRole::LordBritish)
            ) {
                continue;
            }
            changed |= self.animate_script_a_object_tick(&mut object);
            self.active_objects[slot] = object;
        }
        if changed {
            self.mark_visibility_dirty();
        }
        changed
    }

    /// One world tick of `active-objects.md §8`'s per-slot rule for one
    /// **script A** sprite: a non-zero delay nibble is decremented and nothing
    /// else; at a decision point the coin ("an inclusive `0..255` draw
    /// accepted at 128 or above") gates one step of
    /// [`endgame_tableau_script_a_decision`]. Returns whether the displayed
    /// frame changed. Shared by the endgame tableau and the Blackthorn
    /// rescue's crowned figure (`endgame.md §4`).
    pub(crate) fn animate_script_a_object_tick(&mut self, object: &mut ActiveObject) -> bool {
        match object.tick_phase() {
            PhaseTick::Steady | PhaseTick::Countdown => return false,
            PhaseTick::DecisionPoint => {}
        }
        if !crate::animation_eligibility_coin_passes(self.random_range_u8(0, 255)) {
            return false;
        }
        let before = object.tile;
        endgame_tableau_script_a_decision(object, || {
            crate::animation_snap_taken(self.random_range_u8(0, 255))
        });
        object.tile != before
    }

    /// Run [`Self::animate_endgame_tableau_sprites`] once for every BIOS tick
    /// that has passed since the last call, so the tableau keeps animating
    /// "through every hold, key wait and the terminal loop"
    /// (`endgame.md §4`) whatever the frontend is waiting on.
    ///
    /// `endgame.md §5` (`RETRACTIONS.md` R558): while the greeting's any-key
    /// read and both yes/no reads wait, "world-animation ticks **do** run",
    /// while gameplay turns, NPC schedules and time advancement do not - so
    /// this touches the tableau sprites and nothing else. The paced
    /// iterations of `§5.2` are one world tick per BIOS tick above the
    /// slow-CPU gate, which is the rate taken here. Nothing runs while the
    /// absorption hold still shows the arena: `§5.2`'s forty paced
    /// iterations, the first of which draws the tableau, follow it.
    ///
    /// Not at the ten key waits that go through the shared wait
    /// ([`Self::endgame_key_wait_in_shared_wait`], u5-spec#304 Q4): there the
    /// shared wait's own no-key pass is the world step
    /// ([`Self::endgame_shared_wait_step`]), and running this as well would
    /// animate the tableau, and draw for it, twice a tick. The clock is
    /// re-anchored instead, so the pump does not catch up once the wait ends.
    pub fn advance_endgame_tableau_animation_to(&mut self, now_secs: f64) -> bool {
        if self.endgame_absorption_arena.is_some() {
            return false;
        }
        let tick = (now_secs.max(0.0) / crate::narration::BIOS_TICK_SECS) as u64;
        if self.endgame_key_wait_in_shared_wait() {
            if let Some(endgame) = self.endgame.as_mut() {
                endgame.tableau_animation_tick = Some(tick);
            }
            return false;
        }
        let Some(endgame) = self.endgame.as_mut() else {
            return false;
        };
        let Some(last) = endgame.tableau_animation_tick.replace(tick) else {
            return false;
        };
        let due = tick
            .saturating_sub(last)
            .min(ENDGAME_TABLEAU_ANIMATION_CATCH_UP_TICKS);
        let mut changed = false;
        for _ in 0..due {
            changed |= self.animate_endgame_tableau_sprites();
        }
        changed
    }

    /// The final-room frame the absorption hold keeps on screen, while it
    /// holds. See [`PlayState::endgame_absorption_arena`].
    pub fn endgame_absorption_arena_view(&self) -> Option<&PlayState> {
        self.endgame.as_ref()?;
        self.endgame_absorption_arena.as_deref()
    }

    /// `true` while the endgame entry presentation still owes frames.
    pub fn endgame_entry_presentation_pending(&self) -> bool {
        self.endgame
            .as_ref()
            .is_some_and(|endgame| !endgame.is_terminal())
            && self.endgame.as_ref().is_some_and(|endgame| {
                endgame.entry_lord_british_pending
                    || endgame.entry_party_slot != ENDGAME_ENTRY_COMPLETE_SLOT
            })
    }

    pub fn ensure_endgame_messages_loaded(
        &mut self,
        game_dir: &std::path::Path,
    ) -> std::io::Result<()> {
        let Some(state) = self.endgame.as_mut() else {
            return Ok(());
        };
        if state.messages.is_none() {
            state.messages = load_endgame_messages(game_dir)?;
        }
        Ok(())
    }

    /// endgame.md section 10: restore Dead travelling-party members to Good status with
    /// HP equal to their stored maximum. Non-Dead statuses keep their entry
    /// state. This mutation is cinematic only and is not committed to disk.
    pub fn restore_party_for_endgame_tableau(&mut self) -> usize {
        let mut restored = 0;
        for member in &mut self.party {
            if character_status_for_byte(member.status)
                .is_some_and(endgame_needs_tableau_restoration)
            {
                member.status = CharacterStatus::Good.save_byte();
                member.hp = member.max_hp;
                restored += 1;
            }
        }
        restored
    }

    fn restore_endgame_party_member_for_tableau(&mut self, slot: usize) -> bool {
        let Some(member) = self.party.get_mut(slot) else {
            return false;
        };
        if !character_status_for_byte(member.status).is_some_and(endgame_needs_tableau_restoration)
        {
            return false;
        }
        member.status = CharacterStatus::Good.save_byte();
        member.hp = member.max_hp;
        true
    }

    fn begin_endgame_tableau_entry(&mut self) {
        self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        for object in &mut self.active_objects {
            object.type_byte = 0;
            object.tile = 0;
        }
        if let Some(slot) = self
            .active_objects
            .get_mut(ENDGAME_TABLEAU_LORD_BRITISH_SLOT)
        {
            write_endgame_tableau_placement(
                slot,
                endgame_tableau_lord_british_placement(ENDGAME_TABLEAU_LORD_BRITISH_START),
            );
        }
    }

    pub fn install_endgame_tableau(&mut self) {
        let placements = endgame_tableau_actor_placements(&self.party);
        self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        for object in &mut self.active_objects {
            object.type_byte = 0;
            object.tile = 0;
        }
        for placement in placements {
            if let Some(slot) = self.active_objects.get_mut(placement.active_object_slot) {
                write_endgame_tableau_placement(slot, placement);
            }
        }
    }

    pub fn install_endgame_tableau_map(&mut self, map: &MiscmapsCutsceneMap) {
        let mut grid = vec![0; TOWN_GRID_BYTES];
        for y in 0..ENDGAME_TABLEAU_HEIGHT {
            for x in 0..ENDGAME_TABLEAU_WIDTH {
                if let Some(tile) = map.tile(x, y) {
                    grid[y * TOWN_GRID_SIDE + x] = tile;
                    // `endgame.md §3.1`: the record is "re-strided into
                    // the buffer a combat arena's terrain normally
                    // occupies", and the endgame then runs with the scene
                    // selector in its combat range - which is what makes
                    // tableau actor coordinates viewport cells directly.
                    // Without this copy the chamber never appears: the
                    // renderer keeps drawing whatever terrain the arena
                    // held. `cleak/u5-engine#7`.
                    self.combat_terrain[y][x] = tile;
                }
            }
        }
        self.grid = grid;
    }

    pub fn advance_endgame_tableau_toward_targets(&mut self) -> bool {
        let placements = endgame_tableau_actor_placements(&self.party);
        let mut moved = false;
        for placement in placements {
            let Some(object) = self.active_objects.get_mut(placement.active_object_slot) else {
                continue;
            };
            if endgame_tableau_role_for_slot(placement.active_object_slot, *object)
                != Some(placement.role)
            {
                continue;
            }
            let current = (object.x as isize, object.y as isize);
            let target = (placement.target.0 as isize, placement.target.1 as isize);
            let next = endgame_step_toward_target(current, target);
            if next != current {
                object.x = next.0 as usize;
                object.y = next.1 as usize;
                self.animation.tick_static_tiles();
                moved = true;
            }
        }
        moved
    }

    pub fn settle_endgame_tableau_to_targets(&mut self) -> usize {
        endgame_tableau_actor_placements(&self.party)
            .into_iter()
            .map(|placement| {
                self.step_endgame_tableau_slot_to_target(
                    placement.active_object_slot,
                    placement.target,
                )
            })
            .sum()
    }

    fn step_endgame_tableau_slot_to_target(
        &mut self,
        slot: usize,
        target: (usize, usize),
    ) -> usize {
        let mut steps = 0;
        while steps < ENDGAME_TABLEAU_SETTLE_STEP_CAP {
            let Some(object) = self.active_objects.get_mut(slot) else {
                break;
            };
            if object.is_empty() {
                break;
            }
            let current = (object.x as isize, object.y as isize);
            let target = (target.0 as isize, target.1 as isize);
            let next = endgame_step_toward_target(current, target);
            if next == current {
                break;
            }
            object.x = next.0 as usize;
            object.y = next.1 as usize;
            self.animation.tick_static_tiles();
            steps += 1;
        }
        steps
    }

    fn step_endgame_tableau_slot_once_to_target(
        &mut self,
        slot: usize,
        target: (usize, usize),
    ) -> bool {
        self.step_endgame_tableau_slot_once(slot, target, false)
    }

    /// [`Self::step_endgame_tableau_slot_once_to_target`] with
    /// `audio.md §8.7` step 3's movement cue, for the walks the section
    /// gives it to.
    fn step_endgame_tableau_slot_once_with_sting(
        &mut self,
        slot: usize,
        target: (usize, usize),
    ) -> bool {
        self.step_endgame_tableau_slot_once(slot, target, true)
    }

    fn step_endgame_tableau_slot_once(
        &mut self,
        slot: usize,
        target: (usize, usize),
        sting: bool,
    ) -> bool {
        let Some(object) = self.active_objects.get_mut(slot) else {
            return false;
        };
        if object.is_empty() {
            return false;
        }
        let current = (object.x as isize, object.y as isize);
        let target = (target.0 as isize, target.1 as isize);
        let next = endgame_step_toward_target(current, target);
        if next == current {
            return false;
        }
        object.x = next.0 as usize;
        object.y = next.1 as usize;
        // `audio.md §8.7` step 3: "Each actual one-cell movement toward an
        // endgame tableau target runs two shared world-animation ticks,
        // the short two-part sting from Section 5.3, then three more
        // shared world-animation ticks. An empty slot or an actor already
        // at its target emits no movement sting and requests none of these
        // five ticks."
        //
        // Both early returns above are those two cases, so reaching here
        // is exactly "an actual one-cell movement". The engine ran one
        // tick and no sting, which is why the original's absorption span
        // carries 6.5 s of the introductory train - §8.7 counts "9, 13,
        // 17, 21, 23 or 27 stings for party sizes 1 through 6" - and this
        // engine's carried 0.2 s, the action snap alone. Measured
        // 2026-09-19 (`qa/paired/doom-endgame-audio.tsv`, beat
        // `absorption`, via `qa/tools/audio_compare.py`).
        //
        // `sting` is what scopes it. §8.7 gives the cue to movement
        // "toward an endgame tableau target" and to the "later
        // target-directed walks", and says outright that "the final
        // random-jitter loop does not use the movement sting" - so a
        // movement phase without the cue is a shape the section has. The
        // exit walks are left without it rather than assumed into it.
        if sting {
            self.run_endgame_tableau_sting_beat();
        } else {
            self.animation.tick_static_tiles();
        }
        true
    }

    /// `audio.md §8.7` step 3's beat: two world-animation ticks, the short
    /// two-part sting, three more ticks, paid as one chained hold.
    fn run_endgame_tableau_sting_beat(&mut self) {
        {
            for _ in 0..ENDGAME_TABLEAU_STING_LEADING_TICKS {
                self.animation.tick_static_tiles();
            }
            self.emit_sound_effect(SoundEffect::EndgameTableauMovementSting);
            for _ in 0..ENDGAME_TABLEAU_STING_TRAILING_TICKS {
                self.animation.tick_static_tiles();
            }
            // Those five are **tick pauses**, not instant animator steps.
            // §8.7 reads the spacing off its capture as "roughly 275 ms ...
            // consistent with five tick pauses per step", and five BIOS
            // ticks is 274.5 ms. An empty staged beat is a pure hold - a
            // wait with nothing to print - and the frontend's staged-
            // narration gate owns the loop while one is pending, so this
            // is what turns the train from a burst into a walk.
            //
            // **Measured** 2026-09-19 (`qa/tools/audio_burst_spacing.py`
            // over `doom-endgame-audio`, beat `absorption`): the original
            // fires 17 bursts from 1.23 s to 7.66 s at a median 250 ms
            // apart, and this engine fired 19 from 1.29 s to 2.00 s at a
            // median 30 ms - the stings present and the pacing absent.
            // Seventeen is also §8.7's own count for a three-member party,
            // which is what that save carries.
            self.staged_narration
                .push_ticks_chained(ENDGAME_TABLEAU_STING_HOLD_BIOS_TICKS);
        }
    }

    fn clear_endgame_tableau_slot_type_tile(&mut self, slot: usize) {
        if let Some(object) = self.active_objects.get_mut(slot) {
            object.type_byte = 0;
            object.tile = 0;
        }
    }

    /// `endgame.md §5.1`/`§6`: one display frame of the waiting branch's
    /// staging. Returns `false` once only the terminal jitter loop is left.
    pub fn advance_endgame_refusal_staging_frame(&mut self) -> bool {
        let Some(phase) = self.endgame.as_ref().map(|endgame| endgame.refusal_phase) else {
            return false;
        };
        let set_phase = |state: &mut Self, phase| {
            if let Some(endgame) = state.endgame.as_mut() {
                endgame.refusal_phase = phase;
            }
        };
        match phase {
            EndgameRefusalPhase::Inactive | EndgameRefusalPhase::Jitter => false,
            EndgameRefusalPhase::LeadInPauseOwed => {
                // `§5.1`: "It then pauses for forty shared world-animation
                // ticks before printing record 10."
                self.staged_narration.push_ticks(
                    u16::from(crate::endgame_cinematic::ENDGAME_WAITING_LEAD_IN_PAUSE_TICKS),
                    String::new(),
                );
                set_phase(self, EndgameRefusalPhase::RecordOwed);
                true
            }
            EndgameRefusalPhase::RecordOwed => {
                let record = self
                    .endgame
                    .as_ref()
                    .map(EndgameState::refusal_record_text)
                    .unwrap_or_default();
                // The lead-in is already on the transcript; record 10 is a
                // new emission, not a rewrite of that slot.
                self.flush_message_slot();
                // The lead-in ends with its own line feed (`§5.1`), so record
                // 10's leading feed is the blank row between them.
                self.close_message_row();
                self.message = record;
                self.message_flushed.clear();
                self.flush_message_slot();
                // Record 10 ends with a line feed, and nothing prints after
                // it: the cursor sits at column 0 of the row beneath, which
                // the window scrolled up to show, and no command row opens
                // there (the branch reads no command). Measured 2026-10-16
                // (`doom-final-room`, beat `answered`): the original's
                // closing ENDMSG.DAT row is the window's second-to-last row with
                // an empty row under it.
                if self.message.ends_with('\n') {
                    self.push_explicit_blank_message_entry();
                }
                set_phase(self, EndgameRefusalPhase::NudgeOwed);
                true
            }
            EndgameRefusalPhase::NudgeOwed => {
                // `§6` step 1: "Slot 0's Y coordinate is decremented once.
                // Then run two world-animation ticks, the short two-part
                // movement sting ..., and three world-animation ticks."
                if let Some(object) = self.active_objects.get_mut(0)
                    && endgame_tableau_role_for_slot(0, *object)
                        == Some(EndgameTableauActorRole::PartyMember(0))
                {
                    object.y = object.y.saturating_sub(1);
                }
                self.run_endgame_tableau_sting_beat();
                set_phase(
                    self,
                    EndgameRefusalPhase::Walking {
                        next_call: 0,
                        moved_this_round: false,
                    },
                );
                true
            }
            EndgameRefusalPhase::Walking {
                mut next_call,
                mut moved_this_round,
            } => {
                // `§6` step 2 (issue #298 Q42): "Each round is one call per
                // actor in exactly that order (slot 2, slot 31, slot 0), and
                // every actual one-cell step carries its own two-iteration,
                // sting, three-iteration beat; an actor already arrived, or
                // an empty slot, adds nothing." So a frame makes calls until
                // one actor actually steps - its beat's hold then separates
                // it from the next actor's step - and a round in which nobody
                // stepped ends the walk.
                loop {
                    if usize::from(next_call) >= ENDGAME_TABLEAU_REFUSAL_WALK_ORDER.len() {
                        if !moved_this_round {
                            set_phase(self, EndgameRefusalPhase::Jitter);
                            return false;
                        }
                        next_call = 0;
                        moved_this_round = false;
                    }
                    let (slot, target) = ENDGAME_TABLEAU_REFUSAL_WALK_ORDER[usize::from(next_call)];
                    next_call += 1;
                    if self.step_endgame_tableau_slot_once_with_sting(slot, target) {
                        set_phase(
                            self,
                            EndgameRefusalPhase::Walking {
                                next_call,
                                moved_this_round: true,
                            },
                        );
                        return true;
                    }
                }
            }
        }
    }

    /// Run the waiting branch's staging through to the jitter loop at once,
    /// for callers that drive keystrokes only.
    pub fn finish_endgame_refusal_staging(&mut self) {
        let mut frames = 0;
        while self.advance_endgame_refusal_staging_frame() {
            frames += 1;
            if frames > ENDGAME_TABLEAU_SETTLE_STEP_CAP * 4 {
                break;
            }
        }
    }

    pub fn prepare_endgame_refusal_tableau(&mut self) {
        if let Some(object) = self.active_objects.get_mut(0) {
            if endgame_tableau_role_for_slot(0, *object)
                == Some(EndgameTableauActorRole::PartyMember(0))
            {
                object.y = object.y.saturating_sub(1);
            }
        }
        loop {
            let moved = self
                .step_endgame_tableau_slot_once_to_target(2, ENDGAME_TABLEAU_REFUSAL_SLOT2_TARGET)
                | self.step_endgame_tableau_slot_once_to_target(
                    ENDGAME_TABLEAU_LORD_BRITISH_SLOT,
                    ENDGAME_TABLEAU_REFUSAL_LORD_BRITISH_TARGET,
                )
                | self.step_endgame_tableau_slot_once_to_target(
                    0,
                    ENDGAME_TABLEAU_REFUSAL_SLOT0_TARGET,
                );
            if !moved {
                break;
            }
        }
    }

    /// `endgame.md §7` steps 4-6: one display frame of the victory branch's
    /// box hand-over ([`EndgameVictoryStagingPhase`]). Returns `false` once
    /// nothing is owed, so the rite pages take over.
    ///
    /// Each actual step is the shared step helper's, with its two-iteration,
    /// sting, three-iteration beat (`audio.md §8.7`: "Later target-directed
    /// walks use the same cue"); a slot already at its target steps nothing.
    pub fn advance_endgame_victory_staging_frame(&mut self) -> bool {
        loop {
            let Some(phase) = self.endgame.as_ref().map(|endgame| endgame.victory_staging) else {
                return false;
            };
            let set_phase = |state: &mut Self, phase| {
                if let Some(endgame) = state.endgame.as_mut() {
                    endgame.victory_staging = phase;
                }
            };
            match phase {
                EndgameVictoryStagingPhase::Inactive => return false,
                EndgameVictoryStagingPhase::LeadPauseOwed => {
                    self.staged_narration
                        .push_ticks(ENDGAME_VICTORY_HANDOVER_LEAD_TICKS, String::new());
                    set_phase(self, EndgameVictoryStagingPhase::WalkUp);
                    return true;
                }
                EndgameVictoryStagingPhase::WalkUp => {
                    if self.step_endgame_tableau_slot_once_with_sting(
                        0,
                        ENDGAME_TABLEAU_VICTORY_EXIT_TARGET,
                    ) {
                        return true;
                    }
                    set_phase(self, EndgameVictoryStagingPhase::WalkBack);
                }
                EndgameVictoryStagingPhase::WalkBack => {
                    if self.step_endgame_tableau_slot_once_with_sting(
                        0,
                        ENDGAME_TABLEAU_PARTY_TARGETS[0],
                    ) {
                        return true;
                    }
                    set_phase(self, EndgameVictoryStagingPhase::BoxPauseOwed);
                }
                EndgameVictoryStagingPhase::BoxPauseOwed => {
                    self.staged_narration
                        .push_ticks_chained(ENDGAME_VICTORY_BOX_PLACEMENT_TICKS);
                    set_phase(self, EndgameVictoryStagingPhase::BoxOwed);
                    return true;
                }
                EndgameVictoryStagingPhase::BoxOwed => {
                    if self.active_objects.len() < OOL_SLOTS {
                        self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
                    }
                    if let Some(slot) = self.active_objects.get_mut(ENDGAME_TABLEAU_BOX_SLOT) {
                        write_endgame_tableau_placement(slot, endgame_tableau_box_placement());
                    }
                    self.mark_visibility_dirty();
                    set_phase(self, EndgameVictoryStagingPhase::Inactive);
                    // Step 6: record `3` prints now, after the box, and the
                    // forty paced iterations of `§5.1` run from here
                    // ([`crate::endgame_cinematic::EndgameCinematic`]). A
                    // file without rite records opens on the throne tableau,
                    // which has no prose of its own.
                    self.publish_endgame_cinematic_beat();
                    return true;
                }
            }
        }
    }

    /// Put the current cinematic beat's text in the message slot. A victory
    /// rite page is a print into the ordinary scrolling message window
    /// (`endgame.md §3.1`, `§5.1`), so it goes onto the transcript beneath
    /// the page before it; overwriting the unflushed slot dropped record 3
    /// and then the `He says:` page from the history (playtest P21-04).
    /// Other beats keep their plain slot write.
    fn publish_endgame_cinematic_beat(&mut self) {
        let Some(endgame) = self.endgame.as_ref() else {
            self.message.clear();
            return;
        };
        let text = endgame.current_cinematic_text();
        if !matches!(
            endgame.cinematic.step,
            crate::endgame_cinematic::EndgameCinematicStep::RiteMessage(_)
        ) || text.is_empty()
        {
            self.message = text;
            return;
        }
        // The waiting branch's record-10 pattern: commit what the slot
        // holds, then print the page as a new emission.
        self.flush_message_slot();
        self.message = text.clone();
        self.message_flushed.clear();
        self.flush_message_slot();
        // The page is one whole print, so its own last byte is where it
        // leaves the cursor: record 3 stops mid-row (the lead-in's first
        // feed closes that row), and records 4..9 close theirs.
        if text.ends_with('\n') {
            self.close_message_row();
        } else {
            self.leave_last_transcript_row_open();
        }
    }

    /// Run the box hand-over through to record `3` at once, for callers that
    /// drive keystrokes only.
    pub fn finish_endgame_victory_staging(&mut self) {
        for _ in 0..ENDGAME_TABLEAU_SETTLE_STEP_CAP {
            if !self.advance_endgame_victory_staging_frame() {
                break;
            }
        }
    }

    /// Whether the victory rite is in one of its automatic stretches, which
    /// read no key: `endgame.md §7` steps 4-5's box hand-over, the `§5.1`
    /// forty-iteration pause between record `3` and `He says:`, and step
    /// 9-13's gate sequence after the Orb's key read.
    pub fn endgame_victory_reads_no_key(&self) -> bool {
        self.endgame.as_ref().is_some_and(|endgame| {
            endgame.outcome == Some(EndgameOutcome::Victory)
                && (endgame.victory_staging != EndgameVictoryStagingPhase::Inactive
                    || endgame.cinematic.rite_pause_ticks_remaining != 0
                    || matches!(
                        endgame.victory_tableau_phase,
                        EndgameVictoryTableauPhase::GateRise { .. }
                            | EndgameVictoryTableauPhase::GateFullHold { .. }
                            | EndgameVictoryTableauPhase::ActorExit
                            | EndgameVictoryTableauPhase::GateSink { .. }
                            | EndgameVictoryTableauPhase::RestoreFloor
                    ))
        })
    }

    /// Whether the victory branch's box hand-over still owes frames.
    pub fn endgame_victory_staging_pending(&self) -> bool {
        self.endgame
            .as_ref()
            .is_some_and(|endgame| endgame.victory_staging != EndgameVictoryStagingPhase::Inactive)
    }

    pub fn complete_endgame_victory_tableau(&mut self) {
        if matches!(
            self.endgame
                .as_ref()
                .map(|endgame| endgame.victory_tableau_phase),
            Some(EndgameVictoryTableauPhase::Inactive)
        ) {
            self.present_endgame_orb_frame();
        }
        if matches!(
            self.endgame
                .as_ref()
                .map(|endgame| endgame.victory_tableau_phase),
            Some(EndgameVictoryTableauPhase::OrbAwaitingAcknowledgement)
        ) {
            self.advance_endgame_victory_tableau_exit_step();
        }
        for _ in 0..(ENDGAME_TABLEAU_SETTLE_STEP_CAP * OOL_SLOTS) {
            if !self.advance_endgame_victory_tableau_frame() {
                break;
            }
        }
    }

    pub fn advance_endgame_victory_tableau_exit_step(&mut self) -> bool {
        let phase = self
            .endgame
            .as_ref()
            .map(|endgame| endgame.victory_tableau_phase)
            .unwrap_or_default();
        match phase {
            EndgameVictoryTableauPhase::Inactive => self.present_endgame_orb_frame(),
            EndgameVictoryTableauPhase::OrbAwaitingAcknowledgement => {
                // endgame.md §7 step 7: "Change slot 6's actor byte to
                // `0x08`, the Orb spark - the box opens. A blocking key read
                // and a speaker sting follow." This arm is that key read's
                // resolution, so the sting sounds here, after the
                // acknowledgement and before step 8 clears the slot.
                // audio.md §8.7 supplies the envelope only:
                // "the later box/tableau presentation uses envelope
                // (1, 10000, 50000, 1, 5200)". Reaching this arm moves the
                // phase to `GateRise`, so it sounds once per tableau.
                self.emit_sound_effect(SoundEffect::EndgameTableau);
                self.clear_endgame_tableau_slot_type_tile(ENDGAME_TABLEAU_BOX_SLOT);
                let (x, y) = ENDGAME_GATE_CELL;
                if let Some(cell) = self.grid.get_mut(y * TOWN_GRID_SIDE + x) {
                    *cell = NATURAL_MOONGATE_TERRAIN_TILE;
                }
                self.natural_moongate_counter = 0;
                if let Some(endgame) = self.endgame.as_mut() {
                    endgame.victory_tableau_phase =
                        EndgameVictoryTableauPhase::GateRise { next_phase: 1 };
                }
                self.animation.tick_static_tiles();
                true
            }
            _ => false,
        }
    }

    fn present_endgame_orb_frame(&mut self) -> bool {
        let Some(box_object) = self.active_objects.get_mut(ENDGAME_TABLEAU_BOX_SLOT) else {
            return false;
        };
        if endgame_tableau_role_for_slot(ENDGAME_TABLEAU_BOX_SLOT, *box_object)
            != Some(EndgameTableauActorRole::SandalwoodBox)
        {
            return false;
        }
        box_object.type_byte = ENDGAME_TABLEAU_ORB_ACTOR_BYTE;
        box_object.tile = ENDGAME_TABLEAU_ORB_ACTOR_BYTE;
        // endgame.md §7 step 7: the swap is silent. "A blocking key read
        // and a speaker sting follow" - the sting belongs after the read,
        // in the acknowledgement arm of the exit step.
        if let Some(endgame) = self.endgame.as_mut() {
            endgame.victory_tableau_phase = EndgameVictoryTableauPhase::OrbAwaitingAcknowledgement;
        }
        self.animation.tick_static_tiles();
        true
    }

    /// `endgame.md §7` + `cleak/u5-spec#136`: pump one automatic frame
    /// after the Orb acknowledgement. The endgame writes the same
    /// save-backed phase byte as overworld moongates, so the existing
    /// `0x44`/`0xDC` row-splice renderer owns every partial raster.
    fn advance_endgame_victory_tableau_frame(&mut self) -> bool {
        let is_victory_tableau = self.endgame.as_ref().is_some_and(|endgame| {
            matches!(endgame.outcome, Some(EndgameOutcome::Victory))
                && matches!(
                    endgame.cinematic.step,
                    crate::endgame_cinematic::EndgameCinematicStep::ThroneTableau
                )
        });
        if !is_victory_tableau {
            return false;
        }

        loop {
            let phase = self
                .endgame
                .as_ref()
                .map(|endgame| endgame.victory_tableau_phase)
                .unwrap_or_default();
            match phase {
                EndgameVictoryTableauPhase::Inactive => {
                    return self.present_endgame_orb_frame();
                }
                EndgameVictoryTableauPhase::OrbAwaitingAcknowledgement
                | EndgameVictoryTableauPhase::Complete => return false,
                EndgameVictoryTableauPhase::GateRise { next_phase } => {
                    debug_assert!((1..MOONGATE_PHASE_FULL).contains(&next_phase));
                    self.natural_moongate_counter = next_phase;
                    self.animation.tick_static_tiles();
                    if let Some(endgame) = self.endgame.as_mut() {
                        endgame.victory_tableau_phase = if next_phase + 1 < MOONGATE_PHASE_FULL {
                            EndgameVictoryTableauPhase::GateRise {
                                next_phase: next_phase + 1,
                            }
                        } else {
                            EndgameVictoryTableauPhase::GateFullHold {
                                ticks_remaining: ENDGAME_GATE_FULL_HOLD_TICKS,
                            }
                        };
                    }
                    return true;
                }
                EndgameVictoryTableauPhase::GateFullHold { ticks_remaining } => {
                    debug_assert!(ticks_remaining > 0);
                    self.natural_moongate_counter = MOONGATE_PHASE_FULL;
                    self.animation.tick_static_tiles();
                    if let Some(endgame) = self.endgame.as_mut() {
                        endgame.victory_tableau_phase = if ticks_remaining == 1 {
                            EndgameVictoryTableauPhase::ActorExit
                        } else {
                            EndgameVictoryTableauPhase::GateFullHold {
                                ticks_remaining: ticks_remaining - 1,
                            }
                        };
                    }
                    return true;
                }
                EndgameVictoryTableauPhase::ActorExit => {
                    if self.advance_endgame_gate_actor_exit_frame() {
                        return true;
                    }
                    if let Some(endgame) = self.endgame.as_mut() {
                        endgame.victory_tableau_phase =
                            EndgameVictoryTableauPhase::GateSink { next_phase: 15 };
                    }
                }
                EndgameVictoryTableauPhase::GateSink { next_phase } => {
                    debug_assert!((1..MOONGATE_PHASE_FULL).contains(&next_phase));
                    self.natural_moongate_counter = next_phase;
                    self.animation.tick_static_tiles();
                    if let Some(endgame) = self.endgame.as_mut() {
                        endgame.victory_tableau_phase = if next_phase == 1 {
                            EndgameVictoryTableauPhase::RestoreFloor
                        } else {
                            EndgameVictoryTableauPhase::GateSink {
                                next_phase: next_phase - 1,
                            }
                        };
                    }
                    return true;
                }
                EndgameVictoryTableauPhase::RestoreFloor => {
                    self.natural_moongate_counter = 0;
                    let (x, y) = ENDGAME_GATE_CELL;
                    if let Some(cell) = self.grid.get_mut(y * TOWN_GRID_SIDE + x) {
                        *cell = ENDGAME_TABLEAU_WALKABLE_TILE;
                    }
                    if let Some(endgame) = self.endgame.as_mut() {
                        endgame.victory_tableau_phase = EndgameVictoryTableauPhase::Complete;
                        endgame.advance_cinematic();
                    }
                    return true;
                }
            }
        }
    }

    fn advance_endgame_gate_actor_exit_frame(&mut self) -> bool {
        if self
            .active_objects
            .get(ENDGAME_TABLEAU_LORD_BRITISH_SLOT)
            .copied()
            .is_some_and(|object| {
                endgame_tableau_role_for_slot(ENDGAME_TABLEAU_LORD_BRITISH_SLOT, object)
                    == Some(EndgameTableauActorRole::LordBritish)
            })
        {
            if self.step_endgame_tableau_slot_once_to_target(
                ENDGAME_TABLEAU_LORD_BRITISH_SLOT,
                ENDGAME_TABLEAU_VICTORY_EXIT_TARGET,
            ) {
                return true;
            }
            self.clear_endgame_tableau_slot_type_tile(ENDGAME_TABLEAU_LORD_BRITISH_SLOT);
            self.animation.tick_static_tiles();
            return true;
        }

        for slot in 0..self.party.len().min(SAVE_PARTY_SIZE_MAX as usize) {
            let Some(object) = self.active_objects.get(slot).copied() else {
                continue;
            };
            if endgame_tableau_role_for_slot(slot, object)
                != Some(EndgameTableauActorRole::PartyMember(slot as u8))
            {
                continue;
            }
            if self
                .step_endgame_tableau_slot_once_to_target(slot, ENDGAME_TABLEAU_VICTORY_EXIT_TARGET)
            {
                return true;
            }
            self.clear_endgame_tableau_slot_type_tile(slot);
            self.animation.tick_static_tiles();
            return true;
        }

        false
    }

    pub fn endgame_tableau_is_settled(&self) -> bool {
        endgame_tableau_actor_placements(&self.party)
            .into_iter()
            .all(|placement| {
                self.active_objects
                    .get(placement.active_object_slot)
                    .copied()
                    .is_some_and(|object| {
                        endgame_tableau_role_for_slot(placement.active_object_slot, object)
                            == Some(placement.role)
                            && (object.x, object.y) == placement.target
                    })
            })
    }

    /// `endgame.md §6` step 3: one round of the terminal loop - one wander
    /// call for each of slots 1, 3, 4 and 5 - for callers that drive
    /// keystrokes only. Each call still carries its own one-tick hold.
    ///
    /// The round runs through the same call cursor as the display pump
    /// ([`Self::advance_endgame_terminal_tableau_jitter_call`]), so a key
    /// that lands mid-round continues the slot order (and its PRNG draws)
    /// where the pump left it instead of restarting at slot 1.
    pub fn advance_endgame_terminal_tableau_jitter(&mut self) -> bool {
        let mut moved = false;
        for _ in ENDGAME_TABLEAU_JITTER_SLOTS {
            moved |= self.advance_endgame_terminal_tableau_jitter_call();
        }
        moved
    }

    /// `endgame.md §6` step 3: one wander call of the terminal loop, the
    /// next of slots 1, 3, 4 and 5 in turn ([`EndgameState::terminal_jitter_next_call`]).
    pub fn advance_endgame_terminal_tableau_jitter_call(&mut self) -> bool {
        let call = self.endgame.as_mut().map_or(0, |endgame| {
            let call = usize::from(endgame.terminal_jitter_next_call)
                % ENDGAME_TABLEAU_JITTER_SLOTS.len();
            endgame.terminal_jitter_next_call =
                ((call + 1) % ENDGAME_TABLEAU_JITTER_SLOTS.len()) as u8;
            call
        });
        self.run_endgame_terminal_tableau_jitter_call(ENDGAME_TABLEAU_JITTER_SLOTS[call])
    }

    /// One terminal wander call for `slot`.
    ///
    /// `§6` (issue #298 Q42): each call's display-tick advance "is exactly
    /// **one paced iteration** - a world tick plus a one-tick wait request -
    /// and it is paid by every call, for an empty slot and on a refused coin
    /// alike". The tick sits **between** consecutive calls, so a mover's step
    /// shows a tick before the next slot's: each call pushes its own chained
    /// BIOS tick rather than one four-tick block per round, which had every
    /// round's movers step on the same frame. An empty slot (or one beyond
    /// the party) takes no coin draw.
    fn run_endgame_terminal_tableau_jitter_call(&mut self, slot: usize) -> bool {
        self.staged_narration.push_ticks_chained(1);
        let party_slots = self.party.len().min(SAVE_PARTY_SIZE_MAX as usize);
        let occupied = slot < party_slots
            && self.active_objects.get(slot).copied().is_some_and(|object| {
                endgame_tableau_role_for_slot(slot, object)
                    == Some(EndgameTableauActorRole::PartyMember(slot as u8))
            });
        if !occupied {
            self.animation.tick_static_tiles();
            return false;
        }
        self.advance_endgame_tableau_jitter_slot(slot)
    }

    fn advance_endgame_tableau_jitter_slot(&mut self, actor_slot: usize) -> bool {
        if u5_prng_range_u16(&mut self.prng_state, 0, 1) != 0 {
            self.animation.tick_static_tiles();
            return false;
        }
        for _ in 0..8 {
            let dir = u5_prng_range_u16(&mut self.prng_state, 0, 3) as u8;
            let (dx, dy) = match dir {
                0 => (1isize, 0isize),
                1 => (-1, 0),
                2 => (0, 1),
                _ => (0, -1),
            };
            let Some(object) = self.active_objects.get(actor_slot).copied() else {
                self.animation.tick_static_tiles();
                return false;
            };
            if object.is_empty() {
                self.animation.tick_static_tiles();
                return false;
            }
            let nx = object.x as isize + dx;
            let ny = object.y as isize + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let nx = nx as usize;
            let ny = ny as usize;
            if !endgame_tableau_cell_walkable_in_grid(&self.grid, nx, ny) {
                continue;
            }
            if let Some(object) = self.active_objects.get_mut(actor_slot) {
                object.x = nx;
                object.y = ny;
            }
            self.animation.tick_static_tiles();
            return true;
        }
        self.animation.tick_static_tiles();
        false
    }

    pub fn resolve_endgame_confirmation(&mut self, answer: bool) -> MoveOutcome {
        self.resolve_endgame_confirmation_with_narrative(answer, None)
    }

    pub fn resolve_endgame_confirmation_from_game_dir(
        &mut self,
        answer: bool,
        game_dir: &std::path::Path,
    ) -> std::io::Result<MoveOutcome> {
        // `endgame.md §5`: a Yes resolves the dialogue whether it is the
        // first answer (which skips the sandalwood question) or the second,
        // so either one can open the victory rite and needs `END.DAT`.
        let needs_final_narrative = self.endgame.as_ref().is_some_and(|state| {
            !state.is_terminal()
                && answer
                && self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] != 0
        });
        let final_narrative = if needs_final_narrative {
            Some(require_end_narrative(game_dir)?)
        } else {
            None
        };
        Ok(self.resolve_endgame_confirmation_with_narrative(answer, final_narrative))
    }

    fn resolve_endgame_confirmation_with_narrative(
        &mut self,
        answer: bool,
        final_narrative: Option<EndNarrative>,
    ) -> MoveOutcome {
        self.resolve_endgame_confirmation_inner(answer, final_narrative, true)
    }

    /// `echo_final_answer` is `false` only on the first Yes's recursion,
    /// whose answer has already been echoed onto record 1's reply row.
    fn resolve_endgame_confirmation_inner(
        &mut self,
        answer: bool,
        final_narrative: Option<EndNarrative>,
        echo_final_answer: bool,
    ) -> MoveOutcome {
        // `endgame.md §4`/`§5`: the tableau setup pass completes before
        // the dialogue's blocking prompts are answered. If input
        // arrives while the walk-in is still being rendered, settle it
        // first so the branch scripts start from the published targets.
        self.finish_endgame_entry_presentation();
        // `§7.1`: the fade to black is a beat the player watches, and
        // it consumes no input. If a caller that only drives keystrokes
        // reaches here with the fade still owed, run it and let this
        // step present its result - the following `END.DAT` window -
        // rather than advancing past that window.
        if self
            .endgame
            .as_ref()
            .is_some_and(|endgame| endgame.cinematic.fade_to_black_rect.is_some())
        {
            self.advance_endgame_display_frame();
            return MoveOutcome::Observed;
        }
        let Some(current) = self.endgame.clone() else {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        if current.is_terminal() {
            // Victory branch advances the cinematic page-flip on every
            // keystroke until the closer is finished, then keeps the terminal
            // final panel active.
            if matches!(current.outcome, Some(EndgameOutcome::Victory)) {
                // `§7` steps 4-6 read no key, so the key dispatcher buffers
                // one that arrives during the hand-over until it finishes.
                // A direct caller that resolves the terminal step while the
                // hand-over is still owed finishes it here instead.
                if self.endgame_victory_staging_pending() {
                    self.finish_endgame_victory_staging();
                    return MoveOutcome::Observed;
                }
                if matches!(
                    current.cinematic.step,
                    crate::endgame_cinematic::EndgameCinematicStep::ThroneTableau
                ) {
                    if self.advance_endgame_victory_tableau_exit_step() {
                        self.message = current.current_cinematic_text();
                        return MoveOutcome::Observed;
                    }
                    if let Some(state) = self.endgame.as_mut()
                        && matches!(
                            state.victory_tableau_phase,
                            EndgameVictoryTableauPhase::Complete
                        )
                    {
                        state.advance_cinematic();
                        self.message = state.current_cinematic_text();
                    }
                    return MoveOutcome::Observed;
                }
                if let Some(state) = self.endgame.as_mut() {
                    if state.cinematic_is_finished() {
                        self.message = state.current_cinematic_text();
                    } else {
                        state.advance_cinematic();
                        self.publish_endgame_cinematic_beat();
                    }
                    return MoveOutcome::Observed;
                }
            }
            match current.outcome {
                Some(EndgameOutcome::Victory) => {
                    unreachable!("victory branch returns above")
                }
                Some(EndgameOutcome::MissingBoxOrRefused) => {
                    // `endgame.md §6`: the terminal loop only "jitters slots
                    // 1, 3, 4, and 5" - it prints nothing. **Measured**
                    // 2026-09-08 (`qa/paired/doom-final-room-waiting.tsv`):
                    // the stock window still reads record 10's exchange
                    // eight seconds into the jitter, where this engine had
                    // appended `Lord British waits with thee in the ending
                    // tableau.` under it.
                    self.finish_endgame_refusal_staging();
                    self.advance_endgame_terminal_tableau_jitter();
                }
                None => unreachable!("terminal endgame has an outcome"),
            }
            return MoveOutcome::Observed;
        }

        if current.first_confirmation.is_none() {
            self.endgame = Some(EndgameState::awaiting_final_confirmation_with_messages(
                answer,
                current.messages,
            ));
            // §5.1: the answer is echoed "as the literal `Yes` or `No`
            // followed by a blank line". Records 1 and 2 end
            // `You reply: ` **with a trailing space**, so
            // `commands.md §5.3` keeps the echo on that row - measured:
            // the original reads `You reply: Yes`.
            // The echo is committed against the prompt's last line whether
            // or not that line stayed open - `commit_typed_prompt_line`
            // decides where it lands. Only the *cursor* cares about the
            // trailing space, which is why this takes the unfiltered line.
            if let Some(prompt) = self.endgame_prompt_last_line() {
                // The prompt may still be in the message slot; the shell
                // flushes per turn but a direct caller does not.
                self.flush_message_slot();
                self.commit_typed_prompt_line(&prompt, yes_no_word(answer));
                // §5.2: the answer is echoed "as the literal `Yes` or `No`
                // followed by a blank line, and only then does the next
                // record print". The echo closes the reply row and the
                // blank is its own. Measured 2026-09-18
                // (`doom-final-room`, beat `answered`, reading `scroll`):
                // the original has a blank row between `You reply: Yes` and
                // `"I see...` and this engine ran them together, which put
                // the rest of the exchange one row high.
                self.push_producer_blank_message_entry();
            }
            // `endgame.md §5` step 2: "**Only a first No** asks the second
            // question ... A first Yes skips this question whether the
            // party owns the box or not." The box flag decides the outcome
            // (step 3), not whether the question is shown (R404). A paired
            // capture of the Doom final room shows a Yes without the box
            // going straight to record 10's "pull up a chair" exchange
            // (`cleak/u5-engine#12`).
            if answer {
                return self.resolve_endgame_confirmation_inner(answer, final_narrative, false);
            }
            self.message = self
                .endgame
                .as_ref()
                .expect("endgame state was just installed")
                .second_prompt_text(answer);
            return MoveOutcome::Used;
        }

        // `§5.1`: the second answer is echoed exactly as the first is - the
        // literal on record 2's `You reply: ` row, then a blank line - and
        // only then does the next record print. A first Yes reaches here by
        // recursion with its own echo already committed, and the second
        // question's reply row is the only one still open.
        //
        // Measured 2026-10-16 (`doom-final-room-waiting`, beats `waiting`,
        // `tableau1`, `tableau2`, reading `scroll`): the original reads
        // `You reply: No`, a blank row, then `"I see...`; this engine left the
        // reply row bare and ran the lead-in straight under it.
        if echo_final_answer && let Some(prompt) = self.endgame_open_prompt_line() {
            self.flush_message_slot();
            self.commit_typed_prompt_line(&prompt, yes_no_word(answer));
            self.push_producer_blank_message_entry();
            self.message.clear();
            self.message_flushed.clear();
        }
        let first = current.first_confirmation.unwrap_or(false);
        let has_box = self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] != 0;
        let certificate = endgame_certificate_fields(&self.party_leader_name(), self.clock);
        let final_report = endgame_final_report(self.clock);
        let next = EndgameState::terminal(
            first,
            answer,
            has_box,
            certificate,
            final_report,
            current.messages,
            final_narrative,
        );
        let mut next = next;
        self.message = match next.outcome {
            // `§7` steps 4-6: the box hand-over comes first, as display
            // frames ([`EndgameVictoryStagingPhase`]); the first rite beat -
            // or the throne tableau, when `ENDMSG.DAT` carried no rite
            // records - prints only after the box is placed.
            Some(EndgameOutcome::Victory) => {
                next.victory_staging = EndgameVictoryStagingPhase::LeadPauseOwed;
                String::new()
            }
            // `§5.1`: the lead-in alone. The forty-tick pause, record 10
            // and `§6`'s staging are display frames
            // ([`EndgameRefusalPhase`]).
            Some(EndgameOutcome::MissingBoxOrRefused) => {
                next.refusal_phase = EndgameRefusalPhase::LeadInPauseOwed;
                crate::endgame_cinematic::ENDGAME_WAITING_LEAD_IN.to_string()
            }
            None => unreachable!("terminal endgame has an outcome"),
        };
        self.endgame = Some(next);
        MoveOutcome::EndgameEntered
    }

    pub fn party_leader_name(&self) -> String {
        self.party_names
            .first()
            .and_then(|name| party_name_to_string(name))
            .unwrap_or_else(|| "Avatar".to_string())
    }
}

#[cfg(test)]
mod fade_to_black_tests {
    use super::*;
    use crate::display_driver::{DisplayRenderTarget, EgaDisplaySurface};

    fn seeded_surface() -> EgaDisplaySurface {
        // Put distinct non-black content on BOTH surfaces, so a beat
        // that forgot either half would leave visible pixels behind.
        let mut surface = EgaDisplaySurface::new();
        let whole = crate::display_driver::normalize_clamp_pixel_rect(0, 0, 319, 199).unwrap();
        surface.set_render_target(DisplayRenderTarget::Front);
        surface.fill_rect(whole, 9);
        surface.fill_back_rect(whole, 4);
        surface
    }

    #[test]
    fn the_fade_blanks_the_whole_visible_page() {
        // endgame.md §7.1 / cleak/u5-spec#53: the net player-visible
        // effect is that the whole screen goes black.
        let mut surface = seeded_surface();
        assert!(surface.front_pixels().iter().any(|pixel| *pixel != 0));

        run_endgame_fade_to_black(&mut surface).unwrap();

        assert!(
            surface
                .front_pixels()
                .iter()
                .all(|pixel| *pixel == ENDGAME_FADE_TO_BLACK_COLOR),
            "the fade must leave the visible page entirely black"
        );
    }

    #[test]
    fn omitting_the_hidden_fill_would_dissolve_stale_content_onto_the_screen() {
        // #53's warning, made executable: the fill is load-bearing.
        // Running only the dissolve half publishes the stale hidden
        // surface instead of black.
        let mut surface = seeded_surface();
        let whole = crate::display_driver::normalize_clamp_pixel_rect(0, 0, 319, 199).unwrap();
        surface.dissolve_back_to_front_rect(whole);
        assert!(
            surface.front_pixels().iter().all(|pixel| *pixel == 4),
            "without the fill the dissolve publishes stale offscreen content"
        );
    }

    #[test]
    fn the_fade_is_silent_and_uninterruptible_by_the_time_it_runs() {
        // §9.6 / #53: the gate is cleared permanently by the first
        // fixed-cell glyph, and the victory rite has drawn plenty.
        let gate = endgame_fade_to_black_abort_gate();
        assert!(!gate.is_armed());
        assert!(!gate.samples_input_after_copy(1));
        assert!(!gate.samples_input_after_copy(2));
    }

    #[test]
    fn the_dissolve_visits_every_pixel_of_the_rectangle_exactly_once() {
        // #53: "visiting every pixel exactly once in a deterministic
        // pseudo-random order". This is what makes the compositor's
        // flat fill an exact model of the completed beat rather than an
        // approximation of it.
        let mut dissolve =
            crate::RectangleDissolve::new(crate::endgame_cinematic::ENDGAME_FADE_TO_BLACK_RECT)
                .unwrap();
        let total = dissolve.pixel_count() as usize;
        assert_eq!(total, 320 * 200);

        let mut seen = vec![false; total];
        let mut row_major = true;
        let mut previous = None;
        while let Some((x, y)) = dissolve.next_pixel() {
            let index = usize::from(y) * 320 + usize::from(x);
            assert!(!seen[index], "pixel ({x}, {y}) visited twice");
            seen[index] = true;
            if let Some(previous) = previous {
                if index != previous + 1 {
                    row_major = false;
                }
            }
            previous = Some(index);
        }
        assert!(
            seen.into_iter().all(|visited| visited),
            "every pixel visited"
        );
        assert!(!row_major, "the order is scattered, not row-major");
    }
}
