//! Resident A-Z command dispatcher table per `commands.md` §4-§5.
//!
//! This module names each command letter and exposes the verb-prefix
//! string the original prints before invoking the handler or refusal
//! path. Per-mode routing (overworld vs town vs dungeon vs combat) lives
//! in the play-state dispatcher; this is just the canonical
//! letter-to-name table.

use crate::input_case_fold;

/// `commands.md §4` resident A-Z command identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Pass,
    Attack,
    Board,
    Cast,
    Enter,
    Fire,
    Get,
    HoleUp,
    Ignite,
    Jimmy,
    Klimb,
    Look,
    Mix,
    NewOrder,
    Open,
    Push,
    Quit,
    Ready,
    Search,
    Talk,
    Use,
    View,
    Xit,
    Yell,
    ZStats,
    /// Letters `D` and `W` fall through to the stock "What?" refusal.
    UnassignedRefusal,
}

impl Command {
    /// `commands.md §5` resident verb prefix the dispatcher prints
    /// before the handler or refusal path runs.
    pub const fn verb_prefix(self) -> &'static str {
        match self {
            Command::Pass => "Pass",
            Command::Attack => "Attack",
            Command::Board => "Board",
            Command::Cast => "Cast",
            Command::Enter => "Enter",
            Command::Fire => "Fire",
            Command::Get => "Get",
            Command::HoleUp => "Hole up",
            Command::Ignite => "Ignite",
            Command::Jimmy => "Jimmy",
            Command::Klimb => "Klimb",
            Command::Look => "Look",
            Command::Mix => "Mix",
            Command::NewOrder => "New order",
            Command::Open => "Open",
            Command::Push => "Push",
            Command::Quit => "Quit",
            Command::Ready => "Ready",
            Command::Search => "Search",
            Command::Talk => "Talk",
            Command::Use => "Use",
            Command::View => "View",
            Command::Xit => "X-it",
            Command::Yell => "Yell",
            Command::ZStats => "Z-stats",
            Command::UnassignedRefusal => "What?",
        }
    }
}

/// `view.md §3` accepted wishing-well wish keywords. The well's
/// object-spawn branch accepts only the six vehicle/joke names
/// recognized by the original handler. Match is case-insensitive
/// at the caller; this catalog stores the canonical capitalisation.
pub const WISHING_WELL_WISH_KEYWORDS: [&str; 6] =
    ["Corvette", "Ferrari", "Lamborghini", "Lotus", "Porsche", "Horse"];

/// `view.md §3`: returns `true` when the typed wish matches one of
/// the six accepted wishing-well keywords (case-insensitive).
pub fn wishing_well_wish_accepted(typed: &str) -> bool {
    let upper = typed.trim().to_ascii_uppercase();
    WISHING_WELL_WISH_KEYWORDS
        .iter()
        .any(|word| word.to_ascii_uppercase() == upper)
}

/// `view.md §2` V-View command outcome. Dispatcher inputs a single
/// gem stock and the active scene's combat marker; the helper
/// reports whether the call should consume a gem and whether the
/// caller should enter the view overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewCommandOutcome {
    /// No gem owned — print the no-gem refusal and return.
    NoGemRefusal,
    /// Combat scene — print/acknowledge the View label and abort
    /// without spending a gem.
    CombatLabelOnly,
    /// Decrement one gem and enter the view overlay (LOOKOBJ for
    /// overworld/town, DNGLOOK for dungeon).
    EnterOverlay,
}

/// `view.md §2`: classify a V-View command call. The dispatcher's
/// gem-stock check happens before the overlay is invoked.
pub const fn view_command_outcome(gems: u8, in_combat: bool) -> ViewCommandOutcome {
    if in_combat {
        return ViewCommandOutcome::CombatLabelOnly;
    }
    if gems == 0 {
        return ViewCommandOutcome::NoGemRefusal;
    }
    ViewCommandOutcome::EnterOverlay
}

/// `commands.md §11` Y-Yell free-text input cap. When the party is
/// not in the ship-sail branch, the command opens a line-input
/// prompt that accepts up to thirty characters before routing the
/// typed word to the Shadowlord-name or Word-of-Power scanner.
pub const YELL_INPUT_MAX_LEN: usize = 30;

