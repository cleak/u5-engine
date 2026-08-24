//! Clean save-side persistence for destructive town NPC rewrites.
//!
//! The original behavior mutates schedule/dialogue storage, but this clean
//! engine treats original asset files as read-only. This ledger stores only
//! the bytes the published alarm/Shadowlord helpers are allowed to rewrite;
//! waypoint coordinates and all other roster data continue to come from the
//! runtime asset files.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub const TOWN_NPC_MUTATIONS_FILENAME: &str = ".u5-engine-town-npc-mutations";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownNpcMutation {
    pub scene_byte: u8,
    pub npc_slot: usize,
    pub ai: [u8; NPC_SCHEDULE_WAYPOINT_COUNT],
    pub times: [u8; NPC_SCHEDULE_TIME_BOUNDARY_COUNT],
    pub dialog_id: u8,
}

impl TownNpcMutation {
    pub fn from_runtime(scene: Scene, npc: &RuntimeNpc) -> Self {
        Self {
            scene_byte: scene.byte,
            npc_slot: npc.slot,
            ai: std::array::from_fn(|index| npc.schedule[NPC_SCHEDULE_AI_OFFSET + index]),
            times: npc.schedule_time_boundaries(),
            dialog_id: npc.dialog_id,
        }
    }

    pub fn apply_to(self, npc: &mut RuntimeNpc) {
        npc.schedule[NPC_SCHEDULE_AI_OFFSET..NPC_SCHEDULE_AI_OFFSET + self.ai.len()]
            .copy_from_slice(&self.ai);
        npc.schedule[NPC_SCHEDULE_TIME_OFFSET..NPC_SCHEDULE_TIME_OFFSET + self.times.len()]
            .copy_from_slice(&self.times);
        npc.dialog_id = self.dialog_id;
        npc.reset_move_queue();
    }
}

pub fn upsert_town_npc_mutation(mutations: &mut Vec<TownNpcMutation>, mutation: TownNpcMutation) {
    if let Some(existing) = mutations.iter_mut().find(|existing| {
        existing.scene_byte == mutation.scene_byte && existing.npc_slot == mutation.npc_slot
    }) {
        *existing = mutation;
    } else {
        mutations.push(mutation);
        mutations.sort_by_key(|entry| (entry.scene_byte, entry.npc_slot));
    }
}

pub fn load_town_npc_mutations(save_dir: &Path) -> io::Result<Vec<TownNpcMutation>> {
    let path = save_dir.join(TOWN_NPC_MUTATIONS_FILENAME);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut mutations = Vec::new();
    for (line_index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 10 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{} line {} must contain 10 hexadecimal fields",
                    TOWN_NPC_MUTATIONS_FILENAME,
                    line_index + 1
                ),
            ));
        }
        let mut bytes = [0u8; 10];
        for (index, field) in fields.iter().enumerate() {
            bytes[index] = u8::from_str_radix(field, 16).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{} line {} field {} is not a hexadecimal byte",
                        TOWN_NPC_MUTATIONS_FILENAME,
                        line_index + 1,
                        index + 1
                    ),
                )
            })?;
        }
        let scene = Scene::new(bytes[0])?;
        let npc_slot = usize::from(bytes[1]);
        if npc_slot == NPC_SENTINEL_SLOT || npc_slot >= NPC_SLOTS_PER_SUB_MAP {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{} line {} NPC slot must be 1..31",
                    TOWN_NPC_MUTATIONS_FILENAME,
                    line_index + 1
                ),
            ));
        }
        upsert_town_npc_mutation(
            &mut mutations,
            TownNpcMutation {
                scene_byte: scene.byte,
                npc_slot,
                ai: [bytes[2], bytes[3], bytes[4]],
                times: [bytes[5], bytes[6], bytes[7], bytes[8]],
                dialog_id: bytes[9],
            },
        );
    }
    Ok(mutations)
}

pub fn write_town_npc_mutations(save_dir: &Path, mutations: &[TownNpcMutation]) -> io::Result<()> {
    let mut text = String::from("# scene slot ai0 ai1 ai2 time0 time1 time2 time3 dialog\n");
    for mutation in mutations {
        text.push_str(&format!(
            "{:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}\n",
            mutation.scene_byte,
            mutation.npc_slot,
            mutation.ai[0],
            mutation.ai[1],
            mutation.ai[2],
            mutation.times[0],
            mutation.times[1],
            mutation.times[2],
            mutation.times[3],
            mutation.dialog_id
        ));
    }
    fs::write(save_dir.join(TOWN_NPC_MUTATIONS_FILENAME), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_ledger_round_trips_only_the_published_rewrite_fields() {
        let dir = crate::test_fixtures::debug_game_dir();
        let mutations = vec![
            TownNpcMutation {
                scene_byte: 1,
                npc_slot: 4,
                ai: [6, 6, 6],
                times: [0, 0, 0, 0],
                dialog_id: TOWN_NPC_BRUSHOFF_DIALOG_ID,
            },
            TownNpcMutation {
                scene_byte: 32,
                npc_slot: 31,
                ai: [3, 3, 3],
                times: [6, 12, 18, 22],
                dialog_id: TOWN_NPC_COWERING_DIALOG_ID,
            },
        ];

        write_town_npc_mutations(&dir, &mutations).unwrap();
        assert_eq!(load_town_npc_mutations(&dir).unwrap(), mutations);
        let text = fs::read_to_string(dir.join(TOWN_NPC_MUTATIONS_FILENAME)).unwrap();
        assert!(!text.contains("waypoint"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn mutation_ledger_rejects_slot_zero_and_malformed_rows() {
        let dir = crate::test_fixtures::debug_game_dir();
        fs::write(
            dir.join(TOWN_NPC_MUTATIONS_FILENAME),
            "01 00 06 06 06 00 00 00 00 FE\n",
        )
        .unwrap();
        assert_eq!(
            load_town_npc_mutations(&dir).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::write(dir.join(TOWN_NPC_MUTATIONS_FILENAME), "01 04 06\n").unwrap();
        assert_eq!(
            load_town_npc_mutations(&dir).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let _ = fs::remove_dir_all(dir);
    }
}
