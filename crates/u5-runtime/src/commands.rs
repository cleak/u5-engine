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
    /// `commands.md §5.2`, the `0` row: a plain digit is the Set Active
    /// Player command, not a movement key. `input.md §5` keeps the
    /// numpad promotion behind the Shift/NumLock flag, so an unshifted
    /// top-row digit "remains available as ordinary text input" and
    /// reaches this command instead.
    SetActivePlayer,
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
            Command::SetActivePlayer => "Set Active Plr",
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
/// Measured 2026-09-07 (paired capture, Britain's crystal sphere): both
/// outcomes are one word pair and an exclamation mark, printed under the
/// `Look-<direction>` echo with no `Thou dost see` preamble and no member
/// number. The successful case then paints the location overlay, which the
/// next keypress dismisses; the failed case paints nothing. There is no
/// party-member prompt on either path - the command uses the active player.
pub const DEATH_VISION_STRANGE_LINE: &str = "Strange vision!\n";
pub const DEATH_VISION_DEATH_LINE: &str = "Death vision!\n";

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
/// `inventory.md §2.1`: "If the total is greater than Strength, R-Ready
/// prints the \"not strong enough\" refusal and makes no inventory or
/// equipment change." The section names the refusal without quoting it;
/// **measured** against the original (`qa/paired/hut-ready-refusals.tsv`)
/// it is this line, and the item picker stays open under it.
/// `cleak/u5-spec#225`.
/// **Measured** (`qa/paired/ready-slots.tsv`, 2026-09-07): R-Ready's two slot
/// refusals. The occupied-slot line names the slot - `Remove first thy present
/// helm!` - and the two-handed refusal is one long sentence. `inventory.md §5`
/// describes both branches without quoting either; the engine had
/// `Remove current helm first.` and `Both hands must be free.`.
pub const READY_REMOVE_PRESENT_PREFIX: &str = "Remove first thy present ";

/// `inventory.md §5.2`'s refusal table. The occupied-slot message is **not**
/// one composed sentence with the slot name substituted - each slot has its
/// own wording, and only the helm's happens to match the composed form.
pub const READY_REMOVE_HELM_REFUSAL: &str = "Remove first thy present helm!";
pub const READY_REMOVE_ARMOUR_REFUSAL: &str = "Thou must first remove thine other armour!";
pub const READY_REMOVE_AMULET_REFUSAL: &str = "Thou must remove thine other amulet!";
pub const READY_ONE_RING_REFUSAL: &str = "Only one magic ring may be worn at a time!";
/// `inventory.md §5.2`: "Required arrows or quarrels absent". One line covers
/// both; the engine had a pair of its own, `No arrows for that weapon.` and
/// `No quarrels for that weapon.`
pub const READY_NO_AMMUNITION_REFUSAL: &str = "Thou hast no ammunition for that weapon!";
/// `inventory.md §5.2`: "Body-armour change during undecided combat". §5.2
/// also fixes its position - "The combat armour lock applies before the
/// already-readied unequip test."
pub const READY_COMBAT_ARMOUR_LOCK_REFUSAL: &str = "Thou canst not change armour in heated battle!";

/// `inventory.md §5.2`'s occupied-slot message for `slot`. The weapon and
/// off-hand slots have no occupied-slot line of their own: §5.2 covers them
/// with the two hand-conflict refusals instead.
pub const fn ready_occupied_slot_refusal(slot: usize) -> Option<&'static str> {
    Some(match slot {
        crate::EQUIP_SLOT_HELM => READY_REMOVE_HELM_REFUSAL,
        crate::EQUIP_SLOT_ARMOUR => READY_REMOVE_ARMOUR_REFUSAL,
        crate::EQUIP_SLOT_AMULET => READY_REMOVE_AMULET_REFUSAL,
        crate::EQUIP_SLOT_RING => READY_ONE_RING_REFUSAL,
        _ => return None,
    })
}
/// **Measured** 2026-09-07 with a two-handed sword already wielded: readying
/// an off-hand item answers this, where the engine had `Weapon hand holds a
/// two-handed item.`
pub const READY_FREE_A_HAND_REFUSAL: &str = "Thou must free one of thy hands first!";

pub const READY_BOTH_HANDS_REFUSAL: &str = "Both hands must be free before thou canst wield that!";

/// **Measured** 2026-09-07 (a party carrying nothing): opening R-Ready with
/// an empty pack answers `Thou art empty-handed!`, where the engine had
/// `Nothing to ready.`
///
/// The line feed after the hyphen is authored, not wrapped. Measured
/// 2026-09-10 (`combat-ready-armour/ready-open`): the original breaks it
/// `Thou art empty-` / `handed!`, and `text-output.md` §5 defines the
/// wrap-aware printer's soft break as "(space, line-feed, or
/// carriage-return)" - no hyphen. `Thou art empty-` is fifteen cells in a
/// sixteen-cell window, so it does not reach the edge and cannot break there
/// on its own; only an authored feed puts `handed!` on the next row. The
/// soft-hyphen underscore of §8 is the *proportional* renderer's and does not
/// reach this printer.
/// `inventory.md` §5.2: "The ring vanish instead prints `\n\nRing
/// vanishes!\n` and closes without `Done`."
/// `commands.md` §11.1: a recognised Word of Power "immediately prints the
/// uttered-word result". Measured 2026-09-11
/// (`word-of-power-audio/dosbox-word`): the original's rows read a blank
/// row, `A word of power`, `is uttered`.
///
/// The leading blank row belongs to this result, exactly as §5.2's
/// no-effect result owns the blank row above its own text (`Yell what?` /
/// `:WORD` / `[blank]` / `No effect!`). Without it the engine ran the
/// uttered line straight onto the typed-word row and sat one row high for
/// the rest of the capture.
pub const WORD_OF_POWER_UTTERED_MESSAGE: &str = "\nA word of power is uttered";
/// The tail a recognised Word adds when no adjacent cell qualifies, or the
/// coordinate does not match. Measured in the same capture: a blank row and
/// `No effect!`.
pub const WORD_OF_POWER_NO_EFFECT_TAIL: &str = "\n\nNo effect!";

