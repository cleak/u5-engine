//! A save slot's place text (Phase B design 3.2.3, 3.3.5; build plan
//! D-M4.8).
//!
//! A slot shows where the party stood when it saved, so the text is always
//! a place the player has been (foundation 1.4.7). It is never an internal
//! scene key: it comes from the same published names the gem map gives a
//! place once the party is inside it:
//!
//! - outdoors, the plane: `Britannia` or `Underworld`;
//! - in a town, castle, keep or dwelling, its gazetteer name as the gem map
//!   labels it (`Castle British`, `Iolo's Hut`);
//! - in a dungeon, `<Dungeon> level N`, as the gem map titles the level
//!   the party is on;
//! - a place without a published name (the unnamed huts), plain words:
//!   `a dwelling`.

use u5_runtime::{PlayTarget, WorldPlane};

use super::features::PLACE_NAMES;

/// The words for a dwelling the gazetteer gives no name.
pub const UNNAMED_DWELLING: &str = "a dwelling";
/// The gazetteer's word for an unnamed dwelling (10.2.7).
const HUT: &str = "Hut";

/// Where a slot says the party stands: `target` is the scene it is in and
/// `floor` the town floor or the zero-based dungeon level (ignored
/// outdoors), as a save image holds them.
pub fn slot_place_text(target: PlayTarget, floor: i8) -> String {
    match target {
        PlayTarget::World(WorldPlane::Britannia) => "Britannia".to_string(),
        PlayTarget::World(WorldPlane::Underworld) => "Underworld".to_string(),
        PlayTarget::Town(scene) => match PLACE_NAMES.get(usize::from(scene.byte) - 1) {
            Some(name) if *name != HUT => (*name).to_string(),
            _ => UNNAMED_DWELLING.to_string(),
        },
        PlayTarget::Dungeon(scene) => {
            format!("{} level {}", scene.name(), i32::from(floor.max(0)) + 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use u5_runtime::{DungeonScene, Scene};

    #[test]
    fn outdoors_names_the_plane() {
        assert_eq!(
            slot_place_text(PlayTarget::World(WorldPlane::Britannia), 0),
            "Britannia"
        );
        assert_eq!(
            slot_place_text(PlayTarget::World(WorldPlane::Underworld), -1),
            "Underworld"
        );
    }

    #[test]
    fn a_town_has_its_published_name_on_every_floor() {
        let britain = PlayTarget::Town(Scene::new(2).unwrap());
        assert_eq!(slot_place_text(britain, 0), "Britain");
        assert_eq!(slot_place_text(britain, 1), "Britain");
        // The castles carry the gem map's short names.
        let castle = PlayTarget::Town(Scene::new(17).unwrap());
        assert_eq!(slot_place_text(castle, -1), "Castle British");
        let named_hut = PlayTarget::Town(Scene::new(13).unwrap());
        assert_eq!(slot_place_text(named_hut, 0), "Iolo's Hut");
    }

    #[test]
    fn a_dungeon_is_named_with_its_level_from_one() {
        let despise = PlayTarget::Dungeon(DungeonScene::from_record(1).unwrap());
        assert_eq!(slot_place_text(despise, 0), "Despise level 1");
        assert_eq!(slot_place_text(despise, 7), "Despise level 8");
    }

    #[test]
    fn an_unnamed_dwelling_is_plain_words() {
        for byte in 14..=16 {
            let hut = PlayTarget::Town(Scene::new(byte).unwrap());
            assert_eq!(slot_place_text(hut, 0), UNNAMED_DWELLING);
        }
    }

    /// No scene gives an internal key or an empty string.
    #[test]
    fn every_scene_reads_as_words() {
        for byte in 1..=32 {
            let text = slot_place_text(PlayTarget::Town(Scene::new(byte).unwrap()), 0);
            assert!(!text.is_empty() && !text.contains(':'), "{byte}: {text}");
        }
        for record in 0..8 {
            let text = slot_place_text(
                PlayTarget::Dungeon(DungeonScene::from_record(record).unwrap()),
                0,
            );
            assert!(text.ends_with(" level 1") && !text.contains(':'), "{text}");
        }
    }
}
