#[test]
fn use_command_rejects_inline_torch_and_gem_aliases() {
    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    dungeon.torches = 1;

    assert_eq!(
        handle_play_key_input(&mut dungeon, 'U', "T", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(dungeon.torches, 1);
    assert_eq!(dungeon.torch_counter, 0);
    assert_eq!(dungeon.turn, 0);
    assert_eq!(dungeon.message, use_prompt_message());

    let mut world = britannia_state(open_world_grid(), 1, 1);
    world.gems = 1;

    assert_eq!(
        handle_play_key_input(&mut world, 'U', "G", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(world.gems, 1);
    assert_eq!(world.turn, 0);
    assert_eq!(world.message, use_prompt_message());
}

#[test]
fn use_command_routes_inline_sextant_request_at_night() {
    let mut world = world_state(open_world_grid(), 0x23, 0xaf);
    world.clock = GameClock::new(20, 0).unwrap();
    world.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 1;

    assert_eq!(
        handle_play_key_input(&mut world, 'U', "S", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(world.turn, 1);
    assert_eq!(world.message, "Sextant: K'P,C'D\"");
}

#[test]
fn sextant_requires_item_world_scene_and_night() {
    let mut world = world_state(open_world_grid(), 1, 1);
    assert_eq!(world.use_sextant(), MoveOutcome::Blocked);
    assert_eq!(world.message, "No Sextant!");

    world.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 1;
    world.clock = GameClock::new(12, 0).unwrap();
    assert_eq!(world.use_sextant(), MoveOutcome::Blocked);
    assert_eq!(world.message, "Cannot see the stars!");

    let mut town = test_state(open_grid(), 1, 1);
    town.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 1;
    town.clock = GameClock::new(20, 0).unwrap();
    assert_eq!(town.use_sextant(), MoveOutcome::Blocked);
    assert_eq!(town.message, "Not here!");
}

#[test]
fn use_command_routes_inline_pocket_watch_request() {
    let mut town = test_state(open_grid(), 1, 1);
    town.clock = GameClock::new(0, 45).unwrap();
    town.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX] = 1;

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "W", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(town.turn, 1);
    assert_eq!(town.message, "Pocket Watch: 12 A.M.");

    town.clock = GameClock::new(13, 20).unwrap();
    assert_eq!(town.use_pocket_watch(), MoveOutcome::Used);
    assert_eq!(town.message, "Pocket Watch: 1 P.M.");
}

#[test]
fn pocket_watch_requires_item_without_turn() {
    let mut town = test_state(open_grid(), 1, 1);

    assert_eq!(town.use_pocket_watch(), MoveOutcome::Blocked);

    assert_eq!(town.turn, 0);
    assert_eq!(town.message, "No Pocket Watch!");
}

#[test]
fn use_command_routes_inline_spyglass_request_to_britannia_overview() {
    let mut world = britannia_state(open_world_grid(), 0x23, 0xaf);
    world.clock = GameClock::new(20, 0).unwrap();
    world.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    world.gems = 3;

    assert_eq!(
        handle_play_key_input(&mut world, 'U', "SP", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(world.turn, 0);
    assert_eq!(world.gems, 3);
    assert_eq!(
        world.special_items[SPECIAL_ITEM_SPYGLASS_INDEX],
        SPECIAL_ITEM_OWNED_VALUE
    );
    assert!(world.message.starts_with("Spyglass: Looking at the stars"));
    assert!(world.message.contains('+'));
    assert_eq!(
        world.active_view_overlay.as_ref().map(|overlay| overlay.kind),
        Some(ViewOverlayKind::BritanniaChunkMap)
    );
    let viewport = world
        .render_active_view_overlay(TileGraphicsDepth::Ega16)
        .expect("spyglass should install a renderable modal overlay");
    assert_eq!(viewport.cells_wide, 22);
    assert_eq!(viewport.cells_high, 8);
    assert!(viewport.pixels.iter().any(|pixel| *pixel != 0));
    assert_eq!(world.britannia_chunk_overview_map().lines().count(), 8);

    assert_eq!(
        handle_play_key_input(&mut world, ' ', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(world.active_view_overlay.is_none());
    assert_eq!(world.turn, 0);
    assert_eq!(world.gems, 3);
    assert_eq!(world.message, "View closed.");
}

#[test]
fn spyglass_requires_item_britannia_and_night() {
    let mut missing = britannia_state(open_world_grid(), 1, 1);
    missing.clock = GameClock::new(20, 0).unwrap();
    assert_eq!(missing.use_spyglass(), MoveOutcome::Blocked);
    assert_eq!(missing.turn, 0);
    assert_eq!(missing.message, "No Spyglass!");

    let mut day = britannia_state(open_world_grid(), 1, 1);
    day.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    day.clock = GameClock::new(12, 0).unwrap();
    assert_eq!(day.use_spyglass(), MoveOutcome::Blocked);
    assert_eq!(day.turn, 0);
    assert_eq!(day.message, "Cannot see the stars!");

    let mut town = test_state(open_grid(), 1, 1);
    town.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    town.clock = GameClock::new(20, 0).unwrap();
    assert_eq!(town.use_spyglass(), MoveOutcome::Blocked);
    assert_eq!(town.message, "Not here!");

    let mut underworld = world_state(open_world_grid(), 1, 1);
    underworld.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    underworld.clock = GameClock::new(20, 0).unwrap();
    assert_eq!(underworld.use_spyglass(), MoveOutcome::Blocked);
    assert_eq!(underworld.message, "Not here!");
}

#[test]
fn use_command_routes_scrolls_to_item_effects_without_spell_resources() {
    let mut town = test_state(open_grid(), 1, 1);
    town.scroll_stock[SCROLL_LIGHT_INDEX] = 1;
    town.scroll_stock[SCROLL_PROTECTION_INDEX] = 1;
    town.scroll_stock[SCROLL_NEGATE_MAGIC_INDEX] = 1;
    town.scroll_stock[SCROLL_VIEW_INDEX] = 1;
    town.scroll_stock[SCROLL_NEGATE_TIME_INDEX] = 1;

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "LV", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(town.scroll_stock[SCROLL_LIGHT_INDEX], 0);
    assert_eq!(town.light_spell_counter, SCROLL_LIGHT_DURATION - 1);
    assert_eq!(town.turn, 1);
    assert_eq!(town.message, "Light!");

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "IS", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(town.active_effect_tag, Some(PROTECTION_ACTIVE_EFFECT_TAG));
    assert_eq!(town.active_effect_counter, SCROLL_PROTECTION_DURATION - 1);
    assert_eq!(town.turn, 2);
    assert_eq!(town.message, "Protection!");

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "AI", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(town.active_effect_tag, Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG));
    assert_eq!(town.active_effect_counter, SCROLL_NEGATE_MAGIC_DURATION - 1);
    assert_eq!(town.turn, 3);
    assert_eq!(town.message, "Negate magic!");

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "IQW", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(town.scroll_stock[SCROLL_VIEW_INDEX], 0);
    assert_eq!(town.turn, 4);
    assert!(town.message.starts_with("View!\nPeer view of CASTLE:0"));
    assert!(town.active_view_overlay.is_some());

    assert_eq!(
        handle_play_key_input(&mut town, ' ', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(town.active_view_overlay.is_none());
    assert_eq!(town.turn, 4);
    assert_eq!(town.message, "View closed.");

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "AT", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(town.active_effect_tag, Some(NEGATE_TIME_ACTIVE_EFFECT_TAG));
    assert_eq!(town.active_effect_counter, SCROLL_NEGATE_TIME_DURATION - 1);
    assert_eq!(town.turn, 5);
    assert_eq!(town.message, "Negate time!");
}

#[test]
fn scroll_wind_and_resurrection_debit_before_branch_gates() {
    let mut world = britannia_state(open_world_grid(), 1, 1);
    world.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 1;

    assert_eq!(
        handle_play_key_input(&mut world, 'U', "HR8", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(world.scroll_stock[SCROLL_WIND_CHANGE_INDEX], 0);
    assert_eq!(world.wind, WindState::West);
    assert_eq!(world.turn, 1);
    assert_eq!(world.message, "Wind change! Calm Winds -> West Winds.");

    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    dungeon.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 1;
    assert_eq!(
        handle_play_key_input(&mut dungeon, 'U', "HR8", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(dungeon.scroll_stock[SCROLL_WIND_CHANGE_INDEX], 0);
    assert_eq!(dungeon.turn, 0);
    assert_eq!(dungeon.message, "Not here!");

    let mut town = test_state(open_grid(), 1, 1);
    town.party[0].status = b'D';
    town.party[0].hp = 0;
    town.scroll_stock[SCROLL_RESURRECTION_INDEX] = 1;
    assert_eq!(
        town.use_scroll(SCROLL_RESURRECTION_INDEX, None, Some(0)),
        MoveOutcome::Used
    );
    assert_eq!(town.scroll_stock[SCROLL_RESURRECTION_INDEX], 0);
    assert_eq!(town.party[0].status, b'G');
    assert_eq!(town.party[0].hp, 1);
    assert_eq!(town.turn, 1);
    assert!(town.message.starts_with("Resurrection! party member 1"));
}

#[test]
fn scrolls_require_stock_and_negate_time_has_no_effect_in_stonegate() {
    let mut missing = test_state(open_grid(), 1, 1);
    assert_eq!(
        missing.use_scroll(SCROLL_LIGHT_INDEX, None, None),
        MoveOutcome::Blocked
    );
    assert_eq!(missing.message, "No LV scroll!");
    assert_eq!(missing.turn, 0);

    let mut stonegate = test_state(open_grid(), 1, 1);
    stonegate.area = Area::Town {
        scene: Scene::new(STONEGATE_SCENE_BYTE).unwrap(),
        floor: 0,
    };
    stonegate.scroll_stock[SCROLL_NEGATE_TIME_INDEX] = 1;
    assert_eq!(
        handle_play_key_input(&mut stonegate, 'U', "AT", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(stonegate.scroll_stock[SCROLL_NEGATE_TIME_INDEX], 0);
    assert_eq!(stonegate.active_effect_tag, None);
    assert_eq!(stonegate.turn, 1);
    assert_eq!(stonegate.message, "No effect!");
}

#[test]
fn use_command_routes_potion_colors_to_party_effects() {
    let mut town = test_state(open_grid(), 1, 1);
    town.potion_stock[POTION_BLUE_INDEX] = 1;
    town.potion_stock[POTION_YELLOW_INDEX] = 1;
    town.potion_stock[POTION_RED_INDEX] = 1;
    town.potion_stock[POTION_GREEN_INDEX] = 1;
    town.potion_stock[POTION_ORANGE_INDEX] = 1;
    town.party[0].status = b'S';
    town.party[0].hp = 5;
    town.party[0].max_hp = 25;

    assert_eq!(
        town.use_item_command(
            Some(UseItemRequest::Potion {
                index: POTION_BLUE_INDEX,
                target: Some(0)
            }),
            None,
        )
        .unwrap(),
        MoveOutcome::Used
    );
    assert_eq!(town.potion_stock[POTION_BLUE_INDEX], 0);
    assert_eq!(town.party[0].status, b'G');
    assert_eq!(town.turn, 1);
    assert_eq!(town.message, "blue potion: Awakened party member 1.");

    assert_eq!(
        town.use_item_command(
            Some(UseItemRequest::Potion {
                index: POTION_YELLOW_INDEX,
                target: Some(0)
            }),
            None,
        )
        .unwrap(),
        MoveOutcome::Used
    );
    assert_eq!(town.potion_stock[POTION_YELLOW_INDEX], 0);
    assert!(town.party[0].hp > 5);
    assert_eq!(town.turn, 2);
    assert!(town
        .message
        .starts_with("yellow potion: Healed party member 1"));

    town.party[0].status = b'P';
    assert_eq!(
        town.use_potion_with_effect(POTION_RED_INDEX, 0, POTION_RED_INDEX),
        MoveOutcome::Used
    );
    assert_eq!(town.party[0].status, b'G');
    assert_eq!(town.message, "red potion: Cured party member 1.");

    town.player.x = 2;
    assert_eq!(
        town.use_potion_with_effect(POTION_GREEN_INDEX, 0, POTION_GREEN_INDEX),
        MoveOutcome::Used
    );
    assert_eq!(town.party[0].status, b'P');
    assert_eq!(town.message, "green potion: Poisoned party member 1.");

    town.player.x = 3;
    town.party[0].status = b'G';
    assert_eq!(
        town.use_potion_with_effect(POTION_ORANGE_INDEX, 0, POTION_ORANGE_INDEX),
        MoveOutcome::Used
    );
    assert_eq!(town.party[0].status, b'S');
    assert_eq!(town.message, "orange potion: Slept party member 1.");
}

#[test]
fn potions_debit_before_target_and_effect_variation_gates() {
    let mut missing_target = test_state(open_grid(), 1, 1);
    missing_target.potion_stock[POTION_RED_INDEX] = 1;
    assert_eq!(
        handle_play_key_input(&mut missing_target, 'U', "RED", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(missing_target.potion_stock[POTION_RED_INDEX], 0);
    assert_eq!(missing_target.turn, 0);
    assert!(missing_target.message.starts_with("Who?"));

    let mut missing_stock = test_state(open_grid(), 1, 1);
    assert_eq!(
        missing_stock.use_potion(POTION_BLUE_INDEX, Some(0)),
        MoveOutcome::Blocked
    );
    assert_eq!(missing_stock.message, "No blue potion!");
    assert_eq!(missing_stock.turn, 0);

    assert_eq!(
        potion_effect_index_after_variation(POTION_RED_INDEX, 0, 7),
        POTION_RED_INDEX
    );
    assert_eq!(
        potion_effect_index_after_variation(POTION_RED_INDEX, 14, 7),
        POTION_ORANGE_INDEX
    );
    assert_eq!(
        potion_effect_index_after_variation(POTION_RED_INDEX, 15, 6),
        POTION_BLACK_INDEX
    );
}

#[test]
fn potion_combat_and_white_visibility_effects_use_scene_gates() {
    let mut world = britannia_state(open_world_grid(), 2, 1);
    world.potion_stock[POTION_WHITE_INDEX] = 1;
    world.visibility_dirty = false;
    assert_eq!(
        world.use_potion(POTION_WHITE_INDEX, Some(0)),
        MoveOutcome::Observed
    );
    assert_eq!(world.potion_stock[POTION_WHITE_INDEX], 0);
    assert!(world.visibility_dirty);
    assert_eq!(
        world.white_potion_sweep,
        Some(WhitePotionSweep {
            frames_remaining: POTION_WHITE_SWEEP_FRAMES,
            radius: POTION_WHITE_SWEEP_RADIUS,
            center_x: 2,
            center_y: 1,
        })
    );
    assert_eq!(world.turn, 1);
    assert_eq!(world.message, "white potion: Visibility sweep.");

    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    dungeon.potion_stock[POTION_WHITE_INDEX] = 1;
    assert_eq!(
        dungeon.use_potion_with_effect(POTION_WHITE_INDEX, 0, POTION_WHITE_INDEX),
        MoveOutcome::Blocked
    );
    assert_eq!(dungeon.potion_stock[POTION_WHITE_INDEX], 1);
    assert_eq!(dungeon.message, "white potion: No noticeable effect.");

    let mut combat = test_state(open_grid(), 1, 1);
    combat.combat_active = true;
    combat.potion_stock[POTION_BLACK_INDEX] = 1;
    combat.active_objects.push(ActiveObject {
        type_byte: 0x81,
        tile: 0x81,
        x: 5,
        y: 5,
        ..ActiveObject::empty()
    });
    combat.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 0, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 1, 5, 5]);

    assert_eq!(
        combat.use_potion_with_effect(POTION_BLACK_INDEX, 0, POTION_BLACK_INDEX),
        MoveOutcome::Used
    );
    assert!(combat.combat_actors[0].is_hidden_or_unrevealed());
    assert_eq!(
        combat.active_objects[1].tile,
        COMBAT_HIDDEN_ACTIVE_OBJECT_TILE
    );
    assert_eq!(combat.message, "black potion: Invisible party member 1.");
}

#[test]
fn combat_potions_mark_and_clear_linked_presentation_state() {
    let mut combat = test_state(open_grid(), 1, 1);
    combat.combat_active = true;
    combat.visibility_dirty = false;
    combat.active_objects.push(ActiveObject {
        type_byte: 0x81,
        tile: 0x81,
        x: 5,
        y: 5,
        ..ActiveObject::empty()
    });
    combat.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 0, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 1, 5, 5]);

    assert_eq!(
        combat.use_potion_with_effect(POTION_ORANGE_INDEX, 0, POTION_ORANGE_INDEX),
        MoveOutcome::Used
    );
    assert_eq!(combat.party[0].status, b'S');
    assert_eq!(
        combat.combat_potion_presentation,
        Some(CombatPotionPresentation {
            kind: CombatPotionPresentationKind::Sleep,
            actor_slot: 0,
            active_object_slot: 1,
            frames_remaining: COMBAT_POTION_SLEEP_PRESENTATION_FRAMES,
        })
    );
    assert!(combat.visibility_dirty);

    combat.visibility_dirty = false;
    assert_eq!(
        combat.use_potion_with_effect(POTION_BLUE_INDEX, 0, POTION_BLUE_INDEX),
        MoveOutcome::Used
    );
    assert_eq!(combat.party[0].status, b'G');
    assert_eq!(combat.combat_potion_presentation, None);
    assert!(combat.visibility_dirty);

    combat.visibility_dirty = false;
    assert_eq!(
        combat.use_potion_with_effect(POTION_PURPLE_INDEX, 0, POTION_PURPLE_INDEX),
        MoveOutcome::Used
    );
    assert_eq!(
        combat.combat_potion_presentation,
        Some(CombatPotionPresentation {
            kind: CombatPotionPresentationKind::Poof,
            actor_slot: 0,
            active_object_slot: 1,
            frames_remaining: COMBAT_POTION_POOF_PRESENTATION_FRAMES,
        })
    );
    assert!(combat.visibility_dirty);
    assert_eq!(combat.message, "purple potion: Poof!");
}

#[test]
fn wooden_box_use_prompts_without_endgame_handoff() {
    let mut town = test_state(open_grid(), 1, 1);
    town.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = 1;

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "B", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(town.turn, 0);
    assert_eq!(town.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX], 1);
    assert_eq!(town.message, "Wooden Box: How use it?");

    let mut missing = test_state(open_grid(), 1, 1);
    assert_eq!(missing.use_wooden_box(), MoveOutcome::Blocked);
    assert_eq!(missing.turn, 0);
    assert_eq!(missing.message, "No Wooden Box!");
}

#[test]
fn use_command_routes_sceptre_to_top_down_barrier_dissolve() {
    let mut grid = open_grid();
    grid[1 * 32 + 1] = 0x70;
    grid[1 * 32 + 2] = 0x7f;
    grid[2 * 32 + 1] = 0x6f;
    grid[2 * 32 + 2] = 0x80;
    let mut town = test_state(grid, 1, 1);
    town.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 1;
    town.visibility_dirty = false;

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "SC", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(town.grid[1 * 32 + 1], 0x44);
    assert_eq!(town.grid[1 * 32 + 2], 0x44);
    assert_eq!(town.grid[2 * 32 + 1], 0x6f);
    assert_eq!(town.grid[2 * 32 + 2], 0x80);
    assert_eq!(town.turn, 1);
    assert!(town.visibility_dirty);
    assert_eq!(
        town.message,
        "Wielded Sceptre: dissolved 2 barrier cell(s)."
    );
}

#[test]
fn sceptre_requires_item_non_dungeon_and_matching_nearby_barriers() {
    let mut missing = test_state(open_grid(), 1, 1);
    assert_eq!(missing.use_sceptre_of_lord_british(), MoveOutcome::Blocked);
    assert_eq!(missing.turn, 0);
    assert_eq!(missing.message, "No Sceptre!");

    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    dungeon.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 1;
    assert_eq!(dungeon.use_sceptre_of_lord_british(), MoveOutcome::Blocked);
    assert_eq!(dungeon.turn, 0);
    assert_eq!(dungeon.message, "Not here!");

    let mut no_barrier = test_state(open_grid(), 1, 1);
    no_barrier.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 1;
    no_barrier.visibility_dirty = false;
    assert_eq!(
        no_barrier.use_sceptre_of_lord_british(),
        MoveOutcome::Blocked
    );
    assert_eq!(no_barrier.turn, 0);
    assert!(!no_barrier.visibility_dirty);
    assert_eq!(no_barrier.message, "Wielded Sceptre: No effect.");
}

#[test]
fn use_command_routes_worn_regalia_and_badge_toggles() {
    let mut town = test_state(open_grid(), 1, 1);
    town.special_items[SPECIAL_ITEM_CROWN_LB_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    town.special_items[SPECIAL_ITEM_AMULET_LB_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    town.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    town.visibility_dirty = false;

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "CR", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(
        town.special_items[SPECIAL_ITEM_CROWN_LB_INDEX],
        SPECIAL_ITEM_WORN_VALUE
    );
    assert_eq!(
        town.special_items[SPECIAL_ITEM_AMULET_LB_INDEX],
        SPECIAL_ITEM_OWNED_VALUE
    );
    assert_eq!(
        town.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX],
        SPECIAL_ITEM_OWNED_VALUE
    );
    assert_eq!(town.turn, 1);
    assert!(town.visibility_dirty);
    assert_eq!(town.message, "Wearing Crown.");

    town.visibility_dirty = false;
    assert_eq!(
        handle_play_key_input(&mut town, 'U', "AM", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(
        town.special_items[SPECIAL_ITEM_CROWN_LB_INDEX],
        SPECIAL_ITEM_OWNED_VALUE
    );
    assert_eq!(
        town.special_items[SPECIAL_ITEM_AMULET_LB_INDEX],
        SPECIAL_ITEM_WORN_VALUE
    );
    assert_eq!(
        town.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX],
        SPECIAL_ITEM_OWNED_VALUE
    );
    assert_eq!(town.turn, 2);
    assert!(town.visibility_dirty);
    assert_eq!(town.message, "Wearing Amulet.");

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "AM", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        town.special_items[SPECIAL_ITEM_AMULET_LB_INDEX],
        SPECIAL_ITEM_OWNED_VALUE
    );
    assert_eq!(town.turn, 3);
    assert_eq!(town.message, "Removed Amulet.");

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "BB", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        town.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX],
        SPECIAL_ITEM_WORN_VALUE
    );
    assert_eq!(town.turn, 4);
    assert_eq!(town.message, "Wearing Black Badge.");
}

#[test]
fn worn_regalia_requires_owned_item_without_turn() {
    let mut town = test_state(open_grid(), 1, 1);
    town.visibility_dirty = false;

    assert_eq!(
        town.use_worn_regalia(
            SPECIAL_ITEM_CROWN_LB_INDEX,
            "Crown",
            "Wearing Crown.",
            "Removed Crown.",
        ),
        MoveOutcome::Blocked
    );

    assert_eq!(town.turn, 0);
    assert!(!town.visibility_dirty);
    assert_eq!(town.message, "No Crown!");
}

#[test]
fn hms_cape_plans_use_rigs_ship_for_double_speed() {
    let mut ship = world_state(open_world_grid(), 1, 1);
    ship.player.transport = TransportState::Ship {
        type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 2,
    };
    ship.sync_player_object();
    ship.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] = 1;

    assert_eq!(
        handle_play_key_input(&mut ship, 'U', "P", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(ship.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX], 2);
    assert!(ship.ship_rigging_active());
    assert_eq!(ship.turn, 1);
    assert_eq!(ship.message, "Ship rigged for double speed.");
}

#[test]
fn hms_cape_plans_require_item_and_shipboard_context() {
    let mut missing = world_state(open_world_grid(), 1, 1);
    assert_eq!(missing.use_hms_cape_plans(), MoveOutcome::Blocked);
    assert_eq!(missing.turn, 0);
    assert_eq!(missing.message, "No HMS Cape Plans!");

    let mut on_foot = world_state(open_world_grid(), 1, 1);
    on_foot.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] = 1;
    assert_eq!(on_foot.use_hms_cape_plans(), MoveOutcome::Blocked);
    assert_eq!(on_foot.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX], 1);
    assert_eq!(on_foot.turn, 0);
    assert_eq!(on_foot.message, "Not aboard ship!");
}

#[test]
fn use_command_routes_inline_magic_carpet_request() {
    let mut world = world_state(open_world_grid(), 1, 1);
    world.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 2;
    world.visibility_dirty = false;

    assert_eq!(
        handle_play_key_input(&mut world, 'U', "C", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(
        world.player.transport,
        TransportState::Carpet {
            type_byte: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
            tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
        }
    );
    assert_eq!(
        world.active_objects[0].tile,
        FIRST_PLAYABLE_MAGIC_CARPET_TILE
    );
    assert_eq!(world.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 1);
    assert_eq!(world.turn, 1);
    assert!(world.visibility_dirty);
    assert_eq!(world.message, "Boarded carpet.");
}

#[test]
fn magic_carpet_use_requires_stock_footing_and_accepted_tile() {
    let mut no_stock = world_state(open_world_grid(), 1, 1);
    assert_eq!(no_stock.use_magic_carpet(), MoveOutcome::Blocked);
    assert_eq!(no_stock.message, "No Magic Carpet!");
    assert_eq!(no_stock.turn, 0);

    let mut boarded = world_state(open_world_grid(), 1, 1);
    boarded.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 1;
    boarded.player.transport = TransportState::Skiff {
        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
        tile: FIRST_PLAYABLE_SKIFF_TILE,
    };
    boarded.sync_player_object();
    assert_eq!(boarded.use_magic_carpet(), MoveOutcome::Blocked);
    assert_eq!(boarded.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 1);
    assert_eq!(boarded.message, "On foot.");

    let mut blocked_grid = open_world_grid();
    blocked_grid[world_cell_index(1, 1)] = 0x0c;
    let mut blocked = world_state(blocked_grid, 1, 1);
    blocked.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 1;
    assert_eq!(blocked.use_magic_carpet(), MoveOutcome::Blocked);
    assert_eq!(blocked.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 1);
    assert_eq!(blocked.message, "Not here!");

    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    dungeon.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 1;
    assert_eq!(dungeon.use_magic_carpet(), MoveOutcome::Blocked);
    assert_eq!(dungeon.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 1);
    assert_eq!(dungeon.message, "Not here!");
}

#[test]
fn use_command_routes_inline_skull_key_requests_to_town_lock_handler() {
    let dir = debug_game_dir();
    fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 97 96\n").unwrap();

    let mut town_grid = open_grid();
    town_grid[32 + 2] = 97;
    let mut town = test_state(town_grid, 1, 1);
    town.player.facing = Direction::East;
    town.visibility_dirty = false;
    town.keys = 7;
    town.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] = 2;

    assert_eq!(
        handle_play_key_input(&mut town, 'U', "K", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(town.grid[32 + 2], 96);
    assert_eq!(town.turn, 1);
    assert_eq!(town.keys, 7);
    assert_eq!(town.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX], 1);
    assert!(town.visibility_dirty);
    assert_eq!(town.message, "Unlocked!");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn skull_key_refuses_dungeon_without_consuming_special_key() {
    let dir = debug_game_dir();
    let mut dungeon_grid = open_dungeon_record();
    dungeon_grid[dungeon_cell_index(0, 1, 1)] = 0x00;
    let mut dungeon = dungeon_state(dungeon_grid, 0, 1, 1);
    dungeon.visibility_dirty = false;
    dungeon.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] = 1;

    assert_eq!(
        handle_play_key_input(&mut dungeon, 'U', "J", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(dungeon.grid[dungeon_cell_index(0, 1, 1)], 0x00);
    assert_eq!(dungeon.turn, 0);
    assert_eq!(dungeon.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX], 1);
    assert!(!dungeon.visibility_dirty);
    assert_eq!(dungeon.message, "Not here!");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn skull_key_requires_special_key_stock() {
    let mut town = test_state(open_grid(), 1, 1);
    town.player.facing = Direction::East;

    assert_eq!(town.use_skull_key(None).unwrap(), MoveOutcome::Blocked);

    assert_eq!(town.turn, 0);
    assert_eq!(town.keys, DEFAULT_KEY_STOCK);
    assert_eq!(town.message, "No Skull Keys!");
}

#[test]
fn town_push_uses_clean_sidecar_to_swap_target_into_destination() {
    let dir = debug_game_dir();
    fs::write(dir.join(TOWN_PUSHABLE_TABLE_FILE), "CASTLE:0 0 2 1 44\n").unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = 44;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.visibility_dirty = false;

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Pushed
    );

    assert_eq!(state.grid[32 + 2], 16);
    assert_eq!(state.grid[32 + 3], 44);
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    assert!(state.visibility_dirty);
    assert!(state.message.contains("Pushed tile 44 East"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_push_static_chair_requires_stamp_rotates_and_advances_avatar() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    grid[32 + 2] = 0x90;
    grid[32 + 3] = PUSHABLE_GENERIC_FLOOR_STAMP;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.visibility_dirty = false;

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Pushed
    );

    assert_eq!(state.grid[32 + 2], PUSHABLE_GENERIC_FLOOR_STAMP);
    assert_eq!(state.grid[32 + 3], 0x91);
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    assert!(state.visibility_dirty);
    assert!(state.message.contains("Pushed tile 144 East"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_push_runs_door_close_preflight_before_resolving_destination() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    grid[32 + 2] = 0x90;
    grid[32 + 3] = 16;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.door_tracker = Some(DoorTracker {
        previous_tile: PUSHABLE_GENERIC_FLOOR_STAMP,
        x: 3,
        y: 1,
        turns_remaining: 1,
    });

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Pushed
    );

    assert_eq!(state.grid[32 + 2], PUSHABLE_GENERIC_FLOOR_STAMP);
    assert_eq!(state.grid[32 + 3], 0x91);
    assert_eq!(state.door_tracker, None);
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Pushed tile 144 East"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_push_ticks_unrelated_open_door_once_on_consumed_turn() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    grid[32 + 2] = 0x90;
    grid[32 + 3] = PUSHABLE_GENERIC_FLOOR_STAMP;
    grid[32 + 5] = 16;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.door_tracker = Some(DoorTracker {
        previous_tile: 96,
        x: 5,
        y: 1,
        turns_remaining: 4,
    });

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Pushed
    );

    assert_eq!(
        state.door_tracker,
        Some(DoorTracker {
            previous_tile: 96,
            x: 5,
            y: 1,
            turns_remaining: 3,
        })
    );
    assert_eq!(state.grid[32 + 5], 16);
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_push_static_cannon_uses_cannon_stamp() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    grid[32 + 2] = 0xB4;
    grid[32 + 3] = PUSHABLE_CANNON_FLOOR_STAMP;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Pushed
    );

    assert_eq!(state.grid[32 + 2], PUSHABLE_CANNON_FLOOR_STAMP);
    assert_eq!(state.grid[32 + 3], 0xB5);
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_push_static_family_pulls_when_far_cell_blocked_and_player_on_stamp() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    grid[32 + 1] = PUSHABLE_GENERIC_FLOOR_STAMP;
    grid[32 + 2] = 0x90;
    grid[32 + 3] = 24;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Pushed
    );

    assert_eq!(state.grid[32 + 1], 0x93);
    assert_eq!(state.grid[32 + 2], PUSHABLE_GENERIC_FLOOR_STAMP);
    assert_eq!(state.grid[32 + 3], 24);
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Pulled tile 144 East"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_push_dynamic_object_moves_slot_and_avatar() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 0x5B,
        tile: 0x5B,
        x: 2,
        y: 1,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Pushed
    );

    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        (3, 1)
    );
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Pushed object tile 91 East"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_push_refuses_missing_or_mismatched_sidecar_without_turn() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    grid[32 + 2] = 44;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(state.turn, 0);

    fs::write(dir.join(TOWN_PUSHABLE_TABLE_FILE), "CASTLE:0 0 2 1 45\n").unwrap();
    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(state.grid[32 + 2], 44);
    assert_eq!(state.turn, 0);
    assert_eq!(state.message, "Nothing to push there.");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_push_consumes_turn_when_pushable_destination_is_blocked() {
    let dir = debug_game_dir();
    fs::write(dir.join(TOWN_PUSHABLE_TABLE_FILE), "CASTLE:0 0 2 1 44\n").unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = 44;
    grid[32 + 3] = 0x0c;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(state.grid[32 + 2], 44);
    assert_eq!(state.grid[32 + 3], 0x0c);
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Push blocked"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_push_static_family_wraps_and_advances_avatar() {
    let dir = debug_game_dir();
    let mut grid = open_world_grid();
    grid[world_cell_index(255, 1)] = 0x90;
    grid[world_cell_index(0, 1)] = PUSHABLE_GENERIC_FLOOR_STAMP;
    let mut state = world_state(grid, 254, 1);
    state.player.facing = Direction::East;
    state.visibility_dirty = false;

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Pushed
    );

    assert_eq!(
        state.grid[world_cell_index(255, 1)],
        PUSHABLE_GENERIC_FLOOR_STAMP
    );
    assert_eq!(state.grid[world_cell_index(0, 1)], 0x91);
    assert_eq!((state.player.x, state.player.y), (255, 1));
    assert_eq!(state.turn, 1);
    assert!(state.visibility_dirty);
    assert!(state.message.contains("Pushed tile 144 East"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_push_static_family_pulls_when_far_cell_blocked_and_player_on_stamp() {
    let dir = debug_game_dir();
    let mut grid = open_world_grid();
    grid[world_cell_index(1, 1)] = PUSHABLE_GENERIC_FLOOR_STAMP;
    grid[world_cell_index(2, 1)] = 0x90;
    grid[world_cell_index(3, 1)] = 24;
    let mut state = world_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Pushed
    );

    assert_eq!(state.grid[world_cell_index(1, 1)], 0x93);
    assert_eq!(
        state.grid[world_cell_index(2, 1)],
        PUSHABLE_GENERIC_FLOOR_STAMP
    );
    assert_eq!(state.grid[world_cell_index(3, 1)], 24);
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Pulled tile 144 East"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_push_dynamic_object_moves_slot_and_avatar() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 0x5B,
        tile: 0x5B,
        x: 2,
        y: 1,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Pushed
    );

    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        (3, 1)
    );
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Pushed object tile 91 East"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_push_consumes_turn_when_pushable_destination_is_blocked() {
    let dir = debug_game_dir();
    let mut grid = open_world_grid();
    grid[world_cell_index(2, 1)] = 0x90;
    grid[world_cell_index(3, 1)] = 24;
    let mut state = world_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.push_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(state.grid[world_cell_index(2, 1)], 0x90);
    assert_eq!(state.grid[world_cell_index(3, 1)], 24);
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Push blocked"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_push_prompt_routes_to_overworld_push() {
    let dir = debug_game_dir();
    for key in ['P', 'p'] {
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 1)] = 0x90;
        grid[world_cell_index(3, 1)] = PUSHABLE_GENERIC_FLOOR_STAMP;
        let mut state = world_state(grid, 1, 1);

        assert_eq!(
            handle_play_key_input(&mut state, key, "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Push-");

        assert_eq!(
            handle_play_key_input(&mut state, '6', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.grid[world_cell_index(2, 1)],
            PUSHABLE_GENERIC_FLOOR_STAMP
        );
        assert_eq!(state.grid[world_cell_index(3, 1)], 0x91);
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn ship_transport_can_move_over_water_that_blocks_foot() {
    let mut grid = open_world_grid();
    grid[world_cell_index(1, 0)] = 1;
    let mut foot = world_state(grid.clone(), 0, 0);

    assert_eq!(foot.step(Direction::East), MoveOutcome::Blocked);

    let mut ship = world_state(grid, 0, 0);
    ship.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 0,
        skiffs: 0,
    };
    ship.sync_player_object();

    assert_eq!(ship.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((ship.player.x, ship.player.y), (1, 0));
    assert_eq!(ship.player.transport.save_marker(), TRANSPORT_MARKER_SHIP_FURLED_FIRST + 1);
    assert_eq!(ship.active_objects[0].tile, FIRST_PLAYABLE_FRIGATE_TILE + 1);
}

#[test]
fn hoisted_ship_stalls_in_calm_wind_and_consumes_turn() {
    let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 0,
        skiffs: 0,
    };
    state.sync_player_object();

    assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);

    assert_eq!((state.player.x, state.player.y), (10, 10));
    assert_eq!(state.player.facing, Direction::East);
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
    assert!(state.message.contains("calm wind"));
    assert!(state.sail_stall_pending);
}

#[test]
fn rigged_hoisted_ship_wait_uses_one_minute_cleanup() {
    let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
    state.player.transport = TransportState::Ship {
        type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: true,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 2,
    };
    state.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] = 2;
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 5,
        y: 5,
        z: WorldPlane::Britannia.save_floor(),
        phase: 0x22,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);

    assert_eq!((state.player.x, state.player.y), (10, 10));
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
    assert_eq!(state.active_objects[1].phase, 0x22);
    assert!(state.message.contains("calm wind"));

    assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);

    assert_eq!((state.player.x, state.player.y), (10, 10));
    assert_eq!(state.turn, 2);
    assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
    assert_eq!(state.active_objects[1].phase, 0x21);
}

#[test]
fn pass_reports_and_clears_sail_stall_feedback() {
    let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 0,
        skiffs: 0,
    };
    state.sync_player_object();

    assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);
    assert!(state.sail_stall_pending);

    assert_eq!(state.pass_turn(), MoveOutcome::Passed);
    assert_eq!(state.turn, 2);
    assert_eq!(state.clock, GameClock::new(12, 4).unwrap());
    assert!(state.message.contains("stalled by the wind"));
    assert!(!state.sail_stall_pending);

    assert_eq!(state.pass_turn(), MoveOutcome::Passed);
    assert_eq!(state.message, "Passed.");
}

#[test]
fn hoisted_ship_advances_immediately_with_perpendicular_wind() {
    let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
    state.wind = WindState::North;
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 0,
        skiffs: 0,
    };
    state.sync_player_object();

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (11, 10));
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
}

#[test]
fn hoisted_ship_with_wind_uses_one_wait_tick() {
    let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
    state.wind = WindState::East;
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 0,
        skiffs: 0,
    };
    state.sync_player_object();

    assert_eq!(state.step(Direction::West), MoveOutcome::SailStalled);
    assert_eq!((state.player.x, state.player.y), (10, 10));
    assert_eq!(state.turn, 1);

    assert_eq!(state.step(Direction::West), MoveOutcome::Moved);
    assert_eq!((state.player.x, state.player.y), (9, 10));
    assert_eq!(state.turn, 2);
    assert_eq!(state.clock, GameClock::new(12, 4).unwrap());
}

#[test]
fn hoisted_ship_into_wind_uses_two_wait_ticks() {
    let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
    state.wind = WindState::East;
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 0,
        skiffs: 0,
    };
    state.sync_player_object();

    assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);
    assert_eq!((state.player.x, state.player.y), (10, 10));
    assert_eq!(state.sail_cadence, 1);
    assert_eq!(state.turn, 1);

    assert_eq!(state.step(Direction::East), MoveOutcome::SailStalled);
    assert_eq!((state.player.x, state.player.y), (10, 10));
    assert_eq!(state.sail_cadence, 2);
    assert_eq!(state.turn, 2);

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);
    assert_eq!((state.player.x, state.player.y), (11, 10));
    assert_eq!(state.sail_cadence, 0);
    assert_eq!(state.turn, 3);
    assert_eq!(state.clock, GameClock::new(12, 6).unwrap());
}