pub const READY_RING_VANISHES_MESSAGE: &str = "\n\nRing vanishes!\n";

pub const READY_EMPTY_HANDED_REFUSAL: &str = "Thou art empty-\nhanded!";

pub const READY_NOT_STRONG_ENOUGH_REFUSAL: &str = "Thou art not strong enough!";

/// `inventory.md §4.4`: the U-Use flow "print[s] `Item:_` into the message
/// window" and the picker's accepted row completes that line, the same way
/// `Player:_` takes the chosen member's name.
///
/// **Measured** (`qa/paired/hut-use-items.tsv`): the completion is the
/// item's *family* word, not the picker's own label - a scroll row labelled
/// `Scroll IS` completes the line as `Item: Scroll`, and a potion row
/// labelled `Yellow Pot` completes it as `Item: Potion`. The families of
/// the remaining U-Use items are not measured, so they complete nothing
/// rather than guessing. `cleak/u5-spec#225`.
pub const USE_ITEM_ECHO_SCROLL: &str = "Scroll";
/// See [`USE_ITEM_ECHO_SCROLL`].
pub const USE_ITEM_ECHO_POTION: &str = "Potion";
/// **Measured** 2026-09-07: the skull key row completes as `Item: Skull Key`.
pub const USE_ITEM_ECHO_SKULL_KEY: &str = "Skull Key";
/// The rest of the family words, all **measured** 2026-09-07 by standing each
/// item alone in the picker (`seed_inventory ... specials=0 special<N>=1`).
pub const USE_ITEM_ECHO_CARPET: &str = "Carpet";
pub const USE_ITEM_ECHO_AMULET: &str = "Amulet";
pub const USE_ITEM_ECHO_CROWN: &str = "Crown";
pub const USE_ITEM_ECHO_SCEPTRE: &str = "Sceptre";
pub const USE_ITEM_ECHO_GEM_SHARD: &str = "Gem Shard";
pub const USE_ITEM_ECHO_SPYGLASS: &str = "Spyglass";
pub const USE_ITEM_ECHO_PLANS: &str = "Plans";
pub const USE_ITEM_ECHO_SEXTANT: &str = "Sextant";
pub const USE_ITEM_ECHO_WATCH: &str = "Watch";
pub const USE_ITEM_ECHO_BADGE: &str = "Badge";
pub const USE_ITEM_ECHO_BOX: &str = "Box";
pub const USE_ITEM_ECHO_MOONSTONE: &str = "Moonstone";
/// **Measured** 2026-09-07 from a ship at sea: the moonstone's refusal
/// *continues* its `Item: ` row - `Item: Moonstone cannot be buried here!`
/// is one wrapped sentence. The engine named the tile byte instead.
pub const MOONSTONE_BURY_REFUSAL: &str = " cannot be buried here!";

/// **Measured** 2026-09-07 with each item alone in the picker: the spyglass
/// answers `No stars!` when there are none to read, the sextant answers
/// `Only outdoors!` indoors, and the wooden box asks `How?`. The engine had
/// `Cannot see the stars!`, `Sextant:\nCannot see the stars!` and
/// `Wooden Box: How use it?`.
pub const USE_SPYGLASS_NO_STARS: &str = "No stars!";
pub const USE_SEXTANT_ONLY_OUTDOORS: &str = "Only outdoors!";
/// **Measured** 2026-09-07 from a ship at night: the U-Use sextant labels its
/// reading `Position:`, not the `Sextant:` `magic.md` §8 names. The shared
/// coordinate printer below it is unchanged (`cleak/u5-spec#237`).
pub const USE_SEXTANT_READING_LABEL: &str = "Position:";

/// **Measured** 2026-09-07 (`qa/paired/shrine-enter.tsv`): a shrine is
/// *entered*. Standing on the shrine marker tile and pressing `E` opens this
/// question with an input row beneath it; the engine had `M` open a
/// `Shrine of <virtue> mantra?` prompt on the overworld tile, which the
/// original answers with Mix Reagents.
pub const SHRINE_VIRTUE_PROMPT: &str = "Upon what virtue dost thou meditate?";
/// The rest of the measured entry: the `Enter ` echo completes by naming the
/// shrine - which is what `gazetteer.md` §7 means when it says the one
/// coordinate table also supplies "the shrine of *virtue*" name - and two
/// narration lines print before the question, each followed by a blank row.
pub const SHRINE_ENTER_ECHO_PREFIX: &str = "the shrine of ";
/// `karma.md §8`: E on the Codex tile "prints `Enter the Shrine of the
/// Codex!\n` and immediately starts the shrine/Codex presentation". The
/// literal completes the open `Enter ` echo, so it carries no verb of its own.
pub const CODEX_SHRINE_ENTER_ECHO_TAIL: &str = "the Shrine of the Codex!";
/// The line ends in an ellipsis, which is why the capture wraps `tranquil`
/// and `Shrine...` onto separate rows: glyph-index decoding of the paired
/// capture reads `0x2e 0x2e 0x2e` after `Shrine`, and the kneel line below
/// ends in a single period.
pub const SHRINE_APPROACH_NARRATION: &str = "Thou dost approach the tranquil Shrine...";
pub const SHRINE_KNEEL_NARRATION: &str = "...and thou dost kneel before the Altar.";
/// **Measured** 2026-09-07 (`qa/paired/shrine-flow.tsv`): accepting the virtue
/// leaves the question and the typed answer on screen, then opens this prompt
/// a blank row below it. The typed mantra echoes on the same row as the label
/// (`Mantra:AHM`). The engine previously invented
/// `Shrine of <virtue> mantra? _` plus an instructional line.
pub const SHRINE_MANTRA_PROMPT: &str = "Mantra:";

