//! Equipment catalog metadata and R-Ready helpers.

use crate::*;

pub const EQUIPMENT_NAMES: [&str; EQUIPMENT_COUNT] = [
    "Leather Helm",
    "Chain Coif",
    "Iron Helm",
    "Spiked Helm",
    "Small Shield",
    "Large Shield",
    "Spiked Shield",
    "Magic Shield",
    "Jewel Shield",
    "Cloth Armour",
    "Leather Armour",
    "Ring Mail",
    "Scale Mail",
    "Chain Mail",
    "Plate Mail",
    "Mystic Armour",
    "Dagger",
    "Sling",
    "Club",
    "Flaming Oil",
    "Main Gauche",
    "Spear",
    "Throwing Axe",
    "Short Sword",
    "Mace",
    "Morning Star",
    "Bow",
    "Arrows",
    "Crossbow",
    "Quarrels",
    "Long Sword",
    "2H Hammer",
    "2H Axe",
    "2H Sword",
    "Halberd",
    "Sword of Chaos",
    "Magic Bow",
    "Silver Sword",
    "Magic Axe",
    "Glass Sword",
    "Jeweled Sword",
    "Mystic Sword",
    "Ring of Invisibility",
    "Ring of Protection",
    "Ring of Regeneration",
    "Amulet/Turning",
    "Spiked Collar",
    "Ankh",
];

/// Fixed narrow-panel equipment labels from `catalogs/item-list.md §5.1.2`.
/// Unlike the arms buy list, the arms sell browser always uses this row.
pub const EQUIPMENT_SHORT_LABELS: [&str; EQUIPMENT_COUNT] = [
    "Leath Helm",
    "Chain Coif",
    "Iron Helm",
    "Spkd. Helm",
    "Sm. Shield",
    "Lg. Shield",
    "Spkd. Shld",
    "Mag. Shld",
    "Jewel Shld",
    "Cloth",
    "Leather",
    "Ring Mail",
    "Scale",
    "Chain",
    "Plate",
    "Myst. Armr",
    "Dagger",
    "Sling",
    "Club",
    "Flame Oil",
    "Main Gauch",
    "Spear",
    "Thrwng Axe",
    "Sht. Sword",
    "Mace",
    "Morn. Star",
    "Bow",
    "Arrows",
    "Crossbow",
    "Quarrels",
    "Long Sword",
    "2H Hammer",
    "2H Axe",
    "2H Sword",
    "Halberd",
    "Chaos Swrd",
    "Magic Bow",
    "Silver Swd",
    "Magic Axe",
    "Glass Swrd",
    "Jewel Swrd",
    "Myst. Swrd",
    "Inv. Ring",
    "Prot. Ring",
    "Regen Ring",
    "Am/Turning",
    "Sp. Collar",
    "Ankh",
];

/// `inventory.md §3` empty-slot sentinel for the six readied
/// equipment bytes inside a character record.
/// `combat.md §6.1a` Writers #4, the Sword of Chaos compulsion: "if
/// the slot is party-side and its character has item id 35 (Sword of
/// Chaos) readied in either the weapon-hand or shield-hand slot, the
/// engine sets this bit on that party descriptor, clears the
/// active-player sentinel, and runs the turn through the automatic
/// actor driver instead of reading a command from the player. Any
/// other readied equipment takes the ordinary interactive path and
/// never sets the bit."
///
/// The index is anchored to [`EQUIPMENT_NAMES`]; a catalog reshuffle is
/// caught by the accompanying unit test rather than silently changing
/// which weapon compels.
pub const EQUIPMENT_SWORD_OF_CHAOS: usize = 35;

/// `combat.md §6.1a` Writers #4: whether a readied weapon-hand or
/// shield-hand equipment id takes the compulsion branch on the
/// player-driven command path. Only the Sword of Chaos does.
pub const fn equipment_compels_automatic_turn(item_id: usize) -> bool {
    item_id == EQUIPMENT_SWORD_OF_CHAOS
}

pub const EQUIPMENT_EMPTY_SLOT_SENTINEL: u8 = 0xFF;

/// `inventory.md §6`: returns `true` when an R-Ready unequip should
/// increment the shared equipment counter for the cleared item id. A
/// counter already at the `EQUIPMENT_STOCK_CAP` discards the returned
/// copy.
pub const fn r_ready_unequip_returns_stock(current_counter: u8) -> bool {
    current_counter < EQUIPMENT_STOCK_CAP
}

/// `catalogs/item-list.md §5.4`: rings of Invisibility and Regeneration
/// have a 1-in-16 immediate vanish check after a successful R-Ready
/// and another 1-in-16 removal check during the combat round loop.
/// Caller passes the raw `0..16` PRNG roll.
pub const RING_VANISH_DENOMINATOR: u8 = 16;
pub const fn ring_immediately_vanishes(roll: u8) -> bool {
    roll % RING_VANISH_DENOMINATOR == 0
}

