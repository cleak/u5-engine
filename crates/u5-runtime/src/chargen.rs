//! Character-creation save producer.

use std::io;
use std::path::Path;

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChargenStats {
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargenAvatar {
    pub name: [u8; SAVE_CHARACTER_NAME_LEN],
    pub male: bool,
    pub stats: ChargenStats,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingChargenQuestion {
    pub round: u8,
    pub question_index: usize,
    pub option_a: ShrineVirtue,
    pub option_b: ShrineVirtue,
    pub question_record: usize,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChargenSessionResult {
    pub name: [u8; SAVE_CHARACTER_NAME_LEN],
    pub entered_name: Vec<u8>,
    pub male: bool,
    pub tournament: ChargenTournamentOutcome,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChargenSessionPhase {
    #[default]
    AwaitingName,
    AwaitingGender,
    GypsyArrival,
    GypsyInvitation,
    AwaitingAnswer,
    Completed,
    Aborted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChargenSessionStep {
    PromptName,
    PromptGender,
    PresentIntro { record: usize, text: String },
    PresentQuestion(PendingChargenQuestion),
    Completed(ChargenSessionResult),
    Aborted,
    Ignored,
}

#[derive(Clone, Debug)]
pub struct ChargenSession {
    pub phase: ChargenSessionPhase,
    records: Vec<String>,
    rng_bytes: Vec<u8>,
    rng_cursor: usize,
    question_index: usize,
    round_index: usize,
    question_in_round: usize,
    selected_this_round: [bool; VIRTUE_COUNT],
    lost_forever: [bool; VIRTUE_COUNT],
    entered_name: Vec<u8>,
    normalized_name: [u8; SAVE_CHARACTER_NAME_LEN],
    male: Option<bool>,
    current_question: Option<PendingChargenQuestion>,
    questions: Vec<ChargenTournamentQuestion>,
    winners: Vec<ShrineVirtue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChargenError {
    BlankName,
    InvalidNameByte(u8),
    SameVirtuePair,
    SaveTooShort(usize),
}

impl std::fmt::Display for ChargenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlankName => write!(f, "character name must not be blank"),
            Self::InvalidNameByte(byte) => {
                write!(f, "character name contains invalid byte 0x{byte:02x}")
            }
            Self::SameVirtuePair => write!(f, "chargen question pair must use two virtues"),
            Self::SaveTooShort(len) => write!(
                f,
                "chargen seed must contain slot 0 record, got {len} bytes"
            ),
        }
    }
}

impl std::error::Error for ChargenError {}

/// `chargen.md §6` — number of tournament rounds.
pub const CHARGEN_ROUND_COUNT: usize = 3;
/// `chargen.md §6` — questions per round, indexed by round (0..3).
pub const CHARGEN_QUESTIONS_PER_ROUND: [usize; CHARGEN_ROUND_COUNT] = [4, 2, 1];
/// `chargen.md §6` — total questions in the questionnaire
/// tournament. Anchored to the sum of CHARGEN_QUESTIONS_PER_ROUND
/// so the total derives from the per-round breakdown.
pub const CHARGEN_QUESTION_COUNT: usize = CHARGEN_QUESTIONS_PER_ROUND[0]
    + CHARGEN_QUESTIONS_PER_ROUND[1]
    + CHARGEN_QUESTIONS_PER_ROUND[2];

impl ChargenSession {
    pub fn new(records: Vec<String>, rng_bytes: Vec<u8>) -> io::Result<Self> {
        if records.len() < crate::QUESTION_DAT_RECORDS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{} requires at least {} records, got {}",
                    crate::QUESTION_DAT_FILE,
                    crate::QUESTION_DAT_RECORDS,
                    records.len()
                ),
            ));
        }
        Ok(Self {
            phase: ChargenSessionPhase::AwaitingName,
            records,
            rng_bytes,
            rng_cursor: 0,
            question_index: 0,
            round_index: 0,
            question_in_round: 0,
            selected_this_round: [false; VIRTUE_COUNT],
            lost_forever: [false; VIRTUE_COUNT],
            entered_name: Vec::new(),
            normalized_name: [0; SAVE_CHARACTER_NAME_LEN],
            male: None,
            current_question: None,
            questions: Vec::with_capacity(CHARGEN_QUESTION_COUNT),
            winners: Vec::with_capacity(CHARGEN_QUESTION_COUNT),
        })
    }
}