#[test]
fn save_after_wind_driven_ship_move_persists_wind_and_ship_marker() {
    let dir = debug_game_dir();
    fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0xff, 11, 10)).unwrap();
    fs::write(dir.join(BRIT_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
    fs::write(dir.join(UNDER_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
    let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
    state.wind = WindState::North;
    state.wind_save_byte = WindState::North.save_byte();
    state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST + 1,
        tile: FIRST_PLAYABLE_FRIGATE_TILE + 1,
        sails_hoisted: true,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    assert_eq!(saved[SAVE_WIND_OFFSET], WindState::North.save_byte());
    assert_eq!(
        saved[SAVE_TRANSPORT_MARKER_OFFSET],
        TRANSPORT_MARKER_SHIP_HOISTED_FIRST + 1
    );
    assert_eq!(saved[SAVE_X_OFFSET], 11);
    assert_eq!(saved[SAVE_Y_OFFSET], 10);
    let reloaded = load_play_options_from_save(&dir).unwrap();
    assert_eq!(reloaded.wind, WindState::North);
    assert_eq!(
        reloaded.transport,
        TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST + 1,
            tile: FIRST_PLAYABLE_FRIGATE_TILE + 1,
            sails_hoisted: true,
            hull: 77,
            skiffs: 2,
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn horse_world_movement_uses_one_cell_on_grass_and_path() {
    let mut grass = world_state(open_world_grid(), 0, 0);
    mount_horse(&mut grass);

    assert_eq!(grass.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((grass.player.x, grass.player.y), (1, 0));
    assert_eq!(grass.turn, 1);
    assert!(grass.message.contains("Moved East"));

    let mut path_grid = open_world_grid();
    path_grid[world_cell_index(1, 0)] = 16;
    path_grid[world_cell_index(2, 0)] = 20;
    let mut path = world_state(path_grid, 0, 0);
    mount_horse(&mut path);

    assert_eq!(path.step(Direction::East), MoveOutcome::Moved);
    assert_eq!((path.player.x, path.player.y), (1, 0));
}

#[test]
fn horse_world_movement_uses_one_cell_on_rough_terrain() {
    let mut grid = open_world_grid();
    grid[world_cell_index(1, 0)] = 7;
    let mut state = world_state(grid, 0, 0);
    mount_horse(&mut state);

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Moved East"));
}

#[test]
fn overworld_special_underfoot_latch_forces_darkness_and_holds_valid_movement() {
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = OVERWORLD_UNDERFOOT_BLACKOUT_TILE;
    let mut state = britannia_state(grid, 5, 5);
    state.ambient_light = FULL_DAYLIGHT;
    state.visibility_dirty = false;

    assert!(state.refresh_world_underfoot_blackout_latch());
    assert!(state.world_underfoot_blackout_latched);
    assert_eq!(state.ambient_light, 0);
    assert!(state.visibility_dirty);
    assert_eq!(state.world_visibility_light_threshold(), 0);
    assert!(state.world_visibility_pitch_dark());

    state.visibility_dirty = false;
    assert_eq!(
        state
            .step_world(Direction::East, 6, 5, WorldPlane::Britannia, None)
            .unwrap(),
        MoveOutcome::Used
    );
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("special underfoot tile"));
}

#[test]
fn overworld_special_underfoot_latch_allows_blocked_target_probe_without_turn() {
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = OVERWORLD_UNDERFOOT_BLACKOUT_TILE;
    grid[world_cell_index(6, 5)] = 0x28;
    let mut state = britannia_state(grid, 5, 5);

    assert_eq!(
        state
            .step_world(Direction::East, 6, 5, WorldPlane::Britannia, None)
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.turn, 0);
    assert!(state.world_underfoot_blackout_latched);
}

#[test]
fn overworld_special_underfoot_exempt_tag_skips_latch() {
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = OVERWORLD_UNDERFOOT_BLACKOUT_TILE;
    let mut state = britannia_state(grid, 5, 5);
    state.ambient_light = FULL_DAYLIGHT;
    state.timing_status = TimingStatusTag::Opaque(OVERWORLD_UNDERFOOT_BLACKOUT_EXEMPT_TAG);

    assert!(!state.refresh_world_underfoot_blackout_latch());
    assert_eq!(
        state.world_visibility_light_threshold(),
        u32::from(FULL_DAYLIGHT)
    );
    assert!(!state.world_visibility_pitch_dark());
    assert_eq!(
        state
            .step_world(Direction::East, 6, 5, WorldPlane::Britannia, None)
            .unwrap(),
        MoveOutcome::Moved
    );
    assert_eq!((state.player.x, state.player.y), (6, 5));
}

#[test]
fn overworld_special_underfoot_latch_clears_with_zero_minute_daylight_recompute() {
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = OVERWORLD_UNDERFOOT_BLACKOUT_TILE;
    let mut state = britannia_state(grid, 5, 5);
    state.clock = GameClock::new(12, 0).unwrap();
    state.ambient_light = FULL_DAYLIGHT;

    assert!(state.refresh_world_underfoot_blackout_latch());

    state.grid[world_cell_index(5, 5)] = 5;
    state.visibility_dirty = false;
    assert!(!state.refresh_world_underfoot_blackout_latch());

    assert!(!state.world_underfoot_blackout_latched);
    assert_eq!(state.ambient_light, FULL_DAYLIGHT);
    assert!(state.visibility_dirty);
    assert_eq!(state.turn, 0);
}

#[test]
fn horse_world_movement_does_not_skip_first_cell_plane_transition() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
        "BRITANNIA 1 0 UNDERWORLD 10 20\n",
    )
    .unwrap();
    let mut state = britannia_state(open_world_grid(), 0, 0);
    mount_horse(&mut state);

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
            from: WorldPlane::Britannia,
            to: WorldPlane::Underworld,
        })
    );

    assert_eq!((state.player.x, state.player.y), (10, 20));
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn horse_world_movement_does_not_accept_second_cell_plane_transition() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
        "BRITANNIA 2 0 UNDERWORLD 30 40\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(2, 0)] = 24;
    let mut state = britannia_state(grid, 0, 0);
    mount_horse(&mut state);

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert!(matches!(state.area, Area::World { plane: WorldPlane::Britannia }));
    assert!(matches!(state.player.transport, TransportState::Horse { .. }));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Moved East"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn horse_world_movement_does_not_accept_second_cell_waterfall_sidecar() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_WATERFALL_TABLE_FILE),
        "UNDERWORLD 2 0 EAST 1 24\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(2, 0)] = 24;
    let mut state = world_state(grid, 0, 0);
    mount_horse(&mut state);

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("Moved East to (1, 0)"));
    assert!(!state.message.contains("waterfall swept"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn parse_world_plane_transition_entries_accepts_optional_tile_guard() {
    let entries = parse_world_plane_transition_entries(
        "BRITANNIA 10 20 UNDERWORLD 30 40 0x18\nUNDERWORLD 30 40 BRITANNIA 10 20\n",
    )
    .unwrap();

    assert_eq!(
        entries,
        vec![
            WorldPlaneTransitionEntry {
                from_plane: WorldPlane::Britannia,
                x: 10,
                y: 20,
                to_plane: WorldPlane::Underworld,
                to_x: 30,
                to_y: 40,
                expected_tile: Some(0x18),
            },
            WorldPlaneTransitionEntry {
                from_plane: WorldPlane::Underworld,
                x: 30,
                y: 40,
                to_plane: WorldPlane::Britannia,
                to_x: 10,
                to_y: 20,
                expected_tile: None,
            }
        ]
    );
}

#[test]
fn world_plane_transition_table_rejects_duplicate_source_coordinate_rows() {
    let text = "\
BRITANNIA 10 20 UNDERWORLD 30 40
BRITANNIA 10 20 UNDERWORLD 31 41
";

    assert!(parse_world_plane_transition_entries(text).is_err());
}

#[test]
fn world_plane_transition_table_rejects_duplicate_destination_coordinate_rows() {
    let text = "\
BRITANNIA 10 20 UNDERWORLD 30 40
BRITANNIA 11 21 UNDERWORLD 30 40
";

    assert!(parse_world_plane_transition_entries(text).is_err());
}

#[test]
fn world_plane_transition_table_requires_plane_change() {
    assert!(parse_world_plane_transition_entries("BRITANNIA 10 20 BRITANNIA 30 40\n").is_err());
}

#[test]
fn parse_world_damage_tile_entries_accepts_lava_water_and_optional_tile_guard() {
    let entries = parse_world_damage_tile_entries(
        "BRITANNIA 10 20 LAVA 0x0e\nUNDERWORLD 1 2 water\nBRITANNIA 3 4 DROWNING 1\n",
    )
    .unwrap();

    assert_eq!(
        entries,
        vec![
            WorldDamageTileEntry {
                plane: WorldPlane::Britannia,
                x: 10,
                y: 20,
                effect: WorldDamageEffect::Lava,
                expected_tile: Some(14),
            },
            WorldDamageTileEntry {
                plane: WorldPlane::Underworld,
                x: 1,
                y: 2,
                effect: WorldDamageEffect::Drowning,
                expected_tile: None,
            },
            WorldDamageTileEntry {
                plane: WorldPlane::Britannia,
                x: 3,
                y: 4,
                effect: WorldDamageEffect::Drowning,
                expected_tile: Some(1),
            },
        ]
    );
    assert!(parse_world_damage_tile_entries("BRITANNIA 10 20 ACID\n").is_err());
    assert!(
        parse_world_damage_tile_entries("BRITANNIA 10 20 LAVA\nBRITANNIA 10 20 LAVA\n").is_err()
    );
}

#[test]
fn parse_world_encounter_entries_accepts_clean_rows_and_rejects_bad_values() {
    let entries =
        parse_world_encounter_entries("BRITANNIA 5 30 192 8 0\nUNDERWORLD 0x0e 12 255 -8 4 0x12\n")
            .unwrap();

    assert_eq!(
        entries,
        vec![
            WorldEncounterEntry {
                plane: WorldPlane::Britannia,
                tile: 5,
                threshold: 30,
                type_byte: 192,
                dx: 8,
                dy: 0,
                phase: active_object_phase_from_direction(Direction::West, 0),
            },
            WorldEncounterEntry {
                plane: WorldPlane::Underworld,
                tile: 14,
                threshold: 12,
                type_byte: 255,
                dx: -8,
                dy: 4,
                phase: 0x12,
            },
        ]
    );
    assert!(parse_world_encounter_entries("BRITANNIA 5 31 192 8 0\n").is_err());
    assert!(parse_world_encounter_entries("BRITANNIA 5 30 160 8 0\n").is_err());
    assert!(parse_world_encounter_entries("BRITANNIA 5 30 192 0 0\n").is_err());
    assert!(
        parse_world_encounter_entries("BRITANNIA 5 30 192 8 0\nBRITANNIA 5 20 194 -8 0\n").is_err()
    );
}

#[test]
fn world_encounter_sidecar_spawns_one_actor_after_consumed_overworld_turn() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_ENCOUNTER_TABLE_FILE),
        "BRITANNIA 5 30 192 2 0\n",
    )
    .unwrap();
    let mut state = britannia_state(vec![5; WORLD_CELLS], 10, 10);

    assert_eq!(
        state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Passed
    );

    assert_eq!(state.turn, 1);
    assert_eq!(state.active_objects.len(), 2);
    assert_eq!(
        state.active_objects[1],
        ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 12,
            y: 10,
            z: WorldPlane::Britannia.save_floor(),
            phase: active_object_phase_from_direction(Direction::West, 0),
            aux1: 0,
            aux3: 0,
        }
    );
    assert!(state.visibility_dirty);
    assert!(state.message.contains("Wandering encounter spawned"));
}