/// `commands.md §11` scene routing for the typed Y-Yell input. The
/// engine selects the scanner family from the active scene context;
/// other contexts produce no effect after the prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YellInputContext {
    /// Shadowlord arena scene family — the typed word is compared
    /// against the three Shadowlord names.
    ShadowlordName,
    /// Dungeon Word-of-Power context — the typed word is compared
    /// against the eight dungeon words in fixed order.
    WordOfPower,
    /// Any other non-ship context — the prompt completes without
    /// effect after empty or non-matching input.
    NoEffect,
}

/// `commands.md §8` P-Push pushable static-tile family. The
/// non-dynamic-object branch of P-Push accepts only the static
/// tile families documented here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushableTileFamily {
    /// `0x5B` — single non-rotating pushable class.
    NonRotating5B,
    /// `0x90..=0x93` — four-facing chair family; movement rewrites
    /// the facing bits.
    ChairFourFacing,
    /// `0xA5`, `0xA6`, `0xA8`, `0xA9` — non-rotating pushable
    /// classes.
    NonRotatingA5A6A8A9,
    /// `0xAD..=0xAF` — non-rotating pushable run.
    NonRotatingAdAf,
    /// `0xB4..=0xB7` — four-facing cannon family; movement rewrites
    /// the facing bits.
    CannonFourFacing,
}

impl PushableTileFamily {
    /// `commands.md §8`: returns `true` for families whose facing
    /// bits get rewritten by a successful push/pull.
    pub const fn rewrites_facing(self) -> bool {
        matches!(self, Self::ChairFourFacing | Self::CannonFourFacing)
    }
}

/// `commands.md §8`: classify a static tile byte into its pushable
/// family for the P-Push command. Returns `None` when the static
/// tile is not in the pushable set; the caller still accepts a
/// dynamic object at that coordinate as pushable through the other
/// branch.
pub const fn pushable_tile_family(tile: u8) -> Option<PushableTileFamily> {
    Some(match tile {
        0x5B => PushableTileFamily::NonRotating5B,
        0x90..=0x93 => PushableTileFamily::ChairFourFacing,
        0xA5 | 0xA6 | 0xA8 | 0xA9 => PushableTileFamily::NonRotatingA5A6A8A9,
        0xAD..=0xAF => PushableTileFamily::NonRotatingAdAf,
        0xB4..=0xB7 => PushableTileFamily::CannonFourFacing,
        _ => return None,
    })
}

/// `commands.md §6` New-Order swap-accept predicate. The handler
/// refuses the swap if either selected slot is slot zero (the
/// leader must remain first). Same-slot swaps are accepted; the
/// resulting whole-record exchange is a behavioural no-op but the
/// turn is still consumed.
pub const fn new_order_swap_accepted(slot_a: usize, slot_b: usize) -> bool {
    slot_a != 0 && slot_b != 0
}

/// `commands.md §4`: classify a raw key byte into a [`Command`]. Keys
/// are case-folded before dispatch (see `input.md §6`). Returns `None`
/// for any byte outside the `A..=Z` range and the literal `Space` pass
/// input.
pub fn command_for_letter(byte: u8) -> Option<Command> {
    let folded = input_case_fold(byte);
    Some(match folded {
        b' ' => Command::Pass,
        b'A' => Command::Attack,
        b'B' => Command::Board,
        b'C' => Command::Cast,
        b'D' | b'W' => Command::UnassignedRefusal,
        b'E' => Command::Enter,
        b'F' => Command::Fire,
        b'G' => Command::Get,
        b'H' => Command::HoleUp,
        b'I' => Command::Ignite,
        b'J' => Command::Jimmy,
        b'K' => Command::Klimb,
        b'L' => Command::Look,
        b'M' => Command::Mix,
        b'N' => Command::NewOrder,
        b'O' => Command::Open,
        b'P' => Command::Push,
        b'Q' => Command::Quit,
        b'R' => Command::Ready,
        b'S' => Command::Search,
        b'T' => Command::Talk,
        b'U' => Command::Use,
        b'V' => Command::View,
        b'X' => Command::Xit,
        b'Y' => Command::Yell,
        b'Z' => Command::ZStats,
        _ => return None,
    })
}
