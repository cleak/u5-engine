//! Resident A-Z command dispatcher table per `commands.md` §4-§5.
//!
//! This module names each command letter and exposes the verb-prefix
//! string the original prints before invoking the handler or refusal
//! path. Per-mode routing (overworld vs town vs dungeon vs combat) lives
//! in the play-state dispatcher; this is just the canonical
//! letter-to-name table.

use crate::{
    Direction, SCENE_EMPATH_ABBEY, SCENE_OVERWORLD, SCENE_SERPENTS_HOLD, SCENE_THE_LYCAEUM,
    input_case_fold, tile_view_class,
};

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
/// in the overlay renders as a four-pixel square.
pub const LOCAL_VIEW_CELL_PIXEL_SCALE: usize = 4;
/// `view.md §4` absolute screen origin of the local View/Peer/X-Ray
/// overlay. The published cell-anchor formula is
/// `anchor_x = 32 + column * 4`, `anchor_y = 32 + row * 4`, so the
/// 128-by-128 raster occupies `(32,32)..=(159,159)` inside the main
/// play viewport.
pub const LOCAL_VIEW_OVERLAY_ORIGIN_X: usize = 32;
pub const LOCAL_VIEW_OVERLAY_ORIGIN_Y: usize = 32;

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
    /// `0xA` — water corners with river-shoreline source selection.
    WaterCorners,
    /// `0xB` — two diagonal blits (peer/gem-view bank-aware).
    DiagonalBlits,
    /// `0xC` — deep water's single modal micro-blit.
    DeepWater,
    /// `0xD` — fixed-secondary top plus modal-terrain bottom composite.
    FixedModalComposite,
    /// `0xE` — vertical two-line wall/door presentation.
    VerticalWallDoor,
    /// `0xF` — direct normal-terrain filled-frame chain.
    NormalTerrainFrame,
    /// `0x10` — road body, connection stubs, and elbow corner notch.
    Road,
}

/// `view.md §4`: classify a tile id into its LOOKOBJ local-view
/// class. The mapping is exhaustive across all 256 tile ids; tiles
/// not explicitly listed in the spec table fall through to `Empty`
/// (consistent with the `0` view class's "pass-through" contract for
/// unmapped values).
pub const fn local_view_class_for_tile(tile: u8) -> LocalViewClass {
    match tile_view_class(tile) {
        0x00 => LocalViewClass::Empty,
        0x01 => LocalViewClass::SparseCheckers,
        0x02 => LocalViewClass::SolidFill,
        0x03 => LocalViewClass::FilledFrame,
        0x04 => LocalViewClass::HorizontalRails,
        0x05 => LocalViewClass::CentredBars,
        0x06 => LocalViewClass::HollowRectangle,
        0x07 => LocalViewClass::DiagonalStyle,
        0x08 => LocalViewClass::DiagonalStep,
        0x09 => LocalViewClass::VegetationHybrid,
        0x0A => LocalViewClass::WaterCorners,
        0x0B => LocalViewClass::DiagonalBlits,
        0x0C => LocalViewClass::DeepWater,
        0x0D => LocalViewClass::FixedModalComposite,
        0x0E => LocalViewClass::VerticalWallDoor,
        0x0F => LocalViewClass::NormalTerrainFrame,
        0x10 => LocalViewClass::Road,
        _ => LocalViewClass::Empty,
    }
}

/// `view.md §3` accepted wishing-well wish keywords. The well's
/// object-spawn branch accepts only the six vehicle/joke names
/// recognized by the original handler. Match is case-insensitive
/// at the caller; this catalog stores the canonical capitalisation.
pub const WISHING_WELL_WISH_KEYWORDS: [&str; 6] = [
    "Corvette",
    "Ferrari",
    "Lamborghini",
    "Lotus",
    "Porsche",
    "Horse",
];

pub const WISHING_WELL_WISH_MAX_CHARS: usize = 12;

/// `view.md §3`: returns `true` when the typed wish matches one of
/// the six accepted wishing-well keywords (case-insensitive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WishingWellWish {
    Corvette,
    Ferrari,
    Lamborghini,
    Lotus,
    Porsche,
    Horse,
}

impl WishingWellWish {
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Corvette => "Corvette",
            Self::Ferrari => "Ferrari",
            Self::Lamborghini => "Lamborghini",
            Self::Lotus => "Lotus",
            Self::Porsche => "Porsche",
            Self::Horse => "Horse",
        }
    }

    pub const fn has_native_grant(self) -> bool {
        true
    }
}

