//! The `config.toml` document: keys read and written in place (Phase B
//! design 1.3.4).
//!
//! [`Config`](crate::Config) owns three keys of the file. Enhancement
//! sections (`[enhancements]`, `[keys]`, `[display]`, ...) live beside them
//! and belong to other code. [`ConfigDocument`] edits one key at a time over
//! a `toml_edit` document, so comments, key order and every section it does
//! not touch stay exactly as they were:
//!
//! - **A missing file** is an empty document. Loading never fails on
//!   content: a file that is not valid TOML is an empty document with a
//!   note ([`ConfigDocument::damaged`]).
//! - **Only changed keys are written.** [`ConfigDocument::save_if_changed`]
//!   re-reads the file, re-applies this session's changed keys, and writes
//!   only when the bytes differ, so a key the player edited by hand while
//!   the game ran is kept.
//! - **Damaged files are kept.** Before a damaged file is rewritten it is
//!   copied to `config.toml.bak`.
//! - **Writes are atomic**: temporary file, fsync, rename.
//!
//! Path resolution stays in [`paths`](crate::paths); everything here takes
//! the path it works on, so tests never touch the process environment.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, Table, TableLike, Value};

/// A `config.toml` file, edited key by key.
#[derive(Debug)]
pub struct ConfigDocument {
    path: PathBuf,
    doc: DocumentMut,
    /// Some(..) when the file was not valid TOML.
    damaged: Option<String>,
    /// Whether the file existed when it was last read.
    existed: bool,
    /// The document a save starts from when the file is missing or
    /// damaged at save time. Empty unless [`Self::start_fresh`] set it.
    fresh: DocumentMut,
    /// This session's changes, in order, one entry per key: `None` removes
    /// the key.
    changes: Vec<(String, Option<Value>)>,
}

impl ConfigDocument {
    /// Missing file = empty document. Never fails on content; only an I/O
    /// error other than "not found" is an error.
    pub fn load(path: &Path) -> io::Result<Self> {
        let (existed, doc, damaged) = match read_document(path)? {
            OnDisk::Missing => (false, DocumentMut::new(), None),
            OnDisk::Valid { doc, .. } => (true, doc, None),
            OnDisk::Damaged { message, .. } => (true, DocumentMut::new(), Some(message)),
        };
        Ok(Self {
            path: path.to_path_buf(),
            doc,
            damaged,
            existed,
            fresh: DocumentMut::new(),
            changes: Vec::new(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Why the file is not valid TOML, when it is not.
    pub fn damaged(&self) -> Option<&str> {
        self.damaged.as_deref()
    }

    /// Whether the file existed when it was loaded or last saved.
    pub fn exists(&self) -> bool {
        self.existed
    }

    /// The item under a dotted key (`section.name`), if present.
    pub fn get(&self, dotted: &str) -> Option<&Item> {
        let mut parts = dotted.split('.');
        let mut item = self.doc.as_item().as_table_like()?.get(parts.next()?)?;
        for part in parts {
            item = item.as_table_like()?.get(part)?;
        }
        (!item.is_none()).then_some(item)
    }

    /// Every dotted key that holds a value, in file order. Tables (inline or
    /// not) are walked; arrays are values.
    pub fn keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        collect_keys(self.doc.as_table(), "", &mut keys);
        keys
    }

    /// Edits one key in place; comments and order are kept. Returns whether
    /// the value changed. An equal value (whatever its spelling) is left
    /// alone and is not recorded as a change.
    pub fn set(&mut self, dotted: &str, value: Value) -> bool {
        self.change(dotted, Some(value), false)
    }

    /// Removes one key. Returns whether it was present.
    pub fn remove(&mut self, dotted: &str) -> bool {
        self.change(dotted, None, false)
    }

    /// Makes `dotted` hold `value` (or be absent, for `None`) when the file
    /// is saved, even if the loaded file already agrees. For a key this
    /// session owns outright, such as the game-folder keys a setup commit
    /// writes. Returns whether the in-memory value changed.
    pub(crate) fn assign(&mut self, dotted: &str, value: Option<Value>) -> bool {
        self.change(dotted, value, true)
    }

    fn change(&mut self, dotted: &str, value: Option<Value>, always: bool) -> bool {
        let changed = apply(self.doc.as_table_mut(), dotted, value.clone());
        if changed || always {
            self.changes.retain(|(key, _)| key != dotted);
            self.changes.push((dotted.to_string(), value));
        }
        changed
    }

    /// Use `text` as the document a save starts from when the file is
    /// missing or damaged (instead of an empty one), and as the in-memory
    /// document now if it is. `Config::save` uses it so a first save writes
    /// exactly what 1.0.1 wrote.
    pub(crate) fn start_fresh(&mut self, text: &str) -> Result<(), toml_edit::TomlError> {
        let fresh: DocumentMut = text.parse()?;
        if !self.existed || self.damaged.is_some() {
            self.doc = fresh.clone();
        }
        self.fresh = fresh;
        Ok(())
    }

    /// Re-reads the file, re-applies this session's changed keys, then
    /// temp file, fsync, rename. Writes only if bytes changed. A damaged
    /// file is first copied to `config.toml.bak`. Returns whether the file
    /// was written.
    pub fn save_if_changed(&mut self) -> io::Result<bool> {
        let (mut doc, before, damaged_on_disk) = match read_document(&self.path)? {
            OnDisk::Missing => (self.fresh.clone(), None, None),
            OnDisk::Valid { doc, text } => (doc, Some(text), None),
            OnDisk::Damaged { message, text } => (self.fresh.clone(), Some(text), Some(message)),
        };
        for (dotted, value) in &self.changes {
            apply(doc.as_table_mut(), dotted, value.clone());
        }
        let after = doc.to_string();
        // With no change of its own this session writes nothing, so a
        // damaged file is never replaced by an empty one behind the
        // player's back.
        let write = !self.changes.is_empty() && before.as_deref() != Some(after.as_str());
        if write {
            if damaged_on_disk.is_some() {
                // The exact bytes, even when they are not text.
                let damaged = fs::read(&self.path)?;
                write_atomically(&backup_path(&self.path), &damaged)?;
            }
            write_atomically(&self.path, after.as_bytes())?;
            self.damaged = None;
        } else {
            self.damaged = damaged_on_disk;
        }
        self.existed = write || before.is_some();
        self.doc = doc;
        self.changes.clear();
        Ok(write)
    }
}

/// Where a damaged `config.toml` is kept: `config.toml.bak` beside it.
pub fn backup_path(path: &Path) -> PathBuf {
    sibling(path, "bak")
}

/// `name.ext` -> `name.ext.<suffix>` in the same folder.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}

enum OnDisk {
    Missing,
    Valid { doc: DocumentMut, text: String },
    Damaged { message: String, text: String },
}

fn read_document(path: &Path) -> io::Result<OnDisk> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(OnDisk::Missing),
        Err(error) => return Err(error),
    };
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            return Ok(OnDisk::Damaged {
                message: "the file is not UTF-8 text".to_string(),
                text: String::from_utf8_lossy(error.as_bytes()).into_owned(),
            });
        }
    };
    Ok(match text.parse::<DocumentMut>() {
        Ok(doc) => OnDisk::Valid { doc, text },
        Err(error) => OnDisk::Damaged {
            message: error.message().to_string(),
            text,
        },
    })
}

