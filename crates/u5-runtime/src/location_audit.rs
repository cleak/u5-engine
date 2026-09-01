//! Sanitized audits for authored town-family `LOCATION.DAT` cells.
//!
//! The audit reports aggregate ownership counts, hashes, and anomaly totals.
//! It does not emit raw 32x32 map rows or generated map inventories.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub const LOCATION_AUDIT_OWNER_COUNT: usize = 17;
pub const LOCATION_AUDIT_TILE_CLASS_COUNT: usize = 13;
pub const LOCATION_AUDIT_VIEW_CLASS_COUNT: usize = 17;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationCellOwner {
    StaticTerrain,
    NpcStartMarker,
    BeaconLightSource,
    StandingCropTerrain,
    FruitTreeTerrain,
    FloorLinkMarker,
    DawnDuskGateMarker,
    TelescopeLook,
    WalkOnStair,
    ClimbTransition,
    Door,
    LooseBrickTrapdoor,
    PoisonGas,
    Pushable,
    SearchInspectable,
    Animated,
    ActorOrNpcArt,
}

impl LocationCellOwner {
    pub const fn index(self) -> usize {
        match self {
            Self::StaticTerrain => 0,
            Self::NpcStartMarker => 1,
            Self::BeaconLightSource => 2,
            Self::StandingCropTerrain => 3,
            Self::FruitTreeTerrain => 4,
            Self::FloorLinkMarker => 5,
            Self::DawnDuskGateMarker => 6,
            Self::TelescopeLook => 7,
            Self::WalkOnStair => 8,
            Self::ClimbTransition => 9,
            Self::Door => 10,
            Self::LooseBrickTrapdoor => 11,
            Self::PoisonGas => 12,
            Self::Pushable => 13,
            Self::SearchInspectable => 14,
            Self::Animated => 15,
            Self::ActorOrNpcArt => 16,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::StaticTerrain => "static-terrain",
            Self::NpcStartMarker => "npc-start-marker",
            Self::BeaconLightSource => "beacon-light-source",
            Self::StandingCropTerrain => "standing-crop-terrain",
            Self::FruitTreeTerrain => "fruit-tree-terrain",
            Self::FloorLinkMarker => "npc-floor-link-marker",
            Self::DawnDuskGateMarker => "dawn-dusk-gate-marker",
            Self::TelescopeLook => "telescope-look",
            Self::WalkOnStair => "walk-on-stair",
            Self::ClimbTransition => "climb-transition",
            Self::Door => "door",
            Self::LooseBrickTrapdoor => "loose-brick-trapdoor",
            Self::PoisonGas => "poison-gas",
            Self::Pushable => "pushable",
            Self::SearchInspectable => "search-inspectable",
            Self::Animated => "animated",
            Self::ActorOrNpcArt => "actor-or-npc-art",
        }
    }

