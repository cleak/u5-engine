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
    /// `endgame.md §4`: next party slot owned by the entry setup loop.
    /// The loop fully restores (when needed), places, and walks one member
    /// before advancing to the next. [`ENDGAME_ENTRY_COMPLETE_SLOT`] means
    /// the greeting/prompt has been published and no entry work remains.
    pub entry_party_slot: u8,
    /// `endgame.md §7`: display-owned state for the victory tableau's
    /// Orb hold, shared-moongate rise/hold/sink sequence, and actor exits.
    pub victory_tableau_phase: EndgameVictoryTableauPhase,
}

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
/// `formats/location-dat.md section 11`: endgame loads MISCMAPS cutscene-map
/// record 3 as the authored 11x11 terminal tableau scene.
pub const ENDGAME_TABLEAU_CUTSCENE_MAP_RECORD: usize = 3;
pub const ENDGAME_TABLEAU_WALKABLE_TILE: u8 = 0x44;

const ENDGAME_TABLEAU_PARTY_TARGETS: [(usize, usize); SAVE_PARTY_SIZE_MAX as usize] =
    [(5, 5), (4, 6), (6, 6), (3, 7), (5, 7), (7, 7)];
const ENDGAME_TABLEAU_LORD_BRITISH_POS: (usize, usize) = (5, 4);
const ENDGAME_TABLEAU_LORD_BRITISH_START: (usize, usize) = (5, 8);
const ENDGAME_TABLEAU_REFUSAL_SLOT0_TARGET: (usize, usize) = (8, 4);
const ENDGAME_TABLEAU_REFUSAL_SLOT2_TARGET: (usize, usize) = (8, 6);
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
            entry_party_slot: ENDGAME_ENTRY_COMPLETE_SLOT,
            victory_tableau_phase: EndgameVictoryTableauPhase::Inactive,
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
            entry_party_slot: ENDGAME_ENTRY_COMPLETE_SLOT,
            victory_tableau_phase: EndgameVictoryTableauPhase::Inactive,
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
            entry_party_slot: ENDGAME_ENTRY_COMPLETE_SLOT,
            victory_tableau_phase: EndgameVictoryTableauPhase::Inactive,
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

    pub fn first_prompt_text(&self, leader_name: &str) -> String {
        if let Some(messages) = &self.messages {
            let mut lines = Vec::new();
            if let Some(greeting) = messages.initial_greeting() {
                lines.push(format!("{leader_name}: {greeting}"));
            }
            if let Some(prompt) = messages.first_box_prompt() {
                lines.push(prompt.to_string());
            }
            if !lines.is_empty() {
                lines.push("(Y/N)".to_string());
                return lines.join("\n");
            }
        }
        "Endgame: Lord British asks whether thou hast brought his box. (Y/N)".to_string()
    }

    pub fn second_prompt_text(&self, first_answer: bool) -> String {
        if let Some(messages) = &self.messages {
            if let Some(prompt) = messages.second_box_prompt() {
                return format!(
                    "Thou answered {}.\n{prompt}\n(Y/N)",
                    yes_no_word(first_answer)
                );
            }
        }
        "Endgame: Lord British asks again for the sandalwood box. (Y/N)".to_string()
    }

    pub fn refusal_text(&self) -> String {
        self.messages
            .as_ref()
            .and_then(|messages| messages.refusal_branch())
            .map(str::to_string)
            .unwrap_or_else(|| {
                "Endgame: the sandalwood box handoff failed; the ending tableau is terminal."
                    .to_string()
            })
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
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "endgame victory rite beat {index} has no ENDMSG.DAT record; displaying the step's debug banner label instead is a forbidden fallback (endgame.md §7)"
                    )
                }),
            Step::NarrativeWindow(index) => self
                .final_narrative
                .as_ref()
                .and_then(|narrative| narrative.window_by_number(index.saturating_add(1)))
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