pub fn wishing_well_wish(typed: &str) -> Option<WishingWellWish> {
    let upper = typed.trim().to_ascii_uppercase();
    [
        WishingWellWish::Corvette,
        WishingWellWish::Ferrari,
        WishingWellWish::Lamborghini,
        WishingWellWish::Lotus,
        WishingWellWish::Porsche,
        WishingWellWish::Horse,
    ]
    .into_iter()
    .find(|wish| wish.keyword().to_ascii_uppercase() == upper)
}

pub fn wishing_well_wish_accepted(typed: &str) -> bool {
    wishing_well_wish(typed).is_some()
}

/// `formats/look2-dat.md §5` command-owned surface/town special
/// handler range for the LOOKOBJ fountain-style presentation path.
/// The fountain result itself is presentation-only per `view.md §3`.
pub const fn surface_town_fountain_look_tile(tile: u8) -> bool {
    matches!(tile, 0xd8..=0xdb)
}

/// `formats/look2-dat.md §5` / `view.md §3` command-owned
/// surface/town wishing-well LOOKOBJ special handler.
pub const fn surface_wishing_well_look_tile(tile: u8) -> bool {
    tile == 0xa1
}

/// `view.md §3` / public issue #43: wishing-well object grants are
/// accepted only in the two published scene contexts after the coin
/// prompt and wish match.
pub const fn wishing_well_grant_scene(scene: u8) -> bool {
    matches!(scene, 0x16 | 0x1f)
}

/// `view.md §3` entry-dispatch row 2: the **live terrain-layer** tile
/// that routes Look to the death-vision branch — the crystal-sphere
/// tile `0x29`.
///
/// §3 is explicit that the tested byte is "a single terrain-layer byte
/// ... never an active-object or creature descriptor", and it warns
/// about exactly this two-domain confusion for the `0xD8..0xDB`
/// fountain/Daemon band: "Same four numbers, two different lookup
/// domains, no relationship between them." An active object whose type
/// byte happens to be `0x29` must therefore *not* trigger the vision.
///
/// The row is also ordered ahead of the per-map object row 4 and ahead
/// of the shared "thou dost see" preamble: "the vision case is decided
/// before anything is printed".
pub const DEATH_VISION_LOOK_TILE: u8 = 0x29;
pub const DEATH_VISION_ROLL_LOW: u8 = 1;
pub const DEATH_VISION_ROLL_HIGH: u8 = 30;

/// `view.md §3` entry-dispatch row 2 predicate on the live terrain
/// tile at the Look target cell.
pub const fn death_vision_look_tile(tile: u8) -> bool {
    tile == DEATH_VISION_LOOK_TILE
}

/// Public issue #43: active-object classes that share the sign/poster
/// lookup path before generic object description.
pub const fn sign_or_wanted_poster_object_class(type_byte: u8) -> bool {
    matches!(type_byte, 0xa0 | 0xa4 | 0xf8 | 0x89 | 0x8a)
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
pub const YELL_SAILS_HOISTED_MESSAGE: &str = "HOIST!";
pub const YELL_SAILS_FURLED_MESSAGE: &str = "FURL!";
pub const YELL_NOTHING_SAID_MESSAGE: &str = "Nothing said.";

/// `commands.md §11` / `vehicles.md §6`: the no-input sail shortcut
/// is selected only for a frigate marker in the unsigned low scene-byte
/// half. This deliberately admits world, town, dungeon, and defensive
/// custom bytes `0x00..=0x7f`; `0x80..=0xff` use the ordinary word prompt.
pub const fn yell_routes_to_ship_sails(scene_byte: u8, aboard_frigate: bool) -> bool {
    aboard_frigate && scene_byte < 0x80
}

/// `commands.md §11` scene routing for the typed Y-Yell input. The
/// engine selects the scanner family from the active scene context;
/// other contexts produce no effect after the prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YellInputContext {
    /// The three Eternal Flame keeps — the typed word is compared against the
    /// three Shadowlord names.
    ShadowlordName,
    /// Outdoor scene zero on either world surface — the typed word is compared
    /// against the eight dungeon words in fixed order.
    WordOfPower,
    /// Any other non-ship context — the prompt completes without
    /// effect after empty or non-matching input.
    NoEffect,
}