impl ChargenSession {
    pub fn current_step(&self) -> ChargenSessionStep {
        match self.phase {
            ChargenSessionPhase::AwaitingName => ChargenSessionStep::PromptName,
            ChargenSessionPhase::AwaitingGender => ChargenSessionStep::PromptGender,
            ChargenSessionPhase::GypsyArrival => ChargenSessionStep::PresentIntro {
                record: 0,
                text: self.records[0].clone(),
            },
            ChargenSessionPhase::GypsyInvitation => ChargenSessionStep::PresentIntro {
                record: 1,
                text: self.records[1].clone(),
            },
            ChargenSessionPhase::AwaitingAnswer => self
                .current_question
                .clone()
                .map(ChargenSessionStep::PresentQuestion)
                .unwrap_or(ChargenSessionStep::Ignored),
            ChargenSessionPhase::Completed => self
                .result()
                .map(ChargenSessionStep::Completed)
                .unwrap_or(ChargenSessionStep::Ignored),
            ChargenSessionPhase::Aborted => ChargenSessionStep::Aborted,
        }
    }

    pub fn submit_name(&mut self, name: &str) -> ChargenSessionStep {
        if !matches!(self.phase, ChargenSessionPhase::AwaitingName) {
            return ChargenSessionStep::Ignored;
        }
        let bytes = name.trim_end_matches(['\r', '\n']).as_bytes();
        if bytes.is_empty() {
            self.phase = ChargenSessionPhase::Aborted;
            return ChargenSessionStep::Aborted;
        }
        match normalize_chargen_name(bytes) {
            Ok(normalized) => {
                self.entered_name = bytes
                    .iter()
                    .take(CHARGEN_NAME_INPUT_LIMIT)
                    .copied()
                    .collect();
                self.normalized_name = normalized;
                self.phase = ChargenSessionPhase::AwaitingGender;
                ChargenSessionStep::PromptGender
            }
            Err(_) => ChargenSessionStep::Ignored,
        }
    }

    pub fn submit_gender_key(&mut self, key: u8) -> ChargenSessionStep {
        if !matches!(self.phase, ChargenSessionPhase::AwaitingGender) {
            return ChargenSessionStep::Ignored;
        }
        let Some(male) = chargen_gender_key(key) else {
            return ChargenSessionStep::Ignored;
        };
        self.male = Some(male);
        self.phase = ChargenSessionPhase::GypsyArrival;
        self.current_step()
    }

    pub fn advance_intro(&mut self) -> ChargenSessionStep {
        match self.phase {
            ChargenSessionPhase::GypsyArrival => {
                self.phase = ChargenSessionPhase::GypsyInvitation;
                self.current_step()
            }
            ChargenSessionPhase::GypsyInvitation => self.prepare_next_question(),
            _ => ChargenSessionStep::Ignored,
        }
    }

    pub fn submit_answer_key(&mut self, key: u8) -> ChargenSessionStep {
        if !matches!(self.phase, ChargenSessionPhase::AwaitingAnswer) {
            return ChargenSessionStep::Ignored;
        }
        let Some(chose_a) = chargen_answer_key(key) else {
            return ChargenSessionStep::Ignored;
        };
        let Some(current) = self.current_question.take() else {
            return ChargenSessionStep::Ignored;
        };
        let winner = if chose_a {
            current.option_a
        } else {
            current.option_b
        };
        let loser = if chose_a {
            current.option_b
        } else {
            current.option_a
        };
        self.lost_forever[loser.index()] = true;
        self.winners.push(winner);
        self.questions.push(ChargenTournamentQuestion {
            round: current.round,
            option_a: current.option_a,
            option_b: current.option_b,
            question_record: current.question_record,
            chose_a,
            winner,
            loser,
        });
        self.question_index += 1;
        self.question_in_round += 1;

        if self.question_index == CHARGEN_QUESTION_COUNT {
            self.phase = ChargenSessionPhase::Completed;
            return self.current_step();
        }
        if self.question_in_round >= CHARGEN_QUESTIONS_PER_ROUND[self.round_index] {
            self.round_index += 1;
            self.question_in_round = 0;
            self.selected_this_round = [false; VIRTUE_COUNT];
        }
        self.prepare_next_question()
    }

    pub fn result(&self) -> Option<ChargenSessionResult> {
        if !matches!(self.phase, ChargenSessionPhase::Completed) {
            return None;
        }
        let male = self.male?;
        let final_winner = self.winners.last().copied()?;
        Some(ChargenSessionResult {
            name: self.normalized_name,
            entered_name: self.entered_name.clone(),
            male,
            tournament: ChargenTournamentOutcome {
                questions: self.questions.clone(),
                stats: chargen_stats_from_winners(&self.winners),
                final_winner,
            },
        })
    }