    pub const ALL: [Self; LOCATION_AUDIT_OWNER_COUNT] = [
        Self::StaticTerrain,
        Self::NpcStartMarker,
        Self::BeaconLightSource,
        Self::StandingCropTerrain,
        Self::FruitTreeTerrain,
        Self::FloorLinkMarker,
        Self::DawnDuskGateMarker,
        Self::TelescopeLook,
        Self::WalkOnStair,
        Self::ClimbTransition,
        Self::Door,
        Self::LooseBrickTrapdoor,
        Self::PoisonGas,
        Self::Pushable,
        Self::SearchInspectable,
        Self::Animated,
        Self::ActorOrNpcArt,
    ];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationFloorAudit {
    pub scene: Option<Scene>,
    pub family: Family,
    pub physical_page: usize,
    pub logical_floor: Option<i8>,
    pub raw_hash: u64,
    pub runtime_day_hash: u64,
    pub runtime_night_hash: u64,
    pub owner_counts: [usize; LOCATION_AUDIT_OWNER_COUNT],
    pub tile_class_counts: [usize; LOCATION_AUDIT_TILE_CLASS_COUNT],
    pub view_class_counts: [usize; LOCATION_AUDIT_VIEW_CLASS_COUNT],
    pub npc_path_open_count: usize,
    pub foot_walkable_count: usize,
    pub dawn_dusk_marker_count: usize,
    pub dawn_dusk_paired_count: usize,
    pub dawn_dusk_bottom_row_count: usize,
    pub dawn_dusk_unexpected_pair_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationAuditReport {
    pub physical_pages: Vec<LocationFloorAudit>,
    pub logical_floors: Vec<LocationFloorAudit>,
    pub total_cells: usize,
    pub content_hash: u64,
    pub owner_counts: [usize; LOCATION_AUDIT_OWNER_COUNT],
    pub tile_class_counts: [usize; LOCATION_AUDIT_TILE_CLASS_COUNT],
    pub view_class_counts: [usize; LOCATION_AUDIT_VIEW_CLASS_COUNT],
    pub npc_path_open_count: usize,
    pub foot_walkable_count: usize,
    pub dawn_dusk_bottom_row_count: usize,
    pub dawn_dusk_unexpected_pair_count: usize,
}

pub fn classify_location_cell_owner(tile: u8) -> LocationCellOwner {
    if tile == TOWN_TILE_STANDING_CROP {
        return LocationCellOwner::StandingCropTerrain;
    }
    if tile == TOWN_TILE_FRUIT_TREE {
        return LocationCellOwner::FruitTreeTerrain;
    }
    match town_tile_marker(tile) {
        Some(TownTileMarker::NpcStartA | TownTileMarker::NpcStartB) => {
            return LocationCellOwner::NpcStartMarker;
        }
        Some(TownTileMarker::BeaconLightSource) => {
            return LocationCellOwner::BeaconLightSource;
        }
        Some(TownTileMarker::FloorLinkC8 | TownTileMarker::FloorLinkC9) => {
            return LocationCellOwner::FloorLinkMarker;
        }
        None => {}
    }
    if tile == TOWN_DAWN_DUSK_GATE_MARKER_TILE {
        return LocationCellOwner::DawnDuskGateMarker;
    }
    if tile == TELESCOPE_LOOK_TRIGGER_TILE {
        return LocationCellOwner::TelescopeLook;
    }
    if is_town_stair_tile(tile) {
        return LocationCellOwner::WalkOnStair;
    }
    if town_klimb_underfoot_intent(tile).is_some() {
        return LocationCellOwner::ClimbTransition;
    }
    if is_town_door_tile(tile) {
        return LocationCellOwner::Door;
    }
    if tile == TOWN_LOOSE_BRICK_TRAPDOOR_TILE {
        return LocationCellOwner::LooseBrickTrapdoor;
    }
    if tile == TOWN_POISON_GAS_LIVE_TILE {
        return LocationCellOwner::PoisonGas;
    }
    if pushable_tile_family(tile).is_some() {
        return LocationCellOwner::Pushable;
    }
    if town_search_inspectable_tile(tile) {
        return LocationCellOwner::SearchInspectable;
    }
    // What actually animates: the five published `animation.md §6` tile-id
    // families, plus the display driver's water animator (`cleak/u5-spec#179`)
    // — the rotated water and lava ids, and the river, coast and shore ids it
    // composites from them. This used to read the withdrawn
    // `tile_animation_family`, which called water a four-frame tile-id family
    // — see the note in `tile_classes.rs`.
    if static_tile_animation_family(tile).is_some() || water_pass_animates_tile(tile) {
        return LocationCellOwner::Animated;
    }
    match coarse_tile_class(tile) {
        TileClass::Vehicle | TileClass::VehicleArt | TileClass::Npc => {
            LocationCellOwner::ActorOrNpcArt
        }
        _ => LocationCellOwner::StaticTerrain,
    }
}

pub fn audit_location_dat_files(game_dir: &Path) -> io::Result<LocationAuditReport> {
    let mut report = LocationAuditReport {
        physical_pages: Vec::new(),
        logical_floors: Vec::new(),
        total_cells: 0,
        content_hash: 0xcbf29ce484222325,
        owner_counts: [0; LOCATION_AUDIT_OWNER_COUNT],
        tile_class_counts: [0; LOCATION_AUDIT_TILE_CLASS_COUNT],
        view_class_counts: [0; LOCATION_AUDIT_VIEW_CLASS_COUNT],
        npc_path_open_count: 0,
        foot_walkable_count: 0,
        dawn_dusk_bottom_row_count: 0,
        dawn_dusk_unexpected_pair_count: 0,
    };

    for family in [
        Family::Towne,
        Family::Dwelling,
        Family::Castle,
        Family::Keep,
    ] {
        let bytes = read_location_family_file(game_dir, family)?;
        for page in 0..LOCATION_DAT_FLOOR_PAGES_PER_FILE_TOTAL {
            let start = page * LOCATION_DAT_FLOOR_PAGE_LEN;
            let grid = &bytes[start..start + LOCATION_DAT_FLOOR_PAGE_LEN];
            let floor_audit = audit_location_grid(family, page, None, None, grid);
            merge_location_floor_audit(&mut report, &floor_audit);
            report.physical_pages.push(floor_audit);
        }
    }

    // `formats/location-dat.md` §4.1: walk each scene's *published* floor
    // range rather than a fixed -1..=1 window. Because the sixty-four
    // pages partition exactly, this visits every page of every class file
    // exactly once, which makes the logical pass a conformance check on
    // the base-page table rather than a sample of it.
    for scene_byte in SCENE_TOWN_FAMILY_FIRST..=SCENE_TOWN_FAMILY_LAST {
        let scene = Scene::new(scene_byte)?;
        let (lowest_floor, highest_floor) = location_page_run(scene).floor_range();
        for floor in lowest_floor..=highest_floor {
            let page = match resolve_location_floor_page(game_dir, scene, floor) {
                Ok(page) => page,
                Err(err) if err.kind() == io::ErrorKind::InvalidInput => continue,
                Err(err) => return Err(err),
            };
            let grid = load_floor(game_dir, scene, floor)?;
            report.logical_floors.push(audit_location_grid(
                scene.family,
                page,
                Some(scene),
                Some(floor),
                &grid,
            ));
        }
    }

    Ok(report)
}

pub fn location_audit_report_text(report: &LocationAuditReport) -> String {
    let mut text = String::new();
    text.push_str("Ultima V location cell audit\n");
    text.push_str(&format!(
        "physical_pages={} logical_floors={} cells={} hash={:016x}\n",
        report.physical_pages.len(),
        report.logical_floors.len(),
        report.total_cells,
        report.content_hash
    ));
    text.push_str("owner_counts:");
    for owner in LocationCellOwner::ALL {
        let count = report.owner_counts[owner.index()];
        if count > 0 {
            text.push_str(&format!(" {}={count}", owner.label()));
        }
    }
    text.push('\n');
    text.push_str("tile_class_counts:");
    for index in 0..LOCATION_AUDIT_TILE_CLASS_COUNT {
        let count = report.tile_class_counts[index];
        if count > 0 {
            text.push_str(&format!(" {}={count}", tile_class_label(index)));
        }
    }
    text.push('\n');
    text.push_str("view_class_counts:");
    for index in 0..LOCATION_AUDIT_VIEW_CLASS_COUNT {
        let count = report.view_class_counts[index];
        if count > 0 {
            text.push_str(&format!(" {}={count}", view_class_label(index)));
        }
    }
    text.push('\n');
    text.push_str(&format!(
        "movement_counts npc_path_open={} foot_walkable={}\n",
        report.npc_path_open_count, report.foot_walkable_count
    ));
    text.push_str(&format!(
        "dawn_dusk_anomalies bottom_row={} unexpected_pair={}\n",
        report.dawn_dusk_bottom_row_count, report.dawn_dusk_unexpected_pair_count
    ));
    text
}

const LOCATION_DAT_FLOOR_PAGES_PER_FILE_TOTAL: usize =
    LOCATION_DAT_BLOCKS_PER_FILE * LOCATION_DAT_FLOOR_PAGES_PER_BLOCK;

fn read_location_family_file(game_dir: &Path, family: Family) -> io::Result<Vec<u8>> {
    let path = game_dir.join(format!("{}.DAT", family.stem()));
    let bytes = fs::read(&path).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("{}: failed to read location file: {err}", path.display()),
        )
    })?;
    if bytes.len() != LOCATION_DAT_FILE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} must be {LOCATION_DAT_FILE_LEN} bytes, got {}",
                path.display(),
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}

