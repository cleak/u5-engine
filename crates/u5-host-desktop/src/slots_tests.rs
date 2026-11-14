//! Tests of the slot store (design 3.3.5; build plan M4-09). Every working
//! set here is synthetic bytes: no game data is read.

use super::*;
use std::sync::{Arc, Mutex};

type Snapshot = BTreeMap<String, Vec<u8>>;

fn scratch(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "u5-host-slots-{label}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct Fixture {
    working: PathBuf,
    saves: PathBuf,
    /// Stands in for the pristine source tree.
    pristine: PathBuf,
}

fn fixture(label: &str) -> Fixture {
    let base = scratch(label);
    let working = base.join("game");
    let pristine = base.join("pristine");
    fs::create_dir_all(&working).unwrap();
    fs::create_dir_all(&pristine).unwrap();
    // A file that is not part of the working set: never touched.
    fs::write(working.join("TILES.16"), [7u8; 64]).unwrap();
    fs::write(pristine.join("SAVED.GAM"), image(0xEE)).unwrap();
    Fixture {
        working,
        saves: base.join("data").join("saves"),
        pristine,
    }
}

/// A synthetic 4,192-byte image.
fn image(seed: u8) -> Vec<u8> {
    (0..SAVE_IMAGE_LEN as usize)
        .map(|i| seed.wrapping_add(i as u8))
        .collect()
}

/// Writes a working set: the DOS quartet always; with `extras`, the two
/// engine files and a sidecar folder with a nested file.
fn write_set(dir: &Path, seed: u8, extras: bool) {
    fs::write(dir.join("SAVED.GAM"), image(seed)).unwrap();
    fs::write(dir.join("SAVED.OOL"), [seed; 32]).unwrap();
    fs::write(dir.join("BRIT.OOL"), [seed ^ 1; 256]).unwrap();
    fs::write(dir.join("UNDER.OOL"), [seed ^ 2; 256]).unwrap();
    if extras {
        fs::write(dir.join("SAVED.WPS"), [seed ^ 3; 12]).unwrap();
        fs::write(dir.join(".u5-engine-town-npc-mutations"), [seed ^ 4; 9]).unwrap();
        let sidecar = dir.join(SIDECAR_DIR_NAME);
        fs::create_dir_all(sidecar.join("deep")).unwrap();
        fs::write(sidecar.join("automap.u5e"), [seed ^ 5; 40]).unwrap();
        fs::write(sidecar.join("deep").join("journal.u5e"), [seed ^ 6; 8]).unwrap();
    }
}

/// Every file under `dir`, by relative path.
fn snapshot(dir: &Path) -> Snapshot {
    fn walk(dir: &Path, prefix: &str, out: &mut Snapshot) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let key = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if entry.file_type().unwrap().is_dir() {
                walk(&entry.path(), &format!("{key}/"), out);
            } else {
                out.insert(key, fs::read(entry.path()).unwrap());
            }
        }
    }
    let mut out = Snapshot::new();
    walk(dir, "", &mut out);
    out
}

/// The working-set part of a slot folder, keyed as in the working folder.
fn slot_set(dir: &Path) -> Snapshot {
    let mut out = snapshot(dir);
    out.remove(SLOT_FILE);
    out.remove(THUMB_FILE);
    out
}

/// The working-set part of a working folder.
fn working_set(dir: &Path) -> Snapshot {
    let mut out = snapshot(dir);
    out.remove("TILES.16");
    out
}

fn open(f: &Fixture) -> SlotStore {
    SlotStore::open(&f.saves, &f.working, &[&f.pristine])
        .unwrap()
        .with_lock_wait(Duration::from_millis(100))
}

fn request(target: SlotTarget) -> StoreRequest {
    StoreRequest {
        target,
        origin: SlotOrigin::Menu,
        engine: "test".to_string(),
        details: SlotDetails {
            avatar: "Tester".to_string(),
            place: "Britannia".to_string(),
            clock: [139, 4, 7, 9, 15],
            turn: 18234,
            modified: false,
            cheats_used: Vec::new(),
        },
        thumbnail_png: Some(vec![0x89, b'P', b'N', b'G']),
    }
}

fn ok(_: &Path) -> Result<(), String> {
    Ok(())
}

fn crash_at(step: &'static str) -> FaultHook {
    Arc::new(move |at| at == step)
}

