//! The desktop slot host over a real slot store on scratch folders (no
//! game data): the worker's reply, the listing the pages show, the exact
//! restore and its check, the busy lock, rename and delete, the roll-back
//! of a load whose restart failed, and when a launch gets a host at all.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use u5_runtime::enhancements::{LockReason, Toggle};
use u5_runtime::{SAVE_AVATAR_NAME_OFFSET, SAVED_GAM_FILENAME};

use super::*;

/// A working folder with a save of Avatar `name`, a source folder and a
/// saves folder, all under one scratch folder. Removed on drop.
struct Folders {
    base: PathBuf,
    working: PathBuf,
    source: PathBuf,
}

impl Folders {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "u5-tui-menu-host-{label}-{}-{unique}",
            std::process::id()
        ));
        let working = base.join("game");
        let source = base.join("source");
        fs::create_dir_all(&working).unwrap();
        fs::create_dir_all(&source).unwrap();
        let folders = Self {
            base,
            working,
            source,
        };
        folders.write_save("Ann", 0);
        folders
    }

    /// The working `SAVED.GAM`: Avatar `name` (empty: no game) and one
    /// byte that tells saves apart.
    fn write_save(&self, name: &str, mark: u8) {
        let mut image = vec![0u8; u5_runtime::SAVED_GAM_LEN];
        image[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + name.len()]
            .copy_from_slice(name.as_bytes());
        image[100] = mark;
        fs::write(self.working.join(SAVED_GAM_FILENAME), image).unwrap();
    }

    fn save(&self) -> Vec<u8> {
        fs::read(self.working.join(SAVED_GAM_FILENAME)).unwrap()
    }

    fn store(&self) -> SlotStore {
        SlotStore::open(&self.base.join("saves"), &self.working, &[&self.source])
            .unwrap()
            .with_source_dir(&self.source)
    }

    fn host(&self) -> DesktopMenuHost {
        DesktopMenuHost::new(self.store())
    }
}

impl Drop for Folders {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn request(target: EngineTarget) -> StoreSlot {
    StoreSlot {
        target,
        avatar: "Ann".into(),
        place: "Castle British".into(),
        clock: [139, 4, 7, 9, 15],
        turn: 18234,
        cheats_used: Vec::new(),
        thumbnail_png: Some(vec![0x89, b'P', b'N', b'G', 1, 2, 3]),
    }
}

fn stored(host: &DesktopMenuHost, target: EngineTarget) -> Result<SlotInfo, SlotFailure> {
    host.store_slot(request(target))
        .recv_timeout(Duration::from_secs(20))
        .expect("the worker answers")
}

/// The copy runs on the worker and answers on its channel; the listing
/// then shows the slot as the pages need it, thumbnail bytes included.
#[test]
fn a_store_answers_from_the_worker_and_lists_with_its_details() {
    let folders = Folders::new("store");
    let host = folders.host();
    let info = stored(&host, EngineTarget::New("Despise".into())).unwrap();
    assert_eq!(info.name, "Despise");
    assert!(!info.quick && !info.imported && !info.modified);
    assert_eq!(info.damaged, None);
    let quick = stored(&host, EngineTarget::Quick).unwrap();
    assert!(quick.quick);
    assert_eq!(quick.name, "quick");
    let unnamed = stored(&host, EngineTarget::New(String::new())).unwrap();
    assert!(unnamed.name.starts_with("Slot "), "{}", unnamed.name);

    let listed = host.list_slots().unwrap();
    assert_eq!(listed.len(), 3);
    assert!(listed[0].quick, "the quick slot first");
    let despise = listed.iter().find(|slot| slot.name == "Despise").unwrap();
    assert_eq!(despise.avatar, "Ann");
    assert_eq!(despise.place, "Castle British");
    assert_eq!(despise.clock, [139, 4, 7, 9, 15]);
    assert_eq!(despise.turn, 18234);
    assert_eq!(
        despise.thumbnail_png.as_deref(),
        Some(&[0x89, b'P', b'N', b'G', 1, 2, 3][..])
    );
    // Writing over it keeps its name and id.
    let again = stored(&host, EngineTarget::Existing(despise.id.clone())).unwrap();
    assert_eq!(
        (again.id, again.name),
        (despise.id.clone(), "Despise".into())
    );
    // An id the store never made is refused.
    let error = stored(&host, EngineTarget::Existing(SlotId("../x".into()))).unwrap_err();
    assert!(matches!(error, SlotFailure::Reason(_)));
}

/// The restore is exact and checked; a slot whose save holds no game is
/// refused and the working set left as it was (3.3.5 "Restore").
#[test]
fn a_restore_is_exact_and_checked() {
    let folders = Folders::new("restore");
    let host = folders.host();
    let slot = stored(&host, EngineTarget::New("A".into())).unwrap();
    let at_save = folders.save();
    folders.write_save("Ann", 9);
    fs::write(folders.working.join("SAVED.WPS"), [1, 2]).unwrap();
    host.restore_slot(&slot.id).unwrap();
    assert_eq!(folders.save(), at_save);
    assert!(
        !folders.working.join("SAVED.WPS").exists(),
        "what the slot lacks is deleted"
    );
    // A slot of an empty save.
    folders.write_save("", 0);
    let empty = stored(&host, EngineTarget::New("Empty".into())).unwrap();
    folders.write_save("Ann", 7);
    let before = folders.save();
    let error = host.restore_slot(&empty.id).unwrap_err();
    assert!(
        matches!(&error, SlotFailure::Reason(reason) if reason.contains("no active game")),
        "{error:?}"
    );
    assert_eq!(folders.save(), before, "rolled back");
    // The roll-back undoes the last load.
    host.restore_slot(&slot.id).unwrap();
    host.roll_back_restore().unwrap();
    assert_eq!(folders.save(), before);
}

/// Another copy holding the lock is "busy", never a reason.
#[test]
fn a_held_lock_is_busy() {
    let folders = Folders::new("busy");
    let host = DesktopMenuHost::new(folders.store().with_lock_wait(Duration::from_millis(50)));
    let root = host.store().root().to_path_buf();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(u5_host_desktop::slots::LOCK_FILE), b"1").unwrap();
    assert_eq!(host.list_slots(), Err(SlotFailure::Busy));
    assert_eq!(
        stored(&host, EngineTarget::Quick).unwrap_err(),
        SlotFailure::Busy
    );
    assert_eq!(
        host.delete_slot(&SlotId("quick".into())),
        Err(SlotFailure::Busy)
    );
}