#[test]
fn half_time_world_epilogue_alternates_encounter_probe() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_ENCOUNTER_TABLE_FILE),
        "BRITANNIA 5 30 192 2 0\n",
    )
    .unwrap();
    let mut state = britannia_state(vec![5; WORLD_CELLS], 10, 10);
    state.timing_status = TimingStatusTag::HalfTime;

    assert_eq!(
        state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Passed
    );
    assert_eq!(state.turn, 1);
    assert_eq!(state.active_objects.len(), 1);
    assert!(!state.message.contains("Wandering encounter spawned"));

    assert_eq!(
        state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Passed
    );
    assert_eq!(state.turn, 2);
    assert_eq!(state.active_objects.len(), 2);
    assert!(state.message.contains("Wandering encounter spawned"));
}

#[test]
fn no_minute_light_world_epilogue_suppresses_encounter_probe() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_ENCOUNTER_TABLE_FILE),
        "BRITANNIA 5 30 192 2 0\n",
    )
    .unwrap();
    let mut state = britannia_state(vec![5; WORLD_CELLS], 10, 10);
    state.timing_status = TimingStatusTag::NoMinuteLight;

    assert_eq!(
        state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Passed
    );

    assert_eq!(state.turn, 1);
    assert_eq!(state.active_objects.len(), 1);
    assert!(!state.message.contains("Wandering encounter spawned"));
}