/// Makes a crashed writer's lock look 31 s old, past the stale rule.
fn age_lock(store: &SlotStore) {
    let file = OpenOptions::new()
        .write(true)
        .open(store.root().join(LOCK_FILE))
        .unwrap();
    file.set_modified(SystemTime::now() - Duration::from_secs(31))
        .unwrap();
}

fn no_leftovers(store: &SlotStore) {
    let slots = store.root().join(SLOTS_DIR);
    if slots.is_dir() {
        for entry in fs::read_dir(&slots).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            assert!(!name.starts_with('.'), "leftover {name}");
        }
    }
    for name in fs::read_dir(store.working_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
    {
        assert!(!name.contains(".u5slot."), "leftover {name}");
    }
    assert!(!store.root().join(RESTORE_MARK).exists());
    assert!(!store.root().join(LOCK_FILE).exists());
}

#[test]
fn a_synthetic_working_set_round_trips() {
    let f = fixture("round-trip");
    write_set(&f.working, 10, true);
    let a = working_set(&f.working);
    let store = open(&f);
    let info = store
        .store(&request(SlotTarget::New("Despise".into())))
        .unwrap();
    assert_eq!(info.id.as_str(), "s0001");
    assert_eq!(info.meta.name, "Despise");
    assert_eq!(info.meta.format, SLOT_FORMAT);
    assert_eq!(info.meta.clock, [139, 4, 7, 9, 15]);
    assert!(info.damaged.is_none());
    assert!(info.has_thumbnail);
    // Six files and two sidecar files, each with its digest.
    assert_eq!(info.meta.sha256.len(), 8);
    assert!(
        info.meta
            .sha256
            .contains_key(".u5-engine-enh/deep/journal.u5e")
    );
    assert_eq!(slot_set(&store.root().join("slots/s0001")), a);
    // The store reads the working set only.
    assert_eq!(working_set(&f.working), a);

    write_set(&f.working, 20, true);
    store.restore(&info.id, &ok).unwrap();
    assert_eq!(working_set(&f.working), a);
    assert_eq!(fs::read(f.working.join("TILES.16")).unwrap(), [7u8; 64]);
    no_leftovers(&store);
}

#[test]
fn slot_toml_is_format_1_and_ignores_unknown_keys() {
    let f = fixture("toml");
    write_set(&f.working, 1, false);
    let store = open(&f);
    let info = store.store(&request(SlotTarget::Quick)).unwrap();
    let path = store.root().join("slots/quick").join(SLOT_FILE);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("format = 1\n"), "{text}");
    assert!(text.contains("[sha256]"));
    let edited = format!("future_key = \"x\"\n{text}\n[future_table]\nanswer = 42\n");
    fs::write(&path, edited).unwrap();
    let listed = store.find("quick").unwrap();
    assert!(listed.damaged.is_none(), "{:?}", listed.damaged);
    assert_eq!(listed.meta, info.meta);
}

#[test]
fn names_ids_order_and_lookup() {
    let f = fixture("names");
    write_set(&f.working, 1, false);
    let store = open(&f);
    let first = store.store(&request(SlotTarget::New("  ".into()))).unwrap();
    assert_eq!(
        (first.id.as_str(), first.meta.name.as_str()),
        ("s0001", "Slot 1")
    );
    let second = store
        .store(&request(SlotTarget::New("Minoc ship".into())))
        .unwrap();
    assert_eq!(second.id.as_str(), "s0002");
    store.store(&request(SlotTarget::Quick)).unwrap();
    let order: Vec<String> = store
        .list()
        .unwrap()
        .iter()
        .map(|slot| slot.id.to_string())
        .collect();
    assert_eq!(order, ["quick", "s0002", "s0001"]);
    assert_eq!(store.find("MINOC SHIP").unwrap().id, second.id);
    assert_eq!(store.find("s0001").unwrap().id, first.id);
    assert!(matches!(store.find("Nowhere"), Err(SlotError::NotFound(_))));
    for bad in ["x".repeat(SLOT_NAME_MAX + 1), "Caf\u{e9}".to_string()] {
        assert!(matches!(
            store.store(&request(SlotTarget::New(bad))),
            Err(SlotError::InvalidName(_))
        ));
    }
    assert!(check_name(&"y".repeat(SLOT_NAME_MAX)).is_ok());
}

