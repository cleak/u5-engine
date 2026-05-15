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