/// `karma.md §12`: an affordable offering finishes "with ten world ticks".
pub const SHRINE_OFFERING_RESULT_WORLD_TICKS: usize = 10;
/// The daylight refusal, measured aboard a ship at noon.
pub const USE_SEXTANT_DAYTIME_REFUSAL: &str = "Only at night!";
pub const USE_WOODEN_BOX_PROMPT: &str = "How?";
/// **Measured** 2026-09-07: a skull key tried on a cell with no lock answers
/// `Failed!`, not the Jimmy family's `No lock!`.
pub const USE_SKULL_KEY_FAILED: &str = "Failed!";

/// **Measured** (`qa/paired/hut-use-watch.tsv`): the pocket watch reads
/// its time as a sentence, `The pocket watch reads 8:35 AM.`, with the
/// hour unpadded, the minute two digits, the meridiem in capitals and a
/// closing full stop. The engine had `Pocket Watch: 8:35 AM`.
/// `inventory.md §7` describes the item without quoting the line.
/// `cleak/u5-spec#225`.
pub const USE_POCKET_WATCH_PREFIX: &str = "The pocket watch reads ";

/// **Measured** (`qa/paired/hut-use-items.tsv`): using a potion asks which
/// member drinks it with this prompt, and the chosen name completes the row
/// - `On who: Iolo`. `inventory.md §6` describes the target without
/// quoting the prompt. `cleak/u5-spec#225`.
pub const USE_POTION_TARGET_PROMPT: &str = "On who: ";

/// **Measured** (`qa/paired/use-scrolls.tsv`): a scroll that needs an
/// argument prints its effect word first and then the argument prompt -
/// `Wind change!` over `Direction-`, and `Resurrection!` over `On who: `.
/// Neither prints anything more once the argument is accepted.
/// **Measured** (`qa/paired/use-specials.tsv`): the special-item rows answer
/// with their own sentences. `inventory.md §7` describes each item's effect
/// but quotes none of these.
/// **Measured** (`qa/paired/ship-commands.tsv`, 2026-09-07): the three lines a
/// ship under sail answers with. `vehicles.md` describes each case without
/// quoting it. X-it under sail answers `Under sail!` (the engine had `Cannot
/// exit while sails are hoisted.`); a diagonal heading answers the resident
/// `What?` refusal (the engine had `Sails need a cardinal heading.`); and the
/// Pass that follows a stalled heading answers `Sheets in irons!` (the engine
/// had `Ship remains stalled by the wind.`). The stalled sail attempt itself
/// prints nothing at all - the line lands on the following turn.
/// **Measured** (`qa/paired/cast-results.tsv`, 2026-09-07): a spell whose
/// effect lands answers `Success!` where it answers at all, and the two
/// party-target spells share the potions' `On who: ` prompt. An Zu cast on a
/// sleeping member wakes them and prints **nothing** - exactly like the blue
/// potion - and both spells answer `Failed!` when the target is not in the
/// state they treat. `magic.md` describes the effects without quoting a line.
pub const SPELL_SUCCESS_LINE: &str = "Success!";

pub const SHIP_XIT_UNDER_SAIL_REFUSAL: &str = "Under sail!";
pub const SHIP_SAIL_STALLED_LINE: &str = "Sheets in irons!";

pub const USE_MAGIC_CARPET_BOARDED: &str = "Boarded!";
/// **Measured** 2026-09-07 aboard a frigate: unrolling the carpet while
/// mounted or aboard answers `X-it ship first!`, where the engine had the
/// two-word `On foot.`.
pub const USE_MAGIC_CARPET_XIT_FIRST: &str = "X-it ship first!";
/// `inventory.md §7.1` (`cleak/u5-spec#251`): boarding tests the transport
/// marker after the scene and terrain gates. "Ship markers `0x20..0x27`
/// produce `X-it ship first!\n`, and every other marker produces
/// `Only on foot!\n`" - so a horse, skiff, balloon or carpet answers this
/// one, not the ship line the engine used for every non-foot transport.
pub const USE_MAGIC_CARPET_ONLY_ON_FOOT: &str = "Only on foot!";
/// The HMS Cape plans read off a ship. Measured: `Only usable on shipboard!`,
/// where the engine had `Not aboard ship!`.
pub const USE_PLANS_SHIPBOARD_ONLY_REFUSAL: &str = "Only usable on shipboard!";
/// And the accepted form, measured aboard the ship.
pub const USE_PLANS_RIGGED_LINE: &str = "Ship rigged for double speed!";
/// `inventory.md §7`'s Use result table ends all three regalia lines with an
/// ellipsis: `Wearing the Amulet of Lord British...`, `Thou dost don the Crown
/// of Lord British...` and `Wielding the Sceptre of Lord British...`. The
/// engine dropped it on all three. Measured 2026-09-10
/// (`use-specials/crown`): the original reads `British...` over `No effect!`.
pub const USE_AMULET_WORN: &str = "Wearing the Amulet of Lord British...";
pub const USE_CROWN_WORN: &str = "Thou dost don the Crown of Lord British...";
pub const USE_SCEPTRE_WIELDED: &str = "Wielding the Sceptre of Lord British...";
pub const USE_BLACK_BADGE_WORN: &str = "Badge worn!";
pub const USE_REGALIA_REMOVED: &str = "Removed!";
/// The shard line names the shard on its own row run: `Thou dost hold above
/// thee the evil Shard of Falsehood`.
pub const USE_SHARD_ALOFT_PREFIX: &str = "Thou dost hold above thee the evil ";

