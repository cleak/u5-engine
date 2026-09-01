//! Sanitized aggregate audits for `.OOL` active-object overlay tables.
//!
//! Reports contain counts, hashes, and mirror-shape checks only. They do not
//! emit raw records, slot inventories, coordinates, or copied asset bytes.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub const OOL_AUDIT_SOURCE_COUNT: usize = 6;
pub const OOL_AUDIT_SLOT_ROLE_COUNT: usize = 3;
pub const OOL_AUDIT_SLOT_CLASS_COUNT: usize = 7;
pub const OOL_AUDIT_PHASE_CLASS_COUNT: usize = 3;
pub const OOL_AUDIT_TILE_CLASS_COUNT: usize = 13;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OolAuditSource {
    SavedGamLiveTable,
    SavedOolBritannia,
    SavedOolUnderworld,
    BritOol,
    UnderOol,
    InitOol,
}

impl OolAuditSource {
    pub const fn index(self) -> usize {
        match self {
            Self::SavedGamLiveTable => 0,
            Self::SavedOolBritannia => 1,
            Self::SavedOolUnderworld => 2,
            Self::BritOol => 3,
            Self::UnderOol => 4,
            Self::InitOol => 5,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::SavedGamLiveTable => "saved-gam-live",
            Self::SavedOolBritannia => "saved-ool-britannia",
            Self::SavedOolUnderworld => "saved-ool-underworld",
            Self::BritOol => "brit-ool",
            Self::UnderOol => "under-ool",
            Self::InitOol => "init-ool",
        }
    }