#[test]
fn world_encounter_sidecar_respects_zero_threshold_and_blocked_spawn() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_ENCOUNTER_TABLE_FILE),
        "BRITANNIA 5 0 192 2 0\n",
    )
    .unwrap();
    let mut zero_threshold = britannia_state(vec![5; WORLD_CELLS], 10, 10);

    assert_eq!(
        zero_threshold.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Passed
    );
    assert_eq!(zero_threshold.active_objects.len(), 1);

    fs::write(
        dir.join(WORLD_ENCOUNTER_TABLE_FILE),
        "BRITANNIA 5 30 192 2 0\n",
    )
    .unwrap();
    let mut blocked = britannia_state(vec![5; WORLD_CELLS], 10, 10);
    blocked.active_objects.push(ActiveObject {
        type_byte: 194,
        tile: 194,
        x: 12,
        y: 10,
        z: WorldPlane::Britannia.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        blocked.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Passed
    );
    assert_eq!(blocked.active_objects.len(), 2);
    assert_eq!(blocked.active_objects[1].type_byte, 194);
    assert!(!blocked.message.contains("Wandering encounter spawned"));
}

#[test]
fn world_encounter_sidecar_uses_strict_threshold_predicate() {
    let dir = debug_game_dir();
    let entry = WorldEncounterEntry {
        plane: WorldPlane::Britannia,
        tile: 5,
        threshold: 22,
        type_byte: 192,
        dx: 2,
        dy: 0,
        phase: active_object_phase_from_direction(Direction::West, 0),
    };
    let mut equal_roll = britannia_state(vec![5; WORLD_CELLS], 10, 10);
    equal_roll.prng_state = 0x0009;
    assert_eq!(equal_roll.world_encounter_roll(entry), 22);
    assert_eq!(equal_roll.prng_state, u5_prng_advance_state(0x0009));

    let mut equal_roll = britannia_state(vec![5; WORLD_CELLS], 10, 10);
    equal_roll.prng_state = 0x0009;
    assert_eq!(
        equal_roll
            .apply_world_encounter_sidecar_probe(&[entry], &dir, WorldPlane::Britannia)
            .unwrap(),
        None
    );
    assert_eq!(equal_roll.prng_state, u5_prng_advance_state(0x0009));

    let mut below_threshold = britannia_state(vec![5; WORLD_CELLS], 10, 10);
    below_threshold.prng_state = 0x0033;
    let spawning_entry = WorldEncounterEntry {
        threshold: 23,
        ..entry
    };
    assert_eq!(
        below_threshold
            .apply_world_encounter_sidecar_probe(&[spawning_entry], &dir, WorldPlane::Britannia)
            .unwrap(),
        Some(1)
    );
    assert_eq!(below_threshold.prng_state, u5_prng_advance_state(0x0033));
}