    fn prepare_next_question(&mut self) -> ChargenSessionStep {
        let first_idx = match draw_virtue(
            &self.rng_bytes,
            &mut self.rng_cursor,
            &self.selected_this_round,
            &self.lost_forever,
        ) {
            Some(idx) => idx,
            None => {
                self.phase = ChargenSessionPhase::Aborted;
                return ChargenSessionStep::Aborted;
            }
        };
        self.selected_this_round[first_idx] = true;
        let second_idx = match draw_virtue(
            &self.rng_bytes,
            &mut self.rng_cursor,
            &self.selected_this_round,
            &self.lost_forever,
        ) {
            Some(idx) => idx,
            None => {
                self.phase = ChargenSessionPhase::Aborted;
                return ChargenSessionStep::Aborted;
            }
        };
        self.selected_this_round[second_idx] = true;

        let (a_idx, b_idx) = if first_idx < second_idx {
            (first_idx, second_idx)
        } else {
            (second_idx, first_idx)
        };
        let option_a = ShrineVirtue::from_index(a_idx).expect("virtue index in range");
        let option_b = ShrineVirtue::from_index(b_idx).expect("virtue index in range");
        let question_record =
            chargen_question_record_for_pair(option_a, option_b).expect("distinct virtues");
        let text = self.records[question_record].clone();
        let question = PendingChargenQuestion {
            round: (self.round_index + 1) as u8,
            question_index: self.question_index,
            option_a,
            option_b,
            question_record,
            text,
        };
        self.current_question = Some(question.clone());
        self.phase = ChargenSessionPhase::AwaitingAnswer;
        ChargenSessionStep::PresentQuestion(question)
    }
}

pub const fn chargen_gender_key(key: u8) -> Option<bool> {
    match key {
        b'M' | b'm' => Some(true),
        b'F' | b'f' => Some(false),
        _ => None,
    }
}

pub const fn chargen_answer_key(key: u8) -> Option<bool> {
    match key {
        b'A' | b'a' => Some(true),
        b'B' | b'b' => Some(false),
        _ => None,
    }
}

/// `chargen.md §4` Avatar name-prompt input limit. The free-text
/// prompt accepts up to eight characters; shorter names are
/// null-padded into the eight-byte name slice. The save record's
/// name field is nine bytes wide; chargen leaves the ninth byte as
/// the seed padding. Anchored to [`SAVE_CHARACTER_NAME_LEN`] - 1 so
/// the input limit derives from the save-record field width.
pub const CHARGEN_NAME_INPUT_MAX_LEN: usize = SAVE_CHARACTER_NAME_LEN - 1;
/// `chargen.md §4` Avatar name save-record field width. Anchored
/// to [`SAVE_CHARACTER_NAME_LEN`] so the chargen-side field width
/// and the save-record name field share one source of truth.
pub const CHARGEN_NAME_FIELD_LEN: usize = SAVE_CHARACTER_NAME_LEN;

pub fn chargen_question_record_for_pair(
    first: ShrineVirtue,
    second: ShrineVirtue,
) -> Result<usize, ChargenError> {
    let a = first.index();
    let b = second.index();
    if a == b {
        return Err(ChargenError::SameVirtuePair);
    }
    let (low, high) = if a < b { (a, b) } else { (b, a) };
    Ok(2 + (0..low).map(|row| 7 - row).sum::<usize>() + (high - low - 1))
}

pub fn chargen_virtue_stat_delta(virtue: ShrineVirtue) -> ChargenStats {
    match virtue {
        ShrineVirtue::Honesty => ChargenStats {
            strength: 0,
            dexterity: 0,
            intelligence: 2,
        },
        ShrineVirtue::Compassion => ChargenStats {
            strength: 0,
            dexterity: 2,
            intelligence: 0,
        },
        ShrineVirtue::Valor => ChargenStats {
            strength: 2,
            dexterity: 0,
            intelligence: 0,
        },
        ShrineVirtue::Justice => ChargenStats {
            strength: 0,
            dexterity: 1,
            intelligence: 1,
        },
        ShrineVirtue::Sacrifice => ChargenStats {
            strength: 1,
            dexterity: 1,
            intelligence: 0,
        },
        ShrineVirtue::Honor => ChargenStats {
            strength: 1,
            dexterity: 0,
            intelligence: 1,
        },
        ShrineVirtue::Spirituality => ChargenStats {
            strength: 1,
            dexterity: 1,
            intelligence: 1,
        },
        ShrineVirtue::Humility => ChargenStats {
            strength: 0,
            dexterity: 0,
            intelligence: 0,
        },
    }
}

/// `chargen.md §7`: STR floor applied after summing the questionnaire
/// totals. Because the maximum per-question STR contribution is two and
/// the questionnaire has seven questions, the floor always fires for the
/// questionnaire path and every newly-created avatar emerges with exactly
/// twenty Strength.
pub const CHARGEN_STR_FLOOR: u8 = 20;