    pub const ALL: [Self; OOL_AUDIT_SOURCE_COUNT] = [
        Self::SavedGamLiveTable,
        Self::SavedOolBritannia,
        Self::SavedOolUnderworld,
        Self::BritOol,
        Self::UnderOol,
        Self::InitOol,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OolSlotClass {
    Empty,
    Scenery,
    DoorFixture,
    NpcOrVehicleProtectedBand,
    MidrangeObject,
    DynamicActor,
    ProtectedType,
}

impl OolSlotClass {
    pub const fn index(self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Scenery => 1,
            Self::DoorFixture => 2,
            Self::NpcOrVehicleProtectedBand => 3,
            Self::MidrangeObject => 4,
            Self::DynamicActor => 5,
            Self::ProtectedType => 6,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Scenery => "scenery",
            Self::DoorFixture => "door-fixture",
            Self::NpcOrVehicleProtectedBand => "npc-vehicle-protected-band",
            Self::MidrangeObject => "midrange-object",
            Self::DynamicActor => "dynamic-actor",
            Self::ProtectedType => "protected-type",
        }
    }

    pub const ALL: [Self; OOL_AUDIT_SLOT_CLASS_COUNT] = [
        Self::Empty,
        Self::Scenery,
        Self::DoorFixture,
        Self::NpcOrVehicleProtectedBand,
        Self::MidrangeObject,
        Self::DynamicActor,
        Self::ProtectedType,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OolPhaseClass {
    Steady,
    Decrementing,
    AiEligible,
}

impl OolPhaseClass {
    pub const fn index(self) -> usize {
        match self {
            Self::Steady => 0,
            Self::Decrementing => 1,
            Self::AiEligible => 2,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Steady => "steady",
            Self::Decrementing => "decrementing",
            Self::AiEligible => "ai-eligible",
        }
    }

    pub const ALL: [Self; OOL_AUDIT_PHASE_CLASS_COUNT] =
        [Self::Steady, Self::Decrementing, Self::AiEligible];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OolPlaneAudit {
    pub source: OolAuditSource,
    pub raw_hash: u64,
    pub total_slots: usize,
    pub non_empty_slots: usize,
    pub empty_zeroed_slots: usize,
    pub empty_payload_slots: usize,
    pub z_sentinel_slots: usize,
    pub type_equals_tile_slots: usize,
    pub high_direction_nibble_slots: usize,
    pub role_counts: [usize; OOL_AUDIT_SLOT_ROLE_COUNT],
    pub class_counts: [usize; OOL_AUDIT_SLOT_CLASS_COUNT],
    pub phase_counts: [usize; OOL_AUDIT_PHASE_CLASS_COUNT],
    pub tile_class_counts: [usize; OOL_AUDIT_TILE_CLASS_COUNT],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OolAuditReport {
    pub planes: Vec<OolPlaneAudit>,
    pub total_slots: usize,
    pub non_empty_slots: usize,
    pub empty_payload_slots: usize,
    pub content_hash: u64,
    pub saved_britannia_matches_mirror: bool,
    pub saved_underworld_matches_mirror: bool,
    pub init_matches_britannia_seed: bool,
    pub source_counts: [usize; OOL_AUDIT_SOURCE_COUNT],
    pub class_counts: [usize; OOL_AUDIT_SLOT_CLASS_COUNT],
}

pub const fn ool_slot_class(type_byte: u8) -> OolSlotClass {
    match type_byte {
        0x00 => OolSlotClass::Empty,
        ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE => OolSlotClass::ProtectedType,
        ACTIVE_OBJECT_EVICTION_SCENERY_FIRST..=ACTIVE_OBJECT_EVICTION_SCENERY_LAST => {
            OolSlotClass::Scenery
        }
        ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_FIRST..=ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_LAST => {
            OolSlotClass::DoorFixture
        }
        0x12..=0x2F => OolSlotClass::NpcOrVehicleProtectedBand,
        ACTIVE_OBJECT_EVICTION_MIDRANGE_FIRST..=ACTIVE_OBJECT_EVICTION_MIDRANGE_LAST => {
            OolSlotClass::MidrangeObject
        }
        ACTIVE_OBJECT_EVICTION_DYNAMIC_FIRST..=0xFF => OolSlotClass::DynamicActor,
    }
}

pub fn audit_ool_files(game_dir: &Path) -> io::Result<OolAuditReport> {
    let saved_gam = read_exact_file(game_dir, SAVED_GAM_FILENAME, SAVED_GAM_LEN)?;
    let saved_ool = read_exact_file(game_dir, SAVED_OOL_FILENAME, SAVED_OOL_LEN)?;
    let brit_ool = read_exact_file(game_dir, BRIT_OOL_FILENAME, OOL_PLANE_LEN)?;
    let under_ool = read_exact_file(game_dir, UNDER_OOL_FILENAME, OOL_PLANE_LEN)?;
    let init_ool = read_exact_file(game_dir, INIT_OOL_FILENAME, OOL_PLANE_LEN)?;

    let saved_live =
        &saved_gam[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN];
    let saved_britannia = &saved_ool[..OOL_PLANE_LEN];
    let saved_underworld = &saved_ool[OOL_PLANE_LEN..];

    let mut report = OolAuditReport {
        planes: Vec::new(),
        total_slots: 0,
        non_empty_slots: 0,
        empty_payload_slots: 0,
        content_hash: 0xcbf29ce484222325,
        saved_britannia_matches_mirror: saved_britannia == brit_ool.as_slice(),
        saved_underworld_matches_mirror: saved_underworld == under_ool.as_slice(),
        init_matches_britannia_seed: init_ool == brit_ool,
        source_counts: [0; OOL_AUDIT_SOURCE_COUNT],
        class_counts: [0; OOL_AUDIT_SLOT_CLASS_COUNT],
    };

    for (source, bytes) in [
        (OolAuditSource::SavedGamLiveTable, saved_live),
        (OolAuditSource::SavedOolBritannia, saved_britannia),
        (OolAuditSource::SavedOolUnderworld, saved_underworld),
        (OolAuditSource::BritOol, brit_ool.as_slice()),
        (OolAuditSource::UnderOol, under_ool.as_slice()),
        (OolAuditSource::InitOol, init_ool.as_slice()),
    ] {
        let plane = audit_ool_plane_bytes(source, bytes)?;
        merge_ool_plane_audit(&mut report, &plane);
        report.planes.push(plane);
    }

    Ok(report)
}

pub fn audit_ool_plane_bytes(source: OolAuditSource, bytes: &[u8]) -> io::Result<OolPlaneAudit> {
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} OOL plane must be {OOL_PLANE_LEN} bytes, got {}",
                source.label(),
                bytes.len()
            ),
        ));
    }

    let mut audit = OolPlaneAudit {
        source,
        raw_hash: hash_bytes(bytes),
        total_slots: OOL_SLOTS,
        non_empty_slots: 0,
        empty_zeroed_slots: 0,
        empty_payload_slots: 0,
        z_sentinel_slots: 0,
        type_equals_tile_slots: 0,
        high_direction_nibble_slots: 0,
        role_counts: [0; OOL_AUDIT_SLOT_ROLE_COUNT],
        class_counts: [0; OOL_AUDIT_SLOT_CLASS_COUNT],
        phase_counts: [0; OOL_AUDIT_PHASE_CLASS_COUNT],
        tile_class_counts: [0; OOL_AUDIT_TILE_CLASS_COUNT],
    };