/// `catalogs/item-list.md §5.4`: a Ring of Regeneration wearer has a
/// 1-in-8 chance per combat round to recover 1 HP, capped at the
/// member's maximum HP. Caller passes the raw `0..8` PRNG roll.
pub const RING_REGEN_DENOMINATOR: u8 = 8;
pub const fn ring_regenerates(roll: u8) -> bool {
    roll % RING_REGEN_DENOMINATOR == 0
}

/// `inventory.md §3` per-character readied-equipment slot block. The
/// six bytes appear at offsets `+0x19..+0x1E` in the 32-byte
/// character record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipmentSlot {
    Helm,
    BodyArmour,
    WeaponHand,
    OffHand,
    Ring,
    AmuletOrNeck,
}

impl EquipmentSlot {
    /// Six-byte block index `0..=5` matching the spec record offsets
    /// `+0x19..+0x1E`.
    pub const fn block_index(self) -> usize {
        match self {
            EquipmentSlot::Helm => 0,
            EquipmentSlot::BodyArmour => 1,
            EquipmentSlot::WeaponHand => 2,
            EquipmentSlot::OffHand => 3,
            EquipmentSlot::Ring => 4,
            EquipmentSlot::AmuletOrNeck => 5,
        }
    }

    /// `inventory.md §3` absolute byte offset inside the 32-byte
    /// character record. `+0x19` Helm through `+0x1E` Amulet.
    pub const fn record_offset(self) -> usize {
        EQUIPMENT_BLOCK_FIRST_OFFSET + self.block_index()
    }

    /// All six slot variants in record order. Useful for iterators
    /// that need to walk the readied equipment block deterministically.
    pub const fn ordered() -> [Self; EQUIPMENT_BLOCK_LEN] {
        [
            Self::Helm,
            Self::BodyArmour,
            Self::WeaponHand,
            Self::OffHand,
            Self::Ring,
            Self::AmuletOrNeck,
        ]
    }
}

/// `inventory.md §3` first byte of the readied-equipment block in the
/// 32-byte character record (`+0x19`).
pub const EQUIPMENT_BLOCK_FIRST_OFFSET: usize = 0x19;
/// `inventory.md §3` length of the readied-equipment block.
pub const EQUIPMENT_BLOCK_LEN: usize = 6;

/// `catalogs/item-list.md §5` equipment ids for the three ranged
/// weapons whose R-Ready cascade requires a non-zero matching
/// ammunition counter.
pub const ITEM_ID_BOW: u8 = 26;
pub const ITEM_ID_ARROWS: u8 = 27;
pub const ITEM_ID_CROSSBOW: u8 = 28;
pub const ITEM_ID_QUARRELS: u8 = 29;
pub const ITEM_ID_MAGIC_BOW: u8 = 36;

/// `inventory.md §7` U-Use potion family. The eight colour-coded
/// counters dispatch through a party-member target path. Display
/// order is Blue, Yellow, Red, Green, Orange, Purple, Black, White
/// with the spec's per-colour normal effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PotionUseEffect {
    /// Blue (index 0) — Wake. Restores a sleeping member.
    Wake,
    /// Yellow (index 1) — Heal.
    Heal,
    /// Red (index 2) — Cure poison.
    CurePoison,
    /// Green (index 3) — Poison.
    Poison,
    /// Orange (index 4) — Sleep.
    Sleep,
    /// Purple (index 5) — Combat-only "Poof" presentation.
    PoofPresentation,
    /// Black (index 6) — Combat invisibility.
    CombatInvisibility,
    /// White (index 7) — Surface/town visibility-sweep animation.
    VisibilitySweep,
}

/// `inventory.md §7` potion-counter index space (8 entries).
pub const POTION_USE_EFFECT_COUNT: usize = 8;

/// `inventory.md §7`: classify a potion counter index `0..=7` into
/// its dispatched effect. Returns `None` for indices outside that
/// range.
pub const fn potion_use_effect(index: usize) -> Option<PotionUseEffect> {
    Some(match index {
        0 => PotionUseEffect::Wake,
        1 => PotionUseEffect::Heal,
        2 => PotionUseEffect::CurePoison,
        3 => PotionUseEffect::Poison,
        4 => PotionUseEffect::Sleep,
        5 => PotionUseEffect::PoofPresentation,
        6 => PotionUseEffect::CombatInvisibility,
        7 => PotionUseEffect::VisibilitySweep,
        _ => return None,
    })
}

/// `inventory.md §7` potion variation roll denominator. The
/// consumed-potion variation roll gives 1-in-16 chance to force the
/// Orange sleep effect and 1-in-16 to replace the effect with a
/// uniformly random potion row.
pub const POTION_VARIATION_DENOMINATOR: u8 = 16;

/// `catalogs/item-list.md §7.2` completed blocking EGA/Tandy presentation for
/// one accepted potion target. The selected bottle owns this presentation even
/// when the later variation roll substitutes another gameplay effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PotionFlashPlayback {
    pub selected_index: usize,
    pub playfield_left: usize,
    pub playfield_top: usize,
    pub playfield_right: usize,
    pub playfield_bottom: usize,
    pub palette_xor_mask: u8,
    pub envelope_sweep_count: u8,
    pub rumble_accumulator_target: u32,
    pub envelope_sweep_iterations: u32,
}