/// `chargen.md §8` factory-seed Avatar header values the seed image
/// dictates for a freshly questionnaire-created character. Chargen
/// only customises the name, gender, STR, DEX, INT, and MP fields;
/// these record bytes are inherited from `INIT.GAM` unchanged.
///
/// The freshly seeded current-HP value matches the global
/// [`crate::DEFAULT_PARTY_HP`] starting-HP anchor; the seeded
/// max-HP value equals the current-HP value because chargen sets
/// both to the same starting value. Anchor both through to those
/// chains so a chargen-HP rebalance flows through one source of
/// truth.
pub const CHARGEN_AVATAR_SEED_CURRENT_HP: u16 = crate::DEFAULT_PARTY_HP;
pub const CHARGEN_AVATAR_SEED_MAX_HP: u16 = CHARGEN_AVATAR_SEED_CURRENT_HP;
pub const CHARGEN_AVATAR_SEED_EXPERIENCE: u16 = 150;
pub const CHARGEN_AVATAR_SEED_LEVEL: u8 = 2;
pub const CHARGEN_AVATAR_SEED_CLASS_BYTE: u8 = b'A';
pub const CHARGEN_AVATAR_SEED_STATUS_BYTE: u8 = b'G';

/// `chargen.md §8`: starting party size for a fresh-from-questionnaire
/// save (Avatar plus Iolo and Shamino in scene 13, Iolo's Hut).
pub const CHARGEN_STARTING_PARTY_SIZE: u8 = 3;

/// `chargen.md §8` seeded starting-inventory counters for a fresh
/// questionnaire-created save. Chargen does not write these; they
/// come from `INIT.GAM` unchanged. Listing them as named constants
/// lets fixture builders and verification checks compare a freshly
/// generated save against the published seed values.
///
/// The seeded counters are the same values as the global
/// `DEFAULT_*` initial-inventory anchors in `constants.rs`, so
/// they are aliased through to those constants. A future
/// rebalance of either the chargen seed or the global default
/// flows through both names.
pub const CHARGEN_SEED_FOOD: u16 = crate::DEFAULT_FOOD_STOCK;
pub const CHARGEN_SEED_GOLD: u16 = crate::DEFAULT_GOLD_STOCK;
pub const CHARGEN_SEED_KEYS: u8 = crate::DEFAULT_KEY_STOCK;
pub const CHARGEN_SEED_GEMS: u8 = crate::DEFAULT_GEM_STOCK;
pub const CHARGEN_SEED_TORCHES: u8 = crate::DEFAULT_TORCH_STOCK;
pub const CHARGEN_SEED_MAGIC_POWDER: u8 = 0;

/// `chargen.md §8` seeded reagent counters for a fresh
/// questionnaire-created save. Mandrake, spider silk, and sulfurous
/// ash start at zero — the player must source them before mixing the
/// spells those reagents gate.
///
/// Each CHARGEN_SEED_REAGENT_* equals the matching
/// `crate::DEFAULT_REAGENTS[crate::REAGENT_*]` slot. Anchor each
/// seed through the storage-indexed array so a rebalance of either
/// the chargen seed or the global default reagent stock flows
/// through one source of truth.
pub const CHARGEN_SEED_REAGENT_BLACK_PEARL: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_BLACK_PEARL];
pub const CHARGEN_SEED_REAGENT_BLOOD_MOSS: u8 = crate::DEFAULT_REAGENTS[crate::REAGENT_BLOOD_MOSS];
pub const CHARGEN_SEED_REAGENT_GARLIC: u8 = crate::DEFAULT_REAGENTS[crate::REAGENT_GARLIC];
pub const CHARGEN_SEED_REAGENT_GINSENG: u8 = crate::DEFAULT_REAGENTS[crate::REAGENT_GINSENG];
pub const CHARGEN_SEED_REAGENT_MANDRAKE: u8 = crate::DEFAULT_REAGENTS[crate::REAGENT_MANDRAKE];
pub const CHARGEN_SEED_REAGENT_NIGHTSHADE: u8 = crate::DEFAULT_REAGENTS[crate::REAGENT_NIGHTSHADE];
pub const CHARGEN_SEED_REAGENT_SPIDER_SILK: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_SPIDER_SILK];
pub const CHARGEN_SEED_REAGENT_SULFUROUS_ASH: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_SULFUR_ASH];

/// `chargen.md §4` maximum visible characters the name prompt accepts.
/// The shipped prompt prints "By what name shalt thou be known?" and
/// opens a free-text input prompt with this many characters of room.
/// The save record's name field is one byte longer
/// ([`SAVE_CHARACTER_NAME_LEN`] = 9) so the trailing byte stays as
/// seed padding when the player enters the maximum length.
pub const CHARGEN_NAME_INPUT_LIMIT: usize = SAVE_CHARACTER_NAME_LEN - 1;

