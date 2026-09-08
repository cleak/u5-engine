//! Report what `conversation.md §2`'s Talk gates see around the party.
//!
//! `cleak/u5-spec#198` asks which gate makes the stock game answer
//! `No response!` for an NPC that Look identifies normally. Step 4 reads
//! "the live map tile occupying the resolved cell", and that byte is the
//! one thing a screen comparison cannot see - it is under the NPC's own
//! sprite. This prints it, for the party's cell and its four neighbours,
//! from whatever save the given profile holds.
//!
//! Usage: `cargo run -p u5-tui --example talk_gate_probe -- <PROFILE_DIR>`
//!
//! Sanitized by construction: tile ids and dialog ids only, no map dump
//! and no dialogue text.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use u5_runtime::*;
use u5_tui::{replay_play_script_commands, split_play_script};

/// One breadth-first step search from the party's current cell.
///
/// Returns the first key of a shortest route to a cell adjacent to the
/// NPC carrying `dialog_id`, or `None` when no route exists in this
/// snapshot. Callers step, re-search, and repeat - see [`chase`].
fn next_step_toward(state: &PlayState, dialog_id: u8) -> Option<char> {
    let start = (state.player.x, state.player.y);
    let mut came: HashMap<(usize, usize), ((usize, usize), char)> = HashMap::new();
    let mut queue = VecDeque::from([start]);
    came.insert(start, (start, ' '));
    let mut goal = None;
    while let Some((x, y)) = queue.pop_front() {
        for (dx, dy) in [(0isize, -1isize), (0, 1), (-1, 0), (1, 0)] {
            let (nx, ny) = (x as isize + dx, y as isize + dy);
            if !(0..32).contains(&nx) || !(0..32).contains(&ny) {
                continue;
            }
            if state
                .npc_at_current_floor(nx as usize, ny as usize)
                .is_some_and(|npc| npc.dialog_id == dialog_id)
            {
                goal = Some((x, y));
            }
        }
        if goal == Some((x, y)) {
            break;
        }
        for (direction, key) in [
            (Direction::North, 'w'),
            (Direction::South, 's'),
            (Direction::West, 'a'),
            (Direction::East, 'd'),
        ] {
            let mut probe = state.clone();
            probe.player.x = x;
            probe.player.y = y;
            probe.sync_player_object();
            if probe
                .step_with_game_dir(direction, None)
                .unwrap_or(MoveOutcome::Blocked)
                != MoveOutcome::Moved
            {
                continue;
            }
            let next = (probe.player.x, probe.player.y);
            if came.contains_key(&next) {
                continue;
            }
            came.insert(next, ((x, y), key));
            queue.push_back(next);
        }
    }
    let mut cell = goal?;
    if cell == start {
        return None;
    }
    let mut first = ' ';
    while cell != start {
        let (previous, key) = *came.get(&cell)?;
        first = key;
        cell = previous;
    }
    Some(first)
}