fn audit_location_grid(
    family: Family,
    physical_page: usize,
    scene: Option<Scene>,
    logical_floor: Option<i8>,
    grid: &[u8],
) -> LocationFloorAudit {
    let mut day = grid.to_vec();
    normalize_town_runtime_floor(&mut day, 12);
    let mut night = grid.to_vec();
    normalize_town_runtime_floor(&mut night, 0);

    let mut audit = LocationFloorAudit {
        scene,
        family,
        physical_page,
        logical_floor,
        raw_hash: hash_bytes(grid),
        runtime_day_hash: hash_bytes(&day),
        runtime_night_hash: hash_bytes(&night),
        owner_counts: [0; LOCATION_AUDIT_OWNER_COUNT],
        tile_class_counts: [0; LOCATION_AUDIT_TILE_CLASS_COUNT],
        view_class_counts: [0; LOCATION_AUDIT_VIEW_CLASS_COUNT],
        npc_path_open_count: 0,
        foot_walkable_count: 0,
        dawn_dusk_marker_count: 0,
        dawn_dusk_paired_count: 0,
        dawn_dusk_bottom_row_count: 0,
        dawn_dusk_unexpected_pair_count: 0,
    };

    for y in 0..TOWN_GRID_SIDE {
        for x in 0..TOWN_GRID_SIDE {
            let tile = grid[y * TOWN_GRID_SIDE + x];
            let owner = classify_location_cell_owner(tile);
            audit.owner_counts[owner.index()] += 1;
            audit.tile_class_counts[tile_class_index(coarse_tile_class(tile))] += 1;
            let view_class =
                usize::from(tile_view_class(tile)).min(LOCATION_AUDIT_VIEW_CLASS_COUNT - 1);
            audit.view_class_counts[view_class] += 1;
            // npc-schedules.md §10: a clear bit is open, a set bit is an
            // obstacle for NPC pathfinding.
            if !npc_path_tile_obstacle(tile) {
                audit.npc_path_open_count += 1;
            }
            if is_tile_walkable_for_transport(tile, None, TransportState::Foot) {
                audit.foot_walkable_count += 1;
            }
            if tile == TOWN_DAWN_DUSK_GATE_MARKER_TILE {
                audit.dawn_dusk_marker_count += 1;
                if y + 1 >= TOWN_GRID_SIDE {
                    audit.dawn_dusk_bottom_row_count += 1;
                } else {
                    audit.dawn_dusk_paired_count += 1;
                    let paired = grid[(y + 1) * TOWN_GRID_SIDE + x];
                    if !matches!(
                        paired,
                        TOWN_DAWN_DUSK_GATE_OPEN_TILE | TOWN_DAWN_DUSK_GATE_CLOSED_TILE
                    ) {
                        audit.dawn_dusk_unexpected_pair_count += 1;
                    }
                }
            }
        }
    }

    audit
}

