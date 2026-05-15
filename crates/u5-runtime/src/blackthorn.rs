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

/// `blackthorn.md §4`: Blackthorn challenge prompt input limit.
pub const BLACKTHORN_CHALLENGE_INPUT_LIMIT: usize = 14;
/// `blackthorn.md §4`: number of fixed prompt ordinals the challenge
/// loop iterates (the first four virtue/mantra pairs).
pub const BLACKTHORN_CHALLENGE_PROMPT_COUNT: usize = 4;

/// `blackthorn.md §4`: case-insensitive substring match of the
/// player's typed answer against the expected mantra. The expected
/// word may appear anywhere in the typed buffer rather than being the
/// entire input.
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
