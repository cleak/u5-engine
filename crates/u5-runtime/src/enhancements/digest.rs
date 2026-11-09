//! The trajectory digest (Phase B design 1.3.12).
//!
//! The equivalence harness proves that an enhancement does not change the
//! game: a run with a feature on, its own events removed, must pass through
//! exactly the states of the all-off run. [`trajectory_digest`] is how a
//! state is compared. It hashes every field of [`PlayState`] with FNV-1a-64
//! (the same function as [`fnv1a64`]) in a canonical byte form:
//!
//! - integers as little-endian bytes, `usize` and `isize` widened to 64
//!   bits, so a digest does not depend on the host;
//! - sequences with their length first, strings as their bytes;
//! - maps in sorted key order. It never uses `Debug`, which prints a
//!   `HashMap` in a different order on every run, and never
//!   `DefaultHasher`, whose output may change between Rust releases.
//!
//! **Every field is classified.** [`trajectory_digest`] destructures
//! `PlayState` without `..`, so a new field does not compile until it is
//! listed here, in one of the [`DIGEST_GROUPS`] or as skipped with a
//! reason. Nothing is skipped today: the enhancements add no field, and
//! every field the game has is state a feature must not change, the asset
//! caches and the harness switches included.
//!
//! The groups exist for diagnosis: [`digest_groups`] gives one hash per
//! group, so a failing equivalence run names the part of the state that
//! moved. The digest's `state` is the hash of those group hashes; `prng`,
//! `turn` and `transcript` are kept apart because they are the first
//! thing a reader of a failure wants to see.
//!
//! The digest reads `&PlayState` and writes nothing. It exists only in the
//! harness and its tests; no frontend computes it while a game runs.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use super::sidecar::fnv1a64;
use crate::PlayState;

/// One state of a run, as the equivalence harness compares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TrajectoryDigest {
    /// Every field of `PlayState`, through the [`DIGEST_GROUPS`].
    pub state: u64,
    /// The game's random-number state, `prng_state`.
    pub prng: u16,
    /// The turn counter, `turn`.
    pub turn: u64,
    /// The message window: its transcript and the current message.
    pub transcript: u64,
}

impl std::fmt::Display for TrajectoryDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "state {:016x} prng {:04x} turn {} transcript {:016x}",
            self.state, self.prng, self.turn, self.transcript
        )
    }
}

/// The field groups the digest hashes, in order. [`digest_groups`] returns
/// one hash per name.
pub const DIGEST_GROUPS: [&str; 11] = [
    "position",
    "objects",
    "clock",
    "party",
    "world",
    "light",
    "combat",
    "commands",
    "presentation",
    "transcript",
    "tables",
];

/// Fields deliberately left out of the digest, each with its reason. A
/// field is skipped only when it cannot differ between two equivalent runs
/// for a reason outside the game. Today there is none.
pub const SKIPPED_FIELDS: [(&str, &str); 0] = [];

/// The digest of one state.
pub fn trajectory_digest(s: &PlayState) -> TrajectoryDigest {
    let groups = digest_groups(s);
    let mut all = Canonical::new();
    for (name, hash) in DIGEST_GROUPS.iter().zip(groups) {
        name.hash(&mut all);
        hash.hash(&mut all);
    }
    let mut transcript = Canonical::new();
    s.message_transcript.hash(&mut transcript);
    s.message.hash(&mut transcript);
    TrajectoryDigest {
        state: all.finish(),
        prng: s.prng_state,
        turn: s.turn,
        transcript: transcript.finish(),
    }
}

/// The FNV-1a-64 hasher in the canonical byte form described above.
struct Canonical(u64);

impl Canonical {
    fn new() -> Self {
        Self(fnv1a64(&[]))
    }
}

