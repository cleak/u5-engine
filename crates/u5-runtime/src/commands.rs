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