#[test]
fn rename_and_delete() {
    let f = fixture("rename");
    write_set(&f.working, 3, true);
    let store = open(&f);
    let info = store
        .store(&request(SlotTarget::New("Despise".into())))
        .unwrap();
    let renamed = store.rename(&info.id, "Shame").unwrap();
    assert_eq!(renamed.id, info.id);
    assert_eq!(renamed.meta.name, "Shame");
    assert_eq!(renamed.meta.sha256, info.meta.sha256);
    assert!(renamed.damaged.is_none());
    store.store(&request(SlotTarget::Quick)).unwrap();
    assert!(matches!(
        store.rename(&SlotId::quick(), "Other"),
        Err(SlotError::InvalidName(_))
    ));
    store.delete(&info.id).unwrap();
    assert!(matches!(store.find("Shame"), Err(SlotError::NotFound(_))));
    assert!(matches!(
        store.delete(&info.id),
        Err(SlotError::NotFound(_))
    ));
    no_leftovers(&store);
}

#[test]
fn overwriting_keeps_the_id_and_name_and_drops_what_is_gone() {
    let f = fixture("overwrite");
    write_set(&f.working, 4, true);
    let store = open(&f);
    let info = store
        .store(&request(SlotTarget::New("Despise".into())))
        .unwrap();
    fs::remove_dir_all(f.working.join(SIDECAR_DIR_NAME)).unwrap();
    write_set(&f.working, 5, false);
    fs::remove_file(f.working.join("SAVED.WPS")).unwrap();
    let mut again = request(SlotTarget::Existing(info.id.clone()));
    again.thumbnail_png = None;
    let over = store.store(&again).unwrap();
    assert_eq!((over.id, over.meta.name.as_str()), (info.id, "Despise"));
    assert!(!over.has_thumbnail);
    let slot = store.root().join("slots/s0001");
    assert!(!slot.join("SAVED.WPS").exists());
    assert!(!slot.join(SIDECAR_DIR_NAME).exists());
    assert_eq!(slot_set(&slot), working_set(&f.working));
}

/// A crash at each store step leaves the old slot or the new one, never a
/// mix; the next listing repairs the leftovers.
#[test]
fn a_crash_at_each_store_step_keeps_one_whole_slot() {
    for (index, step) in STORE_STEPS.iter().enumerate() {
        let f = fixture("store-crash");
        write_set(&f.working, 30, true);
        let old = working_set(&f.working);
        let store = open(&f);
        let info = store
            .store(&request(SlotTarget::New("Despise".into())))
            .unwrap();
        write_set(&f.working, 40, false);
        fs::remove_dir_all(f.working.join(SIDECAR_DIR_NAME)).unwrap();
        let new = working_set(&f.working);
        let crashing = open(&f).with_fault(crash_at(step));
        let result = crashing.store(&request(SlotTarget::Existing(info.id.clone())));
        assert!(matches!(result, Err(SlotError::Crashed(at)) if at == *step));
        // The store never writes the working set.
        assert_eq!(working_set(&f.working), new);
        // The crashed writer's lock refuses everyone until it is stale.
        assert!(matches!(store.list(), Err(SlotError::Busy)), "{step}");
        age_lock(&store);
        let slots = store.list().unwrap();
        assert_eq!(slots.len(), 1, "{step}");
        assert!(slots[0].damaged.is_none(), "{step}: {:?}", slots[0].damaged);
        let kept = slot_set(&store.root().join("slots/s0001"));
        let placed = STORE_STEPS
            .iter()
            .position(|s| *s == "store.placed")
            .unwrap();
        let expected = if index >= placed { &new } else { &old };
        assert_eq!(&kept, expected, "{step}");
        no_leftovers(&store);
    }
}

/// The same for a new slot: before the rename into place there is no
/// slot, after it the whole new one.
#[test]
fn a_crash_while_making_a_new_slot_leaves_none_or_all_of_it() {
    for (index, step) in STORE_STEPS.iter().enumerate() {
        let f = fixture("new-crash");
        write_set(&f.working, 50, true);
        let set = working_set(&f.working);
        let crashing = open(&f).with_fault(crash_at(step));
        assert!(crashing.store(&request(SlotTarget::Quick)).is_err());
        let store = open(&f);
        age_lock(&store);
        let slots = store.list().unwrap();
        let placed = STORE_STEPS
            .iter()
            .position(|s| *s == "store.placed")
            .unwrap();
        if index >= placed {
            assert_eq!(slots.len(), 1, "{step}");
            assert_eq!(slot_set(&store.root().join("slots/quick")), set);
        } else {
            assert!(slots.is_empty(), "{step}");
        }
        no_leftovers(&store);
    }
}