#[test]
fn native_world_encounter_probe_runs_when_sidecar_is_absent() {
    let dir = debug_game_dir();
    let mut state = britannia_state(vec![0x04; WORLD_CELLS], 1, 11);
    state.prng_state = 0x0033;
    let starting_prng_state = state.prng_state;

    let slot = state
        .apply_world_encounter_probe(&dir, WorldPlane::Britannia)
        .unwrap();

    assert_eq!(slot, Some(1));
    assert_ne!(state.prng_state, starting_prng_state);
    let object = state.active_objects[1];
    assert!(matches!(
        object.type_byte,
        0xC0 | 0xC8 | 0x90 | 0x98 | 0xBC | 0xC4 | 0xD0 | 0xE4 | 0xCC | 0xD4 | 0xDC | 0xD8
    ));
    assert_eq!(object.tile, object.type_byte);
    assert_eq!(object.z, WorldPlane::Britannia.save_floor());
    assert_eq!(object.aux1, 0);
    assert_eq!(object.aux3, 0);
    assert!(state.visibility_dirty);
}

#[test]
fn world_encounter_sidecar_unmatched_tile_falls_back_to_native_probe() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_ENCOUNTER_TABLE_FILE),
        "BRITANNIA 5 0 192 2 0\n",
    )
    .unwrap();
    let mut state = britannia_state(vec![0x04; WORLD_CELLS], 1, 11);
    state.prng_state = 0x0033;

    let slot = state
        .apply_world_encounter_probe(&dir, WorldPlane::Britannia)
        .unwrap();

    assert_eq!(slot, Some(1));
    assert!(matches!(
        state.active_objects[1].type_byte,
        0xC0 | 0xC8 | 0x90 | 0x98 | 0xBC | 0xC4 | 0xD0 | 0xE4 | 0xCC | 0xD4 | 0xDC | 0xD8
    ));
    assert!(state.visibility_dirty);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn native_world_encounter_probe_respects_zero_threshold() {
    let dir = debug_game_dir();
    let mut state = britannia_state(vec![0x20; WORLD_CELLS], 1, 11);
    state.clock.hour = 12;

    let slot = state
        .apply_world_encounter_probe(&dir, WorldPlane::Britannia)
        .unwrap();

    assert_eq!(slot, None);
    assert_eq!(state.active_objects.len(), 1);
    assert!(!state.visibility_dirty);
}