/// Write `bytes` to `path` through a temporary file beside it: write,
/// fsync, rename. The folder is created when missing.
fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    if let Some(parent) = parent {
        fs::create_dir_all(parent)?;
    }
    let temp = sibling(path, "tmp");
    let written = (|| {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temp);
    }
    written?;
    // Make the rename itself durable where the platform allows it.
    #[cfg(unix)]
    if let Ok(folder) = fs::File::open(parent.unwrap_or(Path::new("."))) {
        let _ = folder.sync_all();
    }
    Ok(())
}

/// Set (or remove, for `None`) one dotted key in `root`, creating missing
/// tables. Returns whether anything changed. A value equal to the one
/// already there keeps its original spelling, comment and position.
fn apply(root: &mut Table, dotted: &str, value: Option<Value>) -> bool {
    let parts: Vec<&str> = dotted.split('.').collect();
    let (leaf, sections) = parts.split_last().expect("split yields at least one part");
    let mut table: &mut dyn TableLike = root;
    for (depth, section) in sections.iter().enumerate() {
        let exists = table
            .get(section)
            .is_some_and(|item| item.as_table_like().is_some());
        if !exists {
            if value.is_none() {
                return false;
            }
            let mut created = Table::new();
            // A table that only holds deeper tables needs no header.
            created.set_implicit(depth + 1 < sections.len());
            table.insert(section, Item::Table(created));
        }
        table = table
            .get_mut(section)
            .and_then(Item::as_table_like_mut)
            .expect("the section was just checked or created");
    }
    match value {
        None => table.remove(leaf).is_some_and(|item| !item.is_none()),
        Some(mut value) => {
            if let Some(Item::Value(old)) = table.get_mut(leaf) {
                if same_value(old, &value) {
                    return false;
                }
                // Keep the old value's surrounding spaces and comment.
                *value.decor_mut() = old.decor().clone();
                *old = value;
                return true;
            }
            table.insert(leaf, Item::Value(value));
            true
        }
    }
}