impl Hasher for Canonical {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    fn write_u128(&mut self, i: u128) {
        self.write(&i.to_le_bytes());
    }

    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }

    fn write_i16(&mut self, i: i16) {
        self.write(&i.to_le_bytes());
    }

    fn write_i32(&mut self, i: i32) {
        self.write(&i.to_le_bytes());
    }

    fn write_i64(&mut self, i: i64) {
        self.write(&i.to_le_bytes());
    }

    fn write_i128(&mut self, i: i128) {
        self.write(&i.to_le_bytes());
    }

    fn write_isize(&mut self, i: isize) {
        self.write_i64(i as i64);
    }
}

/// Feeds each field to the hasher in turn.
macro_rules! put {
    ($h:ident, $($field:ident),+ $(,)?) => {
        $($field.hash(&mut $h);)+
    };
}

/// A map in sorted key order, never in its iteration order.
fn hash_sorted_map<K: Hash + Ord, V: Hash>(map: &HashMap<K, V>, h: &mut Canonical) {
    let mut entries: Vec<(&K, &V)> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries.len().hash(h);
    for (key, value) in entries {
        key.hash(h);
        value.hash(h);
    }
}

/// One hash per entry of [`DIGEST_GROUPS`].
pub fn digest_groups(s: &PlayState) -> [u64; DIGEST_GROUPS.len()] {
    // No `..`: a new field fails to compile here until it is placed in a
    // group below or listed in `SKIPPED_FIELDS` with its reason.
    let PlayState {
        // position: where the party is and the map under it.
        area,
        player,
        passability,
        grid,
        world_live_chunks,
        top_down_previous_viewport,
        pending_dungeon_surface_exit,
        pending_dungeon_underfoot_reaction,
        saved_scene_scratch,
        preserved_transport_marker,
        sail_cadence,
        sail_cached_direction,
        pending_vehicle_save,
        debug_enter,
        return_world,
        save_template_source,
        wind,
        wind_save_byte,
        // objects: monsters, people, doors and gates on the map.
        active_objects,
        npcs,
        door_tracker,
        door_tracker_closed,
        opened_town_doors,
        revealed_town_secret_doors,
        natural_moongate_counter,
        natural_moongate_live_cells,
        last_natural_moongate_transit,
        natural_moongate_hook_held_for_warp_arrival,
        dungeon_monster_previous_cell,
        dungeon_monster_painted_poses,
        town_npc_mutations,
        removed_town_npc_flags,
        inn_registry,
        // clock: time, the turn loop's latches and pacing.
        clock,
        cleanup_previous_hour,
        twelve_hour_audio_repeats,
        ambient_audio_sub_tick,
        dungeon_loop_minute_charged,
        dungeon_loop_iteration,
        dungeon_flourish_draw,
        prng_state,
        animation,
        water_scroll,
        fire_flicker,
        dungeon_fountain_frame,
        dungeon_footstep_intensity,
        dungeon_footstep_just_emitted,
        typeahead_flush_requested,
        dungeon_runtime_paint,
        session_start_world_step_frozen,
        overworld_input_helper_entry_owed,
        town_input_helper_quick_poll,
        pending_town_status_provision_pass,
        pending_dungeon_status_provision_pass,
        pending_dungeon_post_action_helper,
        pending_town_npc_schedule_pass,
        pending_town_active_object_pass,
        pending_town_active_object_animate_pass,
        pending_world_object_walker_pass,
        transport_walker_gate_parity,
        quickness_walker_gate_parity,
        town_post_action_cleanup_skipped,
        world_walkers_ran_this_turn,
        under_sail_wait_cursor_poll_pending,
        cached_moon_glyph_bytes,
        time_stop_counter,
        active_effect_tag,
        active_effect_counter,
        turn,
        typeahead_buffer_enabled,
        music_enabled,
        exploration_acted_charge_owed,
        dungeon_acted_turn_before,
        harness_shortcut_keys,
        pace_combat_presentations,
        // party: members, inventory and the active player.
        food,
        gold,
        keys,
        gems,
        climbing_gear,
        special_items,
        party,
        party_names,
        party_experience,
        party_stay_counters,
        party_strengths,
        party_combat_defense,
        party_intelligence,
        party_equipment,
        party_roster,
        equipment_stock,
        spell_charges,
        scroll_stock,
        potion_stock,
        reagents,
        avatar_stats,
        torches,
        torch_counter,
        light_spell_counter,
        camp_cooldown,
        camp_month_cookie,
        active_player,
        active_player_save_byte,
        // world: quest progress and the world's memory.
        rare_reagent_harvest_days,
        fixed_hidden_treasure_found,
        fixed_hidden_treasure_daily_day,
        dungeon_room_clear_bitmap,
        moonstone_slots,
        shadowlord_hideouts,
        resident_shadowlord,
        summoned_shadowlord,
        shrine_ordained_mask,
        shrine_codex_mask,
        word_of_power_seal_flags,
        shrine_ruin_flags,
        moral_standing,
        town_drunkenness_counter,
        tavern_secondary_drink_count,
        toll_progress,
        fortunes_of_war,
        harpsichord_progress,
        talk_branch_flags,
        conversation_signal_flags,
        blackthorn_audience_punished,
        endgame,
        endgame_entry_redraw_pending,
        endgame_absorption_arena,
        // light: light, sight and what the party can see.
        ambient_light,
        light_beacon,
        beacon_bearing_stencils,
        local_light_mask,
        visibility_dirty,
        visibility_grid,
        terrain_band,
        visibility_buffers_ready,
        world_underfoot_blackout_latched,
        world_underfoot_probe_owed,
        visibility_sweep,
        party_marker_tile_override,
        // combat: the arena and its actors.
        combat_round_counter,
        combat_action_result,
        combat_wound_classified_at_hit,
        combat_pending_divide_line,
        combat_pending_hazard_lines,
        combat_held_faint_lines,
        combat_interference_sources,
        combat_active,
        combat_frame_snapshot,
        pending_combat_actor_slot,
        combat_transcript_row_open,
        combat_prompt_row_opened_by_banner,
        active_combat_targeting,
        combat_remembered_targets,
        pending_combat_terrain_trigger_slot,
        next_combat_actor_slot,
        combat_terrain,
        combat_magic_effects,
        combat_cursor_blink,
        combat_round_loop_prologue_ran,
        combat_owed_monster_attack_narrations,
        combat_monster_attack_narrated_slots,
        combat_aim_marker_cell,
        combat_aim_marker_gate,
        combat_ambush_reveals,
        combat_actors,
        combat_entry_chrome,
        post_combat_ribbon_hold,
        combat_exit_skips_world_turn,
        // commands: open commands, prompts and conversations.
        pending_cast_argument,
        cast_resources_spent,
        pending_town_conflict,
        pending_town_npc_contact,
        pending_outdoor_reaction_slots,
        pending_difficult_terrain_step,
        pending_blackthorn_merciful_death,
        pending_blackthorn_execution_swell,
        pending_blackthorn_execution_victim,
        pending_blackthorn_rescue,
        pending_blackthorn_audience_exit,
        pending_shrine_third_mantra,
        active_blackthorn_guard_demand,
        pending_town_arrest,
        active_blackthorn,
        blackthorn_capture,
        active_shop,
        arms_transaction_completed,
        active_conversation,
        active_conversation_npc_slot,
        talk_entered_by_contact,
        pending_talk_shop_turn,
        pending_look_prompt_turn,
        pending_modal_wait_turn,
        pending_talk_conversation_turn,
        active_conversation_join_candidate,
        active_z_stats,
        active_party_selector,
        active_ready,
        active_use,
        active_cast,
        active_cast_followup,
        active_rest,
        active_jimmy,
        active_surface_chest,
        search_acting_member,
        active_shrine,
        codex_presentation_steps,
        active_mix,
        active_new_order,
        active_yell,
        active_shrine_restoration,
        active_wishing_well,
        active_view_overlay,
        active_direction_prompt,
        active_yes_no_prompt,
        // presentation: screen state the runtime keeps between frames.
        pending_map_viewport_dissolves,
        pending_blackthorn_rescue_playbacks,
        pending_combat_terrain_reveals,
        pending_potion_flash,
        message_window_flash_hold,
        pending_stonegate_trapdoor_playback,
        view_overlay_row_opened_by_result,
        view_overlay_idle_polls,
        surface_command_row_follows_history,
        overworld_prompt_pair_suppressed,
        town_hiccup_echo_continues,
        stats_panel_window_inverse,
        staged_narration,
        pending_text_hold,
        message_row_open_mid_line,
        pending_entry_leaves_row_open,
        keys_buffered_during_hold,
        stats_panel,
        stats_panel_hold,
        use_picker_panel,
        staged_stats_panels,
        stats_panel_refresh_requested,
        stats_panel_command_refresh,
        stats_panel_command_baseline,
        tile_restoration_pending,
        pending_driver_tile_graphics_saves,
        pending_driver_tile_graphics_restores,
        party_stats_panel_refresh_pending,
        sound_effect_serial,
        sound_effect_history,
        stats_row_flash_history,
        cutscene_black_viewport,
        blackthorn_rescue_reveals_shown,
        blackthorn_rescue_viewport,
        blackthorn_rescue_spectral_actor,
        blackthorn_rescue_animation_tick,
        blackthorn_audience_actor_variants,
        blackthorn_audience_composite_tick,
        presentation_entry_chrome,
        shrine_approach_walk,
        presentation_viewport_inverted,
        presentation_camp_flash_inverted,
        presentation_stats_row_flash,
        // transcript: the message window and the diagnostics log.
        message,
        message_transcript,
        message_window_top_offset,
        message_transcript_revision,
        message_flushed,
        pending_command_echo,
        pending_dungeon_scroll_rune_rows,
        pending_hourly_status_message,
        diagnostics,
        diagnostics_serial,
        // tables: data loaded from the game files and kept for the scene.
        world_overlays,
        look_table,
        common_word_dictionary,
        blackthorn_audience_map,
        shrine_presentation_map,
    } = s;

    let mut groups = [0; DIGEST_GROUPS.len()];
    let mut h = Canonical::new();
    put!(
        h,
        area,
        player,
        passability,
        grid,
        world_live_chunks,
        top_down_previous_viewport,
        pending_dungeon_surface_exit,
        pending_dungeon_underfoot_reaction,
        saved_scene_scratch,
        preserved_transport_marker,
        sail_cadence,
        sail_cached_direction,
    );
    put!(
        h,
        pending_vehicle_save,
        debug_enter,
        return_world,
        save_template_source,
        wind,
        wind_save_byte,
    );
    groups[0] = h.finish();

    let mut h = Canonical::new();
    put!(
        h,
        active_objects,
        npcs,
        door_tracker,
        door_tracker_closed,
        opened_town_doors,
        revealed_town_secret_doors,
        natural_moongate_counter,
        natural_moongate_live_cells,
        last_natural_moongate_transit,
        natural_moongate_hook_held_for_warp_arrival,
        dungeon_monster_previous_cell,
        dungeon_monster_painted_poses,
    );
    put!(h, town_npc_mutations);
    hash_sorted_map(removed_town_npc_flags, &mut h);
    put!(h, inn_registry);
    groups[1] = h.finish();

    let mut h = Canonical::new();
    put!(
        h,
        clock,
        cleanup_previous_hour,
        twelve_hour_audio_repeats,
        ambient_audio_sub_tick,
        dungeon_loop_minute_charged,
        dungeon_loop_iteration,
        dungeon_flourish_draw,
        prng_state,
        animation,
        water_scroll,
        fire_flicker,
        dungeon_fountain_frame,
    );
    put!(
        h,
        dungeon_footstep_intensity,
        dungeon_footstep_just_emitted,
        typeahead_flush_requested,
        dungeon_runtime_paint,
        session_start_world_step_frozen,
        overworld_input_helper_entry_owed,
        town_input_helper_quick_poll,
        pending_town_status_provision_pass,
        pending_dungeon_status_provision_pass,
        pending_dungeon_post_action_helper,
        pending_town_npc_schedule_pass,
        pending_town_active_object_pass,
    );
    put!(
        h,
        pending_town_active_object_animate_pass,
        pending_world_object_walker_pass,
        transport_walker_gate_parity,
        quickness_walker_gate_parity,
        town_post_action_cleanup_skipped,
        world_walkers_ran_this_turn,
        under_sail_wait_cursor_poll_pending,
        cached_moon_glyph_bytes,
        time_stop_counter,
        active_effect_tag,
        active_effect_counter,
        turn,
    );
    put!(
        h,
        typeahead_buffer_enabled,
        music_enabled,
        exploration_acted_charge_owed,
        dungeon_acted_turn_before,
        harness_shortcut_keys,
        pace_combat_presentations,
    );
    groups[2] = h.finish();

    let mut h = Canonical::new();
    put!(
        h,
        food,
        gold,
        keys,
        gems,
        climbing_gear,
        special_items,
        party,
        party_names,
        party_experience,
        party_stay_counters,
        party_strengths,
        party_combat_defense,
    );
    put!(
        h,
        party_intelligence,
        party_equipment,
        party_roster,
        equipment_stock,
        spell_charges,
        scroll_stock,
        potion_stock,
        reagents,
        avatar_stats,
        torches,
        torch_counter,
        light_spell_counter,
    );
    put!(
        h,
        camp_cooldown,
        camp_month_cookie,
        active_player,
        active_player_save_byte,
    );
    groups[3] = h.finish();

    let mut h = Canonical::new();
    put!(
        h,
        rare_reagent_harvest_days,
        fixed_hidden_treasure_found,
        fixed_hidden_treasure_daily_day,
        dungeon_room_clear_bitmap,
        moonstone_slots,
        shadowlord_hideouts,
        resident_shadowlord,
        summoned_shadowlord,
        shrine_ordained_mask,
        shrine_codex_mask,
        word_of_power_seal_flags,
        shrine_ruin_flags,
    );
    put!(
        h,
        moral_standing,
        town_drunkenness_counter,
        tavern_secondary_drink_count,
        toll_progress,
        fortunes_of_war,
        harpsichord_progress,
    );
    hash_sorted_map(talk_branch_flags, &mut h);
    put!(
        h,
        conversation_signal_flags,
        blackthorn_audience_punished,
        endgame,
        endgame_entry_redraw_pending,
    );
    // The endgame's absorption arena is a whole saved state of its own.
    match endgame_absorption_arena {
        None => 0u8.hash(&mut h),
        Some(arena) => {
            1u8.hash(&mut h);
            trajectory_digest(arena).hash(&mut h);
        }
    }
    groups[4] = h.finish();

    let mut h = Canonical::new();
    put!(
        h,
        ambient_light,
        light_beacon,
        beacon_bearing_stencils,
        local_light_mask,
        visibility_dirty,
        visibility_grid,
        terrain_band,
        visibility_buffers_ready,
        world_underfoot_blackout_latched,
        world_underfoot_probe_owed,
        visibility_sweep,
        party_marker_tile_override,
    );
    groups[5] = h.finish();

    let mut h = Canonical::new();
    put!(
        h,
        combat_round_counter,
        combat_action_result,
        combat_wound_classified_at_hit,
        combat_pending_divide_line,
        combat_pending_hazard_lines,
        combat_held_faint_lines,
        combat_interference_sources,
        combat_active,
        combat_frame_snapshot,
        pending_combat_actor_slot,
        combat_transcript_row_open,
        combat_prompt_row_opened_by_banner,
    );
    put!(
        h,
        active_combat_targeting,
        combat_remembered_targets,
        pending_combat_terrain_trigger_slot,
        next_combat_actor_slot,
        combat_terrain,
        combat_magic_effects,
        combat_cursor_blink,
        combat_round_loop_prologue_ran,
        combat_owed_monster_attack_narrations,
        combat_monster_attack_narrated_slots,
        combat_aim_marker_cell,
        combat_aim_marker_gate,
    );
    put!(
        h,
        combat_ambush_reveals,
        combat_actors,
        combat_entry_chrome,
        post_combat_ribbon_hold,
        combat_exit_skips_world_turn,
    );
    groups[6] = h.finish();

    let mut h = Canonical::new();
    put!(
        h,
        pending_cast_argument,
        cast_resources_spent,
        pending_town_conflict,
        pending_town_npc_contact,
        pending_outdoor_reaction_slots,
        pending_difficult_terrain_step,
        pending_blackthorn_merciful_death,
        pending_blackthorn_execution_swell,
        pending_blackthorn_execution_victim,
        pending_blackthorn_rescue,
        pending_blackthorn_audience_exit,
        pending_shrine_third_mantra,
    );
    put!(
        h,
        active_blackthorn_guard_demand,
        pending_town_arrest,
        active_blackthorn,
        blackthorn_capture,
        active_shop,
        arms_transaction_completed,
        active_conversation,
        active_conversation_npc_slot,
        talk_entered_by_contact,
        pending_talk_shop_turn,
        pending_look_prompt_turn,
        pending_modal_wait_turn,
    );
    put!(
        h,
        pending_talk_conversation_turn,
        active_conversation_join_candidate,
        active_z_stats,
        active_party_selector,
        active_ready,
        active_use,
        active_cast,
        active_cast_followup,
        active_rest,
        active_jimmy,
        active_surface_chest,
        search_acting_member,
    );
    put!(
        h,
        active_shrine,
        codex_presentation_steps,
        active_mix,
        active_new_order,
        active_yell,
        active_shrine_restoration,
        active_wishing_well,
        active_view_overlay,
        active_direction_prompt,
        active_yes_no_prompt,
    );
    groups[7] = h.finish();

    let mut h = Canonical::new();
    put!(
        h,
        pending_map_viewport_dissolves,
        pending_blackthorn_rescue_playbacks,
        pending_combat_terrain_reveals,
        pending_potion_flash,
        message_window_flash_hold,
        pending_stonegate_trapdoor_playback,
        view_overlay_row_opened_by_result,
        view_overlay_idle_polls,
        surface_command_row_follows_history,
        overworld_prompt_pair_suppressed,
        town_hiccup_echo_continues,
        stats_panel_window_inverse,
    );
    put!(
        h,
        staged_narration,
        pending_text_hold,
        message_row_open_mid_line,
        pending_entry_leaves_row_open,
        keys_buffered_during_hold,
        stats_panel,
        stats_panel_hold,
        use_picker_panel,
        staged_stats_panels,
        stats_panel_refresh_requested,
        stats_panel_command_refresh,
        stats_panel_command_baseline,
    );
    put!(
        h,
        tile_restoration_pending,
        pending_driver_tile_graphics_saves,
        pending_driver_tile_graphics_restores,
        party_stats_panel_refresh_pending,
        sound_effect_serial,
        sound_effect_history,
        stats_row_flash_history,
        cutscene_black_viewport,
        blackthorn_rescue_reveals_shown,
        blackthorn_rescue_viewport,
        blackthorn_rescue_spectral_actor,
        blackthorn_rescue_animation_tick,
    );
    put!(
        h,
        blackthorn_audience_actor_variants,
        blackthorn_audience_composite_tick,
        presentation_entry_chrome,
        shrine_approach_walk,
        presentation_viewport_inverted,
        presentation_camp_flash_inverted,
        presentation_stats_row_flash,
    );
    groups[8] = h.finish();

    let mut h = Canonical::new();
    put!(
        h,
        message,
        message_transcript,
        message_window_top_offset,
        message_transcript_revision,
        message_flushed,
        pending_command_echo,
        pending_dungeon_scroll_rune_rows,
        pending_hourly_status_message,
        diagnostics,
        diagnostics_serial,
    );
    groups[9] = h.finish();

    let mut h = Canonical::new();
    put!(
        h,
        world_overlays,
        look_table,
        common_word_dictionary,
        blackthorn_audience_map,
        shrine_presentation_map,
    );
    groups[10] = h.finish();

    groups
}

#[cfg(test)]
mod tests;