#[test]
fn native_world_encounter_probe_skips_during_active_combat() {
    let dir = debug_game_dir();
    let mut state = britannia_state(vec![0x04; WORLD_CELLS], 1, 11);
    state.combat_active = true;

    let slot = state
        .apply_world_encounter_probe(&dir, WorldPlane::Britannia)
        .unwrap();

    assert_eq!(slot, None);
    assert_eq!(state.active_objects.len(), 1);
}

#[test]
fn native_world_encounter_spawner_seeds_sea_creature_auxiliary() {
    let mut state = britannia_state(vec![0x02; WORLD_CELLS], 0, 40);
    state.prng_state = 0x0009;
    let starting_prng_state = state.prng_state;

    let slot = state.spawn_native_world_encounter(WorldPlane::Britannia);

    assert_eq!(slot, Some(1));
    assert_ne!(state.prng_state, starting_prng_state);
    let object = state.active_objects[1];
    assert_eq!(object.type_byte, 0x2C);
    assert_eq!(object.tile, 0x2C);
    assert_eq!(object.z, WorldPlane::Britannia.save_floor());
    assert_eq!(object.aux1, SEA_CREATURE_SPAWN_AUX_SEED);
    assert_eq!(object.aux3, 0);
}

#[test]
fn native_world_encounter_spawner_evicts_on_a_full_table_per_encounters_md_9() {
    // `encounters.md §9`: "A full table does **not** make the spawn
    // fail. The spawner asks the shared slot allocator for a record, and
    // that allocator's priority cascade evicts a lower-priority object
    // [...] rather than returning nothing. An earlier revision of this
    // section said the spawn silently fails when the table is full; that
    // is withdrawn. The only silent no-spawn outcome traced in the
    // spawner itself is the coordinate loop giving up after one hundred
    // twenty-eight rejected candidate cells."
    let mut state = britannia_state(vec![0x02; WORLD_CELLS], 0, 40);
    state.prng_state = 0x0009;
    state.active_objects.resize(
        OOL_SLOTS,
        ActiveObject {
            type_byte: 0x01,
            tile: 0x01,
            x: 200,
            y: 200,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        },
    );

    let slot = state
        .spawn_native_world_encounter(WorldPlane::Britannia)
        .expect("a full ordinary range must evict, not decline the spawn");

    assert!(
        (1..=ACTIVE_OBJECT_ACQUISITION_LAST_SLOT).contains(&slot),
        "eviction must stay inside the ordinary acquisition range, got {slot}"
    );
    assert_eq!(state.active_objects[slot].type_byte, 0x2C);
}