/// `chargen.md §8` starting map tuple for a fresh-from-questionnaire
/// save. The chargen writer seeds scene 13 (Iolo's Hut) on floor /
/// Z 0 at local cell (15, 15) with a zero saved-scene scratch byte.
/// These bytes come from `INIT.GAM`; chargen does not customise them.
/// Anchored to [`crate::SCENE_IOLOS_HUT`] so the chargen starting
/// scene and the gazetteer-named Iolo's Hut scene share one
/// source of truth.
pub const CHARGEN_STARTING_SCENE: u8 = crate::SCENE_IOLOS_HUT;
/// `chargen.md §8` / `town-mode.md §3`: the chargen exit cell
/// uses the engine-wide town-entry default column (X = 15).
/// Anchored to [`crate::LOCATION_DEFAULT_ENTRY_X`] so the
/// town-entry default and the chargen exit column stay one value.
pub const CHARGEN_STARTING_X: u8 = crate::LOCATION_DEFAULT_ENTRY_X as u8;
pub const CHARGEN_STARTING_Y: u8 = 15;
pub const CHARGEN_STARTING_Z: u8 = 0;
pub const CHARGEN_STARTING_SAVED_SCENE_SCRATCH: u8 = 0;

/// `formats/saved-gam.md §3.1` per-character defense byte the
/// factory seed ships for every roster slot. No traced writer
/// currently recomputes this byte from readied equipment, so the
/// shipped value is also the runtime value the combat damage
/// path's random defense subtraction reads.
pub const CHARGEN_SEED_DEFENSE_BYTE: u8 = 7;

/// `chargen.md §8` starting calendar values for a fresh-from-
/// questionnaire save. The save clock begins at year 139, month 4,
/// day 5, 08:35 of the in-world calendar. These bytes come from
/// `INIT.GAM`; chargen does not customise them.
pub const CHARGEN_STARTING_YEAR: u16 = 139;
pub const CHARGEN_STARTING_MONTH: u8 = 4;
pub const CHARGEN_STARTING_DAY: u8 = 5;
pub const CHARGEN_STARTING_HOUR: u8 = 8;
pub const CHARGEN_STARTING_MINUTE: u8 = 35;

/// Per-question outcome from the chargen tournament loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChargenTournamentQuestion {
    /// 1-based round index (1, 2, or 3).
    pub round: u8,
    /// Smaller-numbered virtue (the "A" slot).
    pub option_a: ShrineVirtue,
    /// Larger-numbered virtue (the "B" slot).
    pub option_b: ShrineVirtue,
    /// `QUESTION.DAT` record index used for this question.
    pub question_record: usize,
    /// `true` when the player chose A; `false` for B.
    pub chose_a: bool,
    /// Winner virtue after applying the player's choice.
    pub winner: ShrineVirtue,
    /// Loser virtue; flagged lost-forever after this question.
    pub loser: ShrineVirtue,
}

/// Reasons the tournament could not finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargenTournamentError {
    /// Out of random bytes while drawing the next virtue.
    RngExhausted { question_index: usize },
    /// Out of A/B answers (callers must supply seven).
    AnswersExhausted { question_index: usize },
    /// Two random draws ended up on the same virtue index — should not
    /// be reachable per spec, but we surface it defensively.
    SelfPair { question_index: usize },
}

/// Result of running the full questionnaire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChargenTournamentOutcome {
    /// Seven per-question outcomes, in chronological order.
    pub questions: Vec<ChargenTournamentQuestion>,
    /// Stats produced from the winners with the STR floor applied.
    pub stats: ChargenStats,
    /// Final winner of round 3.
    pub final_winner: ShrineVirtue,
}

