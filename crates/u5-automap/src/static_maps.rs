//! A lazy, load-once cache of the static maps (design 4.3.4).
//!
//! The map of an area the party is not in is drawn from what the party
//! would meet on its next entry: the world maps, the town floors as the
//! `.DAT` files hold them and the dungeon records as `DUNGEON.DAT` holds
//! them. Each is read on first use and kept for the session; a failed
//! read is kept too, so a missing file is not retried every frame.
//!
//! The game folder is passed in by the frontend, which already plays from
//! the writable runtime folder. This module never derives a path of its
//! own; a different folder starts an empty cache.

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use u5_runtime::{
    DUNGEON_RECORD_LEN, DungeonScene, Scene, WorldPlane, load_dungeon_record, load_floor,
    load_world_map,
};

/// Bytes in a decoded world map (256 x 256, row-major).
pub const WORLD_MAP_LEN: usize = 256 * 256;
/// Bytes in one town floor (32 x 32, row-major).
pub const FLOOR_LEN: usize = 32 * 32;

/// Where the static maps come from. [`DiskSource`] reads the game folder
/// through `u5-runtime`'s loaders; tests substitute a counting fake.
pub trait StaticSource {
    fn world_map(&mut self, game_dir: &Path, plane: WorldPlane) -> io::Result<Vec<u8>>;
    fn floor(&mut self, game_dir: &Path, scene: Scene, floor: i8) -> io::Result<Vec<u8>>;
    fn dungeon_record(&mut self, game_dir: &Path, scene: DungeonScene) -> io::Result<Vec<u8>>;
}

/// The real loaders: `load_world_map`, `load_floor` and
/// `load_dungeon_record`.
#[derive(Clone, Copy, Debug, Default)]
pub struct DiskSource;

impl StaticSource for DiskSource {
    fn world_map(&mut self, game_dir: &Path, plane: WorldPlane) -> io::Result<Vec<u8>> {
        load_world_map(game_dir, plane)
    }

    fn floor(&mut self, game_dir: &Path, scene: Scene, floor: i8) -> io::Result<Vec<u8>> {
        load_floor(game_dir, scene, floor)
    }

    fn dungeon_record(&mut self, game_dir: &Path, scene: DungeonScene) -> io::Result<Vec<u8>> {
        load_dungeon_record(game_dir, scene)
    }
}

/// A failed load, kept in the cache. It holds the loader's message, which
/// names files, never map content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticMapError {
    pub kind: io::ErrorKind,
    pub message: String,
}

impl StaticMapError {
    fn from_io(error: &io::Error) -> Self {
        Self {
            kind: error.kind(),
            message: error.to_string(),
        }
    }

    fn bad_length(what: &str, got: usize, want: usize) -> Self {
        Self {
            kind: io::ErrorKind::InvalidData,
            message: format!("{what} is {got} bytes, expected {want}"),
        }
    }

    fn invalid(message: String) -> Self {
        Self {
            kind: io::ErrorKind::InvalidInput,
            message,
        }
    }
}

impl fmt::Display for StaticMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StaticMapError {}

type Cached = Result<Box<[u8]>, StaticMapError>;

/// The cache. One per game folder.
pub struct StaticMaps<S: StaticSource = DiskSource> {
    game_dir: PathBuf,
    source: S,
    worlds: HashMap<WorldPlane, Cached>,
    floors: HashMap<(u8, i8), Cached>,
    dungeons: HashMap<u8, Cached>,
}

impl StaticMaps<DiskSource> {
    /// A cache over the game folder the frontend plays from.
    pub fn new(game_dir: impl Into<PathBuf>) -> Self {
        Self::with_source(game_dir, DiskSource)
    }
}

impl<S: StaticSource> StaticMaps<S> {
    pub fn with_source(game_dir: impl Into<PathBuf>, source: S) -> Self {
        Self {
            game_dir: game_dir.into(),
            source,
            worlds: HashMap::new(),
            floors: HashMap::new(),
            dungeons: HashMap::new(),
        }
    }

    pub fn game_dir(&self) -> &Path {
        &self.game_dir
    }

    /// Points the cache at `game_dir`. A different folder forgets every
    /// cached map; the same folder keeps them.
    pub fn set_game_dir(&mut self, game_dir: &Path) {
        if self.game_dir != game_dir {
            self.game_dir = game_dir.to_path_buf();
            self.clear();
        }
    }

