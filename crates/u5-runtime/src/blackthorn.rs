//! Blackthorn cutscene helpers per `blackthorn.md` §7.

/// `blackthorn.md §7`: scene byte the rescue/refuge path hands control
/// to (`CASTLE:0` — Lord British's Castle, scene byte 17).
pub const BLACKTHORN_RESCUE_HANDOFF_SCENE: u8 = 17;

/// `blackthorn.md §7`: local position (X, Y) the rescue path hands the
/// party to inside the rescue handoff scene.
pub const BLACKTHORN_RESCUE_HANDOFF_X: u8 = 10;
pub const BLACKTHORN_RESCUE_HANDOFF_Y: u8 = 10;

/// `blackthorn.md §7`: the rescue path raises the shared moral-standing
/// selector to at least this floor after printing the verdict.
pub const BLACKTHORN_RESCUE_STANDING_FLOOR: u8 = 75;

/// `blackthorn.md §6` cutscene-VM actor families. The audience and
/// failure beats reference these slots by index when emitting
/// movement descriptors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornCutsceneActor {
    /// Slot 0 — Avatar / party-leader presentation actor.
    Avatar,
    /// Slot 1 — second party member; dragged-away victim of the
    /// failed-challenge punishment beat.
    SecondPartyMember,
    /// Slot 6 — Blackthorn.
    Blackthorn,
    /// Slot 7 — attendant or guard.
    Attendant,
    /// Slot 8 — throne or throne-marker tile.
    Throne,
}

impl BlackthornCutsceneActor {
    /// `blackthorn.md §6`: returns the cinematic actor slot index
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
/// party off to after the throne cleanup beat. Eighteen is the
/// gazetteer's `CASTLE:1` location associated with Lord
/// Blackthorn's Castle; the captive cell sits inside that scene.
pub const BLACKTHORN_CAPTIVE_CELL_SCENE: u8 = 18;

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