/// `commands.md §11`: select exactly one typed-word scanner from the unsigned
/// scene byte. The world plane is intentionally absent because both outdoor
/// surfaces use scene zero.
pub const fn yell_input_context(scene_byte: u8) -> YellInputContext {
    match scene_byte {
        SCENE_OVERWORLD => YellInputContext::WordOfPower,
        SCENE_THE_LYCAEUM | SCENE_EMPATH_ABBEY | SCENE_SERPENTS_HOLD => {
            YellInputContext::ShadowlordName
        }
        _ => YellInputContext::NoEffect,
    }
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

/// `commands.md §8` / `vehicles.md §8`: static town cannon tiles are
/// a four-facing family. F-Fire can use an adjacent cannon as a local
/// fire source without a sidecar row; the low two bits select the
/// projectile direction with the public cardinal facing convention.
pub const TOWN_CANNON_TILE_FIRST: u8 = 0xB4;
pub const TOWN_CANNON_TILE_LAST: u8 = 0xB7;

pub const fn town_cannon_tile_fire_direction(tile: u8) -> Option<Direction> {
    match tile {
        TOWN_CANNON_TILE_FIRST..=TOWN_CANNON_TILE_LAST => match tile & 0x03 {
            0 => Some(Direction::North),
            1 => Some(Direction::East),
            2 => Some(Direction::South),
            _ => Some(Direction::West),
        },
        _ => None,
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

/// `commands.md §8`: four-facing pushable families use the same
/// low-two-bit cardinal facing convention as stairs and transport markers:
/// north `0`, east `1`, south `2`, west `3`.
pub const fn pushable_facing_index(direction: Direction) -> Option<u8> {
    Some(match direction {
        Direction::North => 0,
        Direction::East => 1,
        Direction::South => 2,
        Direction::West => 3,
        _ => return None,
    })
}

/// `commands.md §8`: successful pushes and pulls rotate chair/cannon
/// families to the movement-facing low bits. Non-rotating families keep
/// their original tile byte.
pub const fn pushable_oriented_tile(tile: u8, direction: Direction) -> u8 {
    let Some(family) = pushable_tile_family(tile) else {
        return tile;
    };
    if !family.rewrites_facing() {
        return tile;
    }
    let Some(facing) = pushable_facing_index(direction) else {
        return tile;
    };
    (tile & !0x03) | facing
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
pub const fn new_order_outcome(slot_a: Option<usize>, slot_b: Option<usize>) -> NewOrderOutcome {
    let (a, b) = match (slot_a, slot_b) {
        (Some(a), Some(b)) => (a, b),
        _ => return NewOrderOutcome::Cancelled,
    };
    if a == 0 || b == 0 {
        return NewOrderOutcome::LeaderRefusal;
    }
    NewOrderOutcome::Swap {
        slot_a: a,
        slot_b: b,
    }
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

/// `dungeon-mode.md §4.1`: the dungeon level is presented one-based
/// ("prints the one-based dungeon level"), while the runtime stores the
/// zero-based Z index. Every player-facing level statement goes through
/// this helper so the two numbering schemes never mix.
pub const fn dungeon_display_level(level: u8) -> u16 {
    level as u16 + 1
}

/// `commands.md §5.3` punctuation contract (`cleak/u5-spec#81`): how a
/// verb literal ends decides what continues the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandEchoJoin {
    /// Trailing hyphen — a direction is awaited and its name is appended
    /// on the same line. `Look-` + `Pass` renders as one line.
    AwaitsDirection,
    /// Trailing `...` — a sub-selection is awaited on another surface,
    /// and the handler's own output starts a fresh line.
    AwaitsSelection,
    /// Trailing space — a further keystroke or typed argument continues
    /// the same line.
    AwaitsArgument,
    /// Newline or nothing: the echo is complete on its own line and any
    /// handler output is a new line.
    Complete,
}

impl CommandEchoJoin {
    /// Whether a handler's output continues the echoed line rather than
    /// starting a new one.
    pub const fn continues_line(self) -> bool {
        matches!(
            self,
            CommandEchoJoin::AwaitsDirection | CommandEchoJoin::AwaitsArgument
        )
    }
}

/// One resident verb echo: the literal the dispatcher writes into the
/// message transcript, plus how the handler's own output continues it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandEcho {
    pub text: &'static str,
    pub join: CommandEchoJoin,
}

/// Which command overlay's copy of the verb literals applies.
///
/// `#81`: "each mode overlay carries its own copy of the Attack
/// literal", and dungeon Look hands off to the look overlay instead of
/// prompting for a direction, so two verbs differ by mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandEchoMode {
    /// Overworld and town-family scenes.
    Surface,
    /// Dungeon exploration.
    Dungeon,
}

/// `commands.md §5.2` per-key verb echo literals (`cleak/u5-spec#81`).
///
/// The literals below are the published table, not inferences. Three
/// points from §5.1/§5.2 shape how a renderer must use them:
///
/// * The leading marker is **not** part of any verb literal. The turn
///   loop opens each input line with a newline plus one right-pointing
///   solid triangle glyph, so exactly one line per command turn carries
///   it — see [`MessageEntry::is_command_echo`].
/// * The echo is printed **before** the precondition check, so a refusal
///   is a second line rather than a replacement (`View a gem!` then
///   `You have none!`). The few refusals that do replace the echo fold
///   the verb into the refusal literal instead.
/// * `Use item`, `Ready...` and `Mix Reagents` end in **two** newlines,
///   leaving one blank row before their prompt.
pub const fn command_echo(command: Command, mode: CommandEchoMode) -> Option<CommandEcho> {
    use CommandEchoJoin::{AwaitsArgument, AwaitsDirection, AwaitsSelection, Complete};
    let dungeon = matches!(mode, CommandEchoMode::Dungeon);
    let (text, join) = match command {
        Command::Pass => ("Pass", Complete),
        // §5.2: the dungeon attack takes no direction argument, so its
        // literal carries no hyphen; every other mode prompts.
        Command::Attack => {
            if dungeon {
                ("Attack", Complete)
            } else {
                ("Attack-", AwaitsDirection)
            }
        }
        // §5.2: the stored literal is the bare word `Look`; the
        // dispatcher appends the hyphen dynamically outside dungeons and
        // `...` inside them, where it hands off to the look overlay.
        Command::Look => {
            if dungeon {
                ("Look...", AwaitsSelection)
            } else {
                ("Look-", AwaitsDirection)
            }
        }
        Command::Fire => ("Fire-", AwaitsDirection),
        Command::Get => ("Get-", AwaitsDirection),
        Command::Jimmy => ("Jimmy-", AwaitsDirection),
        Command::Klimb => ("Klimb-", AwaitsDirection),
        Command::Open => ("Open-", AwaitsDirection),
        Command::Push => {
            if dungeon {
                // `commands.md §8.1`: dungeon P bypasses direction
                // handling and replaces the ordinary hyphenated echo with
                // the complete `Push` line before `Not here!`.
                ("Push", Complete)
            } else {
                ("Push-", AwaitsDirection)
            }
        }
        Command::Search => ("Search-", AwaitsDirection),
        Command::Talk => ("Talk-", AwaitsDirection),
        Command::Cast => ("Cast...", AwaitsSelection),
        Command::Ready => ("Ready...", AwaitsSelection),
        Command::ZStats => ("Z-stats...", AwaitsSelection),
        Command::Board => ("Board ", AwaitsArgument),
        Command::Xit => ("X-it ", AwaitsArgument),
        Command::Yell => ("Yell ", AwaitsArgument),
        Command::HoleUp => ("Hole up- ", AwaitsArgument),
        Command::Ignite => ("Ignite torch!", Complete),
        Command::Mix => ("Mix Reagents", Complete),
        Command::Use => ("Use item", Complete),
        Command::NewOrder => ("New Order", Complete),
        Command::View => ("View a gem!", Complete),
        Command::Enter => ("Enter ", AwaitsArgument),
        Command::Quit => ("Quit:", Complete),
        // §5.2: the two sibling refusals reach the screen with a
        // disambiguating prefix.
        Command::UnassignedRefusal => ("What?", Complete),
    };
    Some(CommandEcho { text, join })
}

/// `commands.md §5.2` (`#81`): the `D` and `W` refusals print with a
/// disambiguating prefix, `D-What?` and `W-What?`, while an unmapped key
/// prints the bare `What?`.
pub const fn unassigned_refusal_echo(letter: u8) -> &'static str {
    match letter {
        b'D' | b'd' => "D-What?",
        b'W' | b'w' => "W-What?",
        _ => "What?",
    }
}

/// `commands.md §5.2` (`#81`): dungeon movement has its own verb set,
/// separate from the surface direction names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonMovementEcho {
    Advance,
    BackUp,
    TurnLeft,
    TurnRight,
    TurnAround,
}