#[test]
fn native_world_encounter_spawner_declines_only_after_the_128_candidate_loop() {
    // `encounters.md §4` step 5: "After one hundred twenty-eight
    // rejected candidates, return silently without spawning." Mountains
    // (`0x0C`) are a hard terrain reject, so every candidate fails and
    // the loop is the sole no-spawn path.
    assert_eq!(ENCOUNTER_SPAWNER_RETRY_LIMIT, 128);

    let mut state = britannia_state(vec![0x0C; WORLD_CELLS], 0, 40);
    state.prng_state = 0x0009;

    assert_eq!(state.spawn_native_world_encounter(WorldPlane::Britannia), None);
    assert_eq!(state.active_objects.len(), 1);
}

#[test]
fn native_world_encounter_type_handles_special_terrain_branches() {
    let mut state = britannia_state(vec![0x04; WORLD_CELLS], 1, 11);

    assert_eq!(
        state.native_world_encounter_type(WorldPlane::Underworld, 0x04, 0),
        Some(0xF8)
    );
    assert_eq!(
        state.native_world_encounter_type(WorldPlane::Britannia, 0x0C, 0),
        None
    );
    assert_eq!(
        state.native_world_encounter_type(WorldPlane::Britannia, 0x80, 0),
        None
    );
}

#[test]
fn world_swamp_status_tick_poisons_living_unpoisoned_members_on_foot() {
    let mut grid = open_world_grid();
    grid[world_cell_index(1, 1)] = BRIT_SWAMP_TILE;
    let mut state = britannia_state(grid, 1, 1);
    state.party = vec![
        PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 10,
            max_hp: 10,
            level: 1,
        },
        PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 10,
            max_hp: 10,
            level: 1,
        },
        PartyMember {
            slot: 2,
            class_byte: b'C',
            status: b'S',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 10,
            max_hp: 10,
            level: 1,
        },
        PartyMember {
            slot: 3,
            class_byte: b'D',
            status: b'D',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 0,
            max_hp: 10,
            level: 1,
        },
    ];

    assert_eq!(
        state.apply_world_underfoot_status_tick(WorldPlane::Britannia),
        Some("swamp poison: set party slots 0, 2 to poisoned".to_string())
    );
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.party[1].status, b'P');
    assert_eq!(state.party[2].status, b'P');
    assert_eq!(state.party[3].status, b'D');

    assert_eq!(
        state.apply_world_underfoot_status_tick(WorldPlane::Britannia),
        Some("swamp poison skipped for 3 living member(s)".to_string())
    );
}