/// Walk to a fixed **cell**, ignoring who stands where.
///
/// A scenario that has to reach a wandering resident cannot chase it - the
/// wander draw is not reproducible (`prng.md` §3) - but it *can* walk to the
/// resident's post and then try Talk in each direction. This prints the keys
/// for that walk, which depend only on the map.
fn route_to_cell(state: &mut PlayState, goal: (usize, usize), budget: usize) {
    // Plan *and* walk with the cast lifted out. A route that detours around
    // whoever happens to stand in a doorway is not reproducible (`prng.md`
    // §3), and the paired run will meet a different arrangement anyway; a
    // terrain-only route is the same every time, and an NPC standing on it
    // only ever delays the walk by a turn.
    let mut terrain = state.clone();
    terrain.npcs.clear();
    for object in terrain.active_objects.iter_mut().skip(1) {
        *object = ActiveObject::empty();
    }
    terrain.sync_player_object();
    let mut keys = String::new();
    for _ in 0..budget {
        if (terrain.player.x, terrain.player.y) == goal {
            println!(
                "route: {} key(s) `{keys}` to ({}, {})",
                keys.len(),
                goal.0,
                goal.1
            );
            return;
        }
        let start = (terrain.player.x, terrain.player.y);
        let mut came: HashMap<(usize, usize), ((usize, usize), char, bool)> = HashMap::new();
        let mut queue = VecDeque::from([start]);
        came.insert(start, (start, ' ', false));
        let mut reached = false;
        while let Some((x, y)) = queue.pop_front() {
            if (x, y) == goal {
                reached = true;
                break;
            }
            for (direction, key) in [
                (Direction::North, 'w'),
                (Direction::South, 's'),
                (Direction::West, 'a'),
                (Direction::East, 'd'),
            ] {
                let mut probe = terrain.clone();
                probe.player.x = x;
                probe.player.y = y;
                probe.sync_player_object();
                let mut opened = false;
                if probe
                    .step_with_game_dir(direction, None)
                    .unwrap_or(MoveOutcome::Blocked)
                    != MoveOutcome::Moved
                {
                    // A closed door is not a wall: the party opens it and
                    // walks through, which is the only way to reach half of
                    // a village's residents. The route records the `o` so the
                    // paired scenario presses it too.
                    let (dx, dy) = direction.delta();
                    let (tx, ty) = (x as isize + dx, y as isize + dy);
                    if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
                        continue;
                    }
                    if !is_town_door_tile(probe.grid[ty as usize * 32 + tx as usize]) {
                        continue;
                    }
                    let mut door = terrain.clone();
                    door.player.x = x;
                    door.player.y = y;
                    door.sync_player_object();
                    if door.open_direction_with_game_dir(direction, None).is_err() {
                        continue;
                    }
                    if door
                        .step_with_game_dir(direction, None)
                        .unwrap_or(MoveOutcome::Blocked)
                        != MoveOutcome::Moved
                    {
                        continue;
                    }
                    probe = door;
                    opened = true;
                }
                let next = (probe.player.x, probe.player.y);
                if came.contains_key(&next) {
                    continue;
                }
                came.insert(next, ((x, y), key, opened));
                queue.push_back(next);
            }
        }
        if !reached {
            println!("route: no path to ({}, {}) after `{keys}`", goal.0, goal.1);
            return;
        }
        let mut cell = goal;
        let mut first = (' ', false);
        while cell != start {
            let (previous, key, opened) = came[&cell];
            first = (key, opened);
            cell = previous;
        }
        let (key, opened) = first;
        let direction = match key {
            'w' => Direction::North,
            's' => Direction::South,
            'a' => Direction::West,
            _ => Direction::East,
        };
        if opened {
            if terrain
                .open_direction_with_game_dir(direction, None)
                .is_err()
            {
                println!("route: could not open the door after `{keys}`");
                return;
            }
            keys.push('o');
            keys.push(key);
        }
        if terrain
            .step_with_game_dir(direction, None)
            .unwrap_or(MoveOutcome::Blocked)
            != MoveOutcome::Moved
        {
            println!("route: blocked mid-walk after `{keys}`");
            return;
        }
        keys.push(key);
    }
    println!("route: gave up after {budget} steps (`{keys}`)");
}