pub const SCROLL_WIND_CHANGE_RESULT: &str = "Wind change!";
pub const SCROLL_RESURRECTION_RESULT: &str = "Resurrection!";

/// **Measured** (`qa/paired/use-potions.tsv`, a stocked save built with the
/// `seed_inventory` probe): each potion answers with one short line under
/// the completed `On who: <name>` row, and two of them answer with nothing
/// at all. `inventory.md §7` publishes the colour order and the effects but
/// none of the lines. `cleak/u5-spec#225`.
///
/// - wake on a sleeping member: **no line**, the status letter simply
///   returns to `G`;
/// - wake on a member who is already awake: [`POTION_RESULT_FAILED`];
/// - heal: [`POTION_RESULT_HEALED`];
/// - cure poison on a poisoned member: [`POTION_RESULT_POISON_CURED`],
///   otherwise [`POTION_RESULT_FAILED`];
/// - poison: [`POTION_RESULT_POISONED`], in capitals;
/// - sleep: [`POTION_RESULT_SLEPT`];
/// - the two combat-only colours used outside combat:
///   [`POTION_RESULT_NO_NOTICEABLE_EFFECT`], which opens with a blank row;
/// - the white visibility repaint: **no line**.
pub const POTION_RESULT_FAILED: &str = "Failed!\n";
pub const POTION_RESULT_HEALED: &str = "Healed!\n";
pub const POTION_RESULT_POISON_CURED: &str = "Poison cured!\n";
pub const POTION_RESULT_POISONED: &str = "POISONED!\n";
pub const POTION_RESULT_SLEPT: &str = "Slept!\n";
pub const POTION_RESULT_NO_NOTICEABLE_EFFECT: &str = "\nNo noticeable effect now!\n";

/// `inventory.md §4.4`, the U-Use reference sequence: "refuse with
/// `No_usable_items!\n` if nothing is usable". The engine had
/// `No usable items.` - the same words with the wrong punctuation.
pub const USE_NO_USABLE_ITEMS_REFUSAL: &str = "No usable items!";

/// `lighting.md §8`: I-Ignite "consumes one torch from inventory; with no
/// torches available it refuses and leaves the light state unchanged". The
/// section does not quote the refusal; **measured** against the original
/// (`qa/paired/hut-ignite-torches.tsv`: the shipped four torches lit, then
/// a fifth attempt) it is this line, printed under the verb echo like
/// every other §5.2 refusal. `cleak/u5-spec#225`.
pub const IGNITE_NO_TORCHES_REFUSAL: &str = "None owned!";

/// `magic.md §8`: the creature-prompt targeters' prompt. The dispatcher
/// "prints `Creature: ` and opens the arena cursor", and `§8`'s combat
/// flow lists "`Creature: ` and the confirmed-target newline" between the
/// spell-name echo and the pre-effect. The trailing space keeps the
/// cursor on the row, the same shape `commands.md §5.3` gives every other
/// prompt that ends in one.
pub const COMBAT_CREATURE_TARGET_PROMPT: &str = "Creature: ";

/// `commands.md §9`, Control + `E`: the program-exit prompt.
///
/// The spec had two renderings - §9's bare `Exit to DOS?` and
/// `dungeon-mode.md §8`'s `Exit to DOS? ` with a trailing space - and the
/// engine could not tell which was the literal. **Measured**: the answer
/// lands on the prompt's own row (`Exit to DOS? N`), which is
/// `commands.md §5.3`'s trailing-space contract, so the space is real.
pub const EXIT_TO_DOS_PROMPT: &str = "Exit to DOS? ";
/// The declined answer, measured as the bare upper-case letter on the
/// prompt row. The accepted answer has the same shape.
pub const EXIT_TO_DOS_NO_REPLY: &str = "N";
/// See [`EXIT_TO_DOS_NO_REPLY`].
pub const EXIT_TO_DOS_YES_REPLY: &str = "Y";

/// `commands.md §9`: the typeahead toggle's two states. Measured: no
/// terminating full stop.
pub const TYPEAHEAD_BUFFER_ON_MESSAGE: &str = "Buffer On";
/// See [`TYPEAHEAD_BUFFER_ON_MESSAGE`].
pub const TYPEAHEAD_BUFFER_OFF_MESSAGE: &str = "Buffer Off";
/// `commands.md §9`, Control + `V`: the version banner. **Measured** -
/// the section names the binding but publishes no literal, and the
/// original prints this. `cleak/u5-spec#224`.
pub const VERSION_BANNER_MESSAGE: &str = "1.16";

