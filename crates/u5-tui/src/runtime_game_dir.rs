//! Writable runtime-directory preparation for interactive play.
//!
//! The clean asset install is a read-only input. Ultima V's save/load flow,
//! however, updates `SAVED.*` and the two plane mirrors in the same directory
//! as the data files. Interactive frontends therefore run from a small,
//! persistent writable mirror instead of attempting to modify the pristine
//! install.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use u5_runtime::{
    BRIT_OOL_FILENAME, DEFAULT_GAME_DIR, SAVED_GAM_FILENAME, SAVED_OOL_FILENAME,
    TOWN_NPC_MUTATIONS_FILENAME, UNDER_OOL_FILENAME, WORLD_PROGRESS_STATE_FILE,
};

pub const RUNTIME_DIR_ENV: &str = "U5_ENGINE_RUNTIME_DIR";

/// Return a writable game directory suitable for interactive play.
///
/// Writable fixture/install directories are used as-is. The pristine default
/// install and directories containing read-only mutable save files are mirrored
/// below the platform's local application-data directory. The mirror retains
/// runtime save files between launches while refreshing immutable assets whose
/// source length or modification time changed.
pub fn prepare_writable_game_dir(game_dir: &Path) -> io::Result<PathBuf> {
    if !requires_writable_mirror(game_dir)? {
        return Ok(game_dir.to_path_buf());
    }
    let root = runtime_root();
    prepare_writable_game_dir_in(game_dir, &root)
}

fn prepare_writable_game_dir_in(game_dir: &Path, root: &Path) -> io::Result<PathBuf> {
    let source = game_dir.canonicalize().map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("game directory {}: {err}", game_dir.display()),
        )
    })?;
    if !source.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("game directory is not a directory: {}", source.display()),
        ));
    }

    let target = root.join(runtime_directory_name(&source));
    fs::create_dir_all(&target)?;
    mirror_directory(&source, &target)?;
    Ok(target)
}

fn runtime_root() -> PathBuf {
    if let Some(path) = env::var_os(RUNTIME_DIR_ENV).filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }
    if let Some(path) = env::var_os("LOCALAPPDATA").filter(|value| !value.is_empty()) {
        return PathBuf::from(path).join("u5-engine").join("runtime");
    }
    if let Some(path) = env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(path).join("u5-engine").join("runtime");
    }
    env::temp_dir().join("u5-engine-runtime")
}

fn requires_writable_mirror(game_dir: &Path) -> io::Result<bool> {
    if same_path(game_dir, Path::new(DEFAULT_GAME_DIR)) {
        return Ok(true);
    }

    for name in [
        SAVED_GAM_FILENAME,
        SAVED_OOL_FILENAME,
        BRIT_OOL_FILENAME,
        UNDER_OOL_FILENAME,
    ] {
        match fs::metadata(game_dir.join(name)) {
            Ok(metadata) if metadata.permissions().readonly() => return Ok(true),
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    Ok(false)
}

fn same_path(left: &Path, right: &Path) -> bool {
    fn normalized(path: &Path) -> String {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_ascii_lowercase()
    }
    normalized(left) == normalized(right)
}

fn runtime_directory_name(source: &Path) -> String {
    let label = source
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("ultima-v")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let normalized = source
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    format!("{label}-{:016x}", fnv1a64(normalized.as_bytes()))
}

const fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        index += 1;
    }
    hash
}

fn mirror_directory(source: &Path, target: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            fs::create_dir_all(&target_path)?;
            mirror_directory(&source_path, &target_path)?;
        } else if file_type.is_file()
            && (!target_path.exists()
                || (!is_mutable_runtime_file(&entry.file_name())
                    && source_file_changed(&source_path, &target_path)?))
        {
            copy_writable(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn source_file_changed(source: &Path, target: &Path) -> io::Result<bool> {
    let source_metadata = fs::metadata(source)?;
    let target_metadata = fs::metadata(target)?;
    if source_metadata.len() != target_metadata.len() {
        return Ok(true);
    }
    Ok(fs::read(source)? != fs::read(target)?)
}

fn is_mutable_runtime_file(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    [
        SAVED_GAM_FILENAME,
        SAVED_OOL_FILENAME,
        BRIT_OOL_FILENAME,
        UNDER_OOL_FILENAME,
        WORLD_PROGRESS_STATE_FILE,
        TOWN_NPC_MUTATIONS_FILENAME,
    ]
    .iter()
    .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn copy_writable(source: &Path, target: &Path) -> io::Result<()> {
    if target.exists() {
        clear_readonly(target)?;
    }
    fs::copy(source, target)?;
    clear_readonly(target)
}

fn clear_readonly(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    if permissions.readonly() {
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            env::temp_dir().join(format!("u5-engine-{label}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn readonly_install_is_mirrored_and_runtime_saves_are_preserved() {
        let source = temp_dir("readonly-source");
        let root = temp_dir("runtime-root");
        let static_path = source.join("IBM.CH");
        let save_path = source.join(SAVED_GAM_FILENAME);
        fs::write(&static_path, b"font-v1").unwrap();
        fs::write(&save_path, b"source-save").unwrap();
        let mut permissions = fs::metadata(&save_path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&save_path, permissions).unwrap();

        assert!(requires_writable_mirror(&source).unwrap());
        let runtime = prepare_writable_game_dir_in(&source, &root).unwrap();
        assert_eq!(fs::read(runtime.join("IBM.CH")).unwrap(), b"font-v1");
        assert_eq!(
            fs::read(runtime.join(SAVED_GAM_FILENAME)).unwrap(),
            b"source-save"
        );
        assert!(
            !fs::metadata(runtime.join(SAVED_GAM_FILENAME))
                .unwrap()
                .permissions()
                .readonly()
        );

        fs::write(runtime.join(SAVED_GAM_FILENAME), b"played-save").unwrap();
        fs::write(&static_path, b"font-v2-longer").unwrap();
        let second = prepare_writable_game_dir_in(&source, &root).unwrap();
        assert_eq!(second, runtime);
        assert_eq!(fs::read(runtime.join("IBM.CH")).unwrap(), b"font-v2-longer");
        assert_eq!(
            fs::read(runtime.join(SAVED_GAM_FILENAME)).unwrap(),
            b"played-save"
        );

        clear_readonly(&save_path).unwrap();
        fs::remove_dir_all(source).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn writable_fixture_is_used_directly() {
        let source = temp_dir("writable-source");
        fs::write(source.join(SAVED_GAM_FILENAME), b"save").unwrap();
        assert!(!requires_writable_mirror(&source).unwrap());
        assert_eq!(prepare_writable_game_dir(&source).unwrap(), source);
        fs::remove_dir_all(source).unwrap();
    }
}