/// Completed work counts for the sound-disabled potion timing loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PotionFlashTimingWork {
    pub rumble_iterations: u32,
    pub envelope_sweeps: u8,
    pub envelope_iterations: u32,
}

impl PotionFlashPlayback {
    pub const fn blocks_until_complete(self) -> bool {
        true
    }

    pub const fn polls_input(self) -> bool {
        false
    }

    pub const fn advances_gameplay_clock(self) -> bool {
        false
    }

    pub const fn sound_disabled_still_runs_timing(self) -> bool {
        true
    }
}

/// Build the shared flash playback from the selected potion, before effect
/// variation. Returns `None` only for an out-of-range potion id.
pub const fn potion_flash_playback(selected_index: usize) -> Option<PotionFlashPlayback> {
    if selected_index >= POTION_COUNT {
        return None;
    }
    let selected_index_u32 = selected_index as u32;
    Some(PotionFlashPlayback {
        selected_index,
        playfield_left: POTION_FLASH_PLAYFIELD_LEFT,
        playfield_top: POTION_FLASH_PLAYFIELD_TOP,
        playfield_right: POTION_FLASH_PLAYFIELD_RIGHT,
        playfield_bottom: POTION_FLASH_PLAYFIELD_BOTTOM,
        palette_xor_mask: POTION_FLASH_PALETTE_XOR_MASK,
        envelope_sweep_count: POTION_FLASH_ENVELOPE_SWEEP_COUNT,
        rumble_accumulator_target: POTION_FLASH_RUMBLE_TARGET_BASE
            + POTION_FLASH_RUMBLE_TARGET_STEP * selected_index_u32,
        envelope_sweep_iterations: POTION_FLASH_SWEEP_ITERATIONS_BASE
            + POTION_FLASH_SWEEP_ITERATIONS_STEP * selected_index_u32,
    })
}

/// Execute the shared potion timing work when no PC-speaker backend exists.
///
/// `catalogs/item-list.md §7.2` requires the leading rumble loop and both
/// envelope sweeps to run even with sound disabled. Keeping this beside the
/// typed playback contract prevents graphical and headless frontends from
/// silently choosing different blocking boundaries.
pub fn run_potion_flash_soundless_timing(playback: PotionFlashPlayback) -> PotionFlashTimingWork {
    let mut rumble_iterations = 0u32;
    while rumble_iterations < playback.rumble_accumulator_target {
        std::hint::black_box(rumble_iterations);
        rumble_iterations += 1;
    }

    let mut envelope_iterations = 0u32;
    for sweep in 0..playback.envelope_sweep_count {
        for iteration in 0..playback.envelope_sweep_iterations {
            std::hint::black_box((sweep, iteration));
            envelope_iterations += 1;
        }
    }

    PotionFlashTimingWork {
        rumble_iterations,
        envelope_sweeps: playback.envelope_sweep_count,
        envelope_iterations,
    }
}

/// Apply one of the two lossless indexed-colour XOR passes to a full-screen
/// framebuffer. A short surface is rejected rather than silently clipping the
/// normative inclusive playfield rectangle.
pub fn apply_potion_flash_xor_pass(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    playback: PotionFlashPlayback,
) -> bool {
    if width <= playback.playfield_right
        || height <= playback.playfield_bottom
        || pixels.len() < width.saturating_mul(height)
    {
        return false;
    }
    for y in playback.playfield_top..=playback.playfield_bottom {
        for x in playback.playfield_left..=playback.playfield_right {
            let index = y * width + x;
            pixels[index] ^= playback.palette_xor_mask;
        }
    }
    true
}

/// `catalogs/item-list.md §7.1` scroll-grant subtype mask. The
/// scroll-family Search/container grant masks the grant subtype to
/// the low three bits to select one of the eight published scroll
/// labels (`0..=7`). High bits identify the special HMS Cape plans
/// grant variant rather than the displayed scroll label.
pub const SCROLL_GRANT_LABEL_MASK: u8 = 0x07;

/// `catalogs/item-list.md §7.1`: returns the displayed scroll
/// label id (`0..=7`) for a scroll-family grant subtype byte.
pub const fn scroll_grant_label_id(grant_subtype: u8) -> u8 {
    grant_subtype & SCROLL_GRANT_LABEL_MASK
}

/// `catalogs/item-list.md` Spyglass row / `inventory.md §7`: the scene
/// half of the Spyglass gate. The Spyglass admits **the outdoor world
/// scene or a town-class scene**; dungeon-class and combat-class scenes
/// are excluded. This is deliberately broader than
/// [`sextant_outdoor_position`]'s scene test, which admits the outdoor
/// scene only — the two items share a plane test and a night window but
/// not a scene gate.
pub const fn spyglass_scene_admits(scene_byte: u8) -> bool {
    scene_byte == crate::SCENE_OVERWORLD
        || (scene_byte >= crate::SCENE_TOWN_FAMILY_FIRST
            && scene_byte <= crate::SCENE_TOWN_FAMILY_LAST)
}