/// `combat.md §8.2`: the Attack family's fixed refusal. The same
/// section states that "there is no `<attacker> attacks <target>` line,
/// in any wording, anywhere in the shipped game", which is the shape
/// this engine's composed diagnostics had. Measured on the original:
/// `A` north with nothing there prints `Attack-North` and then this on
/// the row under it.
pub const ATTACK_NOTHING_TO_ATTACK_REFUSAL: &str = "Nothing to attack!";

/// `dungeon-mode.md §10`: the dungeon A-Attack probe "prints the stock refusal
/// `What?\n` on the following row and does not launch combat" when the active
/// monster is not in the forward cell.
pub const DUNGEON_ATTACK_NO_TARGET_REFUSAL: &str = "What?\n";

/// `commands.md §5.2`, the `0` row: "Cancelled: `None!\n`".
pub const SET_ACTIVE_PLAYER_NONE_REPLY: &str = "None!";
/// `commands.md §5.2`, the `0` row: "rejected: `Invalid!\n`". Measured
/// on the original: `9` on a party that has no ninth slot answers this.
pub const SET_ACTIVE_PLAYER_INVALID_REPLY: &str = "Invalid!";

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
        // §5.2 mirrors Look: the dungeon overlay echoes `Search...` and
        // hands off to the relative-focus helper instead of prompting for
        // a cardinal direction (measured 2026-09-07).
        Command::Search => {
            if dungeon {
                ("Search...", AwaitsSelection)
            } else {
                ("Search-", AwaitsDirection)
            }
        }
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
        // §5.2: `Set_Active_Plr:\n` - the answer (`None!`, the chosen
        // name, or `Invalid!`) lands on the row under it.
        Command::SetActivePlayer => ("Set Active Plr:", Complete),
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

/// **Measured** 2026-09-07 (`qa/paired/slow-terrain.tsv`): a step onto brush
/// prints this under the direction echo. The step still succeeds - eight
/// scripted steps north from the Honesty shrine crossed three brush cells and
/// then five tree/foothill cells, and the line changed exactly where the
/// terrain did.
pub const MOVEMENT_SLOW_PROGRESS_LINE: &str = "Slow progress!";
/// **Measured** the same way, for trees and foothills.
pub const MOVEMENT_VERY_SLOW_LINE: &str = "Very slow!";

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
/// `view.md §3` entry-dispatch row 3: "the shared \"thou dost see\" preamble
/// is printed here" for the object, sign and terrain arms; the description
/// follows on the next row (`cleak/u5-spec#194` capture).
pub const LOOK_RESULT_PREFIX: &str = "\nThou dost see";

/// The town arrest sequence's surrender question.
///
/// `town-mode.md §1123` (literals added 2026-09-06, spec issue #206):
/// the sequence "prints the arrest challenge (`"Thou art under
/// arrest!"`, a blank row) and asks whether the party will come quietly
/// (`"Wilt thou come quietly?"`, a blank row, a `:` answer row that
/// echoes `Yes` or `No!`)". The `\n\n` pairs are those blank rows and
/// the `\n:` tail is the answer row, the same envelope
/// [`crate::TLK_KEYWORD_PROMPT`] carries.
///
/// This replaces the engine's own `Surrender? (Y/N).`, which was
/// invented because §1123 published no literal at the time.
pub const TOWN_ARREST_SURRENDER_PROMPT: &str =
    "\"Thou art under arrest!\"\n\n\"Wilt thou come quietly?\"\n\n:";
/// `town-mode.md §1123`: "the surrender line is `The guard strikes thee
/// unconscious!`". The awakening line the same sentence mentions is
/// still unpublished, so the engine prints this one and stops.
pub const TOWN_ARREST_KNOCKOUT_MESSAGE: &str = "The guard strikes thee unconscious!";
/// `town-mode.md §1123`: the `:` answer row echoes `Yes` or `No!`.
pub const TOWN_ARREST_YES_REPLY: &str = "Yes";
pub const TOWN_ARREST_NO_REPLY: &str = "No!";
/// `cleak/u5-spec#194` (black-box): X-it on foot completes its own
/// `X-it ` echo line with `what?`.
pub const XIT_ON_FOOT_REFUSAL: &str = "what?";

