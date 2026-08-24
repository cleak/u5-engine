# Command Routing Status

This matrix summarizes current command routing in the engine. It is a handoff
document, not a replacement for `u5-spec/systems/commands.md`; when behavior is
uncertain, read the spec and the focused runtime tests before editing.

| Command | World | Town | Dungeon | Combat | Representative evidence |
|---|---|---|---|---|---|
| Space / Enter | Pass turn, post-turn world effects | Pass turn, town effects and exit prompts | Pass/turn in dungeon | Combat command dispatch/round flow | `cargo test -p u5-runtime pass_turn_on_native_town_exit_threshold_tile_prompts_then_exits_on_accept` |
| Movement | Wrapping, vehicles, wind, hazards, swamp poison, encounters, transitions | NPC blocking, doors, stairs, trap doors, exits, pickups | Facing-relative movement, traps, rooms, teleports, exits | Actor movement and collision | `world_movement_wraps_and_advances_outdoor_time`, `world_swamp_poison_ticks_after_pass_turn`, `town_step_onto_clean_trap_door_changes_to_target_floor`, `combat_input_dispatch_out_of_arena_move_restores_stored_frame_snapshot` |
| `A` Attack | Adjacent active object and combat-class handoff | NPC targets, destructive alarm schedule rewrites, combat handoff | Dungeon attack routing | Combat attack command | `world_attack_adjacent_combat_class_object_selects_brit_cbt_arena`, `a_attack_guard_like_town_npc_raises_alarm_without_death_mask` |
| `B` Board | Ship, skiff, horse, carpet/vehicle markers | Refusal or mode-appropriate handling | Refusal | Combat command table owns combat behavior | `vehicle_directional_step_refreshes_transport_marker_and_player_tile` |
| `C` Cast | Shared spell parser, resource gates, allowed spells | Shared spell parser plus indoor absorption gates | Dungeon spells, fields, up/down, Gate Travel | Combat spells, active effects, summons, fields | `active_cast_prompt_collects_selector_and_dispatches_spell`, `combat_cast_active_target_spell_routes_damage_application` |
| `E` Enter | Published stock location table, optional sidecar override, or debug entry | Mode-specific refusal/interaction | Dungeon-mode route as specified | Combat scene commands | `world_enter_uses_published_location_table_without_sidecar`, `world_enter_reports_no_matching_coordinate` |
| `F` Fire | Ship cannon broadside | Native static cannons plus optional fire-source sidecars | Mode-specific refusal | Combat fire command | `ship_fire` tests in `chunk_05`; `town_fire_uses_adjacent_static_cannon_without_sidecar` |
| `G` Get | Pickups, crops, Moonstones, sidecar grants | Pickups, table food, object-table grants | Underfoot dungeon chests | Combat SJOG branch | `world_get_native_object_pickup_uses_visual_filter_and_class_code_without_sidecar`, `dungeon_open_chest_does_not_apply_clean_sidecar_grants` |
| `H` Hole up | Rest with watch and sleep ambush path | Inn-bed gated town rest | Rest with watch / dungeon ambush path | Combat has separate abort branch | `rest_with_watch_heals_living_members_and_wakes_initial_sleepers`, `town_hole_up_runs_initial_schedule_burst_and_ten_minute_cleanup` |
| `I` Ignite | Torch/light handling | Torch/light handling | Torch/light gate for dungeon view | Combat command path | `active_effect` and light-counter tests |
| `J` Jimmy | Container routes where valid | Native doors, restraint prisoners, object chests | Dungeon chests; every resolved exit commits one action | Doors and restraints | `active_dungeon_jimmy_picker_preserves_prompt_before_key_check`, `active_dungeon_jimmy_cancel_commits_one_action` |
| `K` Klimb | Grapple and plane-transition climb paths | Stairs and floor changes | Ladders and levels | Combat climb command | `world_k_applies_clean_plane_transition_after_successful_climb`, `town_k_prompts_when_both_floor_directions_are_connected` |
| `L` Look | Terrain/object descriptions and special views | Signs, NPCs, objects | First-person focus descriptions | Combat view/target descriptions | `world_look_uses_look2_description_for_wrapped_object`, `town_look_renders_matching_signs_dat_record_without_spending_turn` |
| `M` Mix / meditate | Reagent mixer or shrine/Codex routes | Reagent mixer or shrine/Codex routes | Reagent mixer route where accepted | `Mix-Not here` style combat refusal | `active_mix_prompt_collects_spell_reagent_and_quantity`, shrine/Codex tests in `chunk_14` |
| `N` New order | Party order prompt | Party order prompt | Party order prompt | Combat command table owns combat behavior | `active_new_order_prompt_swaps_non_leader_slots` |
| `O` Open | Doors/containers where modeled | Doors, chests, locks, object helper | Underfoot dungeon chests / public refusal otherwise | Combat open command | `town_open_locked_sidecar_refuses_without_turn`, `dungeon_open_chest_consumes_turn_and_marks_visit_local_open_chest` |
| `P` Push | Static/dynamic pushables | Furniture, cannons, dynamic objects | Dungeon refusal | Combat push command | `town_push_uses_clean_sidecar_to_swap_target_into_destination`, `world_push_static_family_wraps_and_advances_avatar` |
| `Q` Save / exit | Save prompt and save writer | Save prompt and save writer | Dungeon exit/save prompt handling | Free `Quit-Not here` refusal; never causes defeat | `combat_input_dispatch_quickness_never_consumes_the_ready_player`, save/load tests in `chunk_03` |
| `R` Ready | Equipment picker and gates | Equipment picker and gates | Equipment picker and gates | Combat ready/actor command gate | `active_ready_picker_equips_and_unequips_without_turn` |
| `S` Search | Hidden objects, Moonstones, fixed treasure | Secret doors, objects, traps | Dungeon feature/trap/chest search | Combat search/SJOG branch | `town_search_uses_clean_sidecar_to_reveal_secret_door`, `use_moonstone_town_records_scene_floor_and_clears_stale_pickup` |
| `T` Talk | Refusal outside talkable context | NPC dialogue, shops, Blackthorn/service flows | Refusal or dungeon context | Combat command table owns combat behavior | `game_dir_talk_expands_loaded_dictionary_token_without_placeholder`, shop runtime tests |
| `U` Use | Inventory items, Moonstones, regalia, scrolls, potions | Same plus town-specific key/quest routes | Dungeon-gated item effects | Combat-capable potions/scroll effects | `active_use_picker_lists_shadowlord_shards_and_routes_to_handler`, `use_command_routes_scrolls_to_item_effects_without_spell_resources` |
| `V` View | Gem/peer-style overlays | Local view overlay | Dungeon map/view overlay | Combat label-only view | `world_view_decrements_gem_and_wraps_full_fill_map_without_turn` |
| `X` X-it | Vehicle dismount | Vehicle/mode refusal | Mode-specific refusal | Free `X-it what?` refusal; Escape owns combat cleanup | `combat_player_command_handles_digits_pass_branches_and_escape_cleanup` and vehicle tests |
| `Y` Yell | Low-scene frigates toggle sails; otherwise outdoor Words of Power open matching seals or enter the four-answer ruined-shrine restoration flow | Low-scene frigates toggle sails; only the three Eternal Flame keeps accept Shadowlord names | Low-scene defensive frigates toggle sails; ordinary prompted words have no effect | Scene `0xFF` uses the ordinary prompt and no-effect route; an empty submitted prompt is acted in every mode | `yell_scene_context_is_exhaustive_and_uses_only_outdoors_or_three_keeps`, `y_yell_empty_prompt_submission_is_acted_in_every_exploration_mode`, `combat_input_dispatch_empty_yell_commits_the_pending_actor_action`, `y_yell_word_of_power_opens_matching_surface_seal_only_at_target`, `y_yell_ruined_shrine_four_response_success_restores_only_shrine_state`, `y_yell_shadowlord_name_spawns_in_any_eternal_flame_keep` |
| `Z` Stats | Runtime status panel | Runtime status panel | Runtime status panel | Combat status panel | `z_stats_opens_browser_stats_page_without_turn` |

## Notes

- Scene transitions should return explicit transition outcomes so destination
  underfoot effects do not retrigger during the same input.
- Refusals that the spec says are prompt/cancel/no-op paths should not consume
  a turn.
- Stock town/dungeon entry and return coordinates use the published gazetteer
  table; other exact transition coordinate families remain sidecar-backed where
  the public spec has not published rows.
