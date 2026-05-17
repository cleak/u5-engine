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

/// `view.md §4` LOOKOBJ local-view overlay side length. The
/// overworld/town V-View paints a temporary square overlay
/// `LOCAL_VIEW_OVERLAY_SIDE` cells on each side around the party
/// — the same 32-cell side as the active map window. Anchored to
/// [`crate::TOWN_GRID_SIDE`] so the overlay and the active map
/// window share one source of truth.
pub const LOCAL_VIEW_OVERLAY_SIDE: usize = crate::TOWN_GRID_SIDE;
/// `view.md §4` LOOKOBJ local-view per-cell pixel scale. Each cell
/// in the overlay renders at a four-pixel square inside the
/// message-panel region.
pub const LOCAL_VIEW_CELL_PIXEL_SCALE: usize = 4;

/// `view.md §4` LOOKOBJ local-view 32x32 overlay class. Each
/// sampled cell is reduced to a view class and drawn by a
/// per-class renderer. The classes here are the spec's compact
/// dispatch ids; their exact pixel contracts live with the
/// renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalViewClass {
    /// `0` — empty/pass-through.
    Empty,
    /// `1` — sparse corner/checker pattern.
    SparseCheckers,
    /// `2` — solid 4x4 filled cell.
    SolidFill,
    /// `3` — filled cell-frame style.
    FilledFrame,
    /// `4` — two full-width horizontal rails.
    HorizontalRails,
    /// `5` — two short centered horizontal bars.
    CentredBars,
    /// `6` — hollow four-edge rectangle.
    HollowRectangle,
    /// `7` — diagonal/edge style group used for mountains, shoreline,
    /// undead/wall-flavour tiles.
    DiagonalStyle,
    /// `8` — diagonal two-quadrant step pattern.
    DiagonalStep,
    /// `9` — hybrid vegetation pattern.
    VegetationHybrid,
    /// `0xA` — four-corner room/feature ring.
    FourCornerRing,
    /// `0xB` — two diagonal blits (peer/gem-view bank-aware).
    DiagonalBlits,
    /// `0xC` — table-mapped no-op default for tile id 0x01.
    NoopDefault,
    /// `0xD` — creature-on-terrain composite.
    CreatureComposite,
    /// `0xE` — vertical two-line wall/door presentation.
    VerticalWallDoor,
    /// `0xF` — peer-spell/gem-view variant on the alternate bank.
    PeerVariant,
    /// `0x10` — fence/wall renderer with edge-bit selection.
    FenceWall,
}

