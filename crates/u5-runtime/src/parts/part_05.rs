

pub fn run_report(game_dir: &Path) -> io::Result<()> {
    let mut report = String::new();
    report.push_str("# Lord British throne-room verification slice\n\n");
    report.push_str(&format!("Game data: `{}`\n\n", game_dir.display()));
    report.push_str("This executable is a parity harness. It reads original data at runtime but does not embed or emit raw map, dialogue, or asset dumps.\n\n");

    let lb_candidate = Scene::new(0x11)?; // CASTLE:0 by public scene partition.
    let fifth_castle = Scene::new(0x15)?; // CASTLE:4, the disputed public wording.
    let decomp_special = Scene::new(0x1d)?; // Scene identified by private TOWN note.

    let castle_tlk = parse_tlk(&game_dir.join("CASTLE.TLK"))?;
    let keep_tlk = parse_tlk(&game_dir.join("KEEP.TLK"))?;

    let lb_slots = parse_npc_block(&game_dir, lb_candidate, &castle_tlk)?;
    let fifth_slots = parse_npc_block(&game_dir, fifth_castle, &castle_tlk)?;
    let special_slots = parse_npc_block(&game_dir, decomp_special, &keep_tlk)?;

    let lb_names = names(&lb_slots);
    let fifth_names = names(&fifth_slots);
    let special_names = names(&special_slots);

    let lb_has_castle_staff = contains_all(
        &lb_names,
        &[
            "Alistair", "Stephen", "Treanna", "Margaret", "Desiree", "Saduj",
        ],
    );
    let fifth_has_castle_staff = contains_any(&fifth_names, &["Alistair", "Stephen", "Saduj"]);
    let special_is_keep = decomp_special.family == Family::Keep;

    report.push_str("## Scene binding checks\n\n");
    report.push_str(&format!(
        "- Scene `0x{:02X}` resolves by public partition to `{}`.\n",
        lb_candidate.byte,
        lb_candidate.key()
    ));
    report.push_str(&format!(
        "- Scene `0x{:02X}` resolves by public partition to `{}`.\n",
        fifth_castle.byte,
        fifth_castle.key()
    ));
    report.push_str(&format!(
        "- Scene `0x{:02X}` resolves by public partition to `{}`.\n",
        decomp_special.byte,
        decomp_special.key()
    ));
    report.push_str(&format!(
        "- `CASTLE:0` contains Lord-British-castle staff markers: {}.\n",
        pass_fail(lb_has_castle_staff)
    ));
    report.push_str(&format!(
        "- `CASTLE:4` contains those staff markers: {}.\n",
        pass_fail(fifth_has_castle_staff)
    ));
    report.push_str(&format!(
        "- Private-note special scene `0x1D` maps to keep family under the public partition: {}.\n\n",
        pass_fail(special_is_keep)
    ));

    report.push_str("Representative roster names, limited to avoid dialogue or roster dumps:\n\n");
    report.push_str(&format!(
        "- `{}`: {}\n",
        lb_candidate.key(),
        sample_names(&lb_names)
    ));
    report.push_str(&format!(
        "- `{}`: {}\n",
        fifth_castle.key(),
        sample_names(&fifth_names)
    ));
    report.push_str(&format!(
        "- `{}`: {}\n\n",
        decomp_special.key(),
        sample_names(&special_names)
    ));

    let floor0 = load_floor(&game_dir, lb_candidate, 0)?;
    let floor1 = load_floor(&game_dir, lb_candidate, 1)?;
    let stats0 = analyze_map(lb_candidate, 0, &floor0);
    let stats1 = analyze_map(lb_candidate, 1, &floor1);

    report.push_str("## Map/render checks\n\n");
    append_map_stats(&mut report, &stats0);
    append_map_stats(&mut report, &stats1);

    let start = harvest_location_markers(&floor0)
        .spawn_markers
        .first()
        .copied()
        .or_else(|| first_walkable(&floor0, None))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no walkable floor-0 start"))?;
    let target = stats0
        .npc_markers
        .first()
        .copied()
        .filter(|target| *target != start)
        .or_else(|| first_distinct_walkable(&floor0, start))
        .or_else(|| first_walkable(&floor0, None))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no floor-0 target"))?;
    let path = find_path(&floor0, start, target);

    report.push_str("## Movement/pathfinding checks\n\n");
    report.push_str(&format!(
        "- Movement probe start: ({}, {}), target: ({}, {}).\n",
        start.0, start.1, target.0, target.1
    ));
    match &path {
        Some(steps) => {
            report.push_str(&format!(
                "- Class-derived pathfinding found a path of {} steps: PASS.\n",
                steps.len().saturating_sub(1)
            ));
            let legal = steps.windows(2).all(|w| {
                manhattan(w[0], w[1]) == 1 && is_probe_walkable(floor0[w[1].1 * 32 + w[1].0])
            });
            report.push_str(&format!(
                "- Simulated step-by-step movement over the path: {}.\n",
                pass_fail(legal)
            ));
        }
        None => {
            report.push_str("- Class-derived pathfinding found no path. This is a WARNING, not a hard failure, because exact passability bitmap placement is still open in the public specs.\n");
        }
    }
    report.push_str(&format!(
        "- Door-family tiles detected on tested floors: {}.\n\n",
        stats0.door_count + stats1.door_count
    ));
    match door_probe(&floor1) {
        Some((pos, opened_walkable)) => {
            report.push_str(&format!(
                "- Door interaction smoke probe at ({}, {}) rewrote a door-family cell and produced a walkable result: {}.\n\n",
                pos.0,
                pos.1,
                pass_fail(opened_walkable)
            ));
        }
        None => report.push_str(
            "- Door interaction smoke probe: WARNING, no door-family tile found on floor 1.\n\n",
        ),
    }

    report.push_str("## Schedule/conversation checks\n\n");
    let occupied = lb_slots.iter().filter(|s| s.type_byte != 0).count();
    let named = lb_slots.iter().filter(|s| s.name.is_some()).count();
    report.push_str(&format!(
        "- Occupied `CASTLE:0` roster slots: {occupied}.\n"
    ));
    report.push_str(&format!(
        "- Occupied slots with resolved TLK display names: {named}.\n"
    ));
    report.push_str("- Noon waypoint sample:\n");
    for slot in lb_slots.iter().filter(|s| s.type_byte != 0).take(6) {
        let wp = waypoint_for_hour(&slot.schedule, 12);
        let name = slot.name.as_deref().unwrap_or("(unnamed)");
        report.push_str(&format!(
            "  - slot {} dlg {} `{}` -> waypoint {} at ({}, {}, {}).\n",
            slot.slot,
            slot.dialog_id,
            name,
            wp,
            slot.schedule[3 + wp],
            slot.schedule[6 + wp],
            slot.schedule[9 + wp] as i8
        ));
    }
    if let Some(slot) = lb_slots
        .iter()
        .find(|slot| slot.type_byte != 0 && slot.dialog_id > 1 && slot.name.is_some())
    {
        let fields = castle_tlk
            .get(&(slot.dialog_id as u16))
            .map(|fields| fields.len())
            .unwrap_or(0);
        let keywords = fields.saturating_sub(5) / 2;
        report.push_str(&format!(
            "- Conversation envelope probe: slot {} dlg {} has the five leading TLK fields and {} keyword pairs: {}.\n",
            slot.slot,
            slot.dialog_id,
            keywords,
            pass_fail(fields >= 5)
        ));
    } else {
        report.push_str(
            "- Conversation envelope probe: FAIL, no named dialogue-bearing slot found.\n",
        );
    }
    report.push_str("\n");

    report.push_str("## Findings\n\n");
    report.push_str("- The slice runs end-to-end for file loading, scene partitioning, roster/TLK joins, map analysis, render hashing, schedule sampling, and pathfinding smoke checks.\n");
    report.push_str("- `CASTLE:0`, not the fifth castle slot, is the strongest data-backed public binding for Lord British's castle in this slice.\n");
    report.push_str("- The private TOWN note's `0x1D` special-case label conflicts with the public scene partition and should be treated as an unresolved private-analysis issue until rechecked.\n");
    report.push_str("- The aggregate report keeps its class-derived smoke path; runtime play can also consume an optional clean-room passability bitmap.\n");

    if !lb_has_castle_staff || !special_is_keep {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "required scene binding check failed",
        ));
    }

    fs::create_dir_all("reports")?;
    fs::write(REPORT_PATH, &report)?;
    print!("{report}");
    println!("\nReport written to {REPORT_PATH}");
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayInputDisposition {
    Continue,
    Quit,
}