/// `catalogs/item-list.md` Spyglass row / `inventory.md §7`: the first
/// two of the Spyglass's three conditions — the party is on the surface
/// world plane, and the scene is one [`spyglass_scene_admits`] accepts.
/// The plane test is the same lower-half magnitude test on the
/// world-plane value the Sextant uses, not an equality against the
/// surface value, and **the Underworld is excluded by the same plane
/// condition that excludes it from the Sextant**. Both failures print
/// the same "not here" refusal.
pub const fn spyglass_position_admits(world_plane_byte: u8, scene_byte: u8) -> bool {
    if world_plane_byte >= WORLD_PLANE_SURFACE_MAGNITUDE_LIMIT {
        return false;
    }
    spyglass_scene_admits(scene_byte)
}

/// `catalogs/item-list.md` Spyglass row / `inventory.md §7` Spyglass
/// U-Use eligibility predicate. A look is permitted only when **all
/// three** published conditions hold: the surface world plane, an
/// admitted scene, and a night hour. A scene or plane failure is the
/// "not here" refusal and a daytime hour is the no-stars refusal, so
/// callers that must tell them apart use [`spyglass_position_admits`]
/// and [`sextant_night_hour`] directly.
///
/// *Corrected:* this predicate previously took `(scene_byte,
/// sky_has_stars)`, admitted the overworld scene only, and had **no
/// production caller** — the live `use_spyglass` path did its own
/// gating against the town-lighting night window, which disagrees with
/// the published window at hours `5` and `19`. Both the scene gate and
/// the night window are now published here and the live path calls
/// through, so there is one gate rather than two.
pub const fn spyglass_usable(world_plane_byte: u8, scene_byte: u8, hour: u8) -> bool {
    spyglass_position_admits(world_plane_byte, scene_byte) && sextant_night_hour(hour)
}

/// `inventory.md §7` HMS Cape plans U-Use eligibility predicate.
/// The plans are a shipboard-only utility — usable only when the
/// party is aboard a ship (transport marker family `0x20..=0x27`).
/// On success the caller marks the ship-rigging flag so the ship
/// is rigged for double speed; otherwise the U-Use refuses.
pub const fn hms_cape_plans_usable(transport_marker: u8) -> bool {
    matches!(transport_marker, 0x20..=0x27)
}

/// `catalogs/item-list.md` Sextant row / `inventory.md §7`: the first
/// two of the Sextant's three conditions — the party is on the
/// **surface** world plane and the scene is the outdoor world scene.
///
/// The published plane test is a magnitude comparison: the world-plane
/// value must lie in the lower half of its byte range, rather than an
/// equality against the surface value. The two agree on every reachable
/// state, because on the outdoor scene the plane only ever holds the
/// surface value or the all-ones Underworld value, and the spec names
/// the threshold form the safer contract for an engine that represents
/// the plane numerically.
///
/// The plane test runs **first and short-circuits**, so the Underworld
/// — which *is* the outdoor world scene, only on the other plane — takes
/// the same "outdoors" refusal an indoor scene takes and never reaches
/// the night test. There is no Underworld-specific message.
pub const fn sextant_outdoor_position(world_plane_byte: u8, scene_byte: u8) -> bool {
    if world_plane_byte >= WORLD_PLANE_SURFACE_MAGNITUDE_LIMIT {
        return false;
    }
    // Outdoor world scene byte is zero; any other scene refuses.
    scene_byte == crate::SCENE_OVERWORLD
}

/// `catalogs/item-list.md` Sextant and Spyglass rows: the threshold the
/// published plane test compares the world-plane byte against. The test
/// is a magnitude comparison — the plane value must lie in the lower
/// half of its byte range — so the surface value passes and the
/// all-ones Underworld value does not. Promoted so the two items share
/// one threshold instead of repeating the bare literal.
pub const WORLD_PLANE_SURFACE_MAGNITUDE_LIMIT: u8 = 0x80;

/// `catalogs/item-list.md` / `inventory.md §7`: the night window the
/// Sextant and the Spyglass share — hours `19..=23` and `0..=5`. This is
/// deliberately **not** [`crate::is_town_night_hour`], which is town
/// lighting's own window (`0..=4` and `20..=23`) and disagrees at hours
/// `5` and `19`.
pub const fn sextant_night_hour(hour: u8) -> bool {
    hour <= 5 || hour >= 19
}

/// `catalogs/item-list.md` Sextant row / `inventory.md §7` Sextant U-Use
/// eligibility predicate. A reading is permitted only when **all three**
/// published conditions hold: the surface world plane, the outdoor world
/// scene, and a night hour. The two refusals are distinguishable — a
/// plane or scene failure is the "outdoors" refusal and a daytime hour is
/// the "no stars" refusal — so callers that must tell them apart use
/// [`sextant_outdoor_position`] and [`sextant_night_hour`] directly.
pub const fn sextant_usable(world_plane_byte: u8, scene_byte: u8, hour: u8) -> bool {
    sextant_outdoor_position(world_plane_byte, scene_byte) && sextant_night_hour(hour)
}