/// The same argument slot, completed by the vehicle the party leaves.
/// **Measured** (`qa/paired/ship-commands.tsv`): leaving a frigate reads
/// `X-it ship!` on one row, and the handler prints no line of its own -
/// where this engine wrote a composed sentence naming the skiff it
/// launched. The words for the horse and the carpet are not measured.
/// `cleak/u5-spec#225`.
pub const XIT_SHIP_ARGUMENT: &str = "ship!";
/// `cleak/u5-spec#194` (black-box): a plain Yell that matches neither a
/// Shadowlord name nor a Word of Power prints `No effect!`.
pub const YELL_NO_EFFECT_MESSAGE: &str = "No effect!";
/// The row Y-Yell's typed word is edited on. `commands.md §5.3` puts
/// the echo behind a colon on the row under the question, and
/// `text-output.md §10.6` keeps a prompt that is waiting for input on
/// its own row with the cursor inline - so this is an open prompt line,
/// not a fresh live row. Same literal as the conversation prompts'
/// [`crate::TLK_KEYWORD_PROMPT_OPEN_LINE`], different producer.
pub const YELL_FREE_TEXT_OPEN_LINE: &str = ":";
/// `commands.md §11`: "the ordinary `Yell what?` question", completing
/// the `Yell ` echo.
pub const YELL_QUESTION: &str = "what?";
/// `cleak/u5-spec#194` (black-box): `Get-<dir>` with nothing to pick up.
/// `inventory.md §4.5` "Which U-Use rows omit quantity": "In the picker
/// stock, value 255 is the no-quantity marker; zero means absent from
/// U-Use, and ordinary positive quantities produce the counted layout."
/// A row holding this value prints only its name, starting in window
/// column 1 with all thirteen interior cells available.
/// `rest-and-camp.md §4` "Sleep text is caller-specific": "Town-bed rest
/// prints `Zzzzzzz...\n` (seven z characters, three dots)." The section names
/// four surfaces with four different z-counts and says outright: "Do not apply
/// one generic sleep sentence to all four surfaces."
/// `shops.md §8.C`'s inn result table, "Sleep / morning": `Zzzzzz....\n\n` -
/// four dots - "followed at morning by `Morning!\n`". `rest-and-camp.md §4`
/// distinguishes this six-z/four-dot form from the town bed's seven-z form.
/// `doors-and-z-transitions.md §6`: the cannon's hit narration. "Firing a
/// ship's cannon at a door (or wall) prints `BOOOM!` with the cannon's hit
/// narration; on a hit at a door cell, the engine rewrites the cell to the
/// open-door tile (or rubble) and prints `Door destroyed!`."
pub const CANNON_BOOOM_LINE: &str = "BOOOM!";
/// See [`CANNON_BOOOM_LINE`]: the extra line a door hit adds.
pub const CANNON_DOOR_DESTROYED_LINE: &str = "Door destroyed!";

pub const PAID_INN_REST_SLEEP_LINE: &str = "Zzzzzz....";
/// See [`PAID_INN_REST_SLEEP_LINE`]: the morning line that follows it.
pub const PAID_INN_REST_MORNING_LINE: &str = "Morning!";

pub const TOWN_BED_REST_SLEEP_LINE: &str = "Zzzzzzz...";

pub const USE_PICKER_NO_QUANTITY: u8 = 255;

pub const GET_NOTHING_REFUSAL: &str = "\nNothing to get!";
/// `cleak/u5-spec#194` (black-box): `Hole up- ` off an inn bed completes
/// its echo line with this refusal.
pub const HOLE_UP_NOT_IN_BED_REFUSAL: &str = "Only in bed!";
/// `cleak/u5-spec#194` (black-box): `Enter ` with no entrance underfoot.
pub const ENTER_NOTHING_REFUSAL: &str = "what?";
/// `cleak/u5-spec#194` capture (town) and `dungeon-mode.md` (dungeon):
/// the Search result when nothing is found.
pub const SEARCH_NOTHING_FOUND: &str = "\nThou dost find\nnothing of note.";
pub const ITEM_SELECTION_PROMPT: &str = "Item: ";
/// The cancel result appended to an open selection prompt line.
pub const SELECTION_CANCELLED_LITERAL: &str = "None!";

/// `view.md §3` wishing well (live tile `0xA1`): "Prompt for a coin and a
/// wish". The section publishes the flow and the six accepted words but
/// none of its lines; these are **measured** against the original
/// (`qa/paired/town-wishing-well.tsv`, Paws):
///
/// ```text
/// >Look-North
///
/// Thou dost see
/// a well.
///
/// Drop a coin?Yes
///
/// Thy wish?
/// HORSE
/// Poof!
/// ```
///
/// The Look preamble and description are the shared terrain-description
/// path, the coin answer completes the prompt's own row with no
/// separating space, the typed wish echoes upper-cased on its own row,
/// and the grant prints [`WISHING_WELL_GRANT_LINE`].
/// `cleak/u5-spec#225`.
pub const WISHING_WELL_COIN_PROMPT: &str = "Drop a coin?";
/// See [`WISHING_WELL_COIN_PROMPT`]. `view.md §3`'s row 3 gives the well
/// "its own handler and **its own description**", which is this - not the
/// `LOOK2.DAT` record for tile `0xA1`, which reads differently and is
/// what this engine had been printing.
pub const WISHING_WELL_LOOK_DESCRIPTION: &str = "a well.";
/// See [`WISHING_WELL_COIN_PROMPT`].
pub const WISHING_WELL_COIN_YES_REPLY: &str = "Yes";
/// See [`WISHING_WELL_COIN_PROMPT`].
pub const WISHING_WELL_WISH_PROMPT: &str = "Thy wish?";
/// See [`WISHING_WELL_COIN_PROMPT`].
pub const WISHING_WELL_GRANT_LINE: &str = "Poof!";
/// See [`WISHING_WELL_COIN_PROMPT`]: the declined coin completes the
/// prompt row and prints nothing else. Measured.
pub const WISHING_WELL_COIN_NO_REPLY: &str = "No";
/// See [`WISHING_WELL_COIN_PROMPT`]: a wish the well does not accept
/// answers this under the upper-cased echo. **Measured** for the
/// mismatched-word arm in a granting scene; `view.md §3`'s coinless and
/// ungated arms are not measured and take the same line here.
pub const WISHING_WELL_NO_EFFECT_LINE: &str = "No effect...";