impl DungeonMovementEcho {
    pub const fn literal(self) -> &'static str {
        match self {
            DungeonMovementEcho::Advance => "Advance",
            DungeonMovementEcho::BackUp => "Back up",
            DungeonMovementEcho::TurnLeft => "Turn left",
            DungeonMovementEcho::TurnRight => "Turn right",
            // The only one of the five that carries its own full stop.
            DungeonMovementEcho::TurnAround => "Turn around.",
        }
    }

    pub const fn echo(self) -> CommandEcho {
        CommandEcho {
            text: self.literal(),
            join: CommandEchoJoin::Complete,
        }
    }
}

/// The refused-step line, shared by every mode that has one.
///
/// `audio.md §7.4` censuses it directly: "the game contains exactly five
/// copies of the `Blocked!` string and exactly five pieces of code that print
/// one - town (beeps), overworld (beeps, conditionally), two in the dungeon
/// (both silent), and combat (beeps)". `commands.md §5.2` (`#81`) lists it
/// among the dungeon movement refusals and `combat.md §3` gives the arena
/// transcript "North / Blocked!" on two lines.
///
/// It was named `DUNGEON_MOVEMENT_BLOCKED_REFUSAL` and had no production
/// caller; the name claimed a mode scope the census contradicts.
pub const MOVEMENT_BLOCKED_REFUSAL: &str = "Blocked!";
/// `doors-and-z-transitions.md §9`: the one line the outdoor climb prints,
/// once per living member whose Dexterity roll fails. The successful climb
/// prints nothing at all.
pub const OUTDOOR_CLIMB_FALL_REFUSAL: &str = "Fell!";
/// Published as a movement-family refusal: `#81` could not pin it to a
/// single key, so it is not bound to one here either.
pub const DUNGEON_MOVEMENT_NOT_IN_DOORWAY_REFUSAL: &str = "Not in doorway!";
/// `#81`: `View a gem!` is echoed before the precondition check, so the
/// no-gem refusal is always a second line rather than a replacement.
pub const VIEW_NO_GEM_REFUSAL: &str = "You have none!";
/// `#81`: two refusals fold the verb into the refusal literal instead of
/// following the echo, so the verb is not echoed separately for them.
pub const PUSH_NOT_HERE_REFUSAL: &str = "Not here!";
/// `commands.md §8.1`: a source active object or non-pushable static
/// tile takes the emphatic refusal.
pub const PUSH_WONT_BUDGE_EMPHATIC: &str = "Won't budge!";
/// `commands.md §8.1`: a pushable static source with neither a legal
/// push nor pull takes the shorter refusal.
pub const PUSH_WONT_BUDGE_SHORT: &str = "Won't budge";
pub const PUSHED_SUCCESS: &str = "Pushed!";
pub const PULLED_SUCCESS: &str = "Pulled!";