/// A crash at each restore step leaves the working set as it was or
/// exactly the slot; the next operation rolls a half-done restore back.
#[test]
fn a_crash_at_each_restore_step_keeps_one_whole_working_set() {
    for step in RESTORE_STEPS {
        let f = fixture("restore-crash");
        write_set(&f.working, 60, false);
        let slot = working_set(&f.working);
        let store = open(&f);
        let info = store
            .store(&request(SlotTarget::New("Despise".into())))
            .unwrap();
        write_set(&f.working, 70, true);
        let before = working_set(&f.working);
        let crashing = open(&f).with_fault(crash_at(step));
        let result = crashing.restore(&info.id, &ok);
        assert!(matches!(result, Err(SlotError::Crashed(_))), "{step}");
        age_lock(&store);
        store.list().unwrap();
        let after = working_set(&f.working);
        let expected = if *step == "restore.unmarked" {
            &slot
        } else {
            &before
        };
        assert_eq!(&after, expected, "{step}");
        no_leftovers(&store);
    }
}

/// A crash part way through a roll-back is finished by the next repair.
#[test]
fn a_crash_during_a_roll_back_is_finished_later() {
    for step in [
        "rollback.file.0",
        "rollback.file.3",
        "rollback.file.5",
        "rollback.sidecar",
    ] {
        let f = fixture("rollback-crash");
        write_set(&f.working, 80, false);
        let store = open(&f);
        let info = store.store(&request(SlotTarget::Quick)).unwrap();
        write_set(&f.working, 90, true);
        let before = working_set(&f.working);
        let crashing = open(&f).with_fault(crash_at(step));
        let refuse = |_: &Path| Err("not a save".to_string());
        assert!(matches!(
            crashing.restore(&info.id, &refuse),
            Err(SlotError::Crashed(_))
        ));
        age_lock(&store);
        store.list().unwrap();
        assert_eq!(working_set(&f.working), before, "{step}");
        no_leftovers(&store);
    }
}

/// A slot imported from a DOS save holds only the quartet; restoring it
/// deletes the later playthrough's SAVED.WPS, town mutations and sidecars.
#[test]
fn a_dos_import_restore_deletes_what_the_slot_lacks() {
    let f = fixture("dos-import");
    write_set(&f.working, 100, false);
    let dos = working_set(&f.working);
    let store = open(&f);
    let mut import = request(SlotTarget::New("From DOS".into()));
    import.origin = SlotOrigin::Import;
    import.thumbnail_png = None;
    let info = store.store(&import).unwrap();
    assert_eq!(info.meta.origin, SlotOrigin::Import);
    assert!(!info.has_thumbnail);
    assert_eq!(info.meta.sha256.len(), 4);

    write_set(&f.working, 110, true);
    assert!(f.working.join("SAVED.WPS").exists());
    store.restore(&info.id, &ok).unwrap();
    assert!(!f.working.join("SAVED.WPS").exists());
    assert!(!f.working.join(".u5-engine-town-npc-mutations").exists());
    assert!(!f.working.join(SIDECAR_DIR_NAME).exists());
    assert_eq!(working_set(&f.working), dos);
    no_leftovers(&store);
}

/// A failed check after the copy rolls back byte for byte, files the slot
/// deleted and the sidecar folder included; an explicit roll-back after a
/// good restore does the same.
#[test]
fn a_failed_restore_rolls_back_byte_for_byte() {
    let f = fixture("rollback");
    write_set(&f.working, 120, false);
    let store = open(&f);
    let info = store.store(&request(SlotTarget::Quick)).unwrap();
    write_set(&f.working, 130, true);
    let before = working_set(&f.working);
    let refuse = |dir: &Path| {
        // The check sees the slot's files in place.
        assert!(!dir.join("SAVED.WPS").exists());
        Err("the save has no active Avatar".to_string())
    };
    let error = store.restore(&info.id, &refuse).unwrap_err();
    assert!(matches!(error, SlotError::Invalid(_)));
    assert_eq!(error.to_string(), "the save has no active Avatar");
    assert_eq!(working_set(&f.working), before);
    no_leftovers(&store);

    store.restore(&info.id, &ok).unwrap();
    assert_ne!(working_set(&f.working), before);
    store.roll_back_restore().unwrap();
    assert_eq!(working_set(&f.working), before);
    no_leftovers(&store);
}