/// Walk to an NPC that is *moving*.
///
/// A route planned from a snapshot does not survive the walk: NPC
/// schedules advance a turn per step, so by the time the party arrives
/// the target has moved and the scripted Talk finds empty floor. Three
/// paired scenarios died that way before this existed.
///
/// This re-plans after every step against the state the step produced,
/// which is a pursuit rather than a route, and prints the keystrokes it
/// actually took.
///
/// **Replaying those keys does not reproduce the arrival.** An earlier
/// revision of this comment claimed it did, on the theory that both sides
/// are deterministic; `prng.md` §3 says the opposite in as many words -
/// "ordinary gameplay events re-seed from the host clock", so "the roll
/// stream is not reproducible from game state alone" - and the wander gate
/// of `npc-schedules.md` §9.1 draws from it once per NPC per turn. Two runs
/// of this probe over the same seed and the same script put the same
/// merchant on different cells, and three paired runs found the stock's
/// merchant where this engine's had wandered off. Use `--talk` below to ask
/// what a chased NPC answers *inside one process*; a scripted paired walk to
/// a wandering NPC is a coin flip, not a test.
fn chase(state: &mut PlayState, dialog_id: u8, budget: usize) -> Option<Direction> {
    let mut keys = String::new();
    for _ in 0..budget {
        let adjacent = [
            (Direction::North, 'N'),
            (Direction::South, 'S'),
            (Direction::West, 'W'),
            (Direction::East, 'E'),
        ]
        .into_iter()
        .find(|(direction, _)| {
            let (dx, dy) = direction.delta();
            let (x, y) = (state.player.x as isize + dx, state.player.y as isize + dy);
            (0..32).contains(&x)
                && (0..32).contains(&y)
                && state
                    .npc_at_current_floor(x as usize, y as usize)
                    .is_some_and(|npc| npc.dialog_id == dialog_id)
        });
        if let Some((direction, face)) = adjacent {
            println!(
                "chase: {} step(s) `{keys}` then Talk-{face}  (party at ({}, {}), turn {})",
                keys.len(),
                state.player.x,
                state.player.y,
                state.turn
            );
            return Some(direction);
        }
        let Some(key) = next_step_toward(state, dialog_id) else {
            println!("chase: no route to dialog-id {dialog_id} after `{keys}`");
            return None;
        };
        let direction = match key {
            'w' => Direction::North,
            's' => Direction::South,
            'a' => Direction::West,
            _ => Direction::East,
        };
        if state
            .step_with_game_dir(direction, None)
            .unwrap_or(MoveOutcome::Blocked)
            != MoveOutcome::Moved
        {
            println!("chase: blocked mid-pursuit after `{keys}`");
            return None;
        }
        keys.push(key);
    }
    println!("chase: gave up after {budget} steps (`{keys}`)");
    None
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .expect("usage: talk_gate_probe <PROFILE_DIR> [--script <SCRIPT>] [--chase <ID>]");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let mut state = PlayState::load_scene(dir, options).expect("scene must load");
    let mut chase_id: Option<u8> = None;
    let mut talk_id: Option<u8> = None;
    let mut route_goal: Option<(usize, usize)> = None;
    // A roster read from the seed save answers "who stands here *now*",
    // which is the wrong question whenever the divergence is positional:
    // the party has to reach the cell first, and NPC schedules advance a
    // turn per step on the way. `--script` replays the same semicolon
    // separated command list the engine's `--play-script` takes, against
    // the same handler, so the report below describes the cell the
    // paired scenario actually talks into rather than the seed's.
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--chase" => {
                chase_id = Some(
                    args.next()
                        .expect("--chase needs a dialog id")
                        .parse()
                        .expect("dialog id must be a byte"),
                );
            }
            "--route-to" => {
                let text = args.next().expect("--route-to needs X,Y");
                let (x, y) = text.split_once(',').expect("--route-to takes X,Y");
                route_goal = Some((
                    x.trim().parse().expect("X must be a number"),
                    y.trim().parse().expect("Y must be a number"),
                ));
            }
            "--talk" => {
                talk_id = Some(
                    args.next()
                        .expect("--talk needs a dialog id")
                        .parse()
                        .expect("dialog id must be a byte"),
                );
            }
            "--script" => {
                let script = args.next().expect("--script needs a command list");
                let commands = split_play_script(&script);
                replay_play_script_commands(&mut state, dir, &commands, |_, _, _| Ok(()))
                    .expect("script must replay");
                println!("replayed {} command(s) from --script", commands.len());
                for line in &state.diagnostics {
                    println!("  diagnostic: {line}");
                }
                let hour = state.clock.hour;
                for npc in &state.npcs {
                    let ai = &npc.schedule[NPC_SCHEDULE_AI_OFFSET
                        ..NPC_SCHEDULE_AI_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT];
                    println!(
                        "  slot {:>2} dialog {:>3} ai {ai:?} hour-wp {} cached-wp {}",
                        npc.slot,
                        npc.dialog_id,
                        waypoint_for_hour(&npc.schedule, hour),
                        npc.cached_wp,
                    );
                }
            }
            other => panic!("unknown option {other}"),
        }
    }
    if let Some(goal) = route_goal {
        route_to_cell(&mut state, goal, 128);
        return;
    }
    if let Some(dialog_id) = chase_id {
        chase(&mut state, dialog_id, 64);
        return;
    }
    // Chase *and* talk, inside one process, so the answer is not at the mercy
    // of the wander draw between two invocations.
    if let Some(dialog_id) = talk_id {
        let Some(direction) = chase(&mut state, dialog_id, 64) else {
            return;
        };
        let outcome = state
            .talk_direction_with_game_dir(direction, dir)
            .expect("talk must run");
        println!(
            "talk-{}: outcome {outcome:?} message {:?} shop {} conversation {}",
            Direction::name(direction),
            state.message,
            state.active_shop.is_some(),
            state.active_conversation.is_some(),
        );
        return;
    }
    let state = state;

    println!(
        "scene {:?} floor/level {:?} party at ({}, {}) facing {}",
        state.area,
        state.area,
        state.player.x,
        state.player.y,
        Direction::name(state.player.facing),
    );
    for direction in [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
    ] {
        let (dx, dy) = direction.delta();
        let x = state.player.x as isize + dx;
        let y = state.player.y as isize + dy;
        if !(0..32).contains(&x) || !(0..32).contains(&y) {
            continue;
        }
        let (x, y) = (x as usize, y as usize);
        let tile = state.talk_status_tile_at(x, y);
        let npc = state.npc_at_current_floor(x, y);
        println!(
            "  {:<5} ({x:>2},{y:>2}) floor tile 0x{tile:02x}{}  npc {}",
            Direction::name(direction),
            match tile {
                TALK_MIRROR_TILE => " [MIRROR -> No response!]",
                0xab => " [BED -> Zzzzzz...]",
                t if is_talk_through_tile(t) => " [talk-through]",
                _ => "",
            },
            match npc {
                Some(npc) => format!(
                    "slot {} dialog-id {} (0x{:02x}) type 0x{:02x}",
                    npc.slot, npc.dialog_id, npc.dialog_id, npc.type_byte
                ),
                None => "none".to_string(),
            },
        );
    }

    // Is the grid the gate reads composited with NPC sprites? If it is,
    // step 4 can never see the floor under a talker, which would make
    // the mirror arm unreachable for exactly the case it exists for.
    let mut checked = 0usize;
    let mut equal_to_npc_type = 0usize;
    for y in 0..32usize {
        for x in 0..32usize {
            let Some(npc) = state.npc_at_current_floor(x, y) else {
                continue;
            };
            checked += 1;
            if state.talk_status_tile_at(x, y) == npc.type_byte {
                equal_to_npc_type += 1;
            }
        }
    }
    println!(
        "npcs on this floor: {checked}; cells whose gate tile equals the npc's own type byte: {equal_to_npc_type}"
    );

    // The roster, so a paired scenario can be aimed at a talker instead
    // of at empty floor - which is what two earlier Talk scenarios did.
    for y in 0..32usize {
        for x in 0..32usize {
            let Some(npc) = state.npc_at_current_floor(x, y) else {
                continue;
            };
            println!(
                "  npc slot {:>2} at ({x:>2},{y:>2}) dialog-id {:>3} type 0x{:02x} floor tile 0x{:02x}",
                npc.slot,
                npc.dialog_id,
                npc.type_byte,
                state.talk_status_tile_at(x, y),
            );
        }
    }

    // Shortest walking route from the party to a cell adjacent to each
    // *speaker*. Paired Talk scenarios need a keystroke script, and
    // hand-guessing routes wastes runs on `Blocked!` - two earlier
    // scenarios never reached an NPC at all and compared nothing.
    //
    // The search is geometric: it repositions a clone of the loaded state
    // on each frontier cell and asks the ordinary movement handler
    // whether a step lands. NPC schedules still drift during the real
    // walk, so prefer the shortest route offered.
    //
    // Caveat, learned the hard way: the movement handler refuses cells
    // occupied by vehicles, furniture and other NPCs as well as by
    // walls, and those move. A snapshot search therefore reports
    // *fewer* reachable cells than the player has, and can call a town
    // sealed when it is merely congested - which is what it did for
    // Minoc, whose entrance plaza is fenced by carts at every hour this
    // search samples. Treat "unreachable on foot" as "no route in this
    // snapshot", never as geometry.
    let start = (state.player.x, state.player.y);
    let mut came: HashMap<(usize, usize), ((usize, usize), char)> = HashMap::new();
    let mut queue = VecDeque::from([start]);
    came.insert(start, (start, ' '));
    while let Some((x, y)) = queue.pop_front() {
        for (direction, key) in [
            (Direction::North, 'w'),
            (Direction::South, 's'),
            (Direction::West, 'a'),
            (Direction::East, 'd'),
        ] {
            let mut probe = state.clone();
            probe.player.x = x;
            probe.player.y = y;
            probe.sync_player_object();
            if probe
                .step_with_game_dir(direction, None)
                .unwrap_or(MoveOutcome::Blocked)
                != MoveOutcome::Moved
            {
                continue;
            }
            let next = (probe.player.x, probe.player.y);
            if came.contains_key(&next) {
                continue;
            }
            came.insert(next, ((x, y), key));
            queue.push_back(next);
        }
    }
    let path_to = |mut cell: (usize, usize)| -> Option<String> {
        let mut keys = Vec::new();
        while cell != start {
            let (previous, key) = *came.get(&cell)?;
            keys.push(key);
            cell = previous;
        }
        keys.reverse();
        Some(keys.into_iter().collect())
    };
    println!("shortest routes to a cell adjacent to each NPC (dialog-id 0 is a non-speaker):");
    for y in 0..32usize {
        for x in 0..32usize {
            let Some(npc) = state.npc_at_current_floor(x, y) else {
                continue;
            };

            let mut best: Option<(usize, String, char)> = None;
            for (dx, dy, face) in [
                (0isize, -1isize, 'N'),
                (0, 1, 'S'),
                (-1, 0, 'W'),
                (1, 0, 'E'),
            ] {
                let (ax, ay) = (x as isize - dx, y as isize - dy);
                if !(0..32).contains(&ax) || !(0..32).contains(&ay) {
                    continue;
                }
                let Some(route) = path_to((ax as usize, ay as usize)) else {
                    continue;
                };
                if best.as_ref().is_none_or(|(len, _, _)| route.len() < *len) {
                    best = Some((route.len(), route, face));
                }
            }
            match best {
                Some((len, route, face)) => println!(
                    "  slot {:>2} dialog-id {:>3} at ({x:>2},{y:>2}) [{}]: {len} step(s) `{route}` then Talk-{face}",
                    npc.slot,
                    npc.dialog_id,
                    state
                        .talk_target_description(x, y)
                        .unwrap_or_else(|| "?".to_string()),
                ),
                None => println!(
                    "  slot {:>2} dialog-id {:>3} at ({x:>2},{y:>2}): unreachable on foot",
                    npc.slot, npc.dialog_id
                ),
            }
        }
    }
}