/// `commands.md §5.4` (`#81`): "The direction prompt prints nothing. The
/// hyphen *is* the prompt." It ignores every key except the four
/// directions, Space and Escape. Ordinarily Space and Escape both print
/// `Pass`; `commands.md §8.1` gives Push its narrower exception: Space
/// prints Pass, while Escape emits nothing and leaves the prompt active.
pub const DIRECTION_PROMPT_CANCEL_LITERAL: &str = "Pass";

/// `commands.md §5.6` (`#81`) selection prompts, each with exactly one
/// trailing space, printed into the message window on the line after the
/// verb echo.
pub const PARTY_SELECTION_PROMPT: &str = "Player: ";
pub const ITEM_SELECTION_PROMPT: &str = "Item: ";
/// The cancel result appended to an open selection prompt line.
pub const SELECTION_CANCELLED_LITERAL: &str = "None!";

/// `commands.md §5.7`: the rest/hole-up input sequence uses these exact
/// message-window literals in both outdoor and town contexts.
pub const REST_HOURS_PROMPT: &str = "For how many hours? (1-9) ";
pub const REST_WATCH_PROMPT: &str = "\nWilt thou set a watch? ";
pub const REST_WATCH_MEMBER_PROMPT: &str = "Who will stand guard? ";
pub const REST_WATCH_YES_LITERAL: &str = "Yes\n\n";
pub const REST_WATCH_NO_LITERAL: &str = "No\n\n";
pub const REST_NO_WATCH_LITERAL: &str = "None posted!\n\n";

/// `commands.md §5.5` (`#81`) dungeon narration. The engine's old
/// `Entered <name> level N at (x, y).` has no counterpart in the
/// original; these two lines are the ones that do exist.
pub const DUNGEON_ROOM_ENTRY_NARRATION: &str = "Entering room...\n";
/// `doors-and-z-transitions.md §12.1`, dungeon exit: two prints, `\nExit to `
/// with a trailing space and no line feed of its own, then the plane name with
/// `\n\n`. Silent: no key wait and no sound at all. Rendered into the
/// fifteen-column window the plane name lands whole on its own row.
pub const DUNGEON_EXIT_TO_BRITANNIA_NARRATION: &str = "\nExit to Britannia!\n\n";
pub const DUNGEON_EXIT_TO_UNDERWORLD_NARRATION: &str = "\nExit to Underworld!\n\n";