/// `view.md §3`: the surface/town fountain look "prompt[s] for the
/// drinking party member". The section publishes the flow but none of its
/// three literals; a paired capture of the stock game supplies them.
///
/// The full sequence for a fountain one cell north reads
/// `Look-North`, the shared [`LOOK_RESULT_PREFIX`] preamble, this
/// description, a blank row, the prompt, and then the reply on its own
/// line - `Refreshing...` for an accepted member, and the universal
/// [`SELECTION_CANCELLED_LITERAL`] when the selector is cancelled.
/// `cleak/u5-spec#197` asks for all of them to be published, along with
/// the one literal a healthy party cannot reach: the refusal for the
/// dead or asleep member the section does describe.
pub const FOUNTAIN_LOOK_DESCRIPTION: &str = "a gurgling fountain!";
/// See [`FOUNTAIN_LOOK_DESCRIPTION`]. No trailing space: the reply lands
/// on the next line rather than continuing this one.
pub const FOUNTAIN_DRINK_PROMPT: &str = "Who will drink?";
/// See [`FOUNTAIN_LOOK_DESCRIPTION`].
pub const FOUNTAIN_DRINK_REFRESHED: &str = "Refreshing...";
/// `view.md §3` (literals added 2026-09-06, `cleak/u5-spec#197`): "A Dead
/// or Asleep member prints `Incapacitated!` and a blank row; any other
/// member prints `Refreshing...`." The trailing feed pair is that blank
/// row, the same shape [`CAMP_BODY_SLEEP_LINE`] carries.
pub const FOUNTAIN_DRINK_INCAPACITATED: &str = "Incapacitated!\n\n";

/// `commands.md §5.7`: the rest/hole-up input sequence uses these exact
/// message-window literals in both outdoor and town contexts.
pub const REST_HOURS_PROMPT: &str = "For how many hours? (1-9) ";
/// `commands.md §5.5`: the H-Hole-up family's land form, "`Hole_up_&_`
/// plus ... `camp!\n\n` on land" - a complete echo on its own row with a
/// blank row after it, not the `Hole up- ` argument form the bed path
/// completes with `Only in bed!`.
/// `dungeon-mode.md §11`: the camp body's sleep line, "`Zzzzzz...\n\n`,
/// with **two** trailing line feeds", which the same sentence warns "is a
/// different literal from the turn loop's `Zzzzzz...\n`" and "must not be
/// shared with it".
pub const CAMP_BODY_SLEEP_LINE: &str = "Zzzzzz...\n\n";
/// `commands.md §5.5`: the H-Hole-up family's sea form, "`Hole_up_&_`
/// plus `\nrepair...\n\n` ... at sea". Measured aboard a frigate: the
/// echo occupies two rows, `>Hole up &` and `repair...`, and the result
/// or refusal opens a block under it.
pub const HOLE_UP_REPAIR_ECHO: CommandEcho = CommandEcho {
    text: "Hole up &",
    join: CommandEchoJoin::Complete,
};

/// The rest of the same literal - `commands.md §5.5` writes it as
/// `\nrepair...\n\n`, so it lands on the row under the echo and opens a
/// block before the result.
pub const HOLE_UP_REPAIR_BODY: &str = "repair...\n";

/// `commands.md §5.5`: "`Hull_now_` plus `!\n\n` at sea".
///
/// **Measured** (`qa/paired/ship-repair.tsv`, eight runs of three
/// hole-ups from the same 77-hull seed): the hull climbs by an inclusive
/// `1..3` roll per hole-up and both this line and the stats panel's
/// `Ship:` counter report the value *after* the repair. Twenty-four
/// increments came out 1x12, 2x4, 3x8 - never zero, never above three.
/// `vehicles.md §10` says no repair path was traced; this is it. The
/// cap is not measured; [`SHIP_HULL_REPAIR_CAP`] takes the published
/// shipwright purchase hull for it. `cleak/u5-spec#225`.
pub const HOLE_UP_HULL_NOW_PREFIX: &str = "Hull now ";
/// See [`HOLE_UP_HULL_NOW_PREFIX`]: the measured `1..3` repair roll.
pub const SHIP_HULL_REPAIR_ROLL_LOW: u8 = 1;
/// See [`HOLE_UP_HULL_NOW_PREFIX`].
pub const SHIP_HULL_REPAIR_ROLL_HIGH: u8 = 3;
/// See [`HOLE_UP_HULL_NOW_PREFIX`]. `vehicles.md §4` gives the shipwright
/// frigate "hull condition `99`", so the repair is held there; the
/// original's own cap is unmeasured.
pub const SHIP_HULL_REPAIR_CAP: u8 = 99;

/// `commands.md §5.5`: "`Sails_must_be\n` plus `lowered!\n\n`" - the
/// sea refusal, printed under the repair echo. Measured.
pub const HOLE_UP_SAILS_MUST_BE_LOWERED: &str = "Sails must be\nlowered!";

pub const HOLE_UP_CAMP_ECHO: CommandEcho = CommandEcho {
    text: "Hole up & camp!",
    join: CommandEchoJoin::Complete,
};
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
/// sixteen-column window (`RETRACTIONS.md` R347) the published rows are a
/// blank row, `Exit to`, the plane name, and a trailing blank row.
///
/// The engine ships the two prints as one concatenated literal. Its break
/// before the plane name is then the interior space, which the collector
/// remembers as a legal soft break - not R349's hard-chunk boundary, which
/// is what the original's *separate* second print takes: "the printer
/// collects the eight characters that still fit, finds no break byte ...
/// and prints them from column 0". §12.1 states the rendered rows are the
/// same under either mechanism for these two strings, so the concatenation
/// is output-equivalent here. The hard-chunk mechanism itself lives in the
/// wrap-aware printer (`text-output.md` §6, `RETRACTIONS.md` R346) and is
/// exercised against it directly.
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
/// now stands on Britannia `(54, 138)`. Rendered into the sixteen-column
/// window the printer breaks it on the space after `into` - the same two
/// rows either way, which is "a coincidence of these particular strings,
/// not a vindication of the old number" (`RETRACTIONS.md` R347).
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