/// `inventory.md §7` U-Use scroll family. The handler exposes eight
/// scroll counters dispatching to spell-like effects in this order:
/// Light, Wind Change, Protection, Negate Magic, View, Summon
/// Daemon, Resurrection, Negate Time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollUseEffect {
    /// Index 0 — Light: starts the light counter at value 240.
    Light,
    /// Index 1 — Wind Change.
    WindChange,
    /// Index 2 — Protection: installs the `P` active-effect tag with
    /// duration 100.
    Protection,
    /// Index 3 — Negate Magic: installs the `N` active-effect tag
    /// with duration 20.
    NegateMagic,
    /// Index 4 — View.
    View,
    /// Index 5 — Summon Daemon.
    SummonDaemon,
    /// Index 6 — Resurrection.
    Resurrection,
    /// Index 7 — Negate Time: installs the `T` active-effect tag
    /// with duration 20, except in Stonegate and Doom where it
    /// reports no effect.
    NegateTime,
}

/// `inventory.md §7` U-Use scroll-counter index space (8 entries).
pub const SCROLL_USE_EFFECT_COUNT: usize = 8;

/// `inventory.md §7`: classify a scroll counter index `0..=7` into
/// its dispatched effect. Returns `None` for indices outside that
/// range.
pub const fn scroll_use_effect(index: usize) -> Option<ScrollUseEffect> {
    Some(match index {
        0 => ScrollUseEffect::Light,
        1 => ScrollUseEffect::WindChange,
        2 => ScrollUseEffect::Protection,
        3 => ScrollUseEffect::NegateMagic,
        4 => ScrollUseEffect::View,
        5 => ScrollUseEffect::SummonDaemon,
        6 => ScrollUseEffect::Resurrection,
        7 => ScrollUseEffect::NegateTime,
        _ => return None,
    })
}

/// `inventory.md §6` R-Ready ranged-weapon ammo precondition.
/// Returns the equipment-stock item id that must have a non-zero
/// counter before the weapon can be readied: arrows (27) for Bow
/// and Magic Bow, quarrels (29) for Crossbow. Returns `None` for
/// any other item id (no ammo gate applies).
pub const fn ranged_weapon_required_ammo(item_id: u8) -> Option<u8> {
    match item_id {
        ITEM_ID_BOW | ITEM_ID_MAGIC_BOW => Some(ITEM_ID_ARROWS),
        ITEM_ID_CROSSBOW => Some(ITEM_ID_QUARRELS),
        _ => None,
    }
}

/// `inventory.md §3`: ownership predicate used by inventory browsing.
/// Returns `true` when any of the six readied-equipment bytes equals
/// the supplied item id (and the id is not the empty-slot sentinel).
pub fn character_has_readied(equipment_block: &[u8; 6], item_id: u8) -> bool {
    if item_id == EQUIPMENT_EMPTY_SLOT_SENTINEL {
        return false;
    }
    equipment_block.iter().any(|&slot| slot == item_id)
}

/// `inventory.md §3.1` published equipment-class tag bytes used by
/// R-Ready slot routing and refusal logic.
pub const EQUIPMENT_CLASS_HELM: u8 = 0x80;
pub const EQUIPMENT_CLASS_BODY_ARMOUR: u8 = 0x40;
pub const EQUIPMENT_CLASS_ONE_HAND: u8 = 0x20;
pub const EQUIPMENT_CLASS_TWO_HAND: u8 = 0x30;
pub const EQUIPMENT_CLASS_RING: u8 = 0x02;
pub const EQUIPMENT_CLASS_AMULET: u8 = 0x04;
pub const EQUIPMENT_CLASS_NONE: u8 = 0x00;

/// `inventory.md §3.1` typed equipment-class tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipmentClassTag {
    Helm,
    BodyArmour,
    OneHand,
    TwoHand,
    Ring,
    Amulet,
    /// Ammunition rows have no readied-equipment tag (`0x00`).
    None,
}

/// `inventory.md §3.1`: classify an equipment-class tag byte. Returns
/// `None` for any unknown bit pattern.
pub const fn equipment_class_tag(tag: u8) -> Option<EquipmentClassTag> {
    Some(match tag {
        EQUIPMENT_CLASS_HELM => EquipmentClassTag::Helm,
        EQUIPMENT_CLASS_BODY_ARMOUR => EquipmentClassTag::BodyArmour,
        EQUIPMENT_CLASS_ONE_HAND => EquipmentClassTag::OneHand,
        EQUIPMENT_CLASS_TWO_HAND => EquipmentClassTag::TwoHand,
        EQUIPMENT_CLASS_RING => EquipmentClassTag::Ring,
        EQUIPMENT_CLASS_AMULET => EquipmentClassTag::Amulet,
        EQUIPMENT_CLASS_NONE => EquipmentClassTag::None,
        _ => return None,
    })
}