#[test]
fn rename_and_delete_go_to_the_store() {
    let folders = Folders::new("rename");
    let host = folders.host();
    let slot = stored(&host, EngineTarget::New("Old".into())).unwrap();
    host.rename_slot(&slot.id, "New name").unwrap();
    assert_eq!(host.list_slots().unwrap()[0].name, "New name");
    let quick = stored(&host, EngineTarget::Quick).unwrap();
    assert!(matches!(
        host.rename_slot(&quick.id, "Mine"),
        Err(SlotFailure::Reason(_))
    ));
    host.delete_slot(&slot.id).unwrap();
    let names: Vec<String> = host
        .list_slots()
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, ["quick"]);
}

/// A load's restart that cannot start rolls the load back and says so;
/// any other restart only reports (3.2.3 "A failed relaunch").
#[test]
fn a_failed_restart_undoes_the_load() {
    let folders = Folders::new("relaunch");
    let host: Arc<dyn MenuHost> = Arc::new(folders.host());
    let slot = stored_dyn(&host, "A");
    folders.write_save("Ann", 5);
    let before = folders.save();
    host.restore_slot(&slot).unwrap();
    assert_ne!(folders.save(), before);
    let failed = crate::relaunch::SpawnFailed(io::Error::other("no such program"));
    let load = u5_bevy::Relaunch {
        boot_key: Some(b'J'),
    };
    let error = relaunch_failed(load, &failed, Some(&host));
    assert_eq!(folders.save(), before, "the game is unchanged");
    assert!(error.to_string().contains("could not restart the game"));
    assert!(error.to_string().contains("the load was undone"));
    // A restart to the title touches nothing.
    host.restore_slot(&slot).unwrap();
    let restored = folders.save();
    let title = u5_bevy::Relaunch { boot_key: None };
    let error = relaunch_failed(title, &failed, Some(&host));
    assert_eq!(folders.save(), restored);
    assert!(!error.to_string().contains("undone"));
    // No restart asked: nothing happens.
    relaunch_after_shell(u5_bevy::ShellExit::default(), Some(&host), None).unwrap();
}

fn stored_dyn(host: &Arc<dyn MenuHost>, name: &str) -> SlotId {
    host.store_slot(request(EngineTarget::New(name.into())))
        .recv_timeout(Duration::from_secs(20))
        .unwrap()
        .unwrap()
        .id
}

/// A launch gets a slot host only with named slots, a menu and no lock.
#[test]
fn only_a_slots_launch_gets_a_host() {
    let folders = Folders::new("attach");
    let with = |toggles: &[Toggle], lock: LockState| {
        let mut launch = u5_bevy::VisualLaunchConfig::classic();
        for toggle in toggles {
            launch.enhancements.toggles.set(*toggle, true);
        }
        launch.enhancements.lock = lock;
        attach_menu_host(&mut launch, &folders.working, &folders.source);
        launch.menu_host.is_some()
    };
    let off = LockState::Off;
    assert!(!with(&[], off));
    assert!(!with(&[Toggle::MenuNamedSlots], off), "no menu");
    assert!(!with(&[Toggle::MenuOverlay], off), "no slots");
    assert!(with(&[Toggle::MenuOverlay, Toggle::MenuNamedSlots], off));
    assert!(with(&[Toggle::MenuTitleLayer, Toggle::MenuNamedSlots], off));
    assert!(!with(
        &[Toggle::MenuOverlay, Toggle::MenuNamedSlots],
        LockState::Locked(LockReason::CliFlag)
    ));
}