/// `doors-and-z-transitions.md §12.1`, town-family boundary exit — the only
/// key wait on any plane-change path. The prompt re-polls until `Y`, `N` or
/// Escape, discards every other key, and does **not** echo, so the answer word
/// below is printed by the handler.
pub const TOWN_EXIT_PROMPT: &str = "\nDost thou wish to leave? ";
/// The accepted answer plus the exit preamble. Unlike the dungeon form the
/// break before the plane name **is** in the data, and the blank row sits
/// before `Exit to` rather than after the plane name.
pub const TOWN_EXIT_ACCEPTED_NARRATION: &str = "Yes\n\nExit to\n";
/// `§12.1`: scene `0x19` (Ararat) is the only location on the underworld
/// plane, so it is the only town-family exit that names the Underworld.
pub const TOWN_EXIT_TO_UNDERWORLD_NARRATION: &str = "Underworld!\n";
pub const TOWN_EXIT_TO_BRITANNIA_NARRATION: &str = "Britannia!\n";
/// `§12.1`: declining (`N` or Escape) prints `No\n` and nothing else.
pub const TOWN_EXIT_DECLINED_NARRATION: &str = "No\n";

/// `overworld.md §8.1` falls chain, step 1. There is no leading blank row and
/// no trailing blank row, and the chain carries **no per-member narration**
/// at all: the fall's per-member feedback is a stats-row flash and a rumble.
pub const OVERWORLD_FALLS_BANNER: &str = "F-A-L-L-S!!!\n";
/// `overworld.md §8.1` falls chain, step 7 — printed **only** when the party
/// now stands on Britannia `(54, 138)`. Rendered into the fifteen-column
/// window the printer breaks it on the space after `into`.
pub const OVERWORLD_FALLS_UNDERWORLD_NARRATION: &str = "Falling into underworld!!\n";
/// `overworld.md §8.1` whirlpool swallow, step 1 — "note the leading line
/// feed, which costs one blank row". It is the first and only text on the
/// path; there is no advance warning line.
pub const OVERWORLD_WHIRLPOOL_BANNER: &str = "\nWHIRLPOOL!\n";

/// `dungeon-mode.md §8.1` post-action underfoot consequences, in print order
/// per event. None of these carries a leading `\n` of its own: the blank line
/// the player sees before each message is the render-and-poll step's line feed
/// plus border repaint, "not the string's", and "an implementation that adds
/// one per message will double the spacing".
pub const DUNGEON_SLEEP_FIELD_LINE: &str = "Sleep spell!\n";
pub const DUNGEON_POISON_FIELD_LINE: &str = "Poison!\n";
/// Two exclamation marks.
pub const DUNGEON_FIRE_FIELD_LINE: &str = "Fire!!\n";
/// `§8.1` fall trap `0x61` / `0x69`, **once per descent step**: this line,
/// then `Falling...`, then the level change and view repaint, then the splat.
pub const DUNGEON_PIT_TRAP_LINE: &str = "Pit Trap!\n";
pub const DUNGEON_FALLING_LINE: &str = "Falling...\n";
/// **Six leading spaces**, and they are significant.
pub const DUNGEON_SPLAT_LINE: &str = "      ...splat!\n";
pub const DUNGEON_BOMB_TRAP_LINE: &str = "Bomb Trap!\n";
pub const DUNGEON_KABOOM_LINE: &str = "KABOOM!!\n";
/// `§8.1` electric contact is a movement-time consequence: these two lines
/// print **before** the destination-class test, so they precede any
/// `Blocked!` the same step later produces.
pub const DUNGEON_ELECTRIC_OUCH_LINE: &str = "Ouch!\n";
pub const DUNGEON_ELECTRIC_FIELD_LINE: &str = "Electric field!\n";

/// `dungeon-mode.md §8.1` darkness refusals — both break after the colon
/// (`RETRACTIONS.md` R323). There is no "too dark" literal anywhere; the
/// Search form is a *find* line with a leading blank row.
pub const DUNGEON_LOOK_DARKNESS_REFUSAL: &str = "You see:\ndarkness.\n";
pub const DUNGEON_SEARCH_DARKNESS_REFUSAL: &str = "\nYou find:\ndarkness.\n";