#[test]
fn roll_back_without_a_restore_is_refused() {
    let f = fixture("no-backup");
    write_set(&f.working, 1, false);
    assert!(matches!(
        open(&f).roll_back_restore(),
        Err(SlotError::Refused(_))
    ));
}

/// Damaged slots (a missing file, the wrong length, a digest mismatch, an
/// unreadable or newer `slot.toml`, an unlisted file) are listed and
/// refused, and can still be deleted.
#[test]
fn damaged_slots_are_listed_and_refused() {
    type Damage = fn(&Path);
    let cases: [(&str, Damage); 7] = [
        ("BRIT.OOL is missing", |dir| {
            fs::remove_file(dir.join("BRIT.OOL")).unwrap();
        }),
        ("SAVED.GAM has the wrong length", |dir| {
            fs::write(dir.join("SAVED.GAM"), [1u8; 100]).unwrap();
        }),
        ("SAVED.OOL does not match its digest", |dir| {
            fs::write(dir.join("SAVED.OOL"), [0xAB; 32]).unwrap();
        }),
        (
            ".u5-engine-enh/automap.u5e does not match its digest",
            |dir| {
                fs::write(dir.join(SIDECAR_DIR_NAME).join("automap.u5e"), [0; 40]).unwrap();
            },
        ),
        (
            ".u5-engine-enh/extra.u5e is not listed in slot.toml",
            |dir| {
                fs::write(dir.join(SIDECAR_DIR_NAME).join("extra.u5e"), [0; 4]).unwrap();
            },
        ),
        ("slot.toml is not valid", |dir| {
            fs::write(dir.join(SLOT_FILE), "format = [").unwrap();
        }),
        ("it was made by a newer version", |dir| {
            let path = dir.join(SLOT_FILE);
            let text = fs::read_to_string(&path).unwrap();
            fs::write(&path, text.replacen("format = 1", "format = 2", 1)).unwrap();
        }),
    ];
    for (reason, damage) in cases {
        let f = fixture("damaged");
        write_set(&f.working, 140, true);
        let store = open(&f);
        let info = store
            .store(&request(SlotTarget::New("Despise".into())))
            .unwrap();
        damage(&store.root().join("slots").join(info.id.as_str()));
        write_set(&f.working, 150, true);
        let before = working_set(&f.working);
        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].damaged.as_deref(), Some(reason));
        let error = store.restore(&info.id, &ok).unwrap_err();
        assert!(matches!(error, SlotError::Damaged { .. }), "{reason}");
        assert!(error.to_string().contains("damaged"), "{error}");
        assert_eq!(working_set(&f.working), before, "{reason}");
        assert!(!store.root().join(BACKUP_DIR).exists());
        store.delete(&info.id).unwrap();
        assert!(store.list().unwrap().is_empty());
    }
}

/// Two writers on one slot root (two copies of the game) take turns.
#[test]
fn two_writers_serialise_on_the_lock() {
    let f = fixture("writers");
    write_set(&f.working, 160, true);
    let events: Arc<Mutex<Vec<String>>> = Arc::default();
    let recorder = |who: &'static str, pause: bool| -> FaultHook {
        let events = Arc::clone(&events);
        Arc::new(move |step| {
            events.lock().unwrap().push(format!("{who} {step}"));
            if pause && step == "store.copied" {
                std::thread::sleep(Duration::from_millis(300));
            }
            false
        })
    };
    let first = SlotStore::open(&f.saves, &f.working, &[])
        .unwrap()
        .with_fault(recorder("a", true));
    let second = SlotStore::open(&f.saves, &f.working, &[])
        .unwrap()
        .with_fault(recorder("b", false));
    let a = std::thread::spawn(move || first.store(&request(SlotTarget::New("A".into()))));
    std::thread::sleep(Duration::from_millis(50));
    let b = std::thread::spawn(move || second.store(&request(SlotTarget::New("B".into()))));
    let a = a.join().unwrap().unwrap();
    let b = b.join().unwrap().unwrap();
    assert_eq!((a.id.as_str(), b.id.as_str()), ("s0001", "s0002"));
    let events = events.lock().unwrap().clone();
    let a_steps: Vec<String> = STORE_STEPS.iter().map(|s| format!("a {s}")).collect();
    let b_steps: Vec<String> = STORE_STEPS.iter().map(|s| format!("b {s}")).collect();
    assert_eq!(events, [a_steps, b_steps].concat());
}