/// Drive the chargen questionnaire per `chargen.md §6`. The rejection-
/// sampled random picker draws virtue indices from `rng_bytes`; the
/// `answers` slice supplies A (`true`) or B (`false`) for each of the
/// seven questions in order. The picker re-rolls on bytes that pick a
/// virtue already flagged selected-this-round or lost-forever.
pub fn run_chargen_tournament(
    rng_bytes: &[u8],
    answers: &[bool],
) -> Result<ChargenTournamentOutcome, ChargenTournamentError> {
    let mut rng_cursor = 0usize;
    let mut question_index = 0usize;
    let mut lost_forever = [false; 8];
    let mut questions: Vec<ChargenTournamentQuestion> = Vec::with_capacity(CHARGEN_QUESTION_COUNT);
    let mut winners: Vec<ShrineVirtue> = Vec::with_capacity(CHARGEN_QUESTION_COUNT);

    for (round_zero, &questions_in_round) in CHARGEN_QUESTIONS_PER_ROUND.iter().enumerate() {
        let mut selected_this_round = [false; 8];
        for _ in 0..questions_in_round {
            let answer = *answers
                .get(question_index)
                .ok_or(ChargenTournamentError::AnswersExhausted { question_index })?;
            let first_idx = draw_virtue(
                rng_bytes,
                &mut rng_cursor,
                &selected_this_round,
                &lost_forever,
            )
            .ok_or(ChargenTournamentError::RngExhausted { question_index })?;
            selected_this_round[first_idx] = true;
            let second_idx = draw_virtue(
                rng_bytes,
                &mut rng_cursor,
                &selected_this_round,
                &lost_forever,
            )
            .ok_or(ChargenTournamentError::RngExhausted { question_index })?;
            selected_this_round[second_idx] = true;

            if first_idx == second_idx {
                return Err(ChargenTournamentError::SelfPair { question_index });
            }
            let (a_idx, b_idx) = if first_idx < second_idx {
                (first_idx, second_idx)
            } else {
                (second_idx, first_idx)
            };
            let option_a = ShrineVirtue::from_index(a_idx).expect("virtue index in range");
            let option_b = ShrineVirtue::from_index(b_idx).expect("virtue index in range");
            let question_record = chargen_question_record_for_pair(option_a, option_b)
                .map_err(|_| ChargenTournamentError::SelfPair { question_index })?;
            let winner = if answer { option_a } else { option_b };
            let loser = if answer { option_b } else { option_a };
            lost_forever[loser.index()] = true;
            winners.push(winner);
            questions.push(ChargenTournamentQuestion {
                round: (round_zero + 1) as u8,
                option_a,
                option_b,
                question_record,
                chose_a: answer,
                winner,
                loser,
            });
            question_index += 1;
        }
    }

    let final_winner = winners
        .last()
        .copied()
        .expect("tournament always produces a final winner");
    let stats = chargen_stats_from_winners(&winners);
    Ok(ChargenTournamentOutcome {
        questions,
        stats,
        final_winner,
    })
}

fn draw_virtue(
    rng_bytes: &[u8],
    cursor: &mut usize,
    selected_this_round: &[bool; 8],
    lost_forever: &[bool; 8],
) -> Option<usize> {
    loop {
        let byte = *rng_bytes.get(*cursor)?;
        *cursor += 1;
        let idx = (byte & 0x07) as usize;
        if !selected_this_round[idx] && !lost_forever[idx] {
            return Some(idx);
        }
    }
}

pub fn chargen_stats_from_winners(winners: &[ShrineVirtue]) -> ChargenStats {
    let mut strength = 0u8;
    let mut dexterity = 0u8;
    let mut intelligence = 0u8;
    for winner in winners {
        let delta = chargen_virtue_stat_delta(*winner);
        strength = strength.saturating_add(delta.strength);
        dexterity = dexterity.saturating_add(delta.dexterity);
        intelligence = intelligence.saturating_add(delta.intelligence);
    }
    ChargenStats {
        strength: strength.max(CHARGEN_STR_FLOOR),
        dexterity,
        intelligence,
    }
}

pub fn apply_chargen_to_save(
    save: &mut [u8],
    name_bytes: &[u8],
    male: bool,
    stats: ChargenStats,
) -> Result<ChargenAvatar, ChargenError> {
    if save.len() < SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN {
        return Err(ChargenError::SaveTooShort(save.len()));
    }
    let name = normalize_chargen_name(name_bytes)?;
    let record = SAVE_ROSTER_OFFSET;
    save[record..record + SAVE_CHARACTER_NAME_LEN - 1]
        .copy_from_slice(&name[..SAVE_CHARACTER_NAME_LEN - 1]);
    let mut saved_name = name;
    saved_name[SAVE_CHARACTER_NAME_LEN - 1] = save[record + SAVE_CHARACTER_NAME_LEN - 1];
    save[record + SAVE_CHARACTER_GENDER_OFFSET] = if male {
        SAVE_GENDER_MALE_BYTE
    } else {
        SAVE_GENDER_FEMALE_BYTE
    };
    save[record + SAVE_CHARACTER_STR_OFFSET] = stats.strength;
    save[record + SAVE_CHARACTER_DEX_OFFSET] = stats.dexterity;
    save[record + SAVE_CHARACTER_INT_OFFSET] = stats.intelligence;
    save[record + SAVE_CHARACTER_MANA_OFFSET] = stats.intelligence;

    Ok(ChargenAvatar {
        name: saved_name,
        male,
        stats,
    })
}