/// `dungeon-mode.md §8.1`: Search prints this preamble **unconditionally**,
/// then exactly one outcome line. `Nothing of note.` is an *outcome*, not the
/// preamble (`RETRACTIONS.md` R324).
pub const DUNGEON_SEARCH_PREAMBLE: &str = "You find:\n";
pub const DUNGEON_SEARCH_NOTHING_OF_NOTE: &str = "Nothing of note.\n";
/// The line break is in the data.
pub const DUNGEON_SEARCH_NOTHING_IN_PIT: &str = "Nothing hidden\nin the pit.\n";
pub const DUNGEON_SEARCH_NOTHING_ON_LADDER: &str = "Nothing hidden on the ladder.\n";
pub const DUNGEON_SEARCH_NOTHING_ON_FOUNTAIN: &str = "Nothing hidden on the fountain.\n";
pub const DUNGEON_SEARCH_NOTHING_ON_DOOR: &str = "Nothing hidden on the door.\n";
pub const DUNGEON_SEARCH_NOTHING_ON_WALL: &str = "Nothing hidden on the wall.\n";
pub const DUNGEON_SEARCH_TREASURE: &str = "Treasure!\n";
pub const DUNGEON_SEARCH_IMPOSSIBLE_TILE: &str = "This tile is impossible.\n";
pub const DUNGEON_SEARCH_NOTHING_ON_STALACTITE: &str = "Nothing on the stalactite.\n";
pub const DUNGEON_SEARCH_NOTHING_IN_CAVED_IN_PASSAGE: &str = "Nothing in the caved in passage.\n";
pub const DUNGEON_SEARCH_NOTHING_ON_SKELETON: &str = "Nothing hidden on the skeleton.\n";
pub const DUNGEON_SEARCH_SKELETON_CRUMBLES: &str = "It crumbles away.\n";
/// `RETRACTIONS.md` R322: `A hidden door!` belongs to the `0xD?` **wall**
/// branch, and `A pit!` is the exact-`0x61` outcome — the two were swapped.
pub const DUNGEON_SEARCH_HIDDEN_DOOR: &str = "A hidden door!\n";
pub const DUNGEON_SEARCH_A_PIT: &str = "A pit!\n";
pub const DUNGEON_SEARCH_A_BOMB_TRAP: &str = "A bomb trap!\n";
/// The four trap-tier lines, **none of which carries a terminal period**.
/// Selected by `dungeon_chest_search_trap_line` from the tier the dungeon
/// chest Search detection roll computes (`dungeon-mode.md` Section 8).
pub const DUNGEON_SEARCH_NO_TRAP: &str = "No trap\n";
pub const DUNGEON_SEARCH_SIMPLE_TRAP: &str = "A simple trap\n";
pub const DUNGEON_SEARCH_GENERIC_TRAP: &str = "A trap\n";
pub const DUNGEON_SEARCH_COMPLEX_TRAP: &str = "A complex trap\n";

/// **PUBLISHED, UNWIRED.** The seven literals below are transcribed from the
/// spec and pinned by the conformance test, but no handler reads them yet:
/// the dungeon fountain drink path still renders this engine's own diagnostic
/// prose. They are kept as the published record of the text, deliberately and
/// visibly ahead of the wiring - see the convention note at
/// [`DUNGEON_KLIMB_PROMPT_BOTH`]. Nothing here may be read as a claim that
/// the engine already prints them.
///
/// `dungeon-mode.md §8.1` fountain drink flow. The prompt blocks until `Y` or
/// `N`; the accepted answer carries **two spaces** after the period.
pub const DUNGEON_FOUNTAIN_DRINK_PROMPT: &str = "Will you drink?\n";
pub const DUNGEON_FOUNTAIN_DECLINED: &str = "No.\n";
pub const DUNGEON_FOUNTAIN_ACCEPTED: &str = "Yes.  Gulp!\n";
pub const DUNGEON_FOUNTAIN_CURED: &str = "Cured!\n";
pub const DUNGEON_FOUNTAIN_HEALED: &str = "Healed!\n";
pub const DUNGEON_FOUNTAIN_POISONED: &str = "Poisoned!\n";
pub const DUNGEON_FOUNTAIN_BAD_TASTE: &str = "Bad taste.\n";