/// `dungeon-mode.md §11`: dungeon Look and Search both hand off to the
/// shared relative-focus helper, whose blocking prompt is the open `Dir-`
/// line. Measured 2026-09-07 against the stock game: the four movement keys
/// complete that line with the relative word — up is ahead, left and right
/// rotate a quarter turn, and down selects the party's own cell — while
/// Space completes it with the shared `Pass` word and aborts. Letter keys
/// and Escape are ignored at this prompt.
/// `dungeon-mode.md §11` ("Who acts"): dungeon Look, Search, Jimmy and Open
/// run the shared acting-member prompt first. It "is silent when zero or one
/// member is eligible and prints `Player: ` otherwise, echoing the chosen
/// member's name. A pick whose status is neither Good nor Poisoned answers
/// `Disabled!\n\n` and re-prompts from `Player: `; a cancel, or a party with
/// no eligible member at all, answers `None!\n` and the command aborts."
pub const DUNGEON_ACTING_MEMBER_DISABLED: &str = "Disabled!\n\n";
pub const DUNGEON_ACTING_MEMBER_NONE: &str = "None!\n";

pub const DUNGEON_DIRECTION_PROMPT: &str = "Dir-";
pub const DUNGEON_DIRECTION_LABEL_AHEAD: &str = "Ahead";
pub const DUNGEON_DIRECTION_LABEL_LEFT: &str = "Left";
pub const DUNGEON_DIRECTION_LABEL_RIGHT: &str = "Right";
pub const DUNGEON_DIRECTION_LABEL_HERE: &str = "Here";

/// `dungeon-mode.md §8.1` darkness refusals — both break after the colon
/// (`RETRACTIONS.md` R323). There is no "too dark" literal anywhere; the
/// Search form is a *find* line with a leading blank row.
pub const DUNGEON_LOOK_DARKNESS_REFUSAL: &str = "You see:\ndarkness.\n";
/// The sighted form breaks after the colon the same way (measured
/// 2026-09-07: `You see:` then `a wall.` on the next row).
pub const DUNGEON_LOOK_PREAMBLE: &str = "You see:\n";
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

/// `doors-and-z-transitions.md §8` (`RETRACTIONS.md` R412): the town and
/// dwelling hidden-door terrain, revealed by ordinary Search with no object
/// flag, and the two floor-dependent replacements.
pub const TOWN_HIDDEN_DOOR_TILE: u8 = 0x4E;
pub const TOWN_HIDDEN_DOOR_REVEAL_ABOVE_GROUND: u8 = 0xB9;
pub const TOWN_HIDDEN_DOOR_REVEAL_BELOW_GROUND: u8 = 0xB8;
/// The line that reveal prints. Distinct from the dungeon's
/// [`DUNGEON_SEARCH_HIDDEN_DOOR`], which has no `Thou dost find` opening.
pub const TOWN_SEARCH_HIDDEN_DOOR_LINE: &str = "\nThou dost find\na hidden door!\n";
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

/// `movement.md §8.2`: the outdoor swamp save's own bare status line.
pub const OUTDOOR_SWAMP_POISONED_LINE: &str = "Poisoned!";
/// §8.2's inclusive `1..30` draw. The town underfoot save uses `0..29`, so
/// the two immunity thresholds differ by one and the ranges must not be
/// shared.
pub const OUTDOOR_SWAMP_POISON_ROLL_LOW: u8 = 1;
pub const OUTDOOR_SWAMP_POISON_ROLL_HIGH: u8 = 30;
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
/// `combat.md §7.1`: combat `J` "first requires the party to hold at least one
/// key: with a key count of zero it prints `No keys!` and returns immediately,
/// before the direction prompt and before any tile is examined."
pub const COMBAT_JIMMY_NO_KEYS_MESSAGE: &str = "No keys!";
pub const DUNGEON_CHEST_JIMMY_KEY_BROKE: &str = "\nKey broke!\n";
pub const DUNGEON_CHEST_JIMMY_UNLOCKED: &str = "\nChest unlocked\n";
pub const DUNGEON_CHEST_JIMMY_ALREADY_OPEN: &str = "\nAlready open!\n";
pub const DUNGEON_CHEST_JIMMY_WHAT: &str = "\nWhat?\n";
pub const DUNGEON_CHEST_OPENED: &str = "\nChest opened\n";
/// Capital `O` — a **different** literal from Jimmy's, and it carries no
/// leading line feed, so it renders on the prefix's own row.
pub const DUNGEON_CHEST_OPEN_ALREADY_OPEN: &str = "Already Open!\n";

/// `dungeon-mode.md §14`: what a dungeon monster's contact prints when the
/// party already faces it. The direction-bearing form appends ` from the `,
/// the lower-case compass word and `!\n` to the same prefix. This is
/// deliberately **not** shared with the Doom cave entrance's
/// `Attacked at entrance!`.
/// `magic.md §5.1`, An Grav: "A successful dungeon-cell removal prints
/// `Field destroyed!` with no generic completion line; successful combat field
/// removal prints `Success!`."
pub const DISPEL_FIELD_DESTROYED_LINE: &str = "Field destroyed!";

pub const DUNGEON_MONSTER_CONTACT_PREFIX: &str = "Attacked";
pub const DUNGEON_MONSTER_CONTACT_LINE: &str = "Attacked!\n";
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
