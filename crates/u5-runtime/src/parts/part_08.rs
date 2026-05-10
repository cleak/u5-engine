

pub fn load_world_location_entries(game_dir: &Path) -> io::Result<Option<Vec<WorldLocationEntry>>> {
    let path = game_dir.join(WORLD_LOCATION_TABLE_FILE);
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
    parse_world_location_entries(&text).map(Some)
}