/// `view.md §4`: classify a tile id into its LOOKOBJ local-view
/// class. The mapping is exhaustive across all 256 tile ids; tiles
/// not explicitly listed in the spec table fall through to `Empty`
/// (consistent with the `0` view class's "pass-through" contract for
/// unmapped values).
pub const fn local_view_class_for_tile(tile: u8) -> LocalViewClass {
    match tile {
        0x00 | 0xC0..=0xC3 | 0xCC..=0xCF | 0xFF => LocalViewClass::Empty,
        0x05 | 0x30..=0x37 => LocalViewClass::SparseCheckers,
        0x09..=0x0A | 0x2D => LocalViewClass::SolidFill,
        0x07
        | 0x1C
        | 0x1E..=0x1F
        | 0x40
        | 0x44
        | 0x48..=0x49
        | 0x6A..=0x6B
        | 0x70..=0x7F
        | 0x87
        | 0x8C
        | 0x8F
        | 0xAA
        | 0xBC
        | 0xDD => LocalViewClass::FilledFrame,
        0x1D
        | 0x38
        | 0x47
        | 0x5A
        | 0x5C..=0x5D
        | 0x94..=0x96
        | 0x9A..=0x9C
        | 0xAB..=0xAC
        | 0xBE => LocalViewClass::HorizontalRails,
        0x10..=0x1B
        | 0x29..=0x2B
        | 0x2E..=0x2F
        | 0x41..=0x43
        | 0x4C
        | 0x58..=0x59
        | 0x5B
        | 0x5E..=0x5F
        | 0x80..=0x85
        | 0x88..=0x8B
        | 0x8D..=0x8E
        | 0x90..=0x93
        | 0x9D..=0xA9
        | 0xAD..=0xB7
        | 0xBD
        | 0xBF
        | 0xC8..=0xCB
        | 0xDE..=0xDF
        | 0xE8..=0xEB
        | 0xFA..=0xFD => LocalViewClass::CentredBars,
        0x0D
        | 0x45
        | 0x4A..=0x4B
        | 0x86
        | 0x97..=0x99
        | 0xB8..=0xBB
        | 0xC4..=0xC7
        | 0xEC..=0xF9 => LocalViewClass::HollowRectangle,
        0x0C
        | 0x27..=0x28
        | 0x39..=0x3F
        | 0x46
        | 0x4D..=0x57
        | 0xD0..=0xD3
        | 0xFE => LocalViewClass::DiagonalStyle,
        0x0B | 0x0E..=0x0F => LocalViewClass::DiagonalStep,
        0x06 | 0x08 | 0x2C => LocalViewClass::VegetationHybrid,
        0x03 | 0x60..=0x69 | 0x6C..=0x6F | 0xE4..=0xE7 => LocalViewClass::FourCornerRing,
        0x02 | 0xD4..=0xD7 => LocalViewClass::DiagonalBlits,
        0x01 => LocalViewClass::NoopDefault,
        0x04 => LocalViewClass::CreatureComposite,
        0xE0..=0xE3 => LocalViewClass::VerticalWallDoor,
        0xD8..=0xDC => LocalViewClass::PeerVariant,
        0x20..=0x26 => LocalViewClass::FenceWall,
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

/// `view.md §4` Britannia chunk-map renderer dimensions. The full
/// chunk-map view paints an eight-row by twenty-two-column shorthand
/// map of Britannia chunks, wrapping the chunk walk at the world
/// edges and marking the party's current chunk with a crosshair-style
/// marker.
pub const BRITANNIA_CHUNK_MAP_ROWS: u8 = 8;
pub const BRITANNIA_CHUNK_MAP_COLUMNS: u8 = 22;

/// `view.md §4`: the LOOKOBJ chunk-map renderer is entered from
/// ordinary Look via this tile id. Final tile-catalog naming for
/// `0x59` is a separate verification item.
pub const BRITANNIA_CHUNK_MAP_LOOK_TRIGGER_TILE: u8 = 0x59;

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

/// `view.md §3` overworld/town fountain drink eligibility. The
/// LOOKOBJ fountain prompt asks the player to pick a drinker; dead
/// or asleep members refuse as incapacitated, while every other
/// status lets the drinker receive the (presentation-only) refresh
/// result. Dungeon fountains use the separate state-changing
/// fountain family in `dungeon-mode.md`.
pub const fn town_fountain_drink_accepts(status: crate::CharacterStatus) -> bool {
    !matches!(
        status,
        crate::CharacterStatus::Dead | crate::CharacterStatus::Sleeping,
    )
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

/// `commands.md §11` published Y-Yell narration strings. The
/// ship-aboard branch prints the sail-state message; the free-text
/// branch prints the nothing-said message on empty input.
pub const YELL_SAILS_HOISTED_MESSAGE: &str = "Sails hoisted.";
pub const YELL_SAILS_FURLED_MESSAGE: &str = "Sails furled.";
pub const YELL_NOTHING_SAID_MESSAGE: &str = "Nothing said.";

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

    /// `commands.md §8`: per-family floor/occupancy stamp the push/pull
    /// resolution writes into the vacated source cell on a successful
    /// move. The cannon family uses its own stamp byte; every other
    /// pushable family uses the generic cobble stamp. Both stamps render
    /// as cobble in the LOOK2-backed tile catalog, but the byte is
    /// load-bearing for P-Push's family-matching rule.
    pub const fn floor_stamp(self) -> u8 {
        match self {
            Self::CannonFourFacing => PUSHABLE_CANNON_FLOOR_STAMP,
            _ => PUSHABLE_GENERIC_FLOOR_STAMP,
        }
    }
}

/// `commands.md §8` generic cobble floor/occupancy stamp written by
/// a successful P-Push when the moved object is *not* in the cannon
/// family.
pub const PUSHABLE_GENERIC_FLOOR_STAMP: u8 = 0x44;
/// `commands.md §8` cannon-family floor/occupancy stamp. Renders the
/// same as the generic cobble stamp; the byte is still load-bearing
/// for the cannon family-matching rule.
pub const PUSHABLE_CANNON_FLOOR_STAMP: u8 = 0x45;

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

/// `commands.md §6` resolved outcome for one N-New Order command.
/// Cancellation of either prompt or a leader-slot selection both
/// abort without consuming a turn; only a successful non-leader
/// pair consumes the turn (same-slot pairs included — the swap is a
/// behavioural no-op but the turn is still consumed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewOrderOutcome {
    /// Either prompt was cancelled. The command prints the
    /// no-selection result and returns without consuming a turn.
    Cancelled,
    /// At least one selected slot is the leader (slot 0). The
    /// command refuses and returns without consuming a turn.
    LeaderRefusal,
    /// Both selections are non-leader slots. The handler exchanges
    /// the two roster records and consumes the turn. Same-slot pairs
    /// are accepted here; the swap is a no-op but the turn still
    /// counts.
    Swap { slot_a: usize, slot_b: usize },
}

/// `commands.md §6`: resolve the N-New Order outcome from the two
/// shared party-member selector results. Either selection being
/// `None` means the prompt was cancelled; otherwise the helper
/// applies the leader-slot refusal before returning a swap.
pub const fn new_order_outcome(
    slot_a: Option<usize>,
    slot_b: Option<usize>,
) -> NewOrderOutcome {
    let (a, b) = match (slot_a, slot_b) {
        (Some(a), Some(b)) => (a, b),
        _ => return NewOrderOutcome::Cancelled,
    };
    if a == 0 || b == 0 {
        return NewOrderOutcome::LeaderRefusal;
    }
    NewOrderOutcome::Swap { slot_a: a, slot_b: b }
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