/// **PUBLISHED, UNWIRED - and the convention note for that marker.** A
/// literal carrying this marker is transcribed from the spec and pinned by
/// the conformance test, but has no reader in any handler, because the engine
/// does not yet reach the situation that prints it. Deleting such a literal
/// until its handler lands would lose the transcription and invite a later
/// re-invention of the text, so the constants stay, marked. The project's own
/// "does anything read this?" check should treat a marked constant as a
/// known, disclosed gap and an unmarked one with no reader as a defect.
///
/// Unwired here: the three prompt forms and the Space answer.
/// `DUNGEON_KLIMB_WHAT_REFUSAL`, `DUNGEON_KLIMB_UP`, `DUNGEON_KLIMB_DOWN` and
/// `DUNGEON_KLIMB_FAILED` below **are** wired.
///
/// `dungeon-mode.md §8.1` Klimb prompts. `Klimb-U/D-` blocks until up or down
/// is chosen and Space answers `Pass\n\n`; `Klimb-` is the one-direction form;
/// `Klimb-\nWith What?\n` is the climb-with-equipment refusal — whether it is
/// specifically the *no-grapple* refusal is **probable, not established**, so
/// no caller may treat the gating byte as confirmed to be the grapple count.
pub const DUNGEON_KLIMB_PROMPT_BOTH: &str = "Klimb-U/D-";
pub const DUNGEON_KLIMB_PROMPT_ONE: &str = "Klimb-";
pub const DUNGEON_KLIMB_WITH_WHAT_REFUSAL: &str = "Klimb-\nWith What?\n";
pub const DUNGEON_KLIMB_WHAT_REFUSAL: &str = "Klimb-what?\n";
pub const DUNGEON_KLIMB_PASS: &str = "Pass\n\n";
/// Applying a climb prints the direction word **first**, before any test.
pub const DUNGEON_KLIMB_UP: &str = "Up!\n";
pub const DUNGEON_KLIMB_DOWN: &str = "Down!\n";
/// `§8.1`: an impassable destination adds this, with the short **rising**
/// sweep — the same recipe the spell-failure tail uses, not the falls descent.
pub const DUNGEON_KLIMB_FAILED: &str = "Failed!\n";

/// **PUBLISHED, UNWIRED.** The twelve chest literals below have no reader in
/// any handler yet - the dungeon Jimmy/Open/Get arms still render this
/// engine's own prose, and wiring them means also reproducing the resident
/// dispatcher's `Jimmy-`/`Open-` prefix row. Marked per the convention note
/// at [`DUNGEON_KLIMB_PROMPT_BOTH`].
///
/// `dungeon-mode.md §8.1` chest lines. The resident dispatcher's prefix and
/// the overlay's line share a row unless the overlay's line begins with a
/// line feed, so Jimmy's results all start with a bare `\n` and Open's last
/// two arms deliberately do not.
pub const DUNGEON_CHEST_JIMMY_NO_KEYS: &str = "\nNo keys!\n";
pub const DUNGEON_CHEST_JIMMY_KEY_BROKE: &str = "\nKey broke!\n";
pub const DUNGEON_CHEST_JIMMY_UNLOCKED: &str = "\nChest unlocked\n";
pub const DUNGEON_CHEST_JIMMY_ALREADY_OPEN: &str = "\nAlready open!\n";
pub const DUNGEON_CHEST_JIMMY_WHAT: &str = "\nWhat?\n";
pub const DUNGEON_CHEST_OPENED: &str = "\nChest opened\n";
/// Capital `O` — a **different** literal from Jimmy's, and it carries no
/// leading line feed, so it renders on the prefix's own row.
pub const DUNGEON_CHEST_OPEN_ALREADY_OPEN: &str = "Already Open!\n";
pub const DUNGEON_CHEST_OPEN_WHAT: &str = "What?\n";
pub const DUNGEON_CHEST_GET_ECHO: &str = "Get\n";
pub const DUNGEON_CHEST_GET_MUST_OPEN_FIRST: &str = "Must open first!\n";
pub const DUNGEON_CHEST_GET_NOT_HERE: &str = "Not here!\n";
pub const DUNGEON_CHEST_GET_CONTENTS: &str = "contents\nof chest\nYou find:\n";

/// `commands.md §5.2` (`cleak/u5-spec#81`): surface and town movement
/// keys echo the direction's name plus a newline — the same four words
/// the direction prompt appends. Diagonal steps are not part of that
/// four-word block, so they emit no echo.
pub const fn movement_echo(direction: Direction) -> Option<CommandEcho> {
    let text = match direction {
        Direction::North => "North",
        Direction::South => "South",
        Direction::East => "East",
        Direction::West => "West",
        Direction::NorthEast
        | Direction::NorthWest
        | Direction::SouthEast
        | Direction::SouthWest => return None,
    };
    Some(CommandEcho {
        text,
        // `commands.md §5.2`: movement echoes the direction name plus a
        // newline — "the same four words the direction prompt appends".
        join: CommandEchoJoin::Complete,
    })
}