/// `inventory.md §3.1` R-Ready slot family selected by an equipment
/// class tag. Helm, BodyArmour, Ring, and Amulet tags route directly
/// to their named slot. The OneHand and TwoHand tags share the
/// generic Hand family — the R-Ready hand branch later resolves
/// which of WeaponHand and OffHand the item ends up in (one-handed
/// can land in either; two-handed occupies both). The None tag is
/// ammunition stock and does not use a readied slot at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipmentSlotFamily {
    Helm,
    BodyArmour,
    Hand,
    Ring,
    Amulet,
    /// Ammunition rows have no readied-equipment slot.
    None,
}

impl EquipmentClassTag {
    /// `inventory.md §3.1`: R-Ready slot family this class tag
    /// routes to. OneHand/TwoHand both map to the generic Hand
    /// family; ammunition (class tag `0x00`) maps to None.
    pub const fn slot_family(self) -> EquipmentSlotFamily {
        match self {
            Self::Helm => EquipmentSlotFamily::Helm,
            Self::BodyArmour => EquipmentSlotFamily::BodyArmour,
            Self::OneHand | Self::TwoHand => EquipmentSlotFamily::Hand,
            Self::Ring => EquipmentSlotFamily::Ring,
            Self::Amulet => EquipmentSlotFamily::Amulet,
            Self::None => EquipmentSlotFamily::None,
        }
    }
}

pub const EQUIPMENT_CLASS_TAGS: [u8; EQUIPMENT_COUNT] = [
    0x80, 0x80, 0x80, 0x80, 0x20, 0x20, 0x20, 0x20, 0x20, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40,
    0x20, 0x30, 0x20, 0x30, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x30, 0x00, 0x30, 0x00, 0x20, 0x30,
    0x30, 0x30, 0x30, 0x30, 0x30, 0x20, 0x20, 0x20, 0x20, 0x30, 0x02, 0x02, 0x02, 0x04, 0x04, 0x04,
];

pub const EQUIPMENT_READY_BURDENS: [u8; EQUIPMENT_COUNT] = [
    0, 1, 2, 3, 2, 3, 4, 0, 0, 0, 2, 4, 6, 10, 12, 0, 1, 2, 3, 2, 3, 4, 6, 5, 7, 8, 8, 0, 6, 0, 9,
    16, 15, 13, 18, 0, 0, 8, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// `catalogs/item-list.md §5.1` per-equipment "equipped weight stat"
/// lookup. This is a separate resident table from
/// [`EQUIPMENT_READY_BURDENS`] and is summed by the equipped-item
/// statistic helper; the R-Ready strength gate does not consult it.
pub const EQUIPMENT_EQUIPPED_WEIGHTS: [u8; EQUIPMENT_COUNT] = [
    1, 2, 3, 3, 2, 3, 3, 5, 0, 1, 2, 3, 4, 5, 7, 10, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0, 0, 2, 0,
];

/// `inventory.md §2.1` Protection active-effect equipped-weight bonus.
/// The traced equipped-item statistic helper adds 3 to the summed
/// six-slot table when the shared active-effect/status tag is the
/// Protection tag.
pub const EQUIPPED_WEIGHT_PROTECTION_BONUS: u8 = 3;

/// `inventory.md §2.1` / `catalogs/item-list.md §5.1`: sum the
/// per-equipment "equipped weight stat" across the six readied
/// slots, treating empty slots as zero and `EQUIPMENT_EMPTY`
/// sentinels as no contribution. Adds the Protection bonus when
/// `protection_active` is set.
pub fn equipped_item_weight_stat(
    equipment: &[u8; EQUIPMENT_SLOT_COUNT],
    protection_active: bool,
) -> u8 {
    let total = equipment
        .iter()
        .copied()
        .filter(|item| *item != EQUIPMENT_EMPTY)
        .filter_map(|item| EQUIPMENT_EQUIPPED_WEIGHTS.get(item as usize).copied())
        .fold(0u8, u8::saturating_add);
    if protection_active {
        total.saturating_add(EQUIPPED_WEIGHT_PROTECTION_BONUS)
    } else {
        total
    }
}

pub const EQUIPMENT_ATTACK_MAXES: [u8; EQUIPMENT_COUNT] = [
    0, 0, 0, 4, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 6, 8, 8, 8, 10, 10, 12, 15, 15, 10, 1, 12,
    1, 15, 20, 20, 20, 30, 99, 15, 12, 20, 99, 1, 30, 0, 0, 0, 0, 0, 0,
];

pub const EQUIPMENT_BASE_PRICES: [u16; EQUIPMENT_COUNT] = [
    15, 50, 120, 150, 40, 70, 120, 2000, 0, 20, 50, 100, 150, 300, 700, 0, 1, 10, 5, 5, 15, 7, 3,
    40, 50, 60, 75, 10, 150, 15, 70, 85, 150, 200, 250, 0, 800, 250, 1000, 0, 0, 0, 450, 500, 200,
    900, 240, 0,
];

/// `catalogs/item-list.md §5.3` arena-wide weapon range cap. A
/// non-adjacent range cap of this value reaches every cell on the
/// eleven-by-eleven combat arena; Magic Bow and Magic Axe use it.
/// (`sqrt(10*10 + 10*10) ≈ 14.14`, truncated to 14, so 15 covers the
/// full diagonal with one extra-cell margin.)
pub const WEAPON_RANGE_ARENA_WIDE_CAP: u8 = 15;

pub const EQUIPMENT_WEAPON_RANGE_CAPS: [u8; EQUIPMENT_COUNT] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 4, 0, 4, 0, 5, 4, 0, 0, 2, 7, 0, 8, 0, 0, 0,
    0, 0, 2, 0, 15, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub const EQUIPMENT_WEAPON_EFFECT_CODES: [u8; EQUIPMENT_COUNT] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 3, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub fn equipment_name(item_id: usize) -> &'static str {
    EQUIPMENT_NAMES
        .get(item_id)
        .copied()
        .unwrap_or("Unknown equipment")
}

