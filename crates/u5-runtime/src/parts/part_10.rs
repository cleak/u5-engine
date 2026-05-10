pub fn parse_world_waterfall_entries(text: &str) -> io::Result<Vec<WorldWaterfallEntry>> {
    let mut entries = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line
            .split_once('#')
            .map_or(line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line
            .split(|ch: char| ch == ',' || ch == '\t' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
            .collect();
        if !(5..=6).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} must be: PLANE X Y DIRECTION STEPS [TILE]"
                ),
            ));
        }

        let plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has unknown plane `{}`",
                    parts[0]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let direction = parse_cardinal_direction(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has invalid direction `{}`: {err}",
                    parts[3]
                ),
            )
        })?;
        let steps = parse_u8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has invalid step count `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        if steps == 0 || steps > 16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} step count must be in 1..16, got {steps}"
                ),
            ));
        }
        let expected_tile = if let Some(tile) = parts.get(5) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries
            .iter()
            .any(|entry: &WorldWaterfallEntry| entry.plane == plane && entry.x == x && entry.y == y)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} duplicates {}/{x},{y}",
                    plane.key()
                ),
            ));
        }
        entries.push(WorldWaterfallEntry {
            plane,
            x,
            y,
            direction,
            steps,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_world_damage_tile_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<WorldDamageTileEntry>>> {
    let path = game_dir.join(WORLD_DAMAGE_TILE_TABLE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_world_damage_tile_entries(&text).map(Some)
}

pub fn parse_world_damage_tile_entries(text: &str) -> io::Result<Vec<WorldDamageTileEntry>> {
    let mut entries = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line
            .split_once('#')
            .map_or(line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line
            .split(|ch: char| ch == ',' || ch == '\t' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
            .collect();
        if !(4..=5).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} must be: PLANE X Y EFFECT [TILE]"
                ),
            ));
        }

        let plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} has unknown plane `{}`",
                    parts[0]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let effect = WorldDamageEffect::from_key(parts[3]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} has unknown effect `{}`",
                    parts[3]
                ),
            )
        })?;
        let expected_tile = if let Some(tile) = parts.get(4) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &WorldDamageTileEntry| {
            entry.plane == plane && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} duplicates {}/{x},{y}",
                    plane.key()
                ),
            ));
        }
        entries.push(WorldDamageTileEntry {
            plane,
            x,
            y,
            effect,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_world_encounter_entries(game_dir: &Path) -> io::Result<Option<Vec<WorldEncounterEntry>>> {
    let path = game_dir.join(WORLD_ENCOUNTER_TABLE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_world_encounter_entries(&text).map(Some)
}

pub fn parse_world_encounter_entries(text: &str) -> io::Result<Vec<WorldEncounterEntry>> {
    let mut entries = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line
            .split_once('#')
            .map_or(line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line
            .split(|ch: char| ch == ',' || ch == '\t' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
            .collect();
        if !matches!(parts.len(), 6 | 7) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} must be: PLANE TILE THRESHOLD TYPE DX DY [PHASE]"
                ),
            ));
        }

        let plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has unknown plane `{}`",
                    parts[0]
                ),
            )
        })?;
        let tile = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid tile `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let threshold = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid threshold `{}`: {err}",
                    parts[2]
                ),
            )
        })?;
        if threshold > 30 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} threshold must be in 0..30, got {threshold}"
                ),
            ));
        }
        let type_byte = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid type `{}`: {err}",
                    parts[3]
                ),
            )
        })?;
        if !(192..=255).contains(&type_byte) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} type must be a monster/NPC sprite byte in 192..255, got {type_byte}"
                ),
            ));
        }
        let dx = parse_i8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid DX `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        let dy = parse_i8_literal(parts[5]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid DY `{}`: {err}",
                    parts[5]
                ),
            )
        })?;
        if dx == 0 && dy == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} offset cannot be 0,0"),
            ));
        }
        if dx.unsigned_abs() > ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS as u8
            || dy.unsigned_abs() > ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS as u8
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} offset must stay within +/-{ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS}"
                ),
            ));
        }
        let phase = if let Some(phase) = parts.get(6) {
            let phase = parse_u8_literal(phase).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid phase `{phase}`: {err}"
                    ),
                )
            })?;
            if direction_from_active_object_phase(phase).is_none() || (phase & 0x0f) == STEADY_PHASE
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} phase must encode a wander direction with a non-steady low nibble"
                    ),
                ));
            }
            phase
        } else {
            active_object_phase_toward_player(dx, dy)
        };
        if entries
            .iter()
            .any(|entry: &WorldEncounterEntry| entry.plane == plane && entry.tile == tile)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} duplicates {}/tile {tile}",
                    plane.key()
                ),
            ));
        }
        entries.push(WorldEncounterEntry {
            plane,
            tile,
            threshold,
            type_byte,
            dx,
            dy,
            phase,
        });
    }
    Ok(entries)
}

pub fn load_dungeon_deeper_transition_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<DungeonDeeperTransitionEntry>>> {
    let path = game_dir.join(DUNGEON_DEEPER_TRANSITION_TABLE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_dungeon_deeper_transition_entries(&text).map(Some)
}

pub fn parse_dungeon_deeper_transition_entries(
    text: &str,
) -> io::Result<Vec<DungeonDeeperTransitionEntry>> {
    let mut entries = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line
            .split_once('#')
            .map_or(line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line
            .split(|ch: char| ch == ',' || ch == '\t' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() != 7 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} must be: DUNGEON LEVEL X Y TO_PLANE TO_X TO_Y"
                ),
            ));
        }
        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid dungeon `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Dungeon(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} source must be a dungeon"
                    ),
                ));
            }
        };
        let level = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid level `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        if level >= DUNGEON_SIDE as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} level must be inside 0..7, got {level}"
                ),
            ));
        }
        if level != (DUNGEON_SIDE - 1) as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} must use bottom level 7 for a deeper transition, got {level}"
                ),
            ));
        }
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} source coordinate must be inside 0..7, got ({x}, {y})"
                ),
            ));
        }
        let to_plane = WorldPlane::from_key(parts[4]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has unknown destination plane `{}`",
                    parts[4]
                ),
            )
        })?;
        let to_x = parse_u8_literal(parts[5]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid destination X `{}`: {err}",
                    parts[5]
                ),
            )
        })? as usize;
        let to_y = parse_u8_literal(parts[6]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid destination Y `{}`: {err}",
                    parts[6]
                ),
            )
        })? as usize;
        if entries.iter().any(|entry: &DungeonDeeperTransitionEntry| {
            entry.scene == scene && entry.level == level && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} duplicates {} level {level} ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(DungeonDeeperTransitionEntry {
            scene,
            level,
            x,
            y,
            to_plane,
            to_x,
            to_y,
        });
    }
    Ok(entries)
}

pub fn load_dungeon_teleport_entries(game_dir: &Path) -> io::Result<Option<Vec<DungeonTeleportEntry>>> {
    let path = game_dir.join(DUNGEON_TELEPORT_TABLE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_dungeon_teleport_entries(&text).map(Some)
}

pub fn parse_dungeon_teleport_entries(text: &str) -> io::Result<Vec<DungeonTeleportEntry>> {
    let mut entries = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line
            .split_once('#')
            .map_or(line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line
            .split(|ch: char| ch == ',' || ch == '\t' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
            .collect();
        if !(7..=8).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} must be: DUNGEON LEVEL X Y TO_LEVEL TO_X TO_Y [CELL]"
                ),
            ));
        }
        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid dungeon `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Dungeon(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} source must be a dungeon"
                    ),
                ));
            }
        };
        let level = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid level `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        if level >= DUNGEON_SIDE as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} level must be inside 0..7, got {level}"
                ),
            ));
        }
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} source coordinate must be inside 0..7, got ({x}, {y})"
                ),
            ));
        }
        let to_level = parse_u8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid destination level `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        if to_level >= DUNGEON_SIDE as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} destination level must be inside 0..7, got {to_level}"
                ),
            ));
        }
        if to_level == level {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} must change dungeon level"
                ),
            ));
        }
        let to_x = parse_u8_literal(parts[5]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid destination X `{}`: {err}",
                    parts[5]
                ),
            )
        })? as usize;
        let to_y = parse_u8_literal(parts[6]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid destination Y `{}`: {err}",
                    parts[6]
                ),
            )
        })? as usize;
        if to_x >= DUNGEON_SIDE || to_y >= DUNGEON_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} destination coordinate must be inside 0..7, got ({to_x}, {to_y})"
                ),
            ));
        }
        let expected_cell = if let Some(cell) = parts.get(7) {
            Some(parse_u8_literal(cell).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid cell `{cell}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &DungeonTeleportEntry| {
            entry.scene == scene && entry.level == level && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} duplicates {} level {level} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(DungeonTeleportEntry {
            scene,
            level,
            x,
            y,
            to_level,
            to_x,
            to_y,
            expected_cell,
        });
    }
    Ok(entries)
}

pub fn load_dungeon_chest_content_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<DungeonChestContentEntry>>> {
    let path = game_dir.join(DUNGEON_CHEST_TABLE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_dungeon_chest_content_entries(&text).map(Some)
}