    for (slot, record) in bytes.chunks_exact(OOL_RECORD_LEN).enumerate() {
        let role = active_object_slot_role(slot).expect("slot from fixed table");
        audit.role_counts[ool_role_index(role)] += 1;

        let type_byte = record[ACTIVE_OBJECT_FIELD_TYPE];
        let slot_class = ool_slot_class(type_byte);
        audit.class_counts[slot_class.index()] += 1;
        audit.tile_class_counts[tile_class_index(coarse_tile_class(type_byte))] += 1;

        if type_byte == 0 {
            if record.iter().all(|byte| *byte == 0) {
                audit.empty_zeroed_slots += 1;
            } else {
                audit.empty_payload_slots += 1;
            }
        } else {
            audit.non_empty_slots += 1;
            if record[ACTIVE_OBJECT_FIELD_Z] == OOL_NO_Z_SENTINEL {
                audit.z_sentinel_slots += 1;
            }
            if type_byte == record[ACTIVE_OBJECT_FIELD_TILE] {
                audit.type_equals_tile_slots += 1;
            }
            if active_object_direction_step(record[ACTIVE_OBJECT_FIELD_PHASE]) != 0 {
                audit.high_direction_nibble_slots += 1;
            }
        }

        let phase = match animation_phase_step(record[ACTIVE_OBJECT_FIELD_PHASE]) {
            AnimationPhaseStep::Steady => OolPhaseClass::Steady,
            AnimationPhaseStep::Decrement(_) => OolPhaseClass::Decrementing,
            AnimationPhaseStep::AiEligible => OolPhaseClass::AiEligible,
        };
        audit.phase_counts[phase.index()] += 1;
    }

    Ok(audit)
}

pub fn ool_audit_report_text(report: &OolAuditReport) -> String {
    let mut text = String::new();
    text.push_str("Ultima V OOL active-object audit\n");
    text.push_str(&format!(
        "planes={} slots={} non_empty={} empty_payload={} hash={:016x}\n",
        report.planes.len(),
        report.total_slots,
        report.non_empty_slots,
        report.empty_payload_slots,
        report.content_hash
    ));
    text.push_str(&format!(
        "mirror_checks saved_britannia={} saved_underworld={} init_britannia={}\n",
        report.saved_britannia_matches_mirror,
        report.saved_underworld_matches_mirror,
        report.init_matches_britannia_seed
    ));
    text.push_str("sources:");
    for source in OolAuditSource::ALL {
        let count = report.source_counts[source.index()];
        if count > 0 {
            text.push_str(&format!(" {}={count}", source.label()));
        }
    }
    text.push('\n');
    text.push_str("slot_classes:");
    for class in OolSlotClass::ALL {
        let count = report.class_counts[class.index()];
        if count > 0 {
            text.push_str(&format!(" {}={count}", class.label()));
        }
    }
    text.push('\n');
    text
}

fn read_exact_file(game_dir: &Path, name: &str, expected_len: usize) -> io::Result<Vec<u8>> {
    let path = game_dir.join(name);
    let bytes = fs::read(&path).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("{}: failed to read OOL audit input: {err}", path.display()),
        )
    })?;
    if bytes.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} must be {expected_len} bytes, got {}",
                path.display(),
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}

fn merge_ool_plane_audit(report: &mut OolAuditReport, plane: &OolPlaneAudit) {
    report.total_slots += plane.total_slots;
    report.non_empty_slots += plane.non_empty_slots;
    report.empty_payload_slots += plane.empty_payload_slots;
    report.content_hash ^= plane.raw_hash;
    report.content_hash = report.content_hash.wrapping_mul(0x100000001b3);
    report.source_counts[plane.source.index()] += 1;
    for index in 0..OOL_AUDIT_SLOT_CLASS_COUNT {
        report.class_counts[index] += plane.class_counts[index];
    }
}

fn ool_role_index(role: ActiveObjectSlotRole) -> usize {
    match role {
        ActiveObjectSlotRole::Player => 0,
        ActiveObjectSlotRole::OrdinaryAcquisition => 1,
        ActiveObjectSlotRole::Reserved => 2,
    }
}

fn tile_class_index(class: TileClass) -> usize {
    match class {
        TileClass::Sentinel => 0,
        TileClass::Water => 1,
        TileClass::Terrain => 2,
        TileClass::Path => 3,
        TileClass::Wall => 4,
        TileClass::Furniture => 5,
        TileClass::River => 6,
        TileClass::Decoration => 7,
        TileClass::Barrier => 8,
        TileClass::Special => 9,
        TileClass::Vehicle => 10,
        TileClass::VehicleArt => 11,
        TileClass::Npc => 12,
    }
}