pub fn commit_chargen_save(
    game_dir: &Path,
    name_bytes: &[u8],
    male: bool,
    stats: ChargenStats,
) -> io::Result<ChargenAvatar> {
    let mut save = read_save_image_file(&game_dir.join(INIT_GAM_FILENAME), INIT_GAM_FILENAME)?;
    let init_ool = read_init_ool_plane(game_dir)?;
    let avatar = apply_chargen_to_save(&mut save, name_bytes, male, stats)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

    let mut saved_ool = vec![0; SAVED_OOL_LEN];
    saved_ool[OOL_PLANE_LEN..].copy_from_slice(&init_ool);
    write_disk_file(&game_dir.join(SAVED_OOL_FILENAME), saved_ool)?;
    write_disk_file(&game_dir.join(SAVED_GAM_FILENAME), save)?;

    Ok(avatar)
}

fn normalize_chargen_name(
    name_bytes: &[u8],
) -> Result<[u8; SAVE_CHARACTER_NAME_LEN], ChargenError> {
    let mut name = [0; SAVE_CHARACTER_NAME_LEN];
    let mut has_non_space = false;
    for (index, &byte) in name_bytes.iter().take(CHARGEN_NAME_INPUT_LIMIT).enumerate() {
        if !(0x20..=0x7e).contains(&byte) {
            return Err(ChargenError::InvalidNameByte(byte));
        }
        if byte != b' ' {
            has_non_space = true;
        }
        name[index] = byte;
    }
    if !has_non_space {
        return Err(ChargenError::BlankName);
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn always_a() -> [bool; 7] {
        [true; 7]
    }

    /// 64-byte deterministic stream (`0..64`) that gives the
    /// rejection-sampled picker plenty of headroom even when later
    /// rounds whittle the eligible pool down to two virtues.
    fn rng_pool() -> [u8; 64] {
        let mut bytes = [0u8; 64];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = i as u8;
        }
        bytes
    }

    fn question_records() -> Vec<String> {
        let mut records = vec![
            "Arrival narrative.".to_string(),
            "Invitation narrative.".to_string(),
        ];
        for index in 2..30 {
            records.push(format!("Question record {index}."));
        }
        records
    }

    #[test]
    fn chargen_session_empty_name_aborts_before_gender_or_questions() {
        let mut session = ChargenSession::new(question_records(), rng_pool().to_vec()).unwrap();
        assert_eq!(session.current_step(), ChargenSessionStep::PromptName);
        assert_eq!(session.submit_name("\n"), ChargenSessionStep::Aborted);
        assert_eq!(session.phase, ChargenSessionPhase::Aborted);
        assert!(session.result().is_none());
    }

    #[test]
    fn chargen_session_ignores_invalid_gender_until_m_or_f() {
        let mut session = ChargenSession::new(question_records(), rng_pool().to_vec()).unwrap();
        assert_eq!(
            session.submit_name("Avatar"),
            ChargenSessionStep::PromptGender
        );
        assert_eq!(session.submit_gender_key(b'X'), ChargenSessionStep::Ignored);
        assert!(matches!(
            session.submit_gender_key(b'f'),
            ChargenSessionStep::PresentIntro { record: 0, .. }
        ));
    }

    #[test]
    fn chargen_session_walks_intro_questions_and_completed_result() {
        let mut session = ChargenSession::new(question_records(), rng_pool().to_vec()).unwrap();
        session.submit_name("Avatar");
        assert!(matches!(
            session.submit_gender_key(b'M'),
            ChargenSessionStep::PresentIntro { record: 0, .. }
        ));
        assert!(matches!(
            session.advance_intro(),
            ChargenSessionStep::PresentIntro { record: 1, .. }
        ));
        let mut step = session.advance_intro();
        let mut answered = 0usize;
        while let ChargenSessionStep::PresentQuestion(question) = step {
            assert_eq!(
                question.text,
                format!("Question record {}.", question.question_record)
            );
            step = session.submit_answer_key(b'A');
            answered += 1;
        }
        assert_eq!(answered, CHARGEN_QUESTION_COUNT);
        let ChargenSessionStep::Completed(result) = step else {
            panic!("expected completed result");
        };
        assert_eq!(result.entered_name, b"Avatar");
        assert!(result.male);
        assert_eq!(result.tournament.questions.len(), CHARGEN_QUESTION_COUNT);
        assert_eq!(result.tournament.stats.strength, CHARGEN_STR_FLOOR);
        assert_eq!(session.phase, ChargenSessionPhase::Completed);
    }

    #[test]
    fn tournament_completes_seven_questions_with_three_round_layout() {
        let rng = rng_pool();
        let outcome = run_chargen_tournament(&rng, &always_a()).unwrap();
        assert_eq!(outcome.questions.len(), CHARGEN_QUESTION_COUNT);
        let rounds: Vec<u8> = outcome.questions.iter().map(|q| q.round).collect();
        assert_eq!(rounds, vec![1, 1, 1, 1, 2, 2, 3]);
    }

    #[test]
    fn tournament_marks_loser_lost_forever_so_no_loser_re_appears() {
        let rng = rng_pool();
        let outcome = run_chargen_tournament(&rng, &always_a()).unwrap();
        let mut seen_after_loss = std::collections::HashSet::new();
        for question in &outcome.questions {
            assert!(
                !seen_after_loss.contains(&question.option_a),
                "loser {:?} should not reappear",
                question.option_a
            );
            assert!(!seen_after_loss.contains(&question.option_b));
            seen_after_loss.insert(question.loser);
        }
    }

    #[test]
    fn tournament_sorts_pair_so_option_a_has_smaller_index() {
        let mut rng = rng_pool();
        rng[0..8].copy_from_slice(&[7, 0, 6, 1, 5, 2, 4, 3]);
        let outcome = run_chargen_tournament(&rng, &always_a()).unwrap();
        for question in &outcome.questions {
            assert!(question.option_a.index() < question.option_b.index());
        }
    }

    #[test]
    fn tournament_chose_a_picks_smaller_indexed_virtue_as_winner() {
        let rng = rng_pool();
        let outcome = run_chargen_tournament(&rng, &always_a()).unwrap();
        for q in &outcome.questions {
            assert_eq!(q.winner, q.option_a);
            assert_eq!(q.loser, q.option_b);
        }
    }

    #[test]
    fn tournament_chose_b_picks_larger_indexed_virtue_as_winner() {
        let rng = rng_pool();
        let outcome = run_chargen_tournament(&rng, &[false; 7]).unwrap();
        for q in &outcome.questions {
            assert_eq!(q.winner, q.option_b);
            assert_eq!(q.loser, q.option_a);
        }
    }

    #[test]
    fn tournament_applies_str_floor_to_emitted_stats() {
        let rng = rng_pool();
        let outcome = run_chargen_tournament(&rng, &always_a()).unwrap();
        assert_eq!(outcome.stats.strength, CHARGEN_STR_FLOOR);
    }

    #[test]
    fn tournament_rejection_samples_past_selected_and_lost_virtues() {
        // First four rng bytes pick 0, 0, 1, 2 - the picker should skip
        // the duplicate 0 and the rejected 0 in later rounds.
        let mut rng = rng_pool();
        rng[0..4].copy_from_slice(&[0, 0, 1, 2]);
        let outcome = run_chargen_tournament(&rng, &always_a()).unwrap();
        // First question used draws 0 (kept) and then needed another byte
        // before picking the second virtue.
        let first = &outcome.questions[0];
        assert!(first.option_a.index() == 0 || first.option_b.index() == 0);
    }

    #[test]
    fn tournament_returns_rng_exhausted_when_byte_stream_runs_out() {
        let rng = [0, 1];
        let err = run_chargen_tournament(&rng, &always_a()).unwrap_err();
        assert!(matches!(err, ChargenTournamentError::RngExhausted { .. }));
    }

    #[test]
    fn tournament_returns_answers_exhausted_when_fewer_than_seven_supplied() {
        let rng = rng_pool();
        let err = run_chargen_tournament(&rng, &[true; 3]).unwrap_err();
        assert!(matches!(
            err,
            ChargenTournamentError::AnswersExhausted { .. }
        ));
    }

    #[test]
    fn tournament_records_question_dat_record_indices_in_range() {
        let rng = rng_pool();
        let outcome = run_chargen_tournament(&rng, &always_a()).unwrap();
        for q in &outcome.questions {
            // QUESTION.DAT has 30 records (0-29); records 2..=29 are
            // the 28 virtue-pair dilemmas.
            assert!((2..=29).contains(&q.question_record));
        }
    }

    #[test]
    fn final_winner_is_last_questions_winner() {
        let rng = rng_pool();
        let outcome = run_chargen_tournament(&rng, &always_a()).unwrap();
        assert_eq!(
            outcome.final_winner,
            outcome.questions.last().unwrap().winner
        );
    }

    #[test]
    fn tournament_keeps_winners_eligible_for_next_round() {
        // Round 1 winners (with always_a) are the smaller-indexed
        // virtues; round 2 must draw from those four winners only.
        let rng = rng_pool();
        let outcome = run_chargen_tournament(&rng, &always_a()).unwrap();
        let round1_winners: std::collections::HashSet<ShrineVirtue> = outcome
            .questions
            .iter()
            .filter(|q| q.round == 1)
            .map(|q| q.winner)
            .collect();
        for q in outcome.questions.iter().filter(|q| q.round == 2) {
            assert!(round1_winners.contains(&q.option_a));
            assert!(round1_winners.contains(&q.option_b));
        }
    }
}

fn read_init_ool_plane(game_dir: &Path) -> io::Result<Vec<u8>> {
    let bytes = read_disk_file(&game_dir.join(INIT_OOL_FILENAME))?;
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{INIT_OOL_FILENAME} must be {OOL_PLANE_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}