pub fn equipment_attack_max(item_id: usize) -> Option<u8> {
    EQUIPMENT_ATTACK_MAXES.get(item_id).copied()
}

pub fn equipment_base_price(item_id: usize) -> Option<u16> {
    EQUIPMENT_BASE_PRICES.get(item_id).copied()
}

pub fn equipment_weapon_range_cap(item_id: usize) -> Option<u8> {
    EQUIPMENT_WEAPON_RANGE_CAPS.get(item_id).copied()
}

pub fn equipment_weapon_effect_code(item_id: usize) -> Option<u8> {
    EQUIPMENT_WEAPON_EFFECT_CODES.get(item_id).copied()
}

pub fn default_party_strengths(party_len: usize) -> Vec<u8> {
    vec![AVATAR_STAT_MAX; party_len]
}

pub fn default_party_equipment(party_len: usize) -> Vec<[u8; EQUIPMENT_SLOT_COUNT]> {
    vec![[EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]; party_len]
}

pub fn equipment_stock_summary(stock: &[u8; EQUIPMENT_COUNT]) -> String {
    let entries = stock
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(item_id, count)| format!("{}={count}", equipment_name(item_id)))
        .collect::<Vec<_>>();
    if entries.is_empty() {
        "none".to_string()
    } else {
        entries.join(", ")
    }
}

pub fn readied_equipment_summary(equipment: &[u8; EQUIPMENT_SLOT_COUNT]) -> String {
    let entries = equipment
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(slot, item)| {
            (item != EQUIPMENT_EMPTY)
                .then(|| format!("{}={}", slot_name(slot), equipment_name(item as usize)))
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        "none".to_string()
    } else {
        entries.join(", ")
    }
}

/// `combat.md §8.1` / `§8.2`: the readied slots the turn banner names and
/// the Attack walker scans, in the published order - "helm, weapon hand,
/// shield hand". Body armour is never scanned.
pub const COMBAT_ARMAMENT_SCAN_SLOTS: [usize; 3] =
    [EQUIP_SLOT_HELM, EQUIP_SLOT_WEAPON, EQUIP_SLOT_OFFHAND];

/// `combat.md §8.1`: "only items whose per-item weapon-capability entry is
/// non-zero are named. Ordinary helms, ordinary shields and all body armour
/// therefore never appear in the clause - while the **spiked helm and
/// spiked shield do**, because they carry a non-zero capability entry."
///
/// The per-item weapon-capability entry is [`EQUIPMENT_ATTACK_MAXES`];
/// its Spiked Helm and Spiked Shield rows are non-zero while every other
/// helm, shield and body-armour row is zero, which is exactly the split
/// §8.1 describes.
pub fn equipment_has_weapon_capability(item_id: usize) -> bool {
    equipment_attack_max(item_id).is_some_and(|capability| capability != 0)
}

/// `combat.md §8.1` / `§8.2`: the qualifying readied items for one
/// character, in helm / weapon-hand / shield-hand order. An empty result
/// is the bare-handed case - one attack attempt with range one for §8.2,
/// and the `bare hands` clause for the §8.1 banner.
pub fn combat_armament_item_ids(equipment: &[u8; EQUIPMENT_SLOT_COUNT]) -> Vec<usize> {
    COMBAT_ARMAMENT_SCAN_SLOTS
        .into_iter()
        .filter_map(|slot| equipment.get(slot).copied())
        .filter(|item| *item != EQUIPMENT_EMPTY)
        .map(usize::from)
        .filter(|item| equipment_has_weapon_capability(*item))
        .collect()
}

/// `combat.md §8.1`: the banner opens with "a newline".
pub const COMBAT_TURN_BANNER_LEAD_NEWLINE: &str = "\n";
/// `combat.md §8.1` armament clause introducer.
pub const COMBAT_TURN_BANNER_ARMED_WITH: &str = ", armed with ";
/// `combat.md §8.1` separator between named readied items.
pub const COMBAT_TURN_BANNER_ITEM_SEPARATOR: &str = ", ";
/// `combat.md §8.1` stand-in printed "when none qualifies".
pub const COMBAT_TURN_BANNER_BARE_HANDS: &str = "bare hands";
/// `combat.md §8.1`: the banner is "terminated by a colon".
pub const COMBAT_TURN_BANNER_TERMINATOR: &str = ":";