pub fn handle_play_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    if state.resolve_moongate_prompt(key, game_dir)?.is_some() {
        return Ok(PlayInputDisposition::Continue);
    }
    if key == PLAY_IGNORED_INPUT_KEY {
        state.message = match suffix {
            "function" => "Function key ignored.",
            _ => "Input ignored.",
        }
        .to_string();
        return Ok(PlayInputDisposition::Continue);
    }
    if key == PLAY_TYPEAHEAD_TOGGLE_KEY {
        state.toggle_typeahead_buffer();
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'q' {
        return Ok(PlayInputDisposition::Quit);
    }
    if matches!(state.area, Area::Dungeon { .. }) && key == 'Q' {
        return Ok(state.exit_to_dos_prompt(parse_inline_yes_no(suffix)));
    }
    if key == 'C' && !suffix.is_empty() {
        let turn_before = state.turn;
        let outcome = state.cast_spell_from_suffix(suffix, game_dir)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'M' && !suffix.is_empty() {
        if inline_mix_candidate(suffix) {
            state.mix_reagents_from_suffix(suffix);
        } else if state
            .meditate_shrine_from_suffix(suffix, game_dir)?
            .is_none()
        {
            state.mix_reagents_from_suffix(suffix);
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'N' && !suffix.is_empty() {
        state.new_order_from_suffix(suffix);
        return Ok(PlayInputDisposition::Continue);
    }
    let inline_direction = suffix.chars().find_map(Direction::from_play_key);
    let inline_hours = parse_inline_hours(suffix);
    let inline_drink = parse_inline_yes_no(suffix);
    let inline_party_index = parse_inline_party_index(suffix);
    let inline_use_request = parse_inline_use_request(suffix);
    let inline_talk_keyword = non_empty_talk_keyword(suffix);
    if state.handle_dungeon_key_with_inline(
        key,
        game_dir,
        inline_hours,
        inline_drink,
        inline_party_index,
        inline_use_request,
    )? {
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(key, 'T' | 't') && inline_talk_keyword.is_some() {
        let turn_before = state.turn;
        let outcome = state.talk_facing_with_game_dir_and_keyword(game_dir, inline_talk_keyword)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if state.handle_top_down_key_with_inline(
        key,
        game_dir,
        inline_direction,
        inline_hours,
        inline_drink,
        inline_use_request,
    )? {
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(state.area, Area::Dungeon { .. }) {
        state.advance_visual_tick();
        state.message = "Zzzzzz...".to_string();
        return Ok(PlayInputDisposition::Continue);
    }
    state.message = format!("Unhandled command `{key}`.");
    Ok(PlayInputDisposition::Continue)
}