/// A live lock refuses after the wait with the player's message; a stale
/// one is broken.
#[test]
fn a_held_lock_refuses_and_a_stale_one_is_broken() {
    let f = fixture("lock");
    write_set(&f.working, 170, false);
    let store = open(&f);
    fs::create_dir_all(store.root()).unwrap();
    fs::write(store.root().join(LOCK_FILE), "pid 1\n").unwrap();
    let started = Instant::now();
    let error = store.store(&request(SlotTarget::Quick)).unwrap_err();
    assert!(started.elapsed() >= Duration::from_millis(100));
    assert!(matches!(error, SlotError::Busy));
    assert_eq!(
        error.to_string(),
        "Slots are busy (another copy of the game?)"
    );
    assert!(store.root().join(LOCK_FILE).exists());
    age_lock(&store);
    store.store(&request(SlotTarget::Quick)).unwrap();
    assert!(!store.root().join(LOCK_FILE).exists());
}

/// Slots never land in the game folder, the pristine tree or the mirror.
#[test]
fn slots_never_land_in_game_pristine_or_mirror_folders() {
    let f = fixture("fences");
    write_set(&f.working, 180, true);
    let mirror_root = f.working.parent().unwrap().join("runtime");
    fs::create_dir_all(&mirror_root).unwrap();
    for inside in [
        f.working.join("saves"),
        f.pristine.join("deep").join("saves"),
        mirror_root.join("saves"),
    ] {
        let refused = SlotStore::open(&inside, &f.working, &[&f.pristine, &mirror_root]);
        assert!(
            matches!(refused, Err(SlotError::Refused(_))),
            "{}",
            inside.display()
        );
        assert!(!inside.exists(), "{}", inside.display());
    }

    let pristine = snapshot(&f.pristine);
    let store = SlotStore::open(&f.saves, &f.working, &[&f.pristine, &mirror_root]).unwrap();
    assert_eq!(
        store.root(),
        lenient_canonical(&f.saves).join(work_key(&f.working.canonicalize().unwrap()))
    );
    let before: Vec<_> = fs::read_dir(&f.working)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    let info = store.store(&request(SlotTarget::Quick)).unwrap();
    store.restore(&info.id, &ok).unwrap();
    let mut after: Vec<_> = fs::read_dir(&f.working)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    let mut before = before;
    before.sort();
    after.sort();
    assert_eq!(before, after);
    assert_eq!(snapshot(&f.pristine), pristine);
    assert!(snapshot(&mirror_root).is_empty());
    assert!(store.root().join("slots/quick/SAVED.GAM").is_file());
}

/// A working set spelled in lower case is copied by its canonical names
/// and restored over the same files.
#[test]
fn working_file_case_is_kept() {
    let f = fixture("case");
    fs::write(f.working.join("saved.gam"), image(1)).unwrap();
    fs::write(f.working.join("Saved.Ool"), [1; 32]).unwrap();
    let store = open(&f);
    let info = store.store(&request(SlotTarget::Quick)).unwrap();
    assert!(info.meta.sha256.contains_key("SAVED.GAM"));
    assert!(info.meta.sha256.contains_key("SAVED.OOL"));
    fs::write(f.working.join("saved.gam"), image(2)).unwrap();
    store.restore(&info.id, &ok).unwrap();
    assert_eq!(fs::read(f.working.join("saved.gam")).unwrap(), image(1));
    let names: Vec<String> = fs::read_dir(&f.working)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!names.contains(&"SAVED.GAM".to_string()), "{names:?}");
}

#[test]
fn a_working_set_without_a_save_is_refused() {
    let f = fixture("empty");
    let store = open(&f);
    assert!(matches!(
        store.store(&request(SlotTarget::Quick)),
        Err(SlotError::Refused(_))
    ));
    assert!(store.list().unwrap().is_empty());
}