/// `combat.md §8.1`, the turn banner: "a newline, the actor's name, and -
/// for a party-side actor - the clause `, armed with ` followed by the
/// names of that actor's readied items separated by `, `, or `bare hands`
/// when none qualifies, terminated by a colon."
///
/// `equipment` is `None` for the other keyboard-driven case the section
/// names: "A charmed monster acting under player control gets only its
/// name and the colon, with no armament clause."
pub fn combat_turn_banner(name: &str, equipment: Option<&[u8; EQUIPMENT_SLOT_COUNT]>) -> String {
    let mut banner = String::from(COMBAT_TURN_BANNER_LEAD_NEWLINE);
    banner.push_str(name);
    if let Some(equipment) = equipment {
        banner.push_str(COMBAT_TURN_BANNER_ARMED_WITH);
        let items = combat_armament_item_ids(equipment);
        if items.is_empty() {
            banner.push_str(COMBAT_TURN_BANNER_BARE_HANDS);
        } else {
            let names = items
                .into_iter()
                .map(equipment_name)
                .collect::<Vec<_>>()
                .join(COMBAT_TURN_BANNER_ITEM_SEPARATOR);
            banner.push_str(&names);
        }
    }
    banner.push_str(COMBAT_TURN_BANNER_TERMINATOR);
    banner
}

pub fn slot_name(slot: usize) -> &'static str {
    match slot {
        EQUIP_SLOT_HELM => "helm",
        EQUIP_SLOT_ARMOUR => "armour",
        EQUIP_SLOT_WEAPON => "weapon",
        EQUIP_SLOT_OFFHAND => "offhand",
        EQUIP_SLOT_RING => "ring",
        EQUIP_SLOT_AMULET => "amulet",
        _ => "slot",
    }
}

pub fn ready_burden(equipment: &[u8; EQUIPMENT_SLOT_COUNT]) -> u8 {
    equipment
        .iter()
        .copied()
        .filter(|item| *item != EQUIPMENT_EMPTY)
        .filter_map(|item| EQUIPMENT_READY_BURDENS.get(item as usize).copied())
        .fold(0u8, u8::saturating_add)
}

/// `inventory.md §2.1` R-Ready strength gate. The command sums the
/// member's existing readied-equipment burden, adds the candidate
/// item's R-Ready burden, and compares the saturated total against
/// the member's Strength byte. The ready is accepted only when the
/// total is at most the Strength byte; a strictly greater total
/// triggers the "not strong enough" refusal and makes no equipment
/// change.
pub const fn r_ready_burden_gate_accepts(
    current_burden: u8,
    candidate_burden: u8,
    member_strength: u8,
) -> bool {
    current_burden.saturating_add(candidate_burden) <= member_strength
}

pub fn is_amulet_turning_readied(equipment: &[u8; EQUIPMENT_SLOT_COUNT]) -> bool {
    equipment[EQUIP_SLOT_AMULET] == EQUIPMENT_ID_AMULET_TURNING as u8
}

pub fn is_shield_item(item_id: usize) -> bool {
    (4..=8).contains(&item_id)
}

pub fn is_magic_vanish_ring(item_id: usize) -> bool {
    matches!(
        item_id,
        EQUIPMENT_ID_RING_INVISIBILITY | EQUIPMENT_ID_RING_REGENERATION
    )
}

#[cfg(test)]
mod sword_of_chaos_tests {
    use super::*;

    #[test]
    fn sword_of_chaos_index_is_anchored_to_the_catalog_name() {
        // `combat.md §6.1a` Writers #4 names the compelling weapon by
        // catalog id: "item id 35 (Sword of Chaos)". Assert the name so
        // a catalog reshuffle cannot silently move the compulsion onto
        // a different weapon.
        assert_eq!(EQUIPMENT_SWORD_OF_CHAOS, 35);
        assert_eq!(EQUIPMENT_NAMES[EQUIPMENT_SWORD_OF_CHAOS], "Sword of Chaos");
        assert_eq!(
            EQUIPMENT_SHORT_LABELS[EQUIPMENT_SWORD_OF_CHAOS],
            "Chaos Swrd"
        );
    }

    #[test]
    fn only_the_sword_of_chaos_compels_an_automatic_turn() {
        // §6.1a: "Any other readied equipment takes the ordinary
        // interactive path and never sets the bit."
        assert!(equipment_compels_automatic_turn(EQUIPMENT_SWORD_OF_CHAOS));
        for item_id in 0..EQUIPMENT_COUNT {
            assert_eq!(
                equipment_compels_automatic_turn(item_id),
                item_id == EQUIPMENT_SWORD_OF_CHAOS,
                "{}",
                EQUIPMENT_NAMES[item_id]
            );
        }
    }
}