#[test]
fn world_swamp_status_tick_skips_carpet_overflight() {
    let mut grid = open_world_grid();
    grid[world_cell_index(1, 1)] = BRIT_SWAMP_TILE;
    let mut state = britannia_state(grid, 1, 1);
    state.player.transport = TransportState::Carpet {
        type_byte: 184,
        tile: 184,
    };
    state.sync_player_object();

    assert_eq!(
        state.apply_world_underfoot_status_tick(WorldPlane::Britannia),
        None
    );
    assert_eq!(state.party[0].status, b'G');
}

#[test]
fn world_swamp_poison_ticks_after_pass_turn() {
    let dir = debug_game_dir();
    let mut grid = open_world_grid();
    grid[world_cell_index(1, 1)] = BRIT_SWAMP_TILE;
    let mut state = britannia_state(grid, 1, 1);

    assert_eq!(
        state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Passed
    );

    assert_eq!(state.party[0].status, b'P');
    assert!(state.message.contains("swamp poison"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_swamp_poison_ticks_after_movement_landing() {
    let mut grid = open_world_grid();
    grid[world_cell_index(2, 1)] = BRIT_SWAMP_TILE;
    let mut state = britannia_state(grid, 1, 1);

    assert_eq!(
        state.step_with_game_dir(Direction::East, None).unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.party[0].status, b'P');
    assert!(state.message.contains("swamp poison"));
}

#[test]
fn world_encounter_spawn_is_included_in_saved_overworld_overlay() {
    let dir = debug_game_dir();
    fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0, 10, 10)).unwrap();
    fs::write(
        dir.join(WORLD_ENCOUNTER_TABLE_FILE),
        "BRITANNIA 5 30 192 2 0\n",
    )
    .unwrap();
    let mut state = britannia_state(vec![5; WORLD_CELLS], 10, 10);

    assert_eq!(
        state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Passed
    );
    assert_eq!(state.active_objects[1].type_byte, 192);

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let saved_ool = fs::read(dir.join("SAVED.OOL")).unwrap();
    let britannia = decode_ool_plane_objects(&saved_ool[..OOL_PLANE_LEN]).unwrap();
    assert_eq!(britannia[0], state.active_objects[1]);

    let saved_gam = fs::read(dir.join("SAVED.GAM")).unwrap();
    let saved_active = decode_active_object_table(
        &saved_gam[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN],
        "SAVED.GAM",
    )
    .unwrap();
    assert_eq!(saved_active[0], state.active_objects[1]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_waterfall_sidecar_does_not_sweep_after_successful_water_movement() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_WATERFALL_TABLE_FILE),
        "BRITANNIA 1 0 EAST 3 1\n",
    )
    .unwrap();
    let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 20,
        skiffs: 1,
    };
    state.sync_player_object();

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (1, 0)
    );
    assert_eq!(state.turn, 1);
    assert!(!state.message.contains("waterfall swept"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn ordinary_water_movement_does_not_queue_waterfall_sweep_or_lava_sidecar() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_WATERFALL_TABLE_FILE),
        "BRITANNIA 1 0 EAST 3 1\n",
    )
    .unwrap();
    fs::write(
        dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
        "BRITANNIA 3 0 LAVA 1\n",
    )
    .unwrap();
    let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 20,
        skiffs: 1,
    };
    state.sync_player_object();

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert_eq!(state.turn, 1);
    assert!(!state.message.contains("waterfall swept"));
    assert!(!state.message.contains("lava damage"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn waterfall_sidecar_does_not_apply_clean_plane_transition() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_WATERFALL_TABLE_FILE),
        "BRITANNIA 1 0 EAST 3 1\n",
    )
    .unwrap();
    fs::write(
        dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
        "BRITANNIA 3 0 UNDERWORLD 30 40\n",
    )
    .unwrap();
    let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 20,
        skiffs: 1,
    };
    state.sync_player_object();

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert!(matches!(state.player.transport, TransportState::Ship { .. }));
    assert_eq!(
        state.active_objects[0].z,
        WorldPlane::Britannia.save_floor()
    );
    assert_eq!(state.turn, 1);
    assert!(!state.message.contains("waterfall swept"));
    assert!(!state.message.contains("F-A-L-L-S!"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_waterfall_tile_guard_mismatch_keeps_normal_movement() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_WATERFALL_TABLE_FILE),
        "BRITANNIA 1 0 EAST 3 2\n",
    )
    .unwrap();
    let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 20,
        skiffs: 1,
    };
    state.sync_player_object();

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert_eq!(state.turn, 1);
    assert!(!state.message.contains("waterfall swept"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn malformed_retired_waterfall_sidecar_is_ignored_by_runtime_movement() {
    let dir = debug_game_dir();
    fs::write(dir.join(WORLD_WATERFALL_TABLE_FILE), "not a promoted runtime sidecar\n").unwrap();
    let mut state = britannia_state(vec![1; WORLD_CELLS], 0, 0);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 20,
        skiffs: 1,
    };
    state.sync_player_object();

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert_eq!(state.turn, 1);
    assert!(!state.message.contains("waterfall swept"));
    let _ = fs::remove_dir_all(dir);
}