/// Whether two values mean the same thing, however they are spelled
/// (`'a'` and `"a"`, `0x10` and `16`, comments and spacing ignored).
fn same_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(a), Value::String(b)) => a.value() == b.value(),
        (Value::Integer(a), Value::Integer(b)) => a.value() == b.value(),
        (Value::Float(a), Value::Float(b)) => {
            let (a, b) = (*a.value(), *b.value());
            a == b || (a.is_nan() && b.is_nan())
        }
        (Value::Boolean(a), Value::Boolean(b)) => a.value() == b.value(),
        (Value::Datetime(a), Value::Datetime(b)) => a.value() == b.value(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| same_value(a, b))
        }
        (Value::InlineTable(a), Value::InlineTable(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, a)| b.get(key).is_some_and(|b| same_value(a, b)))
        }
        _ => false,
    }
}

fn collect_keys(table: &dyn TableLike, prefix: &str, keys: &mut Vec<String>) {
    for (key, item) in table.iter() {
        let dotted = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        };
        match item.as_table_like() {
            Some(inner) => collect_keys(inner, &dotted, keys),
            None if item.is_value() => keys.push(dotted),
            // Arrays of tables are kept but are not settings keys.
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!(
                "u5-host-settings-{label}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn config(&self) -> PathBuf {
            self.0.join("config.toml")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const WITH_SECTIONS: &str = "\
# Ultima V engine settings. Written by the setup wizard; safe to edit.
# game_dir: the folder you chose. data_dir: the managed copy the game runs from.
game_dir = '/games/u5'
version = \"gog-1.0\"

[enhancements]
preset = \"classic\"   # the player's own comment

[display]
vsync = true
";

    #[test]
    fn missing_file_is_an_empty_document_and_is_not_created_by_a_noop_save() {
        let scratch = Scratch::new("missing");
        let path = scratch.0.join("nested").join("config.toml");
        let mut doc = ConfigDocument::load(&path).unwrap();
        assert!(!doc.exists());
        assert!(doc.damaged().is_none());
        assert!(doc.get("enhancements.preset").is_none());
        assert!(!doc.save_if_changed().unwrap());
        assert!(!path.exists());
    }

    #[test]
    fn get_reads_dotted_keys_in_tables_inline_tables_and_dotted_keys() {
        let scratch = Scratch::new("get");
        let path = scratch.config();
        fs::write(
            &path,
            "top = 1\naudio.volume = 50\n[display]\nvsync = false\nbezel = { color = \"#000\" }\n",
        )
        .unwrap();
        let doc = ConfigDocument::load(&path).unwrap();
        assert_eq!(doc.get("top").unwrap().as_integer(), Some(1));
        assert_eq!(doc.get("audio.volume").unwrap().as_integer(), Some(50));
        assert_eq!(doc.get("display.vsync").unwrap().as_bool(), Some(false));
        assert_eq!(
            doc.get("display.bezel.color").unwrap().as_str(),
            Some("#000")
        );
        assert!(doc.get("display.missing").is_none());
        assert!(doc.get("top.deeper").is_none());
        assert_eq!(
            doc.keys(),
            vec![
                "top",
                "audio.volume",
                "display.vsync",
                "display.bezel.color"
            ]
        );
    }

    #[test]
    fn unchanged_file_round_trips_byte_for_byte() {
        let scratch = Scratch::new("roundtrip");
        let path = scratch.config();
        fs::write(&path, WITH_SECTIONS).unwrap();
        let mut doc = ConfigDocument::load(&path).unwrap();
        assert_eq!(doc.doc.to_string(), WITH_SECTIONS);
        // Same values, spelled differently, are not changes.
        assert!(!doc.set("game_dir", Value::from("/games/u5")));
        assert!(!doc.set("display.vsync", Value::from(true)));
        assert!(!doc.save_if_changed().unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), WITH_SECTIONS);
    }

    #[test]
    fn set_edits_in_place_and_keeps_comments_order_and_other_sections() {
        let scratch = Scratch::new("set");
        let path = scratch.config();
        fs::write(&path, WITH_SECTIONS).unwrap();
        let mut doc = ConfigDocument::load(&path).unwrap();
        assert!(doc.set("enhancements.preset", Value::from("modern")));
        assert!(doc.set("display.scale", Value::from(2)));
        assert!(doc.set("keys.menu", Value::from("F10")));
        assert!(doc.save_if_changed().unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            WITH_SECTIONS
                .replace("\"classic\"", "\"modern\"")
                .replace("vsync = true\n", "vsync = true\nscale = 2\n")
                + "\n[keys]\nmenu = \"F10\"\n"
        );
        // A second save with nothing new does not write.
        assert!(!doc.save_if_changed().unwrap());
    }

    #[test]
    fn nested_sections_are_created_without_empty_headers() {
        let scratch = Scratch::new("nested");
        let path = scratch.config();
        let mut doc = ConfigDocument::load(&path).unwrap();
        assert!(doc.set("cheats.engaged.hp", Value::from(true)));
        assert!(doc.save_if_changed().unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[cheats.engaged]\nhp = true\n"
        );
        assert!(doc.set("cheats.enabled", Value::from(true)));
        assert!(doc.save_if_changed().unwrap());
        let reread = ConfigDocument::load(&path).unwrap();
        assert_eq!(reread.get("cheats.enabled").unwrap().as_bool(), Some(true));
        assert_eq!(
            reread.get("cheats.engaged.hp").unwrap().as_bool(),
            Some(true)
        );
    }

    #[test]
    fn remove_drops_a_key_and_reports_absence() {
        let scratch = Scratch::new("remove");
        let path = scratch.config();
        fs::write(&path, WITH_SECTIONS).unwrap();
        let mut doc = ConfigDocument::load(&path).unwrap();
        assert!(doc.remove("version"));
        assert!(!doc.remove("version"));
        assert!(!doc.remove("nowhere.at_all"));
        assert!(doc.save_if_changed().unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            WITH_SECTIONS.replace("version = \"gog-1.0\"\n", "")
        );
    }

    #[test]
    fn a_hand_edit_made_during_play_survives_a_save() {
        let scratch = Scratch::new("handedit");
        let path = scratch.config();
        fs::write(&path, WITH_SECTIONS).unwrap();
        let mut doc = ConfigDocument::load(&path).unwrap();
        assert!(doc.set("enhancements.preset", Value::from("modern")));
        // The player edits another key while the game runs.
        let edited = WITH_SECTIONS.replace("vsync = true", "vsync = false # tearing is fine");
        fs::write(&path, &edited).unwrap();
        assert!(doc.save_if_changed().unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            edited.replace("\"classic\"", "\"modern\"")
        );
        assert_eq!(doc.get("display.vsync").unwrap().as_bool(), Some(false));
        // The session's change is not re-applied over a later hand edit.
        let later = fs::read_to_string(&path)
            .unwrap()
            .replace("\"modern\"", "\"classic\"");
        fs::write(&path, &later).unwrap();
        assert!(!doc.save_if_changed().unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), later);
    }

    #[test]
    fn a_damaged_file_loads_empty_and_is_backed_up_before_it_is_rewritten() {
        let scratch = Scratch::new("damaged");
        let path = scratch.config();
        fs::write(&path, "game_dir = [\n[display]\nvsync = false\n").unwrap();
        let mut doc = ConfigDocument::load(&path).unwrap();
        assert!(doc.exists());
        assert!(doc.damaged().is_some());
        assert!(doc.get("display.vsync").is_none());
        // Nothing to write: the damaged file is left alone.
        assert!(!doc.save_if_changed().unwrap());
        assert!(!backup_path(&path).exists());

        assert!(doc.set("display.vsync", Value::from(true)));
        assert!(doc.save_if_changed().unwrap());
        assert_eq!(
            fs::read_to_string(backup_path(&path)).unwrap(),
            "game_dir = [\n[display]\nvsync = false\n"
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[display]\nvsync = true\n"
        );
        assert!(doc.damaged().is_none());
        assert_eq!(backup_path(&path), scratch.0.join("config.toml.bak"));
    }

    #[test]
    fn writes_are_atomic_and_leave_no_temporary_file() {
        let scratch = Scratch::new("atomic");
        let path = scratch.config();
        let temp = scratch.0.join("config.toml.tmp");
        // A temporary file left by a crashed write is replaced, not read.
        fs::write(&temp, "half a fi").unwrap();
        let mut doc = ConfigDocument::load(&path).unwrap();
        assert!(doc.set("display.vsync", Value::from(false)));
        assert!(doc.save_if_changed().unwrap());
        assert!(!temp.exists());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[display]\nvsync = false\n"
        );
        let names: Vec<_> = fs::read_dir(&scratch.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("config.toml")]);
    }

    #[test]
    fn a_non_table_in_the_way_is_replaced_by_the_section() {
        let scratch = Scratch::new("shadow");
        let path = scratch.config();
        fs::write(&path, "display = 3\n").unwrap();
        let mut doc = ConfigDocument::load(&path).unwrap();
        assert!(doc.set("display.vsync", Value::from(true)));
        assert_eq!(doc.get("display.vsync").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn same_value_ignores_spelling() {
        let parse = |text: &str| text.parse::<Value>().unwrap();
        assert!(same_value(&parse("'a'"), &parse("\"a\"")));
        assert!(same_value(&parse("0x10"), &parse("16")));
        assert!(same_value(&parse("[1, 2]"), &parse("[ 1,2 ]")));
        assert!(same_value(&parse("{ a = 1, b = 2 }"), &parse("{b=2,a=1}")));
        assert!(!same_value(&parse("1"), &parse("1.0")));
        assert!(!same_value(&parse("[1]"), &parse("[1, 2]")));
        assert!(!same_value(&parse("true"), &parse("\"true\"")));
    }
}