fn merge_location_floor_audit(report: &mut LocationAuditReport, floor: &LocationFloorAudit) {
    report.total_cells += TOWN_GRID_BYTES;
    report.content_hash ^= floor.raw_hash;
    report.content_hash = report.content_hash.wrapping_mul(0x100000001b3);
    report.dawn_dusk_bottom_row_count += floor.dawn_dusk_bottom_row_count;
    report.dawn_dusk_unexpected_pair_count += floor.dawn_dusk_unexpected_pair_count;
    report.npc_path_open_count += floor.npc_path_open_count;
    report.foot_walkable_count += floor.foot_walkable_count;
    for index in 0..LOCATION_AUDIT_OWNER_COUNT {
        report.owner_counts[index] += floor.owner_counts[index];
    }
    for index in 0..LOCATION_AUDIT_TILE_CLASS_COUNT {
        report.tile_class_counts[index] += floor.tile_class_counts[index];
    }
    for index in 0..LOCATION_AUDIT_VIEW_CLASS_COUNT {
        report.view_class_counts[index] += floor.view_class_counts[index];
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

fn tile_class_label(index: usize) -> &'static str {
    match index {
        0 => "sentinel",
        1 => "water",
        2 => "terrain",
        3 => "path",
        4 => "wall",
        5 => "furniture",
        6 => "river",
        7 => "decoration",
        8 => "barrier",
        9 => "special",
        10 => "vehicle",
        11 => "vehicle-art",
        12 => "npc",
        _ => "unknown",
    }
}

fn view_class_label(index: usize) -> String {
    format!("0x{index:02x}")
}

fn town_search_inspectable_tile(tile: u8) -> bool {
    matches!(
        tile,
        0x2B | 0x4F
            | 0x5A
            | 0x5C..=0x5D
            | 0xA1
            | 0xA5..=0xA6
            | 0xA8
            | 0xAB..=0xAD
            | 0xAF
            | 0xB2
            | 0xBC
    )
}