    /// Forgets every cached map and error.
    pub fn clear(&mut self) {
        self.worlds.clear();
        self.floors.clear();
        self.dungeons.clear();
    }

    /// Number of cached entries, failures included.
    pub fn cached_len(&self) -> usize {
        self.worlds.len() + self.floors.len() + self.dungeons.len()
    }

    /// A decoded world map, 256 x 256 tiles, row-major.
    pub fn world_map(&mut self, plane: WorldPlane) -> Result<&[u8], StaticMapError> {
        let Self {
            game_dir,
            source,
            worlds,
            ..
        } = self;
        let entry = worlds.entry(plane).or_insert_with(|| {
            let bytes = source
                .world_map(game_dir, plane)
                .map_err(|e| StaticMapError::from_io(&e))?;
            checked(bytes, WORLD_MAP_LEN, plane.key())
        });
        as_slice(entry)
    }

    /// One town floor as its `.DAT` file holds it, 32 x 32, row-major.
    /// `scene` is the scene byte 1..=32.
    pub fn floor(&mut self, scene: u8, floor: i8) -> Result<&[u8], StaticMapError> {
        let Self {
            game_dir,
            source,
            floors,
            ..
        } = self;
        let entry = floors.entry((scene, floor)).or_insert_with(|| {
            let scene = Scene::new(scene).map_err(|e| StaticMapError::from_io(&e))?;
            let bytes = source
                .floor(game_dir, scene, floor)
                .map_err(|e| StaticMapError::from_io(&e))?;
            checked(bytes, FLOOR_LEN, "floor")
        });
        as_slice(entry)
    }

    /// One dungeon record as `DUNGEON.DAT` holds it: eight levels of
    /// 8 x 8 cells, level-major. `record` is 0..=7.
    pub fn dungeon_record(&mut self, record: u8) -> Result<&[u8], StaticMapError> {
        let Self {
            game_dir,
            source,
            dungeons,
            ..
        } = self;
        let entry = dungeons.entry(record).or_insert_with(|| {
            let scene =
                DungeonScene::from_record(record).map_err(|e| StaticMapError::from_io(&e))?;
            let bytes = source
                .dungeon_record(game_dir, scene)
                .map_err(|e| StaticMapError::from_io(&e))?;
            checked(bytes, DUNGEON_RECORD_LEN, "dungeon record")
        });
        as_slice(entry)
    }

    /// One 8 x 8 dungeon level, row-major. `level` is 0..=7.
    pub fn dungeon_level(&mut self, record: u8, level: u8) -> Result<&[u8], StaticMapError> {
        if level >= 8 {
            return Err(StaticMapError::invalid(format!(
                "dungeon level must be 0..7, got {level}"
            )));
        }
        let start = level as usize * 64;
        Ok(&self.dungeon_record(record)?[start..start + 64])
    }
}

impl<S: StaticSource> fmt::Debug for StaticMaps<S> {
    /// The folder and what is cached, never map bytes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticMaps")
            .field("game_dir", &self.game_dir)
            .field("worlds", &self.worlds.len())
            .field("floors", &self.floors.len())
            .field("dungeons", &self.dungeons.len())
            .finish()
    }
}

fn checked(bytes: Vec<u8>, want: usize, what: &str) -> Cached {
    if bytes.len() == want {
        Ok(bytes.into_boxed_slice())
    } else {
        Err(StaticMapError::bad_length(what, bytes.len(), want))
    }
}