fn yes_no_word(answer: bool) -> &'static str {
    if answer { "yes" } else { "no" }
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
    if parts.is_empty() {
        "0 days".to_string()
    } else {
        parts.join(", ")
    }
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
        && object.type_byte == object.tile
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
        && object.tile == ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE
    {
        Some(EndgameTableauActorRole::LordBritish)
    } else {
        None
    }
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
        if self.advance_endgame_victory_tableau_frame() {
            self.message = self
                .endgame
                .as_ref()
                .map(|endgame| endgame.current_cinematic_text())
                .unwrap_or_default();
            return true;
        }
        let advanced = self
            .endgame
            .as_mut()
            .is_some_and(|endgame| endgame.advance_cinematic_frame_operation());
        if advanced {
            self.message = self
                .endgame
                .as_ref()
                .map(|endgame| endgame.current_cinematic_text())
                .unwrap_or_default();
        }
        advanced
    }

    /// `endgame.md §4`/`§7` entry presentation pump. Returns `true`
    /// while a frame is still owed. Party slots are processed in order; a
    /// Dead member first gets one exact announcement/restoration beat, then
    /// that member is placed and walked to the target one cell per frame
    /// before the next slot starts. The caller renders after each `true`.
    pub fn advance_endgame_entry_presentation(&mut self) -> bool {
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
                if self.step_endgame_tableau_slot_once_to_target(
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
                // The frontend has no PC-speaker backend. This tick retains
                // the published blocking restoration beat; the ordinary
                // status/tableau compositor performs the post-flourish redraw.
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

            let moved = self.step_endgame_tableau_slot_once_to_target(slot, placement.target);
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
                if self
                    .endgame
                    .as_ref()
                    .is_some_and(|endgame| endgame.entry_party_slot == ENDGAME_ENTRY_COMPLETE_SLOT)
                {
                    self.append_endgame_first_prompt();
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

    fn append_endgame_first_prompt(&mut self) {
        let prompt = self
            .endgame
            .as_ref()
            .expect("endgame entry prompt requires endgame state")
            .first_prompt_text(&self.party_leader_name());
        self.message.push_str(&prompt);
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
        self.animation.tick_static_tiles();
        true
    }

    fn clear_endgame_tableau_slot_type_tile(&mut self, slot: usize) {
        if let Some(object) = self.active_objects.get_mut(slot) {
            object.type_byte = 0;
            object.tile = 0;
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

    pub fn prepare_endgame_victory_tableau(&mut self) {
        self.step_endgame_tableau_slot_to_target(0, ENDGAME_TABLEAU_VICTORY_EXIT_TARGET);
        self.step_endgame_tableau_slot_to_target(0, ENDGAME_TABLEAU_PARTY_TARGETS[0]);
        if self.active_objects.len() < OOL_SLOTS {
            self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        }
        if let Some(slot) = self.active_objects.get_mut(ENDGAME_TABLEAU_BOX_SLOT) {
            write_endgame_tableau_placement(slot, endgame_tableau_box_placement());
        }
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

    pub fn advance_endgame_terminal_tableau_jitter(&mut self) -> bool {
        let party_slots = self.party.len().min(SAVE_PARTY_SIZE_MAX as usize);
        let eligible = ENDGAME_TABLEAU_JITTER_SLOTS
            .iter()
            .copied()
            .filter(|slot| *slot < party_slots)
            .filter(|slot| {
                self.active_objects
                    .get(*slot)
                    .copied()
                    .is_some_and(|object| {
                        endgame_tableau_role_for_slot(*slot, object)
                            == Some(EndgameTableauActorRole::PartyMember(*slot as u8))
                    })
            })
            .collect::<Vec<_>>();
        if eligible.is_empty() {
            self.animation.tick_static_tiles();
            return false;
        }
        let mut moved = false;
        for actor_slot in eligible {
            moved |= self.advance_endgame_tableau_jitter_slot(actor_slot);
        }
        moved
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
        let needs_final_narrative = self.endgame.as_ref().is_some_and(|state| {
            !state.is_terminal()
                && state.first_confirmation.is_some()
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
            self.message = "No endgame confirmation is pending.".to_string();
            return MoveOutcome::Blocked;
        };
        if current.is_terminal() {
            // Victory branch advances the cinematic page-flip on every
            // keystroke until the closer is finished, then keeps the terminal
            // final panel active.
            if matches!(current.outcome, Some(EndgameOutcome::Victory)) {
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
                        self.message = state.current_cinematic_text();
                    }
                    return MoveOutcome::Observed;
                }
            }
            if matches!(current.outcome, Some(EndgameOutcome::MissingBoxOrRefused)) {
                self.advance_endgame_terminal_tableau_jitter();
            }
            self.message = match current.outcome {
                Some(EndgameOutcome::Victory) => {
                    unreachable!("victory branch returns above")
                }
                Some(EndgameOutcome::MissingBoxOrRefused) => {
                    "Lord British waits with thee in the ending tableau.".to_string()
                }
                None => unreachable!("terminal endgame has an outcome"),
            };
            return MoveOutcome::Observed;
        }

        if current.first_confirmation.is_none() {
            self.endgame = Some(EndgameState::awaiting_final_confirmation_with_messages(
                answer,
                current.messages,
            ));
            self.message = self
                .endgame
                .as_ref()
                .expect("endgame state was just installed")
                .second_prompt_text(answer);
            return MoveOutcome::Used;
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
        match next.outcome {
            Some(EndgameOutcome::Victory) => self.prepare_endgame_victory_tableau(),
            Some(EndgameOutcome::MissingBoxOrRefused) => self.prepare_endgame_refusal_tableau(),
            None => {}
        }
        self.message = match next.outcome {
            // The victory branch opens on the first rite beat, or on
            // the throne tableau when `ENDMSG.DAT` carried no rite
            // records; neither composes certificate prose here.
            Some(EndgameOutcome::Victory) => next.current_cinematic_text(),
            Some(EndgameOutcome::MissingBoxOrRefused) => next.refusal_text(),
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