fn as_slice(entry: &Cached) -> Result<&[u8], StaticMapError> {
    match entry {
        Ok(bytes) => Ok(bytes),
        Err(error) => Err(error.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake loader that counts calls and fills each map with a byte
    /// derived from its key.
    #[derive(Default)]
    struct Counting {
        worlds: Vec<WorldPlane>,
        floors: Vec<(u8, i8)>,
        dungeons: Vec<usize>,
        dirs: Vec<PathBuf>,
        fail_floor: Option<(u8, i8)>,
        short_dungeon: bool,
    }

    impl StaticSource for Counting {
        fn world_map(&mut self, game_dir: &Path, plane: WorldPlane) -> io::Result<Vec<u8>> {
            self.dirs.push(game_dir.to_path_buf());
            self.worlds.push(plane);
            let fill = match plane {
                WorldPlane::Britannia => 1,
                WorldPlane::Underworld => 2,
            };
            Ok(vec![fill; WORLD_MAP_LEN])
        }

        fn floor(&mut self, game_dir: &Path, scene: Scene, floor: i8) -> io::Result<Vec<u8>> {
            self.dirs.push(game_dir.to_path_buf());
            self.floors.push((scene.byte, floor));
            if self.fail_floor == Some((scene.byte, floor)) {
                return Err(io::Error::new(io::ErrorKind::NotFound, "no such floor"));
            }
            Ok(vec![scene.byte.wrapping_add(floor as u8); FLOOR_LEN])
        }

        fn dungeon_record(&mut self, game_dir: &Path, scene: DungeonScene) -> io::Result<Vec<u8>> {
            self.dirs.push(game_dir.to_path_buf());
            self.dungeons.push(scene.record);
            let len = if self.short_dungeon {
                DUNGEON_RECORD_LEN - 1
            } else {
                DUNGEON_RECORD_LEN
            };
            Ok((0..len)
                .map(|i| (i / 64) as u8 + 0x10 * scene.record as u8)
                .collect())
        }
    }

    fn maps() -> StaticMaps<Counting> {
        StaticMaps::with_source("/game", Counting::default())
    }

    #[test]
    fn each_static_map_loads_once() {
        let mut maps = maps();
        for _ in 0..5 {
            assert_eq!(maps.world_map(WorldPlane::Britannia).unwrap()[0], 1);
            assert_eq!(maps.world_map(WorldPlane::Underworld).unwrap()[100], 2);
            assert_eq!(maps.floor(5, 0).unwrap()[0], 5);
            assert_eq!(maps.floor(5, 1).unwrap()[0], 6);
            assert_eq!(maps.floor(5, -1).unwrap()[0], 4);
            assert_eq!(maps.dungeon_record(3).unwrap()[64], 0x31);
            assert_eq!(maps.dungeon_level(3, 7).unwrap(), [0x37; 64]);
        }
        let source = &maps.source;
        assert_eq!(
            source.worlds,
            [WorldPlane::Britannia, WorldPlane::Underworld]
        );
        assert_eq!(source.floors, [(5, 0), (5, 1), (5, -1)]);
        assert_eq!(source.dungeons, [3]);
        assert!(source.dirs.iter().all(|dir| dir == Path::new("/game")));
        assert_eq!(maps.cached_len(), 6);
    }

    #[test]
    fn a_failed_load_is_cached_too() {
        let mut maps = maps();
        maps.source.fail_floor = Some((9, 0));
        for _ in 0..3 {
            let error = maps.floor(9, 0).unwrap_err();
            assert_eq!(error.kind, io::ErrorKind::NotFound);
        }
        assert_eq!(maps.source.floors, [(9, 0)]);
    }

    #[test]
    fn a_wrong_length_is_an_error_not_a_short_map() {
        let mut maps = maps();
        maps.source.short_dungeon = true;
        let error = maps.dungeon_level(0, 0).unwrap_err();
        assert_eq!(error.kind, io::ErrorKind::InvalidData);
        assert!(maps.dungeon_record(0).is_err());
        assert_eq!(maps.source.dungeons, [0]);
    }

    #[test]
    fn invalid_keys_never_reach_the_loader() {
        let mut maps = maps();
        assert!(maps.floor(0, 0).is_err());
        assert!(maps.floor(33, 0).is_err());
        assert!(maps.dungeon_record(8).is_err());
        assert!(maps.dungeon_level(0, 8).is_err());
        assert!(maps.source.floors.is_empty());
        assert!(maps.source.dungeons.is_empty());
    }

    #[test]
    fn a_new_game_folder_starts_an_empty_cache() {
        let mut maps = maps();
        maps.world_map(WorldPlane::Britannia).unwrap();
        maps.set_game_dir(Path::new("/game"));
        maps.world_map(WorldPlane::Britannia).unwrap();
        assert_eq!(maps.source.worlds.len(), 1);

        maps.set_game_dir(Path::new("/other"));
        assert_eq!(maps.cached_len(), 0);
        maps.world_map(WorldPlane::Britannia).unwrap();
        assert_eq!(maps.source.worlds.len(), 2);
        assert_eq!(maps.source.dirs.last().unwrap(), Path::new("/other"));
        assert_eq!(maps.game_dir(), Path::new("/other"));
    }

    #[test]
    fn debug_shows_counts_not_bytes() {
        let mut maps = maps();
        maps.dungeon_record(1).unwrap();
        let text = format!("{maps:?}");
        assert!(text.contains("dungeons: 1"), "{text}");
        assert!(text.len() < 120, "{text}");
    }
}
