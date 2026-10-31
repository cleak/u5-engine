use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::Path;

use crate::*;

const SURFACE_LOOK_VISIBILITY_RADIUS: usize = 5;

#[derive(Clone, Debug)]
pub struct UseItemPickerRow {
    pub label: String,
    /// `inventory.md §4.5`: the two-cell quantity; `None` is the
    /// "no quantity" marker that prints only the name.
    pub quantity: Option<u8>,
    /// `inventory.md §4.5` (`RETRACTIONS.md` R403): which presentation family
    /// decorates this row's name. "The two markers decorate all eight scrolls
    /// and all eight potions, respectively"; the Sceptre, Skull Keys and the
    /// other named artifacts carry none.
    pub decoration: crate::stats_panel::PanelPickerDecoration,
    pub(crate) request: UseItemRequest,
}

/// `commands.md §5` + observation: the U-Use handler's message-window
/// prompt, printed on the line after the `Use item` verb echo.
pub const USE_ITEM_PROMPT_MESSAGE: &str = ITEM_SELECTION_PROMPT;

/// `shops.md §2` "Transport gate": "After the schedule test passes, either
/// horse transport marker causes the following fixed refusal, except for the
/// horse-trader trigger". The two lines are published verbatim; the
/// horse-trader trigger (`0x83`) is exempt. This is a fixed merchant refusal,
/// not a per-shop-role line, and "the dispatcher does not reject other
/// transport markers here; on foot and magic carpet pass".
///
/// It is printed by the same shared dispatcher as the schedule refusal, and
/// `conversation.md §2`/§9 step 2 put "the dispatcher's own leading line
/// feed" before every line it prints in place of a conversation, so the
/// literal carries that feed and a blank row stands under the `Talk-` echo
/// (playtest P13-08).
pub const SHOP_MOUNTED_REFUSAL: &str = "\nA merchant says:\n\"GET THAT HORSE OUT OF HERE!\"";

/// `shops.md §2`: the fixed refusal a shop trigger prints when its keeper is
/// not open for business, "with these line breaks" and "a newline before the
/// preamble and after the closing quote".
///
/// Measured 2026-09-07 (`qa/paired/nb-stable.tsv`).
pub const SHOP_CLOSED_REFUSAL: &str =
    "\nA merchant says:\n\"Come see me at\nmy shoppe, when\nit's open!\"\n";

/// `shops.md §8.0`: the **vendor** column of the two resident name tables that
/// are indexed by the shop-instance row. The row is the index of the active
/// scene byte inside the shop kind's own scene list, so a scene-byte lookup
/// per kind resolves the same row the spec's tables are keyed by.
///
/// "Two resident name tables are indexed by the same row ...: the shop's
/// display name, which fills the `#` substitution, and the vendor's name,
/// which fills the `$` substitution and the `says <shopkeeper>.` /
/// `yells <shopkeeper>.` attribution tails. ... Neither name is read from the
/// NPC roster or the conversation blob, so the shopkeeper an implementation
/// names in shop text is a property of the location, not of the NPC the player
/// happened to talk to."
///
/// The shipped horse-trader table holds a fourth row for scene `30` whose
/// vendor is `Simplon`; the spec records that no `0x83` trigger exists for
/// scene `30`, so the row is unreachable and is deliberately not listed here.
/// Returns `None` when the kind's table does not list the active scene — the
/// spec's error case, where a clean implementation "should reject the trigger
/// and leave the conversation alone".
pub const fn shop_vendor_name_for_scene(dialog_id: u8, scene_byte: u8) -> Option<&'static str> {
    let table: &[(u8, &'static str)] = match dialog_id {
        // Arms shops (9 rows).
        0x81 => &[
            (2, "Gwenneth"),
            (3, "Nomaan"),
            (4, "Ronan"),
            (5, "Shenstone"),
            (6, "Paul"),
            (17, "Max"),
            (24, "Kitiara"),
            (26, "Steve"),
            (32, "Thol"),
        ],
        // Taverns / meal counters (9 rows).
        0x82 => &[
            (1, "Sam"),
            (2, "Tika"),
            (3, "Nicole"),
            (4, "Duclas"),
            (8, "Felicity"),
            (19, "Jaymes"),
            (22, "Dr. Cat"),
            (24, "Nikki"),
            (30, "Rob"),
        ],
        // Horse traders (3 reachable rows).
        0x83 => &[(6, "Hettar"), (20, "Theoan"), (22, "Ferru")],
        // Shipwrights (4 rows).
        0x84 => &[
            (3, "Bantral"),
            (5, "Captain Blyth"),
            (21, "Master Hawkins"),
            (24, "Jones"),
        ],
        // Reagent vendors (5 rows).
        0x85 => &[
            (1, "Nilrem"),
            (4, "Madam Pendra"),
            (7, "Toama"),
            (23, "Enlor"),
            (30, "Virden"),
        ],
        // Guildmasters (3 rows).
        0x86 => &[(8, "Braunam"), (22, "Danfits"), (24, "Daem")],
        // Healers / sanctums (7 rows).
        0x87 => &[
            (5, "Regina"),
            (6, "Leila"),
            (7, "Temptious"),
            (21, "Milan"),
            (23, "Jessica"),
            (30, "Faye"),
            (31, "Jessip"),
        ],
        // Inns (6 rows).
        0x88 => &[
            (2, "Donya"),
            (3, "Gremnor"),
            (7, "Rogi"),
            (20, "Terbor"),
            (22, "Lorien"),
            (24, "Ransack"),
        ],
        _ => return None,
    };
    let mut index = 0;
    while index < table.len() {
        if table[index].0 == scene_byte {
            return Some(table[index].1);
        }
        index += 1;
    }
    None
}

/// The `LOOK2.DAT` string at `index`, or the domain sentinel at
/// `sentinel_index` when the row is missing (`formats/look2-dat.md §6`).
/// With no table loaded there is no description to print.
fn look2_selected_string(look_table: Option<&LookTable>, index: usize, sentinel_index: usize) -> String {
    look_table
        .and_then(|table| table.description(index).or_else(|| table.description(sentinel_index)))
        .unwrap_or_default()
        .to_string()
}

/// `view.md §3` row 5c: "`58` Shame, `72` Destard, `91` Despise, `126`
/// Wrong, `128` Doom, `156` Covetous, `239` Hythloth, `240` Deceit."
fn sealed_dungeon_name_for_x(x: usize) -> Option<&'static str> {
    Some(match x {
        58 => "Shame",
        72 => "Destard",
        91 => "Despise",
        126 => "Wrong",
        128 => "Doom",
        156 => "Covetous",
        239 => "Hythloth",
        240 => "Deceit",
        _ => return None,
    })
}

/// `view.md §3` rows 5a-5c: the appender rows, matched on the live tile
/// alone - "`0xFA`, `0xFB`, `0xDE` or `0xDF`" - whether or not the row then
/// appends anything.
pub(crate) fn look_terrain_tile_takes_appender_row(tile: u8) -> bool {
    matches!(
        tile,
        0xfa | 0xfb | ETERNAL_FLAME_LOOK_TILE | WORD_OF_POWER_SEALED_TILE
    )
}

impl PlayState {
    /// Apply the primary tavern round's north-first table-setting rewrite.
    /// At either vertical edge the out-of-range lookup uses the town-grid
    /// southeast fallback cell, matching the shared location-grid accessor.
    pub fn rewrite_tavern_round_table_setting(&mut self) -> bool {
        let north_index = self
            .player
            .y
            .checked_sub(1)
            .map(|y| y * TOWN_GRID_SIDE + self.player.x)
            .unwrap_or(TOWN_GRID_BYTES - 1);
        if self.grid[north_index] == TAVERN_BARE_TABLE_SETTING_TILE {
            self.grid[north_index] = TAVERN_NORTH_FOOD_SETTING_TILE;
            self.mark_visibility_dirty();
            return true;
        }

        let south_index = self
            .player
            .y
            .checked_add(1)
            .filter(|y| *y < TOWN_GRID_SIDE)
            .map(|y| y * TOWN_GRID_SIDE + self.player.x)
            .unwrap_or(TOWN_GRID_BYTES - 1);
        if self.grid[south_index] == TAVERN_BARE_TABLE_SETTING_TILE {
            self.grid[south_index] = TAVERN_SOUTH_FOOD_SETTING_TILE;
            self.mark_visibility_dirty();
            return true;
        }

        false
    }

    pub fn cast_rel_hur(
        &mut self,
        caster_index: usize,
        direction: Option<Direction>,
        pass: bool,
    ) -> MoveOutcome {
        if !matches!(self.area, Area::World { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        // `magic.md §5` steps 4-6 precede the handler: "Charge and mana are
        // already spent when a handler prompts", and "No later gate can fail
        // after a prompt opens" (`weather.md §3`: the pipeline "consumes the
        // appropriate pre-mixed charge, prompts for a direction, and then
        // routes to the wind setter").
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, REL_HUR_SPELL_INDEX, REL_HUR_COST)
        {
            return outcome;
        }
        if direction.is_none() && !pass {
            self.request_cast_argument(CastArgumentRequest::Direction);
            return MoveOutcome::Blocked;
        }
        self.confirm_spent_rel_hur(direction, pass)
    }

    /// Finish Rel Hur after the dispatcher has already consumed its charge
    /// and mana (`magic.md §5`).
    pub fn confirm_spent_rel_hur(&mut self, direction: Option<Direction>, pass: bool) -> MoveOutcome {
        if pass {
            self.advance_turn();
            // Measured 2026-09-07: Space completes the scroll's own
            // `Direction-` row with the shared `Pass` word and prints
            // nothing else.
            let _ = self.complete_open_direction_echo(
                SPELL_DIRECTION_PROMPT_PREFIX,
                DIRECTION_PROMPT_LABEL_PASS,
            );
            self.message.clear();
            return MoveOutcome::Cast;
        }
        let Some(direction) = direction else {
            return MoveOutcome::Blocked;
        };

        let next = WindState::rel_hur_target(direction)
            .expect("inline Rel Hur parser returns cardinal directions only");
        let previous = self.wind_status_message();
        self.apply_wind_state(next);
        self.advance_turn();
        // `weather.md §3`: an accepted direction prints its direction name,
        // then the setter stores and repaints the wind banner. `magic.md
        // §5.1` lists Rel Hur under "No generic completion line", and
        // `Wind change!` is the scroll-dispatch banner, not a spell line.
        let _ = self.complete_open_direction_echo(SPELL_DIRECTION_PROMPT_PREFIX, direction.name());
        self.diagnostics.push(format!(
            "Wind change: {} -> {}.",
            previous,
            self.wind_status_message()
        ));
        self.message.clear();
        MoveOutcome::Cast
    }

    /// `audio.md §7.3` accepted wind change cast as the **spell** `Rel Hur`:
    /// play variant 2, then commit and announce the new wind.
    pub fn apply_wind_state(&mut self, wind: WindState) -> bool {
        self.apply_wind_state_from_caller(wind, Some(audio::WindChangeCaller::Spell))
    }

    /// `audio.md §7.3` accepted wind change reached through the **scroll**
    /// (scroll index 1): the same sequence at variant 1.
    ///
    /// "The scroll variant disagrees with the corresponding spell's variant in
    /// six of the eight cases: ... Wind Change 1 against 2."
    pub fn apply_wind_state_from_scroll(&mut self, wind: WindState) -> bool {
        self.apply_wind_state_from_caller(wind, Some(audio::WindChangeCaller::Scroll))
    }

    /// Commit a wind transition **without** the `audio.md §7.3` cue.
    ///
    /// `§7.3`: "The sound belongs to the spell and scroll handlers, never to
    /// the setter"; the autonomous drift "has no sound call, no wrapper, and
    /// no ambient hook", and `§11` lists it in the wind-change sequence's
    /// explicitly-not-produced-by column. The drift therefore commits through
    /// this entry point.
    pub fn apply_wind_state_without_sound(&mut self, wind: WindState) -> bool {
        self.apply_wind_state_from_caller(wind, None)
    }

    fn apply_wind_state_from_caller(
        &mut self,
        wind: WindState,
        caller: Option<audio::WindChangeCaller>,
    ) -> bool {
        // Compare the stored byte, not the normalised state: a preserved
        // out-of-range byte reads as Calm, and a drift to a real Calm must
        // still store `0` over it (`weather.md §2.1`, out-of-range values).
        let out_of_range_byte = self.wind_save_byte > 4;
        // `moons.md §3` (u5-spec#300 Q35): after a surface-overworld fight
        // the banner "keeps the pre-combat wind until the next drift hit,
        // Wind Change, scene entry or load". This setter is the banner's
        // repaint helper, so any call outside combat ends the hold for good
        // - even one that stores the wind already in force, and even if a
        // later change returns to the wind the hold was keyed on.
        if !self.combat_active
            && let Some(hold) = self.post_combat_ribbon_hold.as_mut()
        {
            hold.bottom = None;
        }
        if self.wind == WindState::Calm && wind == WindState::Calm && !out_of_range_byte {
            return false;
        }
        // `audio.md §7.3`: "**The variant is chosen by the caller tag, not by
        // the wind.**" The old wind and the requested compass direction do not
        // participate, so requesting the already-active direction still sounds.
        // The earlier previous-wind matrix is withdrawn (`RETRACTIONS.md`).
        let variant =
            caller.and_then(|caller| audio::wind_change_variant(caller, wind == WindState::Calm));
        if let Some(variant) = variant {
            self.emit_sound_effect(SoundEffect::SharedVariant { variant });
        }
        let changed = self.wind != wind || out_of_range_byte;
        self.wind = wind;
        self.wind_save_byte = wind.save_byte();
        // `weather.md §5.1`, "**A fourth reset of the sailing counter.**":
        // "The routine that stores a new wind state clears the counter
        // immediately afterwards, and both are skipped when the same routine
        // is called only to re-announce the wind already in force. So a ship
        // one pass into a two-pass upwind wait restarts that wait when the
        // wind shifts."
        if changed {
            self.sail_cadence = 0;
        }
        changed
    }

    pub fn wind_status_message(&self) -> &'static str {
        wind_status_message_from_state_and_save_byte(self.wind, self.wind_save_byte)
    }

    /// `magic.md §8` handler-family map: Gate Travel refuses "while the
    /// party is aboard a frigate or a skiff".
    pub(crate) fn party_is_shipboard_for_gate_travel(&self) -> bool {
        matches!(
            self.player.transport,
            TransportState::Ship { .. } | TransportState::Skiff { .. }
        )
    }

    /// The ordinary handler failure for a committed Gate Travel cast:
    /// `Failed!` with the failure sound, one turn, nothing refunded
    /// (`magic.md §5` step 8, §8).
    pub(crate) fn fail_committed_gate_travel(&mut self) -> MoveOutcome {
        self.advance_turn();
        self.message = "Failed!".to_string();
        self.emit_sound_effect(SoundEffect::CastFailure);
        MoveOutcome::Blocked
    }

    pub fn cast_gate_travel(
        &mut self,
        caster_index: usize,
        slot_index: usize,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, GATE_TRAVEL_SPELL_INDEX, GATE_TRAVEL_COST)
        {
            return Ok(outcome);
        }

        // `magic.md §5` steps 4-7: the dispatcher spends the premixed charge
        // and debits mana *before* it computes the dispatch index and calls
        // the effect handler, and `magic.md §8` puts the shipboard test inside
        // the handler — "Gate Travel ... requires the party not to be
        // shipboard, prompts `To phase:`". So a shipboard attempt is a
        // committed cast: the charge and the eight magic points are already
        // gone and nothing is refunded, exactly as with the Doom refusal in
        // `cast_dungeon_level_spell`. Only the scene gate (inside
        // `cast_spell_resource_gate`) spends nothing.
        //
        // `magic.md §5` step 8: "ordinary failure appends `Failed!` and a
        // newline with the failure sound". The shipboard test is *inside* the
        // handler - §8 "requires the party not to be shipboard" - so it is an
        // ordinary handler failure and the dispatcher's own epilogue narrates
        // it. The engine-voice sentence that used to stand here was invented.
        if self.party_is_shipboard_for_gate_travel() {
            return Ok(self.fail_committed_gate_travel());
        }

        // `audio.md §6.1` id 46: the variant (circle 8) plays "only after the
        // player types a digit `1`..`8` at the moongate prompt; any other key
        // is silent". The shipboard refusal above never reaches that prompt.
        if let Some(variant) = audio::spell_shared_variant(GATE_TRAVEL_SPELL_INDEX) {
            self.emit_sound_effect(SoundEffect::SharedVariant { variant });
        }

        let phase = slot_index + 1;
        let slot = self.moonstone_slots[slot_index];
        match gate_travel_destination_in(game_dir, slot)? {
            GateTravelDestination::Ready {
                target,
                floor,
                start,
            } => {
                // `magic.md`, the Gate travel row (issue #313 Q2, R658): "A
                // successful Gate Travel reports "no action" to the mode
                // loop, so the cast turn runs no mode epilogue at all: no
                // minute, no poison point, no hourly food or starvation
                // branch, no dawn/dusk pass, no underfoot pass, no door
                // auto-close tick and no NPC movement". The town and
                // overworld casts therefore charge nothing here; the first
                // time-consuming turn at the destination runs its own
                // epilogue. A dungeon iteration charged its minute at the
                // loop head, and the warp leaves that loop at its scene-byte
                // check before the post-action pass (`dungeon-mode.md §4`
                // step 4), so only the action counter is spent there.
                if matches!(self.area, Area::Dungeon { .. }) {
                    self.advance_turn_leaving_dungeon();
                }
                self.apply_gate_travel(game_dir, phase, target, floor, start)?;
                Ok(MoveOutcome::Transition(AreaTransition::GateTraveled {
                    target,
                }))
            }
            // `magic.md §8`: "an invalid scene sentinel makes the helper
            // return failure and the cast does not teleport" - and §5 step 8
            // turns a handler failure into `Failed!` with the failure sound.
            // Neither the empty slot nor the invalid one has a line of its
            // own; the two sentences that used to stand here named an
            // internal slot number the original never shows.
            // `magic.md`, the Gate travel row: "Every failure reports
            // "acted" and runs the full epilogue of the mode it was cast
            // in", a recovered stone's slot included.
            GateTravelDestination::Empty | GateTravelDestination::Invalid(_) => {
                let _ = phase;
                self.advance_turn();
                self.message = "Failed!".to_string();
                self.emit_sound_effect(SoundEffect::CastFailure);
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    pub fn use_item_command(
        &mut self,
        request: Option<UseItemRequest>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        Ok(match request {
            Some(UseItemRequest::WoodenBox) => self.use_wooden_box(),
            Some(UseItemRequest::HmsCapePlans) => self.use_hms_cape_plans(),
            Some(UseItemRequest::CrownOfLordBritish) => self.use_worn_regalia(
                SPECIAL_ITEM_CROWN_LB_INDEX,
                CROWN_LB_ACTIVE_EFFECT_TAG,
                "Crown",
                USE_CROWN_WORN,
                USE_REGALIA_REMOVED,
            ),
            Some(UseItemRequest::AmuletOfLordBritish) => self.use_worn_regalia(
                SPECIAL_ITEM_AMULET_LB_INDEX,
                AMULET_LB_ACTIVE_EFFECT_TAG,
                "Amulet",
                USE_AMULET_WORN,
                USE_REGALIA_REMOVED,
            ),
            Some(UseItemRequest::Sceptre) => self.use_sceptre_of_lord_british(),
            Some(UseItemRequest::BlackBadge) => self.use_worn_regalia(
                SPECIAL_ITEM_BLACK_BADGE_INDEX,
                BLACK_BADGE_ACTIVE_EFFECT_TAG,
                "Black Badge",
                USE_BLACK_BADGE_WORN,
                USE_REGALIA_REMOVED,
            ),
            Some(UseItemRequest::Spyglass) => self.use_spyglass(),
            Some(UseItemRequest::Scroll {
                index,
                direction,
                target,
            }) => self.use_scroll(index, direction, target),
            Some(UseItemRequest::Potion { index, target }) => self.use_potion(index, target),
            Some(UseItemRequest::MagicCarpet) => self.use_magic_carpet(),
            Some(UseItemRequest::SkullKey) => self.use_skull_key(game_dir)?,
            Some(UseItemRequest::Sextant) => self.use_sextant(),
            Some(UseItemRequest::PocketWatch) => self.use_pocket_watch(),
            Some(UseItemRequest::ShadowlordShard(index)) => {
                self.use_shadowlord_shard(index, game_dir)?
            }
            Some(UseItemRequest::Moonstone(slot_index)) => {
                self.use_moonstone_phase(Some(slot_index))
            }
            Some(UseItemRequest::Invalid) | None => {
                self.message = use_prompt_message();
                MoveOutcome::Blocked
            }
        })
    }

    pub fn start_use_item(&mut self) -> MoveOutcome {
        let rows = self.use_item_picker_rows();
        if rows.is_empty() {
            // `commands.md §5`: the echo is `Use_item\n\n`, printed "before
            // the handler's precondition check", so the refusal sits under
            // the same blank row the `Item: ` prompt does (playtest H34/H45).
            self.message = format!("\n{USE_NO_USABLE_ITEMS_REFUSAL}");
            // inventory.md §7: U-Use returns the normal action result even
            // when item-specific dispatch is never reached.
            self.advance_turn();
            return MoveOutcome::Blocked;
        }
        self.active_use = Some(UseSession::new());
        // `text-output.md §10.4`: the prompt follows the `Use item` echo,
        // so its leading feed buys a blank row. A capture reads
        // `>Use item`, a blank row, then `Item: None!` on one row.
        // `cleak/u5-engine#5`.
        self.open_prompt_block();
        self.message = self.render_active_use();
        // `inventory.md §4.4`: the frame build clears window columns 0-14
        // and draws the border; column 15 keeps what was there.
        self.use_picker_panel = Some(crate::stats_panel::PickerPanelCells::with_frame(
            self.use_picker_panel.as_ref(),
        ));
        // The picker's first draw is a repaint like any other (§9.5: "the
        // initial draw costs 0 or 1"), and it comes after `Item:_` prints.
        self.register_use_picker_render_scroll();
        MoveOutcome::Observed
    }

    pub fn render_active_use(&self) -> String {
        self.active_use
            .as_ref()
            .map(|session| self.render_use_session(session))
            .unwrap_or_else(use_prompt_message)
    }

    /// `cleak/u5-engine#25`/`cleak/u5-spec#259` (published `ed0f743`): every
    /// repaint of the U-Use (or R-Ready) picker panel - the initial draw,
    /// each navigation key, and a key the picker does not recognise - costs
    /// the message window one upward strip scroll exactly when the entry
    /// that begins the panel's last visible list row needs two display
    /// lines, never more than one. Acceptance runs no further picker pass,
    /// so it does not call this; cancellation prints its refusal into the
    /// message window directly rather than repainting the panel, so it does
    /// not either.
    ///
    /// Call this once `self.active_use` (or `self.active_ready`) already
    /// reflects the session the panel is about to show, since
    /// [`crate::stats_panel::active_panel_picker`] reads the live field
    /// rather than taking a session argument.
    ///
    /// The same pass also runs `inventory.md §4.4`'s inverse-video toggles
    /// (u5-spec#296), which flip the panel window's persistent inverse bit
    /// when the highlight sits on a two-line entry
    /// ([`crate::stats_panel::PanelPickerView::inverse_pass`]). Only U-Use can
    /// flip it: R-Ready's and M-Mix's vocabularies never take a second
    /// display line (§4.5), so their passes always toggle an even number of
    /// times.
    fn register_use_picker_render_scroll(&mut self) {
        let view = crate::stats_panel::active_panel_picker(self);
        if let (Some(view), Some(mut panel)) = (view.as_ref(), self.use_picker_panel.take()) {
            // The pass paints into the panel without clearing it, and its
            // toggles run against the window's persistent inverse bit.
            let mut inverse = self.stats_panel_window_inverse;
            panel.run_pass(view, &mut inverse);
            self.stats_panel_window_inverse = inverse;
            self.use_picker_panel = Some(panel);
        } else if view.is_some_and(|view| view.inverse_pass_flips_window()) {
            self.stats_panel_window_inverse = !self.stats_panel_window_inverse;
        }
        if !crate::stats_panel::active_panel_picker(self)
            .is_some_and(|view| view.overflowed_last_row_entry())
        {
            return;
        }
        let Some(prompt) = self
            .active_use
            .as_ref()
            .filter(|session| session.pending.is_none())
            .map(UseSession::item_prompt_literal)
        else {
            return;
        };
        if self.break_open_prompt_row_for_strip_scroll(&prompt).is_some()
            && let Some(session) = self.active_use.as_mut()
        {
            session.strip_scrolls = session.strip_scrolls.saturating_add(1);
        }
    }

    pub fn render_use_session(&self, session: &UseSession) -> String {
        if let Some(pending) = session.pending {
            return self.render_pending_use_action(pending);
        }
        if self.use_item_picker_rows().is_empty() {
            return USE_NO_USABLE_ITEMS_REFUSAL.to_string();
        }
        // `inventory.md §4.4`: U-Use prints `Item:_` into the message
        // window and runs the picker in the panel; the rows are read from
        // `crate::active_panel_picker` by the panel painter.
        session.item_prompt_literal()
    }

    pub(crate) fn use_picker_cursor(&self, session: &UseSession) -> usize {
        session
            .cursor
            .min(self.use_item_picker_rows().len().saturating_sub(1))
    }

    pub fn step_active_use(
        &mut self,
        key: char,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<bool> {
        let Some(mut session) = self.active_use.take() else {
            return Ok(false);
        };
        let key = ready_first_input_key(key, suffix);
        if let Some(pending) = session.pending {
            return self.step_pending_use_action(session, pending, key, suffix, game_dir);
        }
        match use_input_action(key) {
            UseInputAction::Exit => {
                let turn_before = self.turn;
                // The reply lands on the prompt's own row: `Item: None!` -
                // or at its column on the row a strip scroll left it on.
                // `Item: ` may be only the live row (the picker opened and
                // was cancelled before a boundary logged it), so the
                // completed row is logged here rather than left in the slot.
                // `inventory.md §7.1`'s literal is `None!\n`: its feed
                // closes the row.
                self.commit_live_prompt_reply(
                    &session.item_prompt_literal(),
                    ITEM_PICKER_ESCAPE_MESSAGE,
                    false,
                );
                self.ensure_use_action_turn(turn_before);
                self.apply_post_turn_effects_after_outcome(
                    turn_before,
                    game_dir,
                    MoveOutcome::Used,
                )?;
            }
            UseInputAction::NextItem => {
                self.move_use_cursor(&mut session, 1);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
                self.register_use_picker_render_scroll();
            }
            UseInputAction::PreviousItem => {
                self.move_use_cursor(&mut session, -1);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
                self.register_use_picker_render_scroll();
            }
            UseInputAction::PageNext => {
                self.move_use_cursor(&mut session, READY_PICKER_PAGE_STEP as isize);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
                self.register_use_picker_render_scroll();
            }
            UseInputAction::PagePrevious => {
                self.move_use_cursor(&mut session, -(READY_PICKER_PAGE_STEP as isize));
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
                self.register_use_picker_render_scroll();
            }
            // `inventory.md §4.7`: "Home selects the first item; End selects
            // the last." The shared picker's rules are §5's, so the U-Use
            // list takes them too.
            UseInputAction::FirstItem => {
                self.select_use_endpoint(&mut session, false);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
                self.register_use_picker_render_scroll();
            }
            UseInputAction::LastItem => {
                self.select_use_endpoint(&mut session, true);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
                self.register_use_picker_render_scroll();
            }
            UseInputAction::Confirm => {
                let turn_before = self.turn;
                let Some(mut row) = self.use_selected_item(&session) else {
                    self.message = USE_NO_USABLE_ITEMS_REFUSAL.to_string();
                    self.ensure_use_action_turn(turn_before);
                    self.apply_post_turn_effects_after_outcome(
                        turn_before,
                        game_dir,
                        MoveOutcome::Blocked,
                    )?;
                    return Ok(true);
                };
                // `stats-panel.md §2.2.1`: "U-Use, right after the item is
                // picked and before its effect runs" repaints the whole panel.
                self.repaint_stats_panel();
                // The accepted row completes the open `Item: ` line before
                // the handler runs; see [`Self::use_item_echo_family`].
                let prompt = session.item_prompt_literal();
                // `inventory.md §7.1`'s completion table: every family word
                // ends its row with a feed except `Moonstone_`, which is
                // "followed immediately by its outcome" on the same row. The
                // prompt may be only the live row, so the live commit logs it.
                let echoed = Self::use_item_echo_family(row.request).inspect(|family| {
                    let row_open_after = *family == USE_ITEM_ECHO_MOONSTONE;
                    self.commit_live_prompt_reply(&prompt, family, row_open_after);
                });
                // `inventory.md §7.1`: "Combat potion use binds the current
                // party combatant without this prompt" - the `On who:_`
                // selector is the non-combat potion's (playtest F28).
                if let UseItemRequest::Potion {
                    index,
                    target: None,
                } = row.request
                {
                    if let Some(slot) = self.combat_use_bound_party_member() {
                        row.request = UseItemRequest::Potion {
                            index,
                            target: Some(slot),
                        };
                    }
                }
                // `inventory.md §7.1`: in dungeon exploration the Skull Key
                // refuses `Not here!` with no `Direction-` prompt (its key
                // already spent), so it takes the direct handler path.
                // `item-list.md §7.1`: the Resurrection scroll prompts only
                // in non-combat scenes; combat refuses without the
                // `On who:` selector (playtest F29).
                let pending = pending_action_for_use_request(row.request)
                    .filter(|pending| {
                        !(matches!(pending, UsePendingAction::SkullKeyDirection)
                            && self.skull_key_refused_here())
                    })
                    .filter(|pending| {
                        !(matches!(pending, UsePendingAction::ScrollResurrectionTarget { .. })
                            && self.combat_active)
                    })
                    .or_else(|| {
                        (row.request == UseItemRequest::Sceptre
                            && self.sceptre_arena_fallback_prompts())
                        .then_some(UsePendingAction::SceptreFieldDirection)
                    });
                if let Some(pending) = pending {
                    // The potion and scroll rows debit their stock when the
                    // row is accepted, before the argument is asked for, and
                    // `inventory.md §7` puts the skull key in the same place:
                    // it "**Decrements the skull-key/special-key counter,
                    // then** asks for a cardinal target". Its lock attempt is
                    // what waits for the direction, not its cost.
                    match pending {
                        UsePendingAction::SkullKeyDirection => self.spend_skull_key(),
                        // The Sceptre is not consumed; its wielding line is
                        // the prompt's lead line below.
                        UsePendingAction::SceptreFieldDirection => {}
                        _ => {
                            let _ = self.use_item_command(Some(row.request), Some(game_dir))?;
                        }
                    }
                    session.pending = Some(pending);
                    // Measured: a scroll announces its effect on a row of
                    // its own, under a blank one, and *then* asks for its
                    // argument. A potion asks straight away.
                    self.message = match Self::use_pending_action_lead_line(pending) {
                        Some(lead) => {
                            format!("\n{lead}\n{}", self.render_pending_use_action(pending))
                        }
                        None => self.render_use_session(&session),
                    };
                    self.active_use = Some(session);
                    return Ok(true);
                }
                let outcome = self.use_item_command(Some(row.request), Some(game_dir))?;
                // `text-output.md §10.4`: the handler's own line prints
                // under a completed row, so its leading feed buys a blank
                // row. Measured: `>Use item`, blank, `Item: Scroll`,
                // blank, `Protection!`.
                // Measured: a line that opens with a space *continues* the
                // completed `Item: ` row instead of opening a new one -
                // `Item: Moonstone cannot be buried here!` is one wrapped
                // sentence, where the pocket watch's line starts its own row
                // under a blank one.
                // A handler that recorded its own rows - the sextant writes a
                // runic one - has nothing left in the slot to place, and
                // touching it here would flush a second, font-flattened copy.
                if !self.message_slot_needs_flush() {
                } else if echoed.is_some() && self.message.starts_with(' ') {
                    // The continuation joins the completed `Item: ` row, so
                    // the window wraps the whole sentence together:
                    // `Item: Moonstone` / `cannot be buried` / `here!`.
                    let continuation = std::mem::take(&mut self.message);
                    self.emit_message_line_continuing_row(continuation);
                } else if echoed.is_some_and(|family| {
                    // `inventory.md §7.1`'s completion table: `Potion\n` and
                    // `Skull Key\n` end with a single feed, so their result
                    // (`Healed!\n`, `Poof!\n`, `Not here!\n`, ...) sits on
                    // the very next row - combat potion use reaches here with
                    // its target bound (playtest P19-08).
                    family != USE_ITEM_ECHO_POTION && family != USE_ITEM_ECHO_SKULL_KEY
                }) && !self.message.is_empty()
                    && !self.message.starts_with('\n')
                {
                    self.message.insert(0, '\n');
                }
                self.ensure_use_action_turn(turn_before);
                self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
            }
            UseInputAction::Redraw | UseInputAction::Discard => {
                self.normalize_use_cursor(&mut session);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
                self.register_use_picker_render_scroll();
            }
        }
        Ok(true)
    }

    /// The party member a combat U-Use acts as: the combatant whose command
    /// prompt is open. `None` outside combat, or when the acting slot is not
    /// a party slot.
    ///
    /// The acting slot is a combat **descriptor** slot; `combat.md §5` packs
    /// the party descriptors past a dead member, so the roster member is the
    /// descriptor's owner/target/class byte, not the descriptor index.
    fn combat_use_bound_party_member(&self) -> Option<usize> {
        if !self.combat_active {
            return None;
        }
        self.pending_combat_actor_slot
            .and_then(|slot| self.combat_roster_slot_for_actor_slot(slot))
            .filter(|slot| *slot < self.party.len())
    }

    /// Redraw an open U-Use argument prompt after a key that neither commits
    /// nor cancels it.
    ///
    /// `inventory.md §7.1`: there is one `On who:_` line, and "The selected
    /// member's name completes that line". The scroll prompts open under
    /// their banner (`\nResurrection!\nOn who: `), so re-rendering the bare
    /// prompt into the slot made the transcript log it as a second row
    /// (playtest F30). The open row already shows the prompt, so a slot that
    /// still ends with it is left alone.
    fn redraw_pending_use_prompt(&mut self, session: &UseSession) {
        let prompt = self.render_use_session(session);
        if !self.message.ends_with(&prompt) {
            self.message = prompt;
        }
    }

    fn step_pending_use_action(
        &mut self,
        mut session: UseSession,
        pending: UsePendingAction,
        key: char,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<bool> {
        // Measured 2026-09-07: Escape completes the open prompt row with the
        // universal `None!` - `On who: None!` - and Space *accepts* the
        // highlighted member rather than closing, exactly as it does in the
        // item picker itself. A direction prompt takes the shared `Pass`
        // word instead (the wind scroll reads `Direction-Pass`).
        //
        // `input.md §11` / `inventory.md §7.1`: the shared `Direction-`
        // prompt takes only a cardinal or Space; Escape is one of the "other
        // keys [that] are ignored", so it falls through to the per-kind arms
        // below and the prompt reads again.
        let direction_prompt = matches!(
            pending,
            UsePendingAction::SkullKeyDirection
                | UsePendingAction::SceptreFieldDirection
                | UsePendingAction::ScrollWindDirection { .. }
        );
        if key == '\u{1b}' && !direction_prompt {
            let turn_before = self.turn;
            let prompt = self.render_pending_use_action(pending);
            // `inventory.md §7.1`: cancellation completes the `On who:_`
            // line with `None!`, and "A line feed follows only if the cursor
            // is not already at column zero" - the row is closed either way.
            // The prompt may be only the live row (it rides in the slot under
            // its scroll banner until a boundary flushes it).
            self.commit_live_prompt_reply(&prompt, SELECTION_CANCELLED_LITERAL, false);
            self.ensure_use_action_turn(turn_before);
            self.apply_post_turn_effects_after_outcome(turn_before, game_dir, MoveOutcome::Used)?;
            return Ok(true);
        }
        if key == ' ' && direction_prompt {
            let turn_before = self.turn;
            self.commit_prompt_reply(SPELL_DIRECTION_PROMPT_PREFIX, DIRECTION_PROMPT_LABEL_PASS);
            // `catalogs/item-list.md §7.1`, scroll 1 (u5-spec#312 Q3):
            // "`Pass` ends the same way there" - the gate still fails in a
            // dungeon or combat scene and the failure tail follows.
            let outcome = if matches!(pending, UsePendingAction::ScrollWindDirection { .. })
                && self.wind_change_scroll_gate_fails()
            {
                self.wind_change_scroll_failure_tail();
                MoveOutcome::Blocked
            } else {
                MoveOutcome::Used
            };
            self.ensure_use_action_turn(turn_before);
            self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
            return Ok(true);
        }

        let turn_before = self.turn;
        let outcome = match pending {
            UsePendingAction::SkullKeyDirection => {
                if let Some(direction) = pending_use_cardinal_direction(key, suffix) {
                    // Measured: the chosen cardinal completes the open
                    // `Direction-` row, then the key is tried on that cell.
                    self.commit_prompt_reply(SPELL_DIRECTION_PROMPT_PREFIX, direction.name());
                    self.use_skull_key_direction(Some(game_dir), direction)?
                } else {
                    session.pending = Some(pending);
                    self.redraw_pending_use_prompt(&session);
                    self.active_use = Some(session);
                    return Ok(true);
                }
            }
            UsePendingAction::SceptreFieldDirection => {
                if let Some(direction) = pending_use_cardinal_direction(key, suffix) {
                    self.commit_prompt_reply(SPELL_DIRECTION_PROMPT_PREFIX, direction.name());
                    self.use_sceptre_arena_field_direction(direction)
                } else {
                    session.pending = Some(pending);
                    self.redraw_pending_use_prompt(&session);
                    self.active_use = Some(session);
                    return Ok(true);
                }
            }
            UsePendingAction::PotionTarget { index, highlight } => {
                match self.step_use_target_selector(key, suffix, highlight) {
                    UseTargetStep::Moved(next) => {
                        session.pending = Some(UsePendingAction::PotionTarget {
                            index,
                            highlight: next,
                        });
                        self.redraw_pending_use_prompt(&session);
                        self.active_use = Some(session);
                        return Ok(true);
                    }
                    UseTargetStep::Waiting => {
                        session.pending = Some(pending);
                        self.redraw_pending_use_prompt(&session);
                        self.active_use = Some(session);
                        return Ok(true);
                    }
                    UseTargetStep::Committed(target) => {
                        // The answer completes the prompt's own row.
                        let name = self.party_member_display_name(target);
                        // `inventory.md §7.1`: "The selected member's name
                        // completes that line"; the line feed after it closes
                        // the row. The prompt may be only the live row.
                        self.commit_live_prompt_reply(USE_POTION_TARGET_PROMPT, &name, false);
                        self.use_potion_consumed_target(index, target)
                    }
                }
            }
            UsePendingAction::ScrollWindDirection { .. } => {
                if let Some(direction) = pending_use_cardinal_direction(key, suffix) {
                    // The cardinal completes the open `Direction-` row.
                    self.commit_prompt_reply(SPELL_DIRECTION_PROMPT_PREFIX, direction.name());
                    self.use_wind_change_scroll(Some(direction))
                } else {
                    session.pending = Some(pending);
                    self.redraw_pending_use_prompt(&session);
                    self.active_use = Some(session);
                    return Ok(true);
                }
            }
            UsePendingAction::ScrollResurrectionTarget { index, highlight } => {
                match self.step_use_target_selector(key, suffix, highlight) {
                    UseTargetStep::Moved(next) => {
                        session.pending = Some(UsePendingAction::ScrollResurrectionTarget {
                            index,
                            highlight: next,
                        });
                        self.redraw_pending_use_prompt(&session);
                        self.active_use = Some(session);
                        return Ok(true);
                    }
                    UseTargetStep::Waiting => {
                        session.pending = Some(pending);
                        self.redraw_pending_use_prompt(&session);
                        self.active_use = Some(session);
                        return Ok(true);
                    }
                    UseTargetStep::Committed(target) => {
                        let name = self.party_member_display_name(target);
                        // `inventory.md §7.1`: "The selected member's name
                        // completes that line"; the line feed after it closes
                        // the row. The prompt may be only the live row.
                        self.commit_live_prompt_reply(USE_POTION_TARGET_PROMPT, &name, false);
                        self.use_resurrection_scroll_consumed_target(index, target)
                    }
                }
            }
        };
        self.ensure_use_action_turn(turn_before);
        self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        Ok(true)
    }

    /// `inventory.md §7`: the U-Use caller returns the normal action result
    /// regardless of the selected handler's success/refusal result. Existing
    /// handlers that already commit their effect's turn remain authoritative;
    /// otherwise the command layer supplies the one ordinary turn here.
    pub(crate) fn ensure_use_action_turn(&mut self, turn_before: u64) {
        if self.turn == turn_before {
            self.advance_turn();
        }
    }

    /// The family word an accepted U-Use row writes onto the `Item: `
    /// line. See [`USE_ITEM_ECHO_SCROLL`]: measured for scrolls and
    /// potions only, so every other item completes nothing.
    fn use_item_echo_family(request: UseItemRequest) -> Option<&'static str> {
        match request {
            UseItemRequest::Scroll { .. } => Some(USE_ITEM_ECHO_SCROLL),
            UseItemRequest::Potion { .. } => Some(USE_ITEM_ECHO_POTION),
            UseItemRequest::SkullKey => Some(USE_ITEM_ECHO_SKULL_KEY),
            UseItemRequest::MagicCarpet => Some(USE_ITEM_ECHO_CARPET),
            UseItemRequest::AmuletOfLordBritish => Some(USE_ITEM_ECHO_AMULET),
            UseItemRequest::CrownOfLordBritish => Some(USE_ITEM_ECHO_CROWN),
            UseItemRequest::Sceptre => Some(USE_ITEM_ECHO_SCEPTRE),
            UseItemRequest::ShadowlordShard(_) => Some(USE_ITEM_ECHO_GEM_SHARD),
            UseItemRequest::Spyglass => Some(USE_ITEM_ECHO_SPYGLASS),
            UseItemRequest::HmsCapePlans => Some(USE_ITEM_ECHO_PLANS),
            UseItemRequest::Sextant => Some(USE_ITEM_ECHO_SEXTANT),
            UseItemRequest::PocketWatch => Some(USE_ITEM_ECHO_WATCH),
            UseItemRequest::BlackBadge => Some(USE_ITEM_ECHO_BADGE),
            UseItemRequest::WoodenBox => Some(USE_ITEM_ECHO_BOX),
            UseItemRequest::Moonstone(_) => Some(USE_ITEM_ECHO_MOONSTONE),
            _ => None,
        }
    }

    /// Measured: the two scroll prompts announce the scroll's effect before
    /// they ask for their argument - `Wind change!` then `Direction-`, and
    /// `Resurrection!` then `On who: `. The potion prompt has no such line.
    fn use_pending_action_lead_line(pending: UsePendingAction) -> Option<&'static str> {
        match pending {
            UsePendingAction::PotionTarget { .. } => None,
            UsePendingAction::ScrollWindDirection { .. } => Some(SCROLL_WIND_CHANGE_RESULT),
            UsePendingAction::ScrollResurrectionTarget { .. } => Some(SCROLL_RESURRECTION_RESULT),
            UsePendingAction::SkullKeyDirection => None,
            // `inventory.md §7.1`: "`Wielding the Sceptre of Lord
            // British...\n`" on every accepted selection, before the field
            // checks - so ahead of the fallback's prompt.
            UsePendingAction::SceptreFieldDirection => Some(USE_SCEPTRE_WIELDED),
        }
    }

    fn render_pending_use_action(&self, pending: UsePendingAction) -> String {
        match pending {
            // Measured: the potion's target prompt is
            // [`USE_POTION_TARGET_PROMPT`], and the chosen member's name
            // completes its row. The two scroll prompts below are not
            // measured and keep the harness text for now
            // (`cleak/u5-spec#225`).
            UsePendingAction::PotionTarget { index, .. } => {
                let _ = index;
                USE_POTION_TARGET_PROMPT.to_string()
            }
            UsePendingAction::ScrollWindDirection { index } => {
                let _ = index;
                SPELL_DIRECTION_PROMPT_PREFIX.to_string()
            }
            UsePendingAction::ScrollResurrectionTarget { index, .. } => {
                let _ = index;
                USE_POTION_TARGET_PROMPT.to_string()
            }
            UsePendingAction::SkullKeyDirection | UsePendingAction::SceptreFieldDirection => {
                SPELL_DIRECTION_PROMPT_PREFIX.to_string()
            }
        }
    }

    fn use_potion_consumed_target(&mut self, index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            // Out-of-range member: the interactive pickers bound the
            // index, so only the inline harness form reaches this. The
            // original has no line for a state it cannot enter.
            self.message.clear();
            return MoveOutcome::Blocked;
        }

        let effect_index = self.potion_effect_index_after_variation_roll(index);
        self.use_potion_with_effect(index, target_index, effect_index)
    }

    fn use_resurrection_scroll_consumed_target(
        &mut self,
        index: usize,
        target_index: usize,
    ) -> MoveOutcome {
        if index != SCROLL_RESURRECTION_INDEX {
            self.message = "No effect!".to_string();
            return MoveOutcome::Blocked;
        }
        self.use_resurrection_scroll(Some(target_index))
    }

    fn use_selected_item(&self, session: &UseSession) -> Option<UseItemPickerRow> {
        let rows = self.use_item_picker_rows();
        rows.get(session.cursor.min(rows.len().saturating_sub(1)))
            .cloned()
    }

    fn normalize_use_cursor(&self, session: &mut UseSession) {
        let row_count = self.use_item_picker_rows().len();
        if row_count == 0 {
            session.cursor = 0;
        } else if session.cursor >= row_count {
            session.cursor = row_count - 1;
        }
    }

    fn move_use_cursor(&self, session: &mut UseSession, delta: isize) {
        let row_count = self.use_item_picker_rows().len();
        if row_count == 0 {
            session.cursor = 0;
            return;
        }
        let next = session.cursor as isize + delta;
        session.cursor = next.clamp(0, row_count as isize - 1) as usize;
    }

    /// `inventory.md §4.7`: Home and End select the first and last rows
    /// rather than paging toward them.
    fn select_use_endpoint(&self, session: &mut UseSession, last: bool) {
        let row_count = self.use_item_picker_rows().len();
        session.cursor = if row_count == 0 {
            0
        } else if last {
            row_count - 1
        } else {
            0
        };
    }

    /// Rows of the U-Use item picker, in the order the original lists
    /// them.
    ///
    /// `inventory.md §7` says only that "a selected row dispatches by the
    /// handler's use-item enumeration"; the enumeration's order is not
    /// published. Two independent clean sources agree on it, so it is
    /// reproduced here rather than invented:
    ///
    /// - the order of §7's own "Confirmed U-Use families" table - spell
    ///   scrolls, potions, Magic Carpet, Skull Key, regalia, shards,
    ///   Moonstones, Spyglass, HMS Cape plans, Sextant, Pocket Watch,
    ///   Sandalwood Box; and
    /// - the published special-item indices, which run in that same
    ///   sequence (carpet `0x00`, skull key `0x01`, amulet `0x03`, crown
    ///   `0x04`, sceptre `0x05`, shards `0x06..=0x08`, spyglass `0x0A`,
    ///   plans `0x0B`, sextant `0x0C`, watch `0x0D`, badge `0x0E`, box
    ///   `0x0F`).
    ///
    /// A paired capture of the original's picker confirms the one point
    /// both sources constrain and the engine used to get wrong: a carried
    /// scroll and potion are listed *above* the Pocket Watch, not below
    /// it. The engine previously emitted every special item first.
    /// `cleak/u5-spec#196` asks for the enumeration to be published
    /// outright.
    /// The rows the U-Use picker offers, in order. `pub` so the
    /// `use_rows` probe can print the row indexes a paired scenario has to
    /// count keypresses to reach.
    pub fn use_item_picker_rows(&self) -> Vec<UseItemPickerRow> {
        let mut rows = Vec::new();

        for (index, count) in self.scroll_stock.iter().copied().enumerate() {
            if count > 0 {
                rows.push(UseItemPickerRow {
                    // §4.5's scroll row is the marker, the plus and "the
                    // scroll's compact rune label" - not the word `Scroll`,
                    // which the decoration's own glyph stands for.
                    label: scroll_label(index).to_string(),
                    quantity: Some(count),
                    decoration: crate::stats_panel::PanelPickerDecoration::Scroll,
                    request: UseItemRequest::Scroll {
                        index,
                        direction: None,
                        target: None,
                    },
                });
            }
        }

        for (index, count) in self.potion_stock.iter().copied().enumerate() {
            if count > 0 {
                rows.push(UseItemPickerRow {
                    label: crate::z_stats::potion_short_colour_name(index).to_string(),
                    quantity: Some(count),
                    decoration: crate::stats_panel::PanelPickerDecoration::Potion,
                    request: UseItemRequest::Potion {
                        index,
                        target: None,
                    },
                });
            }
        }

        self.push_counted_use_row(
            &mut rows,
            SPECIAL_ITEM_MAGIC_CARPET_INDEX,
            // Measured (`qa/paired/use-specials.tsv`, beat `skullkeysrow`):
            // the original's row reads ` 1 Magic Crpt`. The name field is ten
            // cells, and the original abbreviates rather than truncating -
            // truncation of `Magic Carpet` gives `Magic Carp`, which is what
            // this engine printed.
            "Magic Crpt",
            UseItemRequest::MagicCarpet,
        );

        self.push_counted_use_row(
            &mut rows,
            SPECIAL_ITEM_SKULL_KEY_INDEX,
            "Skull Keys",
            UseItemRequest::SkullKey,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_AMULET_LB_INDEX,
            // **Measured** 2026-09-08 (`qa/paired/use-picker.tsv`): the
            // original's `Items:` panel lists these three by their short
            // names - ` 1 Amulet`, ` 1 Crown`, ` 1 Sceptre` - in a
            // thirteen-column field. The engine's long forms would render
            // truncated, which is not the same text.
            "Amulet",
            UseItemRequest::AmuletOfLordBritish,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_CROWN_LB_INDEX,
            "Crown",
            UseItemRequest::CrownOfLordBritish,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SCEPTRE_LB_INDEX,
            "Sceptre",
            UseItemRequest::Sceptre,
        );

        // `inventory.md §4.5`, "Which entries get a row": a row exists
        // "exactly when its stock value is nonzero ... no usability test
        // anywhere in the path". A carried stone therefore gets its row in
        // the dungeon and in combat too, where the bury branch later refuses
        // it with `cannot be buried here!` (§7.1); the picker used to drop
        // the rows wherever no bury context existed (playtest H33).
        for index in 0..MOONSTONE_SLOT_COUNT {
            // `inventory.md §7`: the handler "opens an item picker
            // over usable carried stock", and the Moonstone row
            // "records the current valid location into the matching
            // saved Moonstone slot", while "Search/Get recovery later
            // invalidates the slot". A slot that still holds a valid
            // location is therefore a stone lying buried in the world,
            // not one in the pack, so only an invalidated slot has a
            // stone to bury. A capture of the original's U-Use picker
            // at game start - every stone still buried - shows no
            // Moonstone row at all, which is what this gate
            // reproduces; the engine used to offer all eight.
            if self.moonstone_slots[index].is_valid() {
                continue;
            }
            rows.push(UseItemPickerRow {
                // §4.5: "`Moonstone` followed by a space in the text
                // font, then one runic phase glyph" - the phase is the
                // glyph, not a spelled-out number.
                label: Z_STATS_MOONSTONE_LABEL.to_string(),
                quantity: None,
                decoration: crate::stats_panel::PanelPickerDecoration::Moonstone {
                    phase: index as u8,
                },
                request: UseItemRequest::Moonstone(index),
            });
        }

        // `inventory.md §7`, "The picker's row order is the use-item
        // enumeration order": "... the Amulet, the Crown, the Sceptre, the
        // eight moonstones by phase, the three shards, the Spyglass ...". The
        // shard rows were emitted ahead of the moonstones, which shifts every
        // row beneath them - the same five Down presses reached a different
        // item than the original's picker offers
        // (`qa/paired/use-moonstone-bury.tsv`).
        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX,
            // `inventory.md §4.5`'s complete 38-entry label table, published
            // 2026-09-10 in answer to cleak/u5-spec#255: the shards are
            // `Shard/Falsehd`, `Shard/Hatred` and `Shard/Cowrdce`, and the
            // section adds "Use these labels verbatim, including
            // abbreviations and the singular `Plan`". The row renderer
            // "prints each authored label and then pads short rows; it does
            // not abbreviate or truncate a longer name at runtime", so the
            // engine's long forms were not merely rendered short - they were
            // the wrong strings.
            "Shard/Falsehd",
            UseItemRequest::ShadowlordShard(SHADOWLORD_FALSEHOOD_INDEX),
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SHARD_HATRED_INDEX,
            "Shard/Hatred",
            UseItemRequest::ShadowlordShard(SHADOWLORD_HATRED_INDEX),
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
            "Shard/Cowrdce",
            UseItemRequest::ShadowlordShard(SHADOWLORD_COWARDICE_INDEX),
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SPYGLASS_INDEX,
            "Spyglass",
            UseItemRequest::Spyglass,
        );

        // `inventory.md §4.5`, "Exactly two families are normalised on the
        // way into the stock": "the HMS Cape plans entry becomes the marker
        // for any nonzero carried value", so the plans row is always the
        // uncounted thirteen-cell `HMS Cape Plan` that never wraps. A saved
        // count of one used to print ` 1 ` before it and push the label onto
        // a second display line over the frame's left rule (playtest J10).
        if self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] > 0 {
            rows.push(UseItemPickerRow {
                label: "HMS Cape Plan".to_string(),
                decoration: crate::stats_panel::PanelPickerDecoration::None,
                quantity: None,
                request: UseItemRequest::HmsCapePlans,
            });
        }

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SEXTANT_INDEX,
            "Sextant",
            UseItemRequest::Sextant,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_POCKET_WATCH_INDEX,
            "Pocket Watch",
            UseItemRequest::PocketWatch,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_BLACK_BADGE_INDEX,
            "Black Badge",
            UseItemRequest::BlackBadge,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_WOODEN_BOX_INDEX,
            "Wooden Box",
            UseItemRequest::WoodenBox,
        );

        rows
    }

    fn push_counted_use_row(
        &self,
        rows: &mut Vec<UseItemPickerRow>,
        special_item_index: usize,
        label: &str,
        request: UseItemRequest,
    ) {
        let count = self.special_items[special_item_index];
        if count > 0 {
            rows.push(UseItemPickerRow {
                label: label.to_string(),
                decoration: crate::stats_panel::PanelPickerDecoration::None,
                quantity: Some(count),
                request,
            });
        }
    }

    fn push_owned_use_row(
        &self,
        rows: &mut Vec<UseItemPickerRow>,
        special_item_index: usize,
        label: &str,
        request: UseItemRequest,
    ) {
        let value = self.special_items[special_item_index];
        if value > 0 {
            rows.push(UseItemPickerRow {
                label: label.to_string(),
                decoration: crate::stats_panel::PanelPickerDecoration::None,
                // `inventory.md §4.5` "Which U-Use rows omit quantity":
                // quantity suppression is independent of name decoration,
                // and it is driven by the *saved value*, not by the item's
                // identity. "In the picker stock, value 255 is the
                // no-quantity marker; zero means absent from U-Use, and
                // ordinary positive quantities produce the counted layout."
                //
                // `RETRACTIONS.md` R456 settles the case that produced the
                // earlier reading here. Normal acquisition gives the
                // Amulet, Crown and Sceptre the no-quantity value, so their
                // rows omit both cells; a *saved* quantity-one value still
                // yields the counted row. The measurement this engine took
                // (`qa/paired/use-specials.tsv`, beat `skullkeysrow`,
                // printing ` 1 Amulet`) came from a save holding one, so it
                // remains correct - it was generalised too far.
                quantity: (value != crate::commands::USE_PICKER_NO_QUANTITY).then_some(value),
                request,
            });
        }
    }

    pub fn use_wooden_box(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        self.message = USE_WOODEN_BOX_PROMPT.to_string();
        MoveOutcome::PromptDeclined
    }

    /// `quest-graph.md §5` "Presentation order": the shard-use handler is a
    /// five-phase theatrical sequence, not a chain of diagnostic gates.
    ///
    /// 1. It "first prints a heading naming the shard family and a line
    ///    describing the party holding the evil shard aloft, completed by the
    ///    shard's own virtue word (Falsehood, Hatred, or Cowardice). This
    ///    happens before any gate is evaluated."
    /// 2. "It then plays the 920-step envelope swell of `systems/audio.md`
    ///    Section 5.4.7, again unconditionally: one held pitch" (corrected
    ///    2026-10-16, u5-spec#296, `RETRACTIONS.md` R522: "The pitch does not
    ///    move").
    /// 3. "Only the **position** gate produces the shared no-effect result."
    /// 4. "Once the position matches, it pauses, prints a line describing the
    ///    shard being cast into the Eternal Flame completed by the opposed
    ///    principle's word (Truth, Love, or Courage), and pauses again —
    ///    **before** testing whether a Shadowlord is on the flame and whether
    ///    the handshake matches."
    /// 5. "If either of those two gates fails, the handler simply returns. It
    ///    prints no refusal line."
    ///
    /// The spec names the two divergences to avoid explicitly: "evaluating the
    /// gates before any output ... and printing a refusal for the
    /// actor/handshake failures (in the original those are silent)". Both were
    /// present here and are corrected below.
    ///
    /// Boundary: the exact heading, aloft, cast and destruction strings are
    /// not published — only their order, their completing words, and which of
    /// them are unconditional — so the literals below are engine voice. The
    /// one-pitch swell of phase 2 is [`SoundEffect::ShardSwell`]
    /// (`audio.md §5.4.7`'s 920-step swell, "Shard use" row; u5-spec#296,
    /// R522/R523) and plays before the position gate on every path; the text
    /// after it waits the swell out.
    /// No `§8.4` major flash runs at the destruction: R523 (u5-spec#296)
    /// withdrew the attribution, and what the success arm plays and draws is
    /// indexed as open in `OPEN-QUESTIONS.md`.
    pub fn use_shadowlord_shard(
        &mut self,
        index: usize,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        let Some(item_index) = shadowlord_shard_special_item_index(index) else {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        };
        let name = special_item_name(item_index);
        // Not a gate: the U-Use picker only offers a carried shard, so this is
        // the picker precondition and it keeps its own refusal.
        if self.special_items[item_index] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        }

        // Phase 1: unconditional heading plus the aloft line, completed by the
        // shard's own virtue word.
        let virtue = Self::shadowlord_title_for_index(index).unwrap_or("Shadowlord");
        let _ = virtue;
        // `inventory.md` §7.1: "The shard continuation is `Thou dost hold
        // above thee the evil Shard of_` followed by `Falsehood...`,
        // `Hatred...` or `Cowardice...`" - the shard's own word carries an
        // ellipsis. Measured 2026-09-11 (`use-specials/sceptre`): the
        // original's row reads `Falsehood...` where this engine read
        // `Falsehood`.
        self.message = format!("{USE_SHARD_ALOFT_PREFIX}{name}...");

        // Phase 3: the position gate is the only one that speaks. The
        // published destruction rows (Lycaeum floor 2 (15,9); Empath Abbey
        // floor 1 (15,3); Serpent's Hold floor 0xFF (15,16)) already carry
        // their opposed flame, so "am I on this shard's row?" is one test and
        // it produces the shared no-effect result on any mismatch.
        let required_flame = eternal_flame_for_shadowlord(index).expect("valid shard index");
        let flame_entry = if let Some(game_dir) = game_dir {
            self.eternal_flame_at_current_position(game_dir)?
        } else {
            self.published_eternal_flame_at_current_position()
        };
        let on_position = flame_entry.is_some_and(|entry| entry.flame == required_flame);
        // Phase 2, the unconditional swell. `quest-graph.md §5` step 2
        // (corrected 2026-10-16, u5-spec#296, R522): "the 920-step envelope
        // swell ... again unconditionally: one held pitch ... It draws
        // nothing, reads no key and runs no world tick or animation pass" -
        // before the position gate, so the destruction row hears it too.
        // Consistent with `qa/paired/use-shard-sweep.tsv` (stock side): the
        // aloft line is up within ~0.1 s of Return, the swell runs 8.25-8.27 s
        // there, and the next text is on screen by the first sample after it
        // ends. The engine
        // printed everything at once (`use-specials/sceptre`, four rows
        // apart, `cleak/u5-engine#25`). Whatever follows waits out the swell:
        // the stock window ends on the aloft row itself during it, and that
        // row is still open when the next line's first feed arrives to close
        // it.
        self.emit_sound_effect(SoundEffect::ShardSwell);
        let swell_secs = SoundEffect::ShardSwell
            .program(&mut crate::audio::RumbleJitter::default())
            .duration(true)
            .as_secs_f64();
        if !on_position {
            // §7.1: "A wrong destruction position adds `\n\nNo effect!\n`" -
            // two line feeds, so a blank row stands between the aloft line and
            // the refusal. This engine spent one.
            //
            // The stock capture's stats panel, inverted from the aloft line
            // onward, is not drawn by this handler: u5-spec#296 traced it to
            // the picker's highlight leak (`inventory.md §4.4`,
            // `PlayState::stats_panel_window_inverse`).
            self.staged_narration
                .push_secs_on_open_row(swell_secs, "\n\nNo effect!\n");
            // `quest-graph.md §5` step 3 (u5-spec#296): "On that arm the
            // result line prints first and the 50-update cast-failure
            // glissando of `systems/audio.md` Section 5.2 (about 52 ms) plays
            // after it." Speaker effects play in order, so queued behind the
            // swell it starts as the swell's hold releases the line.
            self.emit_sound_effect(SoundEffect::CastFailure);
            return Ok(MoveOutcome::Blocked);
        }

        // Phase 4: the cast-into-the-flame line, completed by the opposed
        // principle's word, printed *before* the remaining two gates and,
        // like the refusal above, behind the swell.
        // §7.1: "At the matching position, the next text is
        // `\n\n...and cast it into the Flame of_` followed by `Truth!\n`,
        // `Love!\n` or `Courage!\n`."
        // `quest-graph.md §5` step 4 also brackets this line with a pause
        // before and after; neither length is published, so no extra hold
        // is invented beyond the swell.
        let mut staged_tail = format!("\n\n...and cast it into the {}!\n", required_flame.label());

        // Phase 5: the Shadowlord-on-flame and handshake gates are silent.
        // The cast line is the last text and nothing new is printed.
        // A shard whose Shadowlord slot is already vanquished can have no live
        // matching encounter, so it belongs with these silent tests rather
        // than with the position gate.
        if self.shadowlord_reads_vanquished(index)
            || !self.matching_shadowlord_name_encounter_north(index)
        {
            self.staged_narration
                .push_secs_on_open_row(swell_secs, staged_tail);
            return Ok(MoveOutcome::Blocked);
        }

        self.special_items[item_index] = 0;
        self.vanquish_shadowlord(index);
        // `audio.md §8.4` (corrected 2026-10-16, u5-spec#296,
        // `RETRACTIONS.md` R523) no longer names Shadowlord destruction as a
        // caller of the shared full-viewport flash: "whether a successful
        // Shadowlord destruction invokes this flash at all is **not
        // established** (the destruction arm's presentation has not been
        // traced; static reading shows a per-cell effect repeated seven times
        // and no call identified as this flash)". The engine ran the flash
        // here, with its 1,856 gameplay-PRNG draws, on the withdrawn text; it
        // now plays and draws nothing it cannot cite.
        let cleared = self.clear_shadowlord_name_encounters(index);
        self.mark_visibility_dirty();
        self.advance_turn();
        // Phase 6 closes with `inventory.md §7.1`'s published line: "The
        // later actual Shadowlord destruction adds `\nThe doom of the
        // Shadowlord_`, the matching name Faulinei, Astaroth or
        // Nosfentor, then `_is wrought!\n`."
        //
        // The engine wrote `<name> is vanquished! Cleared N encounter(s).`
        // here, which is an engine diagnostic wearing a game line's
        // clothes. The count keeps its value as bookkeeping and goes where
        // the rest of this engine's bookkeeping goes.
        // The name is the doom line's own mixed-case set, not the
        // capitalised Yell keywords (`inventory.md §7.1`, u5-spec#298 Q34).
        let name = crate::commands::use_shard_doom_name(index).unwrap_or("The Shadowlord");
        staged_tail.push_str(&format!("{USE_SHARD_DOOM_PREFIX}{name}{USE_SHARD_DOOM_SUFFIX}"));
        self.staged_narration
            .push_secs_on_open_row(swell_secs, staged_tail);
        self.diagnostics
            .push(format!("shard destruction cleared {cleared} encounter(s)"));
        Ok(MoveOutcome::Used)
    }

    pub fn eternal_flame_at_current_position(
        &self,
        game_dir: &Path,
    ) -> io::Result<Option<EternalFlameEntry>> {
        if let Some(entry) = self.published_eternal_flame_at_current_position() {
            return Ok(Some(entry));
        }
        let Some(entries) = load_eternal_flame_entries(game_dir)? else {
            return Ok(None);
        };
        Ok(entries
            .into_iter()
            .find(|entry| self.eternal_flame_entry_matches(*entry)))
    }

    pub fn published_eternal_flame_at_current_position(&self) -> Option<EternalFlameEntry> {
        let (target, floor, x, y) = self.current_blink_context();
        const PUBLISHED_FLAMES: [EternalFlameEntry; 3] = [
            EternalFlameEntry {
                target: PlayTarget::Town(Scene {
                    byte: SCENE_THE_LYCAEUM,
                    family: Family::Keep,
                    block: 5,
                }),
                floor: 2,
                x: 15,
                y: 9,
                flame: EternalFlame::Truth,
                expected_tile: None,
            },
            EternalFlameEntry {
                target: PlayTarget::Town(Scene {
                    byte: SCENE_EMPATH_ABBEY,
                    family: Family::Keep,
                    block: 6,
                }),
                floor: 1,
                x: 15,
                y: 3,
                flame: EternalFlame::Love,
                expected_tile: None,
            },
            EternalFlameEntry {
                target: PlayTarget::Town(Scene {
                    byte: SCENE_SERPENTS_HOLD,
                    family: Family::Keep,
                    block: 7,
                }),
                floor: -1,
                x: 15,
                y: 16,
                flame: EternalFlame::Courage,
                expected_tile: None,
            },
        ];
        PUBLISHED_FLAMES.into_iter().find(|entry| {
            entry.target == target && entry.floor == floor && entry.x == x && entry.y == y
        })
    }

    pub fn eternal_flame_entry_matches(&self, entry: EternalFlameEntry) -> bool {
        let (target, floor, x, y) = self.current_blink_context();
        let flame_tile = self.current_area_tile(entry.x, entry.y);
        entry.target == target
            && entry.floor == floor
            && entry.x == x
            && entry.y == y
            && entry
                .expected_tile
                .map_or(matches!(flame_tile, 0x76..=0x77), |expected| {
                    expected == flame_tile
                })
    }

    pub fn clear_shadowlord_name_encounters(&mut self, index: usize) -> usize {
        if self.summoned_shadowlord != Some(index) {
            return 0;
        }
        let Some(floor) = self.current_floor() else {
            return 0;
        };
        let mut cleared = 0;
        for object in self.active_objects.iter_mut().skip(1) {
            if object.z == floor && Self::is_shadowlord_actor(*object) {
                object.free();
                cleared += 1;
            }
        }
        // `formats/saved-gam.md §9.1` `0x0325` (u5-spec#300 Q3, R585):
        // destroying a Shadowlord with its shard leaves the last-summoned
        // id alone, so `summoned_shadowlord` is not reset here.
        cleared
    }

    pub fn use_worn_regalia(
        &mut self,
        special_item_index: usize,
        effect_tag: u8,
        missing_label: &str,
        wear_message: &str,
        remove_message: &str,
    ) -> MoveOutcome {
        if self.special_items[special_item_index] == 0 {
            self.message = format!("No {missing_label}!");
            return MoveOutcome::Blocked;
        }

        let message = if self.active_effect_tag == Some(effect_tag) {
            self.clear_active_effect_slot();
            // `stats-panel.md §2.2.1` row 7: "U-Use of the Amulet, Crown or
            // Black Badge while already worn (the shared toggle's removal
            // arm)" is deferred for all three regalia.
            self.stats_panel_command_refresh = crate::stats_panel::PanelRefreshMode::Deferred;
            remove_message.to_string()
        } else {
            self.active_effect_tag = Some(effect_tag);
            self.active_effect_counter = PERMANENT_ACTIVE_EFFECT_DURATION;
            // Row 8: the Black Badge's install arm is deferred too, unlike
            // the Amulet and Crown installs, which repaint inline (the
            // shared set-timed-effect helper's immediate site in the
            // seventy-four-site table).
            if special_item_index == SPECIAL_ITEM_BLACK_BADGE_INDEX {
                self.stats_panel_command_refresh = crate::stats_panel::PanelRefreshMode::Deferred;
            }
            wear_message.to_string()
        };
        self.mark_visibility_dirty();
        self.advance_turn();
        // The line is placed after the turn, as the pocket watch and the
        // Sceptre place theirs: the turn flushes whatever the slot holds,
        // and a line flushed there misses the picker's join, so it lost
        // the blank row `inventory.md §7.1`'s `Crown\n\n`/`Amulet\n\n`
        // completion leaves above it.
        self.message = message;
        MoveOutcome::Used
    }

    /// `inventory.md §7`/§7.1 Sceptre of Lord British. The wielding line
    /// prints on every accepted selection. Outside first-person dungeon
    /// exploration the 3x3 party-centred scan runs first; when it dissolves
    /// nothing - and always in a dungeon - "The Sceptre's fallback" calls
    /// the An Grav field-removal helper silently:
    ///
    /// - dungeon: "it tests the party's own cell and then the cell one step
    ///   ahead; a field cell is cleared to its visit-marker bit, the helper
    ///   itself prints `Field destroyed!`, and the Sceptre adds nothing;
    ///   otherwise it returns 0 and the Sceptre prints `No effect!`";
    /// - arena: the direction prompt (opened by the picker, see
    ///   [`Self::use_sceptre_arena_field_direction`]);
    /// - overworld and town: "treat the helper as finding no field".
    pub fn use_sceptre_of_lord_british(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        if let (Area::Dungeon { level, .. }, false) = (self.area, self.combat_active) {
            if self.sceptre_dungeon_field_helper(level) {
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = format!("{USE_SCEPTRE_WIELDED}\n{DISPEL_FIELD_DESTROYED_LINE}");
                return MoveOutcome::Used;
            }
            self.message = format!("{USE_SCEPTRE_WIELDED}\nNo effect!");
            return MoveOutcome::Blocked;
        }

        let dissolved = self.dissolve_sceptre_barriers_near_party();
        if dissolved == 0 {
            if self.combat_active {
                // The arena fallback's direction prompt belongs to the
                // picker; the inline form has no direction to give it, so
                // it reads as the cancelled prompt, which "returns
                // silently".
                self.message = USE_SCEPTRE_WIELDED.to_string();
                return MoveOutcome::Blocked;
            }
            self.message = format!("{USE_SCEPTRE_WIELDED}\nNo effect!");
            return MoveOutcome::Blocked;
        }

        self.mark_visibility_dirty();
        self.advance_turn();
        // `commands.md §8.1`: a command "never prints tile ids, coordinates,
        // active-object slot numbers, terrain-class names" - a count of
        // dissolved cells is the same kind of leak, so it goes to the harness
        // and the player sees only the published wielding line.
        self.push_diagnostic(format!("Sceptre dissolved {dissolved} barrier cell(s)."));
        self.message = USE_SCEPTRE_WIELDED.to_string();
        MoveOutcome::Used
    }

    /// True when an accepted Sceptre selection reaches the arena fallback's
    /// direction prompt: a combat scene whose nearby scan finds nothing.
    pub fn sceptre_arena_fallback_prompts(&self) -> bool {
        self.combat_active
            && self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] != 0
            && self.sceptre_barriers_near_party() == 0
    }

    /// `inventory.md §7.1`: "a chosen cell holding a field marker has that
    /// marker removed and returns 1 so the Sceptre prints `Field
    /// dissolved!`, and otherwise it returns 0 (`No effect!`)".
    pub fn use_sceptre_arena_field_direction(&mut self, direction: Direction) -> MoveOutcome {
        let (dx, dy) = direction.delta();
        let target = self
            .pending_combat_actor_slot
            .and_then(|slot| self.combat_actors.get(slot).copied())
            .and_then(|actor| {
                let tx = isize::from(actor.x) + dx;
                let ty = isize::from(actor.y) + dy;
                ((0..COMBAT_ARENA_SIDE as isize).contains(&tx)
                    && (0..COMBAT_ARENA_SIDE as isize).contains(&ty))
                .then_some((tx as u8, ty as u8))
            });
        let removed = target
            .and_then(|(tx, ty)| self.apply_combat_arena_field_removal(tx, ty))
            .is_some();
        self.advance_turn();
        if removed {
            self.message = crate::commands::USE_SCEPTRE_FIELD_DISSOLVED.to_string();
            MoveOutcome::Used
        } else {
            self.message = "No effect!".to_string();
            MoveOutcome::Blocked
        }
    }

    /// The An Grav dungeon arm used silently by the Sceptre: the party's own
    /// cell, then the cell one step ahead; a field is cleared to its
    /// visit-marker bit.
    fn sceptre_dungeon_field_helper(&mut self, level: u8) -> bool {
        let (dx, dy) = self.player.facing.delta();
        let own = (self.player.x as isize, self.player.y as isize);
        let ahead = (own.0 + dx, own.1 + dy);
        for (x, y) in [own, ahead] {
            if !(0..DUNGEON_SIDE as isize).contains(&x) || !(0..DUNGEON_SIDE as isize).contains(&y)
            {
                continue;
            }
            let idx = dungeon_cell_index(level, x as usize, y as usize);
            if dungeon_field_effect(self.grid[idx]).is_some() {
                self.grid[idx] &= 0x08;
                return true;
            }
        }
        false
    }

    pub fn dissolve_sceptre_barriers_near_party(&mut self) -> usize {
        let mut dissolved = 0;
        let x = self.player.x as isize;
        let y = self.player.y as isize;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let tx = x + dx;
                let ty = y + dy;
                if self.dissolve_sceptre_barrier_at(tx, ty) {
                    dissolved += 1;
                }
            }
        }
        dissolved
    }

    pub fn sceptre_barriers_near_party(&self) -> usize {
        let x = self.player.x as isize;
        let y = self.player.y as isize;
        (-1..=1)
            .flat_map(|dy| (-1..=1).map(move |dx| (x + dx, y + dy)))
            .filter(|(tx, ty)| {
                self.top_down_grid_index(*tx, *ty).is_some_and(|index| {
                    (SCEPTRE_BARRIER_TILE_FIRST..=SCEPTRE_BARRIER_TILE_LAST)
                        .contains(&self.grid[index])
                })
            })
            .count()
    }

    fn dissolve_sceptre_barrier_at(&mut self, x: isize, y: isize) -> bool {
        let Some(index) = self.top_down_grid_index(x, y) else {
            return false;
        };
        if !(SCEPTRE_BARRIER_TILE_FIRST..=SCEPTRE_BARRIER_TILE_LAST).contains(&self.grid[index]) {
            return false;
        }
        self.grid[index] = SCEPTRE_BARRIER_DISSOLVED_TILE;
        true
    }

    fn top_down_grid_index(&self, x: isize, y: isize) -> Option<usize> {
        match self.area {
            Area::World { .. } => {
                if !(0..WORLD_SIDE as isize).contains(&x) || !(0..WORLD_SIDE as isize).contains(&y)
                {
                    return None;
                }
                Some(world_cell_index(x as usize, y as usize))
            }
            Area::Town { .. } => {
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    return None;
                }
                Some(y as usize * 32 + x as usize)
            }
            Area::Dungeon { .. } => None,
        }
    }

    pub fn use_hms_cape_plans(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        if !matches!(self.player.transport, TransportState::Ship { .. }) {
            self.message = USE_PLANS_SHIPBOARD_ONLY_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }

        self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] =
            self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX].max(2);
        self.advance_turn();
        // Measured 2026-09-07 aboard a frigate: the line ends in an
        // exclamation mark, like the rest of the U-Use results.
        self.message = USE_PLANS_RIGGED_LINE.to_string();
        MoveOutcome::Used
    }

    pub fn ship_rigging_active(&self) -> bool {
        self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] > 1
    }

    pub fn advance_sailing_wait_turn(&mut self) {
        if self.ship_rigging_active() {
            let advance_active_objects = self.turn % 2 == 1;
            self.advance_turn_with_minutes_and_active_objects(1, advance_active_objects);
        } else {
            self.advance_turn();
        }
        // A sailing wait pass runs its own outdoor epilogue (`weather.md §5`).
        self.apply_pending_world_object_walker_pass();
    }

    pub fn use_magic_carpet(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        // `inventory.md §7.1` (`cleak/u5-spec#251`) publishes the whole gate
        // order, and it is not the carpet movement predicate the engine was
        // reusing: "Activating a carried carpet uses its own terrain rule: it
        // accepts map tile ids `0x00..0x0B` and `0x0D..0xFF`, and rejects only
        // mountains `0x0C`. Neither the on-foot nor the carpet movement
        // predicate is consulted." Chairs `0x90..0x93` therefore permit
        // boarding even though carpet *movement* rejects them, which is what
        // a capture at Iolo's hut shows.
        //
        // Gate 1: "Scene ids `0x21..0xFF` produce `Not here!\n` without a
        // terrain lookup." That band is every dungeon and the temporary
        // combat scene; `0x00..0x20` is the overworld and the town family.
        if self.current_scene_byte() > CARPET_BOARDING_LAST_ELIGIBLE_SCENE {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        // Gate 2: "In an eligible scene, mountains produce that same refusal
        // **before transport is considered**. Thus a ship marker on mountains
        // receives `Not here!\n`."
        let tile = self.current_area_tile(self.player.x, self.player.y);
        if tile == CARPET_BOARDING_REJECTED_TILE {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        // Gate 3: "exact transport marker `0x1C` permits boarding; ship
        // markers `0x20..0x27` produce `X-it ship first!\n`, and every other
        // marker produces `Only on foot!\n`." Foot persists exactly `0x1C`.
        if !self.player.transport.is_foot() {
            self.message = if matches!(self.player.transport, TransportState::Ship { .. }) {
                USE_MAGIC_CARPET_XIT_FIRST.to_string()
            } else {
                USE_MAGIC_CARPET_ONLY_ON_FOOT.to_string()
            };
            return MoveOutcome::Blocked;
        }

        // "Success prints `Boarded!\n`, selects one of the two carpet frames
        // with equal probability" - markers `0x14` and `0x15`. The marker,
        // not the engine's visual tile, is the transport type: X-it parks a
        // carpet object only from a carpet marker (playtest P12-12).
        let pick = u5_prng_range_u16(&mut self.prng_state, 0, 1) as usize;
        let marker = CARPET_MARKER_FRAMES[pick % CARPET_MARKER_FRAMES.len()];
        let transport = TransportState::Carpet {
            type_byte: marker,
            tile: transport_visual_tile_for_marker(marker)
                .unwrap_or(FIRST_PLAYABLE_MAGIC_CARPET_TILE + pick as u8),
        };

        self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] =
            self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX].saturating_sub(1);
        self.player.transport = transport;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = USE_MAGIC_CARPET_BOARDED.to_string();
        MoveOutcome::Boarded
    }

    pub fn use_sextant(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_SEXTANT_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        // `catalogs/item-list.md` Sextant row / `inventory.md §7`: three
        // conditions, of which the plane is tested first and
        // short-circuits. The Underworld is the outdoor world scene on the
        // other plane, so it fails here and takes the same refusal an
        // indoor scene takes — no Underworld-specific message, and it
        // never reaches the night test.
        let outdoors = match self.area {
            Area::World { plane } => {
                sextant_outdoor_position(plane.plane_byte(), self.current_scene_byte())
            }
            Area::Town { .. } | Area::Dungeon { .. } => false,
        };
        if !outdoors {
            // Measured 2026-09-07 indoors: `Only outdoors!`.
            self.message = USE_SEXTANT_ONLY_OUTDOORS.to_string();
            return MoveOutcome::Blocked;
        }
        // The Sextant's night window is `19..=23` / `0..=5`, which is not
        // the town-lighting window `is_town_night_hour` carries.
        if !sextant_night_hour(self.clock.hour) {
            self.message = USE_SEXTANT_DAYTIME_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }

        self.advance_turn();
        // `magic.md §8` / `inventory.md §7`: the Sextant shares Locate's
        // coordinate printer, so both callers go through the one helper.
        // The label is followed by the printer's leading newline, then Y
        // and X each carry their own closing quote separated by
        // comma-space, then a further line break.
        // Measured 2026-09-07: the label prints in the text font and the
        // coordinate row in the runic one. Under `magic.md` §8's published
        // rule the runic cells decode to exactly this pair, so the values
        // were right all along and only the font was wrong
        // (`cleak/u5-spec#237`).
        let pair = sextant_coordinate_pair_line(self.player.y as u8, self.player.x as u8);
        // `inventory.md §7.1`'s completion table gives the Sextant
        // `Sextant\n\n`: the second feed leaves one blank row above
        // `Position:`. The picker inserts that feed for a handler that
        // returns its line in the slot; this one records its own rows, so
        // it carries the feed itself (playtest G12). On a still-open row the
        // feed only closes it (`text-output.md §10.4`).
        self.emit_message_line(format!("\n{USE_SEXTANT_READING_LABEL}"));
        for row in pair.split('\n').filter(|row| !row.is_empty()) {
            self.push_runic_message_entry(row);
        }
        self.adopt_flushed_message(format!("{USE_SEXTANT_READING_LABEL}{pair}"));
        // The coordinate printer ends with its own line feed (`magic.md §8`:
        // "a newline before the pair as well as after it"), so the readout
        // leaves the row closed. The runic rows above do not touch the flag,
        // which still held the open `Position:` row and swallowed the next
        // command's blank row (playtest J12).
        self.message_row_open_mid_line = false;
        MoveOutcome::Used
    }

    pub fn use_pocket_watch(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        let display_hour = self.clock.display_hour();
        let minute = self.clock.minute;
        let suffix = self.clock.am_pm_suffix();
        self.advance_turn();
        self.message = format!("{USE_POCKET_WATCH_PREFIX}{display_hour}:{minute:02} {suffix}.");
        MoveOutcome::Used
    }

    /// `catalogs/item-list.md` Spyglass row / `inventory.md §7`: the
    /// world-plane byte the Sextant/Spyglass plane test reads. On the
    /// outdoor scenes it is the active plane. Inside a town or dungeon
    /// the party is still standing on one of the two planes, and the
    /// return snapshot is what remembers which; with no snapshot the
    /// surface plane is the only reachable answer, because a game that
    /// has never crossed to the Underworld has never left it.
    pub fn current_world_plane_byte(&self) -> u8 {
        match self.area {
            Area::World { plane } => plane.plane_byte(),
            Area::Town { .. } | Area::Dungeon { .. } => self
                .return_world
                .as_ref()
                .map_or(WorldPlane::Britannia, |snapshot| snapshot.plane)
                .plane_byte(),
        }
    }

    pub fn use_spyglass(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        // `catalogs/item-list.md` Spyglass row: three conditions, of
        // which the plane and scene pair is the "not here" refusal and
        // the hour is the no-stars refusal. The scene gate admits the
        // outdoor world scene *or a town-class scene* — broader than the
        // Sextant's — and the night window is the Sextant's `19..=23` /
        // `0..=5`, not the town-lighting window this handler used to
        // read, which disagrees at hours 5 and 19.
        if !spyglass_position_admits(self.current_world_plane_byte(), self.current_scene_byte()) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if !sextant_night_hour(self.clock.hour) {
            self.message = USE_SPYGLASS_NO_STARS.to_string();
            return MoveOutcome::Blocked;
        }

        // `inventory.md §7.1` utility results: "Spyglass accepted | `Looking...\n`,
        // then the sky view". The engine's own sentence named the item and
        // described the act.
        self.activate_night_sky_overlay(Some("Looking..."));
        MoveOutcome::Observed
    }

    /// `systems/view.md §4.2.1`: a telescope shows the sun during
    /// hours 6 through 17, selects the first healthy/poisoned member
    /// when necessary, and applies one point through the shared damage
    /// path. All other hours enter the night-sky overlay.
    ///
    /// The telescope is terrain-path row 2, so `view.md §3` entry-dispatch
    /// row 3's shared "thou dost see" preamble has already been printed: the
    /// row replaces only the description text (playtest J06).
    pub fn look_through_telescope(&mut self) {
        if !sky_view_is_daylight(self.clock.hour) {
            self.activate_night_sky_overlay(Some(LOOK_RESULT_PREFIX));
            return;
        }

        self.active_view_overlay = None;
        if self.active_player.is_none() {
            self.active_player = self
                .party
                .iter()
                .position(|member| matches!(member.status, b'G' | b'P'));
            if self.active_player.is_some() {
                // A live selection replaces a loaded out-of-range byte
                // (playtest P15-02).
                self.active_player_save_byte = 0xff;
            }
        }
        if let Some(slot) = self.active_player.filter(|slot| *slot < self.party.len()) {
            self.apply_shared_party_damage(slot, 1);
        }
        self.message = format!("{LOOK_RESULT_PREFIX}\nthe sun!");
    }

    /// `systems/view.md §4.2.2`: capture the eighty PRNG-selected stars
    /// once when the modal view opens, hide the ordinary visibility
    /// window, and leave it dirty so closing the overlay restores the
    /// world view.
    pub fn activate_night_sky_overlay(&mut self, message_prefix: Option<&str>) {
        let stars =
            std::array::from_fn(|_| (self.random_range_u8(9, 182), self.random_range_u8(9, 172)));
        let sky = SkyOverlayState {
            stars,
            body_columns: sky_body_columns(self.clock),
        };
        self.mark_visibility_dirty();
        for row in 0..VIEWPORT_SIDE {
            for col in 0..VIEWPORT_SIDE {
                self.visibility_grid[visibility_grid_active_index(row, col).unwrap()] =
                    VISIBILITY_HIDDEN;
            }
        }
        let text_map = sky_text_map(&sky, self.shadowlord_hideouts);
        self.active_view_overlay = Some(ViewOverlay {
            title: "the night sky!".to_string(),
            text_map,
            kind: ViewOverlayKind::Sky(sky),
            mode: ViewOverlayMode::SkyView,
        });
        self.message = match message_prefix {
            Some(prefix) => format!("{prefix}\nthe night sky! "),
            None => "the night sky! ".to_string(),
        };
    }

    pub fn use_scroll(
        &mut self,
        index: usize,
        direction: Option<Direction>,
        target: Option<usize>,
    ) -> MoveOutcome {
        if index >= SCROLL_COUNT || self.scroll_stock[index] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        self.scroll_stock[index] = self.scroll_stock[index].saturating_sub(1);

        match index {
            SCROLL_LIGHT_INDEX => {
                self.light_spell_counter = SCROLL_LIGHT_DURATION;
                self.recompute_daylight();
                // `audio.md §6.1`: "Sets a torch radius, then calls the
                // dispatcher directly." The scroll supplies its scroll index,
                // so this is variant 0 - the one variant no spell reaches, and
                // emphatically not In Lor's variant 1.
                self.emit_scroll_shared_variant(SCROLL_LIGHT_INDEX);
                self.advance_turn();
                self.message = "Light!".to_string();
                MoveOutcome::Used
            }
            SCROLL_WIND_CHANGE_INDEX => self.use_wind_change_scroll(direction),
            SCROLL_PROTECTION_INDEX => {
                // `audio.md §6.1`: "Through the scene-flag helper", at the
                // scroll's own index 2 - not the Protection spell's variant 4.
                self.emit_scroll_shared_variant(SCROLL_PROTECTION_INDEX);
                self.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
                self.active_effect_counter = SCROLL_PROTECTION_DURATION;
                self.advance_turn();
                self.message = "Protection!".to_string();
                MoveOutcome::Used
            }
            SCROLL_NEGATE_MAGIC_INDEX => {
                // `audio.md §6.1`: scene-flag helper at scroll index 3, where
                // the Negate Magic spell is variant 6.
                self.emit_scroll_shared_variant(SCROLL_NEGATE_MAGIC_INDEX);
                self.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
                self.active_effect_counter = SCROLL_NEGATE_MAGIC_DURATION;
                self.advance_turn();
                self.message = "Negate magic!".to_string();
                MoveOutcome::Used
            }
            SCROLL_VIEW_INDEX => {
                if self.combat_active {
                    // `audio.md §6.1`: "Refused with `Not here!` and **no
                    // sound** outside the permitted scene class." `§9` lists
                    // this refusal among the explicit silence boundaries.
                    // `item-list.md §7.1`: the scroll path "prints the
                    // scroll-use label and the branch's effect label", and
                    // scroll 4 "Prints `View!`. Combat-class scenes print
                    // `Not here!` and stop" - the banner comes first
                    // (playtest F32).
                    self.message = "View!\nNot here!".to_string();
                    return MoveOutcome::Blocked;
                }
                // `audio.md §6.1`: "Dispatcher, then the look helper" - and
                // "Both look helpers are silent", so the variant is all this
                // scroll plays.
                self.emit_scroll_shared_variant(SCROLL_VIEW_INDEX);
                let _ = self.activate_peer_view_overlay();
                // `view.md §4`: owed to the overlay's dismissing key above
                // ground (playtest P21-03).
                self.advance_turn();
                self.message = "View!".to_string();
                MoveOutcome::Observed
            }
            SCROLL_SUMMON_DAEMON_INDEX => {
                if !self.combat_active {
                    // `audio.md §6.1`: "only in the permitted scene class, else
                    // `Not here!` and silence" (§9). `item-list.md §7.1`,
                    // scroll 5: "Prints `Summon Daemon!`. ... non-combat
                    // scenes print `Not here!`" - banner first (playtest F32).
                    self.message = "Summon Daemon!\nNot here!".to_string();
                    return MoveOutcome::Blocked;
                }
                let Some(caster) = self.combat_actors.first().copied() else {
                    // An arena with no actors cannot take a command.
                    self.message.clear();
                    return MoveOutcome::Blocked;
                };
                if !combat_actor_is_active_not_dead(caster) {
                    // Not a state a keystroke can produce; see the other guards.
                    self.message.clear();
                    return MoveOutcome::Blocked;
                }
                // `audio.md §6.1`: "Through the placement helper", which
                // sounds unconditionally at entry, before the eight-try cell
                // probe. Variant 5 is the scroll index; the Summon Daemon
                // *spell* is variant 8.
                self.emit_scroll_shared_variant(SCROLL_SUMMON_DAEMON_INDEX);
                let legal_cells = self.combat_spawn_cell_mask();
                let applied = self.apply_combat_summon_daemon_with_random_attempts(
                    self.combat_actor_z(0),
                    &legal_cells,
                );
                self.advance_turn();
                // The banner precedes the helper (`item-list.md §7.1`), so a
                // placement that finds no cell still shows it above the
                // U-Use failure tail.
                let Some(_applied) = applied else {
                    self.message = "Summon Daemon!\nFailed!".to_string();
                    return MoveOutcome::Blocked;
                };
                // `item-list.md §7.1`, scroll 5: the scroll invokes "the
                // summon **placement** helper" and what it does "is create a
                // hostile actor at the first accepted cell". The caster
                // self-check and its controlled-bit stamp belong to the Kal
                // Xen Corp *spell* path (`magic.md §8`: "This spell path uses
                // the helper's self-checking mode"; `spell-list.md` row 43),
                // so the scroll's Daemon is never handed to the player and
                // there is no `Oops...` roll (playtest F31).
                self.message = "Summon Daemon!".to_string();
                MoveOutcome::Used
            }
            SCROLL_RESURRECTION_INDEX => self.use_resurrection_scroll(target),
            SCROLL_NEGATE_TIME_INDEX => self.use_negate_time_scroll(),
            _ => {
                self.message = "No effect!".to_string();
                MoveOutcome::Blocked
            }
        }
    }

    pub fn use_wind_change_scroll(&mut self, direction: Option<Direction>) -> MoveOutcome {
        let Some(direction) = direction else {
            self.request_cast_argument(CastArgumentRequest::Direction);
            return MoveOutcome::Blocked;
        };
        // `catalogs/item-list.md §7.1`, scroll 1: the wind applies "only
        // when the active scene byte is below 33". Combat is scene byte 0xFF
        // (`spell-list.md §4`), so a combat read is gated the same way
        // (`moons.md §3`; playtest P12-10). In every scene at 33 or above
        // the branch skips the wind setter and reports failure, so the U-Use
        // tail prints `Failed!` (u5-spec#312 Q3, `RETRACTIONS.md` R656: the
        // earlier silent return of playtest P12-11 is withdrawn).
        if self.wind_change_scroll_gate_fails() {
            self.wind_change_scroll_failure_tail();
            return MoveOutcome::Blocked;
        }

        let previous = self.wind.status_message();
        let next = WindState::rel_hur_target(direction)
            .expect("inline wind scroll parser returns cardinal directions only");
        // `audio.md §7.3`: the scroll carries its own caller tag and plays
        // variant 1, not the spell's variant 2.
        self.apply_wind_state_from_scroll(next);
        self.advance_turn();
        // Measured: the effect line is printed when the prompt opens, and
        // the accepted direction closes the exchange with nothing further.
        let _ = previous;
        self.message.clear();
        MoveOutcome::Used
    }

    /// `catalogs/item-list.md §7.1`, scroll 1 (corrected for u5-spec#312
    /// Q3, R656): the Wind Change scroll's scene test, made only after the
    /// `Direction-` prompt. Every dungeon-class and combat-class scene - the
    /// scene byte at 33 or above - fails it.
    pub(crate) fn wind_change_scroll_gate_fails(&self) -> bool {
        self.combat_active || matches!(self.area, Area::Dungeon { .. })
    }

    /// `catalogs/item-list.md §7.1`, scroll 1, and `inventory.md §7.1`
    /// (u5-spec#312 Q3): where the gate fails "the wind is unchanged, the
    /// wind banner is not repainted and no wind sound plays", and "the
    /// generic U-Use failure tail then prints `Failed!` plus one line feed
    /// (no leading feed) at column zero on the row after the direction word
    /// and plays the common cast-failure glissando". `Pass` ends the same
    /// way. The scroll was spent when its row was accepted.
    pub(crate) fn wind_change_scroll_failure_tail(&mut self) {
        self.message = "Failed!".to_string();
        self.emit_sound_effect(SoundEffect::CastFailure);
    }

    pub fn use_resurrection_scroll(&mut self, target: Option<usize>) -> MoveOutcome {
        if self.combat_active {
            // `item-list.md §7.1`, scroll 6: "Prints `Resurrection!`.
            // Non-combat scenes prompt for a party member ... Combat-class
            // scenes print `Not here!`." No `On who:` prompt opens in combat
            // (playtest F29); the banner still prints first.
            self.message = format!("{SCROLL_RESURRECTION_RESULT}\nNot here!");
            return MoveOutcome::Blocked;
        }
        let Some(target_index) = target else {
            self.request_cast_argument(CastArgumentRequest::PartyTarget);
            return MoveOutcome::Blocked;
        };
        if target_index >= self.party.len() {
            // Out-of-range member: the interactive pickers bound the
            // index, so only the inline harness form reaches this. The
            // original has no line for a state it cannot enter.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        if self.party[target_index].status != b'D' {
            // `magic.md §8` healing effects (corrected 2026-10-16, u5-spec#298
            // Q23, `RETRACTIONS.md` R542): the scroll is the resurrect
            // helper's verbose caller, so a living target prints `Not dead!`,
            // after which the U-Use tail adds `Failed!` with the shared
            // failure glissando.
            self.advance_turn();
            self.message = format!("{}\nFailed!", crate::commands::SCROLL_RESURRECTION_NOT_DEAD);
            self.emit_sound_effect(SoundEffect::CastFailure);
            return MoveOutcome::Blocked;
        }

        // `audio.md §6.1`: "Through the resurrect helper; only in the permitted
        // scene class and only if the target is dead." Variant 6 is the scroll
        // index; the Resurrect spell is variant 8.
        self.emit_scroll_shared_variant(SCROLL_RESURRECTION_INDEX);

        let _ = self
            .resurrect_party_member_to_hp(target_index, 1)
            .expect("target status checked before scroll resurrection");
        self.advance_turn();
        // Measured: the raising itself prints nothing; `Resurrection!` was
        // printed when the target prompt opened.
        self.message.clear();
        MoveOutcome::Used
    }

    pub fn use_negate_time_scroll(&mut self) -> MoveOutcome {
        if matches!(
            self.area,
            Area::Town { scene, .. } if scene.byte == STONEGATE_SCENE_BYTE
        ) || matches!(
            self.area,
            Area::Dungeon { scene, .. } if scene.byte == 40
        ) {
            self.advance_turn();
            self.message = "No effect!".to_string();
            // `audio.md §6.1`: "In two specific scenes it instead prints
            // `No effect!` and plays the 50-update cast-failure glissando."
            self.emit_sound_effect(SoundEffect::CastFailure);
            return MoveOutcome::Blocked;
        }

        // `audio.md §6.1`: "Through the scene-flag helper" at scroll index 7.
        self.emit_scroll_shared_variant(SCROLL_NEGATE_TIME_INDEX);
        self.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
        self.active_effect_counter = SCROLL_NEGATE_TIME_DURATION;
        self.advance_turn();
        self.message = "Negate time!".to_string();
        MoveOutcome::Used
    }

    pub fn use_potion(&mut self, index: usize, target: Option<usize>) -> MoveOutcome {
        if index >= POTION_COUNT || self.potion_stock[index] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        self.potion_stock[index] = self.potion_stock[index].saturating_sub(1);

        let Some(target_index) = target else {
            // Measured: the potion asks `On who: ` and completes the row
            // with the chosen member's name.
            self.request_cast_argument(CastArgumentRequest::PartyTarget);
            self.message = USE_POTION_TARGET_PROMPT.to_string();
            return MoveOutcome::Blocked;
        };
        if target_index >= self.party.len() {
            // Out-of-range member: the interactive pickers bound the
            // index, so only the inline harness form reaches this. The
            // original has no line for a state it cannot enter.
            self.message.clear();
            return MoveOutcome::Blocked;
        }

        let effect_index = self.potion_effect_index_after_variation_roll(index);
        self.use_potion_with_effect(index, target_index, effect_index)
    }

    pub fn use_potion_with_effect(
        &mut self,
        selected_index: usize,
        target_index: usize,
        effect_index: usize,
    ) -> MoveOutcome {
        // `catalogs/item-list.md §7.2`: the accepted selected bottle owns a
        // blocking full-playfield flash before the later variation-selected
        // gameplay effect. The frontend consumes this pending event before it
        // displays the resulting gameplay state.
        self.pending_potion_flash = potion_flash_playback(selected_index);
        // `audio.md §7.2`: the shared presentation begins only once the
        // bottle is decremented (`use_potion`) and a party-member target is
        // accepted, and "the selected bottle id, not the later variation roll,
        // chooses variant 0 through 7 from Section 6". The published flash
        // playback already keys on the selected bottle — its rumble target is
        // `8000 + 1600 * selected_index` and its sweep length
        // `10000 + 4000 * selected_index`, which are exactly §6's target and
        // iteration columns for variant `selected_index` — so the bottle id is
        // the variant, with no remapping. An out-of-range bottle yields no
        // playback and no cue.
        if let Some(playback) = self.pending_potion_flash {
            self.emit_sound_effect(SoundEffect::SharedVariant {
                variant: playback.selected_index as u8,
            });
        }
        match effect_index {
            POTION_BLUE_INDEX => {
                if self.party[target_index].status == b'S' && self.party[target_index].hp > 0 {
                    self.party[target_index].status = b'G';
                    if self.clear_combat_party_sleep_presentation(self.combat_descriptor_for_party_member(target_index)) {
                        self.mark_visibility_dirty();
                    }
                    self.advance_turn();
                    // Measured: waking a sleeping member prints no line at
                    // all - only the status letter changes.
                    self.message.clear();
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = POTION_RESULT_FAILED.to_string();
                    MoveOutcome::Blocked
                }
            }
            POTION_YELLOW_INDEX => {
                if !self.party[target_index].living() {
                    self.advance_turn();
                    self.message = POTION_RESULT_FAILED.to_string();
                    return MoveOutcome::Blocked;
                }
                let amount = self.potion_heal_amount();
                let _ = self.party[target_index].heal_by(amount);
                self.advance_turn();
                self.message = POTION_RESULT_HEALED.to_string();
                MoveOutcome::Used
            }
            POTION_RED_INDEX => {
                if self.party[target_index].status == b'P' {
                    self.party[target_index].status = b'G';
                    self.advance_turn();
                    self.message = POTION_RESULT_POISON_CURED.to_string();
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = POTION_RESULT_FAILED.to_string();
                    MoveOutcome::Blocked
                }
            }
            POTION_GREEN_INDEX => {
                if self.party[target_index].status == b'G' && self.party[target_index].hp > 0 {
                    self.party[target_index].status = b'P';
                    self.advance_turn();
                    self.message = POTION_RESULT_POISONED.to_string();
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = POTION_RESULT_FAILED.to_string();
                    MoveOutcome::Blocked
                }
            }
            POTION_ORANGE_INDEX => {
                if self.party[target_index].status == b'G' && self.party[target_index].hp > 0 {
                    if self.combat_active {
                        if matches!(
                            apply_combat_sleep_to_party_target(&mut self.party[target_index]),
                            CombatPartySleepOutcome::SleptPartyMember { .. }
                        ) && self.apply_combat_party_sleep_presentation(self.combat_descriptor_for_party_member(target_index))
                        {
                            self.mark_visibility_dirty();
                        }
                    } else {
                        self.party[target_index].status = b'S';
                    }
                    self.advance_turn();
                    self.message = POTION_RESULT_SLEPT.to_string();
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = POTION_RESULT_FAILED.to_string();
                    MoveOutcome::Blocked
                }
            }
            POTION_PURPLE_INDEX => {
                if !self.combat_active {
                    self.advance_turn();
                    self.message = POTION_RESULT_NO_NOTICEABLE_EFFECT.to_string();
                    return MoveOutcome::Blocked;
                }
                let applied = self.apply_combat_potion_poof_presentation(self.combat_descriptor_for_party_member(target_index));
                if applied {
                    self.mark_visibility_dirty();
                }
                self.advance_turn();
                // `inventory.md §7.2`: "Purple in combat | `Poof!\n`".
                self.message = POTION_RESULT_POOF.to_string();
                if applied {
                    MoveOutcome::Used
                } else {
                    MoveOutcome::Blocked
                }
            }
            POTION_BLACK_INDEX => {
                if !self.combat_active {
                    self.advance_turn();
                    self.message = POTION_RESULT_NO_NOTICEABLE_EFFECT.to_string();
                    return MoveOutcome::Blocked;
                }
                self.advance_turn();
                let applied = self.apply_combat_party_invisibility_potion(self.combat_descriptor_for_party_member(target_index));
                // `inventory.md §7.2`: "Black in combat | `Invisible!\n`".
                self.message = POTION_RESULT_INVISIBLE.to_string();
                if applied {
                    MoveOutcome::Used
                } else {
                    MoveOutcome::Blocked
                }
            }
            POTION_WHITE_INDEX => {
                if matches!(self.area, Area::Dungeon { .. }) || self.combat_active {
                    self.advance_turn();
                    self.message = POTION_RESULT_NO_NOTICEABLE_EFFECT.to_string();
                    return MoveOutcome::Blocked;
                }
                self.start_visibility_sweep();
                self.advance_turn();
                // Measured: the white potion's repaint prints no line.
                self.message.clear();
                MoveOutcome::Observed
            }
            _ => {
                self.advance_turn();
                self.message = POTION_RESULT_FAILED.to_string();
                MoveOutcome::Blocked
            }
        }
    }

    /// `catalogs/item-list.md §7.2`: "fourteen chances in sixteen use the
    /// selected colour's effect, one chance in sixteen forces the Orange
    /// sleep effect, and one chance in sixteen replaces the effect with a
    /// random potion id from `0..7`". `prng.md §1` puts such small game
    /// decisions on the shared range generator, so both rolls are draws from
    /// it - never a hash of the turn, position and clock (playtest K21).
    /// "Draw order of the variation roll" (u5-spec#300 Q23): the first draw
    /// over `0..15` gives Orange on `0`, and only a `1` makes the second draw
    /// over `0..7`; `2..15` keep the selected colour.
    pub fn potion_effect_index_after_variation_roll(&mut self, selected_index: usize) -> usize {
        let variation_roll = self.potion_variation_roll();
        let random_roll = if variation_roll & POTION_VARIATION_ROLL_MASK
            == POTION_VARIATION_RANDOM_ID_ROLL
        {
            self.potion_random_effect_roll()
        } else {
            0
        };
        potion_effect_index_after_variation(selected_index, variation_roll, random_roll)
    }

    pub fn potion_variation_roll(&mut self) -> u8 {
        self.random_range_u8(0, POTION_VARIATION_ROLL_MASK)
    }

    pub fn potion_random_effect_roll(&mut self) -> u8 {
        self.random_range_u8(0, POTION_VARIATION_RANDOM_INDEX_MASK)
    }

    /// `catalogs/item-list.md §7.2` Yellow: "Heal the selected target
    /// through the shared small-heal helper" - the same rolled amount the
    /// Heal spell draws.
    pub fn potion_heal_amount(&mut self) -> u16 {
        self.heal_spell_amount()
    }

    pub fn clear_combat_party_sleep_presentation(&mut self, target_index: usize) -> bool {
        if !self.combat_active || target_index >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(actor) = self.combat_actors.get_mut(target_index) else {
            return false;
        };
        if !actor.is_status_disabled() {
            return false;
        }
        actor.clear_status_disabled();
        // `catalogs/item-list.md` "Orange combat sleep presentation": wake
        // "selects the retained base/type tile as the display tile, except
        // that an actor still under the combat invisibility state uses tile
        // `0x1D`". Invisibility "rides on bit `0x10`, the phase/blink bit"
        // (`RETRACTIONS.md` R380); bit `0x04` is the dragged-under state.
        let hidden = actor.is_phase_suppressed();
        let active_object_slot = usize::from(actor.active_object_slot);
        if let Some(object) = self.active_objects.get_mut(active_object_slot) {
            object.tile = if hidden {
                COMBAT_POTION_INVISIBLE_WAKE_DISPLAY_TILE
            } else {
                object.type_byte
            };
        }
        true
    }

    pub fn apply_combat_party_sleep_presentation(&mut self, target_index: usize) -> bool {
        if !self.combat_active || target_index >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(actor) = self.combat_actors.get_mut(target_index) else {
            return false;
        };
        if actor.is_empty() || actor.is_marked_dead() {
            return false;
        }
        let active_object_slot = usize::from(actor.active_object_slot);
        let Some(object) = self.active_objects.get_mut(active_object_slot) else {
            return false;
        };
        actor.set_status_disabled();
        object.tile = COMBAT_POTION_SLEEP_DISPLAY_TILE;
        true
    }

    pub fn apply_combat_potion_poof_presentation(&mut self, target_index: usize) -> bool {
        if !self.combat_active || target_index >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(actor) = self.combat_actors.get(target_index).copied() else {
            return false;
        };
        if actor.is_empty() || actor.is_marked_dead() {
            return false;
        }
        let active_object_slot = usize::from(actor.active_object_slot);
        if active_object_slot >= self.active_objects.len() {
            return false;
        }
        let object = &mut self.active_objects[active_object_slot];
        object.type_byte = COMBAT_POTION_POOF_TILE;
        object.tile = COMBAT_POTION_POOF_TILE;
        true
    }

    /// `catalogs/item-list.md §7.2` (White visibility repaint sequence) and
    /// `systems/magic.md §8` (X-Ray, *Wis An Ylem*): the shared spell/potion
    /// visibility sweep. It "invokes the ordinary visibility producer exactly
    /// once, centred on the party's local viewport position — and it invokes
    /// it in the producer's **no-line-of-sight mode**, by passing the negative
    /// sentinel in the light argument". Every one of the 121 cells of the
    /// eleven-by-eleven window is refilled straight from the map: no distance
    /// test, no propagation frontier, no blocker rule. "A wall does not stop
    /// the reveal, and a cell in the far corner is revealed exactly as readily
    /// as the party's own."
    ///
    /// **R318.** The withdrawn text had this call pass the value `32` as an
    /// inclusive squared-Euclidean gate admitting 101 of the 121 cells, with a
    /// blocker inside the gate visible but stopping propagation past itself.
    /// That argument is never read by the producer.
    ///
    /// "The normal map reader supplies tiles, so overworld coordinate wrapping
    /// and named-location bounds remain exactly their ordinary rules; White
    /// adds no scan, clipping, or wrapping rule of its own" — the window-bound
    /// clipping stays in the render grid, which is the map reader here.
    pub fn start_visibility_sweep(&mut self) {
        let wrap_world = matches!(self.area, Area::World { .. });
        let visible = self.surface_visibility_produce(
            self.player.x as isize,
            self.player.y as isize,
            VIEWPORT_PLAYER_ROW,
            VISIBILITY_NO_LINE_OF_SIGHT_LIGHT,
            wrap_world,
        );
        let mut visible_cells = [false; VIEWPORT_SIDE * VIEWPORT_SIDE];
        visible_cells.copy_from_slice(&visible);
        self.visibility_sweep = Some(VisibilitySweep {
            frames_remaining: POTION_WHITE_SWEEP_FRAMES,
            pause_bios_ticks_per_frame: POTION_WHITE_SWEEP_BIOS_TICKS_PER_FRAME,
            center_x: self.player.x,
            center_y: self.player.y,
            visible_cells,
        });
    }

    /// Take the selected-bottle flash that must be shown before the resulting
    /// potion effect is repainted.
    pub fn take_pending_potion_flash(&mut self) -> Option<PotionFlashPlayback> {
        self.pending_potion_flash.take()
    }

    pub fn apply_combat_party_invisibility_potion(&mut self, target_index: usize) -> bool {
        if target_index >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(actor) = self.combat_actors.get_mut(target_index) else {
            return false;
        };
        apply_combat_linked_invisibility(actor, &mut self.active_objects)
            .is_some_and(|outcome| outcome.actor_flags_before != outcome.actor_flags_after)
    }

    pub fn use_moonstone_phase(&mut self, slot_index: Option<usize>) -> MoveOutcome {
        let Some(slot_index) = slot_index else {
            self.message = use_prompt_message();
            return MoveOutcome::Blocked;
        };
        // `inventory.md §7.1`: "`cannot be buried here!\n` answers every
        // rejection cause there is - a scene outside `0x00..0x20`, and any
        // underfoot tile outside the accepted ids `4..10`, `44` and `45`.
        // There is no shipboard-specific message, because the branch never
        // reads the party's transport marker". A scene with no underfoot cell
        // at all - combat, the dungeon band - is outside the accepted scene
        // range too, so it takes the same one literal rather than `Not here!`.
        let Some((scene, z, tile, label)) = self.current_moonstone_bury_context() else {
            // `cleak/u5-engine#44`: this refusal continues the `Item: `
            // prompt's own open row rather than starting a fresh one. A
            // direct `self.message = ...` assignment wraps the literal
            // alone from column zero, which is not where the original's
            // cursor sits - `inventory.md §7.1`'s completion is written at
            // the retained cursor column, "subject to normal wrapping"
            // (`cleak/u5-spec#259`, published `ed0f743`). Routing it
            // through the continuing-row append instead merges it onto the
            // `Item: Moonstone` entry before wrapping, which reproduces the
            // original's `cannot be buried` / `here!` break instead of this
            // engine's previous ` cannot be` / `buried here!`.
            self.open_message_row();
            self.emit_message_line_continuing_row(MOONSTONE_BURY_REFUSAL);
            return MoveOutcome::Blocked;
        };
        if !moonstone_bury_scene_allowed(scene) || !moonstone_bury_tile_allowed(tile) {
            self.open_message_row();
            self.emit_message_line_continuing_row(MOONSTONE_BURY_REFUSAL);
            return MoveOutcome::Blocked;
        }

        let removed_pickup = self.clear_moonstone_pickups(slot_index);
        self.moonstone_slots[slot_index] = MoonstoneGateSlot {
            scene,
            x: self.player.x as u8,
            y: self.player.y as u8,
            z,
        };
        if removed_pickup {
            self.mark_visibility_dirty();
        }
        self.advance_turn();
        // `inventory.md §7.1`, published for `cleak/u5-spec#262`: the label
        // `Moonstone_` "is printed before any test, is exactly ten characters
        // ending in a space, and carries no line feed; each completion line
        // carries its own". The echoed picker row supplies the label, so this
        // handler contributes only the completion, opening with the space that
        // makes it continue that row.
        self.diagnostics.push(format!(
            "buried Moonstone phase {} at {label} ({}, {})",
            slot_index + 1,
            self.player.x,
            self.player.y
        ));
        self.message = MOONSTONE_BURY_SUCCESS.to_string();
        MoveOutcome::Used
    }

    pub fn current_moonstone_bury_context(&self) -> Option<(u8, u8, u8, String)> {
        // `inventory.md §7`: "dungeon and combat scenes are outside the
        // band", so a combat arena has no bury context whatever map it was
        // entered from.
        if self.combat_active {
            return None;
        }
        match self.area {
            Area::World { plane } => Some((
                0,
                plane.save_floor() as u8,
                self.grid[world_cell_index(self.player.x, self.player.y)],
                plane.key().to_string(),
            )),
            Area::Town { scene, floor } => Some((
                scene.byte,
                floor as u8,
                self.grid[self.player.y * 32 + self.player.x],
                scene.key(),
            )),
            Area::Dungeon { .. } => None,
        }
    }

    /// `magic.md §5` steps 1 and 3-6 / `magic.md §7`: the live C-Cast
    /// dispatcher gate. Returns `Some(outcome)` when the cast was refused
    /// and the caller must stop, `None` when the spell handler should run.
    ///
    /// The gate decision itself is [`cast_dispatcher_gate`], the crate's one
    /// implementation of the spec's ordering; this method supplies the state
    /// it reads and applies the resource debits its outcome describes:
    ///
    /// - **Step 1, active-player resolve.** A missing or unconscious caster
    ///   aborts before any gate.
    /// - **Gate 5, scene.** `magic.md §7`: "the scene gate runs before
    ///   charge consumption, so `Not here!` does not spend a charge". It
    ///   does cost time: `magic.md §5` step 3 (corrected 2026-10-25,
    ///   u5-spec#300 Q14, `RETRACTIONS.md` R604) - "this rejection reports
    ///   acted ... So in overworld and town play the rejection costs the
    ///   ordinary turn". The withdrawn text said it consumed no time.
    /// - **Gate 6, charges.** `None mixed!` when the counter is zero;
    ///   otherwise the charge is spent immediately.
    /// - **Gate 7, mana.** `M.P. too low!` with the charge already gone.
    /// - **Gate 8, level.** `M.P. too low!` with charge *and* mana gone.
    ///
    /// `mana_cost` is the spell's circle: `catalogs/spell-list.md §1`
    /// publishes `mana_cost(id) = circle(id)` and
    /// `minimum_level(id) = circle(id)`, so the mana gate and the level gate
    /// compare against the same number. The circle is re-derived from
    /// `spell_index` here rather than trusted from the caller, so the level
    /// gate is a level-vs-circle test by construction.
    pub fn cast_spell_resource_gate(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
    ) -> Option<MoveOutcome> {
        // `magic.md §5`: the dispatcher already ran these gates before the
        // handler (and before any handler prompt); see
        // [`PlayState::cast_resources_spent`].
        if self.cast_resources_spent {
            return None;
        }
        // Inside the arena the cast pipeline names its caster by combat
        // descriptor (the combat-actor lookups need it); the mana, level and
        // consciousness belong to the member that descriptor seats
        // (`combat.md §5`; `magic.md §5` step 1). A packed arena put a later
        // member at a lower descriptor index, so the gate read the dead
        // member ahead of him and the cast ended silently (playtest S05).
        let caster_index = if self.combat_active {
            self.combat_roster_slot_for_actor_slot(caster_index)
                .unwrap_or(caster_index)
        } else {
            caster_index
        };
        let Some(caster) = self.party.get(caster_index).copied() else {
            // Out-of-range caster: the `Player: ` prompt bounds the index.
            self.message.clear();
            return Some(MoveOutcome::Blocked);
        };
        if !caster.conscious() {
            // Not a state a keystroke can produce; see the other guards.
            self.message.clear();
            return Some(MoveOutcome::Blocked);
        }
        let Some(circle) = spell_circle_for(spell_index as u8) else {
            self.message = "No effect!".to_string();
            return Some(MoveOutcome::Blocked);
        };
        debug_assert_eq!(
            mana_cost, circle,
            "cast_spell_resource_gate: caller passed mana cost {mana_cost} for spell \
             {spell_index}, but catalogs/spell-list.md §1 publishes mana_cost(id) = \
             circle(id) = {circle}",
        );

        let outcome = cast_dispatcher_gate(
            self.spell_allowed_in_current_cast_context(spell_index),
            self.spell_charges[spell_index],
            caster.mana,
            caster.level,
            circle,
        );
        if outcome.consumed_charge() {
            self.spell_charges[spell_index] = self.spell_charges[spell_index].saturating_sub(1);
        }
        if outcome.consumed_mana() {
            self.party[caster_index].mana = self.party[caster_index].mana.saturating_sub(circle);
        }
        if outcome.plays_failure_glissando() {
            // `magic.md §7` result table: the glissando follows the
            // rejection text; `None mixed!` is silent.
            self.emit_sound_effect(crate::audio::SoundEffect::CastFailure);
        }
        match outcome {
            CastGateOutcome::Cast => None,
            // `magic.md §5` step 3 (u5-spec#300 Q14, R604): the scene
            // rejection "reports acted", so outside combat it spends the
            // ordinary turn. Inside the arena the combat refusal rules
            // govern and are left as they were.
            CastGateOutcome::NotHere => {
                self.emit_cast_gate_refusal(outcome);
                if !self.combat_active {
                    self.advance_turn();
                }
                Some(MoveOutcome::Blocked)
            }
            // The charges rejection keeps this crate's existing no-turn
            // behaviour (outside this change's `magic.md` scope).
            CastGateOutcome::NoneMixed => {
                self.emit_cast_gate_refusal(outcome);
                Some(MoveOutcome::Blocked)
            }
            CastGateOutcome::ManaTooLowChargeOnly | CastGateOutcome::LevelTooLowChargeAndMana => {
                self.emit_cast_gate_refusal(outcome);
                self.advance_turn();
                Some(MoveOutcome::Blocked)
            }
        }
    }

    /// Write a cast-gate rejection, with the blank row the arena needs after
    /// it.
    ///
    /// `combat.md §8.1`: the turn handler "emits the line feed itself,
    /// unconditionally, between printing the banner and reading the command
    /// byte", so the blank row the original shows between a result and the
    /// next `Avatar, armed with...:` banner is only there when the result
    /// closed its own row. Outside combat the next command cycle's leading
    /// feed supplies it, which is why the surface refusals need nothing.
    ///
    /// A trailing newline on the slot will not do it: the log trims one as it
    /// wraps the text into rows, so the row has to be asked for.
    ///
    /// Measured 2026-09-12 (`qa/paired/combat-ready-armour.tsv`, beat
    /// `invisibility`): the original has a blank row under `None mixed!` and
    /// this engine ran the banner straight on, after which the two sides read
    /// different keys and the rest of the scenario drifted.
    pub(crate) fn emit_cast_gate_refusal(&mut self, outcome: CastGateOutcome) {
        let text = outcome.message().to_string();
        if self.combat_active {
            self.emit_message_line(text);
            self.push_explicit_blank_message_entry();
            // Set explicitly for this caller: the blank just pushed is the
            // row the banner's leading feed closes, so one blank row stands
            // between the refusal and the banner. G11's name-commit clear
            // left this false and the arena drew two (rc5 regression,
            // `combat-ready-armour/invisibility`).
            self.combat_transcript_row_open = true;
            self.message_row_open_mid_line = false;
            return;
        }
        self.message = text;
    }

    pub fn apply_gate_travel(
        &mut self,
        game_dir: &Path,
        phase: usize,
        target: PlayTarget,
        floor: i8,
        start: (usize, usize),
    ) -> io::Result<()> {
        let prior_sound_serial = self.sound_effect_serial;
        // `overworld.md §4`/§5 step 1: a warp within one plane keeps the
        // chunks the old and new windows share, so a gate the refresh had
        // already stamped on the destination cell is still live there; any
        // other arrival cell was read fresh from disk and is grass until the
        // next world tick (playtest P23-01).
        let arrival_gate_carried = match (self.area, target) {
            (Area::World { plane }, PlayTarget::World(target_plane)) if plane == target_plane => {
                let idx = world_cell_index(start.0, start.1);
                self.natural_moongate_live_cells.contains(&idx)
                    && self.grid.get(idx).copied() == Some(NATURAL_MOONGATE_TERRAIN_TILE)
            }
            _ => false,
        };
        self.cache_current_world_overlay();
        // `overworld.md §9.2`: from the overworld "the warp writes the outdoor
        // object table to the plane's canonical `.OOL` while Stage B still
        // holds the marker at zero", so a save taken after a cross-plane
        // warp still holds what was parked on the plane left behind
        // (playtest L04).
        self.write_current_world_plane_object_table(game_dir, true)?;
        let previous_turn = self.turn;
        // `overworld.md §9.2` (issue #299 Q26): "Transport therefore
        // survives a transit unchanged. The vehicle in use is not a separate
        // object - it is the party's transport marker - so a party on a horse
        // or a magic carpet arrives mounted or airborne", a town destination
        // included, after which `town-mode.md §15.1` governs the visit as it
        // does an `E`-Enter entry. "A dungeon destination cannot arise"; a
        // dungeon target keeps this engine's on-foot interior.
        let arrival_transport = if matches!(target, PlayTarget::Dungeon(_)) {
            TransportState::Foot
        } else {
            self.player.transport
        };
        let mut options = PlayOptions {
            target,
            floor,
            start: Some(start),
            clock: self.clock,
            food: self.food,
            gold: self.gold,
            keys: self.keys,
            gems: self.gems,
            climbing_gear: self.climbing_gear,
            special_items: self.special_items,
            party: self.party.clone(),
            party_names: self.party_names.clone(),
            party_experience: self.party_experience.clone(),
            party_stay_counters: self.party_stay_counters.clone(),
            party_strengths: self.party_strengths.clone(),
            party_combat_defense: self.party_combat_defense.clone(),
            party_intelligence: self.party_intelligence.clone(),
            party_equipment: self.party_equipment.clone(),
            party_roster: self.synced_party_roster(),
            equipment_stock: self.equipment_stock,
            spell_charges: self.spell_charges,
            scroll_stock: self.scroll_stock,
            potion_stock: self.potion_stock,
            reagents: self.reagents,
            rare_reagent_harvest_days: self.rare_reagent_harvest_days,
            fixed_hidden_treasure_found: self.fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: self.fixed_hidden_treasure_daily_day,
            dungeon_room_clear_bitmap: self.dungeon_room_clear_bitmap,
            saved_dungeon_working_buffer: None,
            moonstone_slots: self.moonstone_slots,
            shadowlord_hideouts: self.shadowlord_hideouts,
            // `formats/saved-gam.md §9.1` `0x0325` (u5-spec#300 Q3, R585):
            // "Nothing clears the byte: leaving the keep, the actor going
            // away with the scene reload, ... leave it alone".
            summoned_shadowlord: self.summoned_shadowlord,
            removed_town_npc_flags: self.removed_town_npc_flags.clone(),
            talk_branch_flags: self.talk_branch_flags.clone(),
            shrine_ordained_mask: self.shrine_ordained_mask,
            shrine_codex_mask: self.shrine_codex_mask,
            word_of_power_seal_flags: self.word_of_power_seal_flags,
            shrine_ruin_flags: self.shrine_ruin_flags,
            moral_standing: self.moral_standing,
            toll_progress: self.toll_progress,
            cleanup_previous_hour: self.cleanup_previous_hour,
            twelve_hour_audio_repeats: self.twelve_hour_audio_repeats,
            cached_moon_glyph_bytes: self.cached_moon_glyph_bytes,
            ambient_light: self.ambient_light,
            // `overworld.md §9.1` (spec HEAD c00bf63): the
            // gate-presence counter survives scene changes.
            natural_moongate_counter: self.natural_moongate_counter,
            // `animation.md §9`/`§12.1`: the driver-side animation layer is
            // never reset and "survives scene changes, save loads, and
            // everything else short of reloading the asset". Carry the live
            // phases into the rebuilt state so water, fountains, banners and
            // clocks do not snap back to phase zero on area entry.
            animation_asset_buffer: self.animation_asset_buffer(),
            avatar_stats: self.avatar_stats,
            torches: self.torches,
            torch_counter: self.torch_counter,
            light_spell_counter: self.light_spell_counter,
            wind: self.wind,
            wind_save_byte: self.wind_save_byte,
            time_stop_counter: self.time_stop_counter,
            active_effect_tag: self.active_effect_tag,
            active_effect_counter: self.active_effect_counter,
            fortunes_of_war: self.fortunes_of_war,
            camp_cooldown: self.camp_cooldown,
            camp_month_cookie: self.camp_month_cookie,
            // A rebuilt area is not a load: no world tick is owed bare.
            world_tick_gate: WORLD_TICK_GATE_STEADY,
            active_player: self.active_player,
            active_player_save_byte: self.active_player_save_byte,
            combat_round_counter: self.combat_round_counter,
            combat_interference_sources: self.combat_interference_sources,
            // `overworld.md §9.2`: "Nothing about ... party transport ...
            // gates it" and "The party's transport marker is restored on
            // every path that ran the transition"; `magic.md` refuses Gate
            // Travel only shipboard. An outdoor warp therefore keeps the
            // mounted horse or the carpet, which `active-objects.md §10`'s
            // overworld entry re-syncs into slot zero (playtest F18).
            transport: arrival_transport,
            facing: Some(self.player.facing),
            // Location setup clears only the previous-tile gate.
            door_tracker: self.inert_door_tracker(),
            // The rebuilt state takes the preserved raw marker from
            // `carry_session_settings_from` instead.
            loaded_transport_marker: None,
            // `shops.md §8.7`: "The next overworld entry consumes that state
            // and places the watercraft", once "before the first overworld
            // turn after the purchase". A warp to an overworld slot is such an
            // entry (`magic.md`, Gate travel row), so it delivers a frigate or
            // skiff bought before the cast (playtest P23-02).
            pending_vehicle: match target {
                PlayTarget::World(_) => self.pending_vehicle_save.acquisition(),
                _ => None,
            },
            pending_vehicle_save: self.pending_vehicle_save,
            inn_registry: self.inn_registry.clone(),
            initial_britannia_overlay: self.world_overlays.get(WorldPlane::Britannia),
            debug_enter: self.debug_enter,
            saved_active_objects: None,
            saved_player_aux: None,
            saved_npc_band: None,
            town_npc_mutations: self.town_npc_mutations.clone(),
            save_template_source: self.save_template_source,
        };
        if let PlayTarget::World(plane) = target {
            options.saved_active_objects = self.world_overlays.get(plane);
        }

        let mut next = Self::load_scene(game_dir, options)?;
        next.turn = previous_turn;
        next.world_overlays = self.world_overlays.clone();
        if let PlayTarget::World(plane) = target {
            // `catalogs/quest-graph.md §5`: the shards and the Amulet are
            // placed "by the outdoor setup pass that runs whenever the party
            // is on a non-surface outdoor plane", and `overworld.md §9`'s
            // warp ends in a normal scene entry, so a moonstone transit into
            // the Underworld runs the same pass the falls do (playtest G29).
            next.place_underworld_fixed_objects(plane);
            next.cache_current_world_overlay();
        }
        if matches!(target, PlayTarget::Dungeon(_)) {
            next.force_foot_transport();
        }
        next.sync_player_object();
        next.pending_town_arrest = None;
        next.active_blackthorn = None;
        // `commands.md §8.1`: a command "never prints tile ids,
        // coordinates, active-object slot numbers, terrain-class names".
        // This named a scene key and a cell, which is three of the four.
        // Nothing in `overworld.md`'s Gate Travel contract gives the
        // transit a line of its own - `audio.md §8.3` gives it a *cue*,
        // and the arrival is what the player sees - so the slot is left
        // as the caller set it.
        next.push_diagnostic(format!(
            "Gate Travel phase {phase} -> {} at ({}, {}).",
            target.key(),
            start.0,
            start.1
        ));
        // `audio.md §2` keeps one serial speaker, and the frontend reads it
        // through a monotonic serial. A scene rebuild constructs `next` from
        // scratch, so its history is numbered from 1 and collides with this
        // state's early serials. Carry the outgoing history across and
        // re-serialize the destination's own entry effects on top of it:
        // clearing here used to drop every cue emitted before the transition
        // — the transit envelope among them — while the serial kept counting,
        // so the frontend advanced past serials that had no history entry and
        // the cue was silently lost.
        let entry_effects = next.sound_effects_after(0);
        next.sound_effect_serial = prior_sound_serial;
        next.sound_effect_history = std::mem::take(&mut self.sound_effect_history);
        for effect in entry_effects {
            next.emit_sound_effect(effect);
        }
        // A scene rebuild constructs `next` from scratch, so every frontend
        // presentation flag defaults off. `pace_combat_presentations` is set
        // once by the graphical shell at bootstrap: dropping it here left a
        // fight entered after any location or gate transition resolving a
        // whole sixteen-actor round inside one host frame, with the paced
        // presentation path silently unreachable for the rest of the session.
        next.pace_combat_presentations = self.pace_combat_presentations;
        next.harness_shortcut_keys = self.harness_shortcut_keys;
        // `inventory.md §4.4` (u5-spec#296): the stats-panel window's inverse
        // bit is display state, not scene state - "Turns, movement and
        // messages do not clear it ... It flips back only when a later picker
        // session leaves odd parity again." A scene rebuild is none of those,
        // so a leaked highlight keeps the panel inverted across the entry.
        next.stats_panel_window_inverse = self.stats_panel_window_inverse;
        // `input.md §6`: "Three things write the setting and nothing else
        // does" - Ctrl+B, the combat toggle and the free-text reader's
        // save/restore - and `audio.md §3` resets the sound setting only for
        // a new process. A scene rebuild is neither, so both carry across
        // (playtest N15).
        next.carry_session_settings_from(self);
        // The message window is display state too: nothing in
        // `overworld.md §9.2`'s transit or the Gate Travel warp clears it,
        // and the rebuild used to arrive with an empty window (playtest
        // E25). Carry the log, its cursor and its open-row flag across as
        // they stood, the way a location entry does.
        next.message_transcript = std::mem::take(&mut self.message_transcript);
        next.message_transcript_revision = self.message_transcript_revision;
        next.message_flushed = std::mem::take(&mut self.message_flushed);
        next.message = std::mem::take(&mut self.message);
        next.message_window_top_offset = self.message_window_top_offset;
        next.message_row_open_mid_line = self.message_row_open_mid_line;
        next.pending_command_echo = None;
        // `overworld.md §5` (draw order items 4 and 5) and §9: the entry
        // pass stamps no gate, so a warp that lands on a gate cell it did
        // not carry across gives the player one command before the §9.2
        // hook can fire. This engine's entry refresh has already stamped
        // it, so hold the hook back for that command (playtest P23-01).
        next.natural_moongate_hook_held_for_warp_arrival = !arrival_gate_carried
            && next.current_live_natural_moongate_cell_index().is_some_and(|idx| {
                next.grid.get(idx).copied() == Some(NATURAL_MOONGATE_TERRAIN_TILE)
            });
        *self = next;
        Ok(())
    }

    pub fn turn_dungeon(&mut self, clockwise: bool) -> MoveOutcome {
        let Area::Dungeon { scene, level } = self.area else {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        let next = if clockwise {
            self.player.facing.turn_right_cardinal()
        } else {
            self.player.facing.turn_left_cardinal()
        };
        let Some(next) = next else {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        // `dungeon-mode.md §8.1`: "Turn refused | `Not in doorway!\n`" - it
        // "fires when the party's **own** cell is the point-blank door
        // class", the `0xE?` heavy door of `§6.4` (the one door class that
        // can be stood in and sets the point-blank flag). The facing is
        // left unchanged.
        if self.dungeon_cell(level, self.player.x, self.player.y) >> 4 == 0xE {
            self.message = DUNGEON_MOVEMENT_NOT_IN_DOORWAY_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }
        self.player.facing = next;
        // `audio.md §7.4`: a turn's redraw is a footstep tick (S25), and
        // `dungeon-mode.md §8.1` has the turn run the shared redraw, which
        // paints the new view first (playtest P19-06).
        self.run_dungeon_shared_redraw();
        self.advance_turn();
        // `dungeon-mode.md §14`: a turn prints its verb and nothing else -
        // "`Turn left\n` / `Turn right\n`" - and the handler itself only
        // "decrement[s] the facing byte ... Status row updates; the
        // first-person view is repainted on the next loop iteration". The
        // verb echo is emitted by the input path, so this handler adds no
        // line of its own; it used to compose a `Turned to face East on
        // DEC (Deceit) level 0.` sentence that the original never prints,
        // and which a paired capture showed landing twice.
        // `cleak/u5-engine#5`.
        let _ = (scene, level);
        self.message.clear();
        MoveOutcome::Moved
    }

    pub fn look_dungeon(&mut self) -> MoveOutcome {
        self.look_dungeon_with_drink(None, None)
    }

    pub fn look_dungeon_with_drink(
        &mut self,
        drink: Option<bool>,
        party_index: Option<usize>,
    ) -> MoveOutcome {
        self.look_dungeon_with_focus(drink, party_index, DungeonLookFocus::Ahead)
    }

    pub fn look_dungeon_with_focus(
        &mut self,
        drink: Option<bool>,
        party_index: Option<usize>,
        focus: DungeonLookFocus,
    ) -> MoveOutcome {
        let Area::Dungeon { scene, level } = self.area else {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        if !self.has_personal_light() {
            // `RETRACTIONS.md` R323: the stored line breaks after the colon.
            self.message = DUNGEON_LOOK_DARKNESS_REFUSAL.to_string();
            return MoveOutcome::Observed;
        }
        let (x, y) = self.dungeon_look_focus_coord(focus);

        let tile = self.dungeon_cell(level, x, y);
        // `dungeon-mode.md §2`: the flavour-three `0xC?` corpse line becomes
        // the software-pirate line "on a one-in-255 draw".
        let flavour = scene.presentation_flavour();
        let pirate = dungeon_look_description_byte(tile) >> 4 == 0xC
            && flavour == DungeonPresentationFlavour::FlavourByte3
            && self.random_mod_u8(255) == 0;
        let description = dungeon_look_description_with_flavour(tile, flavour, pirate);
        if (tile >> 4) == 0x5 {
            // `dungeon-mode.md §12` step 6: the fountain class prints the
            // ordinary look line and then runs the drink question, whose
            // literals are the published `Will you drink?` / `No.` /
            // `Yes.  Gulp!` group rather than a composed sentence.
            let look_line = format!("{DUNGEON_LOOK_PREAMBLE}{description}.\n");
            self.message = match drink {
                None => {
                    let member_index = party_index.unwrap_or(0);
                    self.emit_message_line(look_line);
                    return self.start_dungeon_fountain_drink_prompt(member_index, focus);
                }
                Some(false) => {
                    format!("{look_line}{DUNGEON_FOUNTAIN_DRINK_PROMPT}{DUNGEON_FOUNTAIN_DECLINED}")
                }
                Some(true) => {
                    let member_index = party_index.unwrap_or(0);
                    let effect = self
                        .apply_dungeon_fountain_effect(member_index, tile)
                        .unwrap_or_default();
                    format!(
                        "{look_line}{DUNGEON_FOUNTAIN_DRINK_PROMPT}{DUNGEON_FOUNTAIN_ACCEPTED}{effect}"
                    )
                }
            };
            return if drink == Some(false) {
                MoveOutcome::PromptDeclined
            } else {
                MoveOutcome::Observed
            };
        }

        self.message = format!(
            "{DUNGEON_LOOK_PREAMBLE}{}",
            crate::tile_helpers::dungeon_look_sentence(description)
        );
        // `dungeon-mode.md §12`: each class line ends "in one line feed", so
        // the Look closes its row. The slot is taken verbatim and its text
        // carries no feed, so the row state is stated here: a multi-member
        // Look had left it open from the `Dir-` prompt, and the next
        // line-feed-led line - `§13.2`'s `Exit to ` - spent its feed closing
        // that row and lost its blank (playtest F17).
        self.close_message_row();
        MoveOutcome::Observed
    }

    /// Answer a fountain's drink prompt that is already on screen.
    ///
    /// [`Self::look_dungeon_with_focus`] composes the whole block - the
    /// look line, the question and the answer - because the `L` command
    /// can carry its drink answer inline, and then all three are printed
    /// by one call. The prompt path is not that: `You see:`,
    /// `a fountain.` and `Will you drink?` are already in the transcript
    /// when the `Y` or `N` arrives, so re-composing the block prints them
    /// a second time. Measured 2026-09-19
    /// (`qa/paired/dungeon-fountain-heal.tsv`): the original answers with
    /// `Yes.  Gulp!` and the effect line alone, on the rows below the open
    /// question, where this engine printed the look line and the question
    /// again above them.
    pub fn answer_dungeon_fountain_drink_prompt(
        &mut self,
        drink: bool,
        party_index: usize,
        focus: DungeonLookFocus,
    ) -> MoveOutcome {
        let Area::Dungeon { level, .. } = self.area else {
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        if !drink {
            self.message = DUNGEON_FOUNTAIN_DECLINED.to_string();
            return MoveOutcome::PromptDeclined;
        }
        let (x, y) = self.dungeon_look_focus_coord(focus);
        let tile = self.dungeon_cell(level, x, y);
        let effect = self
            .apply_dungeon_fountain_effect(party_index, tile)
            .unwrap_or_default();
        self.message = format!("{DUNGEON_FOUNTAIN_ACCEPTED}{effect}");
        // `dungeon-mode.md §12`: "L-Look retains the resident dispatcher's
        // acted result ... Look has no special time exemption." An acted
        // command bumps the action counter, and the turn's upkeep - the
        // `time.md §5` one-point poison tick among it - runs behind it.
        //
        // Measured 2026-09-19 (`qa/paired/dungeon-fountain-poison.tsv`,
        // beat `poisoned`): the member the poison fountain has just
        // poisoned reads one hit point lower on the original than here,
        // in the same frame that prints `Poisoned!`. That point is the
        // drink's own turn.
        //
        // That turn is the dungeon loop's acted charge, not one this answer
        // takes for itself: the command epilogue charges it once the prompt
        // has closed and runs the `§4` step 6 post-action pass behind it,
        // exactly as on the `No.` arm. Charging it here left the epilogue
        // nothing to settle, so the post-action pass never ran and the
        // upkeep's `Starving!` waited under the next command (playtest
        // P19-02).
        if self.dungeon_acted_turn_before.is_none() {
            self.advance_turn();
        }
        MoveOutcome::Observed
    }

    pub fn dungeon_look_focus_coord(&self, focus: DungeonLookFocus) -> (usize, usize) {
        let direction = match focus {
            DungeonLookFocus::Ahead => Some(self.player.facing),
            DungeonLookFocus::Right => self.player.facing.turn_right_cardinal(),
            DungeonLookFocus::Left => self.player.facing.turn_left_cardinal(),
            DungeonLookFocus::Here => None,
        };
        let (dx, dy) = direction.map(Direction::delta).unwrap_or((0, 0));
        (
            (self.player.x as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as usize,
            (self.player.y as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as usize,
        )
    }

    pub fn apply_dungeon_fountain_effect(
        &mut self,
        member_index: usize,
        tile: u8,
    ) -> Option<String> {
        let subtype = tile & 0x0f;
        match subtype {
            0 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let before = member.status;
                member.status = b'G';
                let _ = (slot, before);
                Some(DUNGEON_FOUNTAIN_CURED.to_string())
            }
            1 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let (before, after) = member.heal_to_max();
                let _ = (slot, before, after);
                Some(DUNGEON_FOUNTAIN_HEALED.to_string())
            }
            2 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                member.status = b'P';
                let _ = slot;
                Some(DUNGEON_FOUNTAIN_POISONED.to_string())
            }
            _ => {
                let damage = self.dungeon_fountain_damage_roll();
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let applied = member.apply_damage(damage);
                let _ = (slot, applied);
                Some(DUNGEON_FOUNTAIN_BAD_TASTE.to_string())
            }
        }
    }

    pub fn view_gem(&mut self) -> MoveOutcome {
        if self.gems == 0 {
            self.message = VIEW_NO_GEM_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }

        // `view.md §4`: the overlay "waits for a keypress" inside the
        // command, through the shared modal wait that "emits neither a line
        // feed nor a prompt marker while it waits". The `View_a_gem!\n` echo's
        // own feed opened the row that key is read on, so - as for the death
        // vision - neither `text-output.md §10.4`'s blank nor the next cycle's
        // marker is drawn until it is dismissed (playtest P13-11).
        match self.area {
            Area::Dungeon { scene, level } => {
                self.gems = self.gems.saturating_sub(1);
                let title = format!(
                    "Dungeon view of {} ({}) level {} ({} gem(s) remain; 22x22 flood map)",
                    scene.key(),
                    scene.name(),
                    dungeon_display_level(level),
                    self.gems
                );
                let text_map = self.dungeon_vision_map(level);
                self.active_view_overlay = Some(ViewOverlay {
                    title: title.clone(),
                    text_map: text_map.clone(),
                    kind: ViewOverlayKind::Dungeon { level },
                    mode: ViewOverlayMode::GemView,
                });
                self.message.clear();
                self.view_overlay_row_opened_by_result = true;
                MoveOutcome::Observed
            }
            Area::Town { scene, floor } => {
                self.gems = self.gems.saturating_sub(1);
                let title = format!(
                    "Gem view of {} floor {} ({} gem(s) remain; 32x32 class map)",
                    scene.key(),
                    floor,
                    self.gems
                );
                let text_map = self.surface_view_map();
                self.active_view_overlay = Some(ViewOverlay {
                    title: title.clone(),
                    text_map: text_map.clone(),
                    kind: ViewOverlayKind::Surface,
                    mode: ViewOverlayMode::GemView,
                });
                self.message.clear();
                self.view_overlay_row_opened_by_result = true;
                MoveOutcome::Observed
            }
            Area::World { plane } => {
                self.gems = self.gems.saturating_sub(1);
                let title = format!(
                    "Gem view of {} at ({}, {}) ({} gem(s) remain; 32x32 class map)",
                    plane.key(),
                    self.player.x,
                    self.player.y,
                    self.gems
                );
                let text_map = self.surface_view_map();
                self.active_view_overlay = Some(ViewOverlay {
                    title: title.clone(),
                    text_map: text_map.clone(),
                    kind: ViewOverlayKind::Surface,
                    mode: ViewOverlayMode::GemView,
                });
                self.message.clear();
                self.view_overlay_row_opened_by_result = true;
                MoveOutcome::Observed
            }
        }
    }

    pub fn clear_active_view_overlay(&mut self) {
        self.active_view_overlay = None;
        self.view_overlay_idle_polls = 0;
        self.view_overlay_row_opened_by_result = false;
        self.message.clear();
    }

    pub fn render_active_view_overlay(&self, depth: TileGraphicsDepth) -> Option<TileViewport> {
        self.render_active_view_overlay_with_fonts(depth, None)
    }

    /// [`PlayState::render_active_view_overlay`] with the two fixed fonts
    /// the dungeon V-View draws its glyphs from (`view.md §6.2`). A caller
    /// without them gets the engine's vector stand-ins.
    pub fn render_active_view_overlay_with_fonts(
        &self,
        depth: TileGraphicsDepth,
        fonts: Option<DungeonViewFonts<'_>>,
    ) -> Option<TileViewport> {
        let overlay = self.active_view_overlay.as_ref()?;
        match overlay.kind {
            ViewOverlayKind::Surface => {
                Some(self.render_surface_view_overlay_for_mode(depth, overlay.mode))
            }
            ViewOverlayKind::Sky(sky) => {
                Some(render_sky_overlay(depth, &sky, self.shadowlord_hideouts))
            }
            ViewOverlayKind::Dungeon { level } => {
                Some(self.render_dungeon_view_overlay_with_fonts(level, depth, fonts))
            }
        }
    }

    pub fn render_surface_view_overlay(&self, depth: TileGraphicsDepth) -> TileViewport {
        self.render_surface_view_overlay_for_mode(depth, ViewOverlayMode::GemView)
    }

    pub fn render_surface_view_overlay_for_mode(
        &self,
        depth: TileGraphicsDepth,
        mode: ViewOverlayMode,
    ) -> TileViewport {
        let cells = LOCAL_VIEW_OVERLAY_SIDE;
        let scale = LOCAL_VIEW_CELL_PIXEL_SCALE;
        let width = cells * scale;
        let mut viewport = TileViewport {
            depth,
            cells_wide: cells,
            cells_high: cells,
            width,
            height: width,
            pixels: vec![0; width * width],
        };
        for cell_y in 0..cells {
            for cell_x in 0..cells {
                let tile = self.surface_view_tile_at(cell_x, cell_y);
                let class = surface_view_class(tile);
                draw_surface_view_cell(
                    &mut viewport,
                    cell_x,
                    cell_y,
                    scale,
                    class,
                    tile,
                    false,
                    mode,
                );
            }
        }
        // The party marker goes where the party is. On the overworld window
        // that is the centre cell by construction; on a town floor, whose
        // overlay is the floor itself, it is the party's own coordinate.
        let (marker_x, marker_y) = match self.area {
            Area::Town { .. } => (
                (self.player.x).min(cells - 1),
                (self.player.y).min(cells - 1),
            ),
            _ => (cells / 2, cells / 2),
        };
        if surface_view_marker_visible(self.view_overlay_idle_polls) {
            draw_surface_view_cell(&mut viewport, marker_x, marker_y, scale, 0, 0, true, mode);
        }
        viewport
    }

    /// One idle poll of the modal local view's key wait (`view.md §4`,
    /// and §3's dismissal loop that "re-draws the party marker every fourth
    /// idle pass"). The frontend calls this once per wait pump; with no
    /// surface overlay up the count resets so the next view starts afresh.
    pub fn poll_view_overlay_idle(&mut self) {
        if matches!(
            self.active_view_overlay.as_ref().map(|overlay| overlay.kind),
            Some(ViewOverlayKind::Surface)
        ) {
            self.view_overlay_idle_polls = self.view_overlay_idle_polls.wrapping_add(1);
        } else {
            self.view_overlay_idle_polls = 0;
        }
    }

    pub fn render_surface_view_class_cell_for_mode(
        depth: TileGraphicsDepth,
        class: u8,
        tile: u8,
        player_marker: bool,
        mode: ViewOverlayMode,
    ) -> TileViewport {
        let scale = LOCAL_VIEW_CELL_PIXEL_SCALE;
        let mut viewport = TileViewport {
            depth,
            cells_wide: 1,
            cells_high: 1,
            width: scale,
            height: scale,
            pixels: vec![0; scale * scale],
        };
        draw_surface_view_cell(&mut viewport, 0, 0, scale, class, tile, player_marker, mode);
        viewport
    }

    /// The cell of the map a local-view overlay cell samples.
    ///
    /// A town floor **is** thirty-two by thirty-two, so `view.md` §4's
    /// "thirty-two-by-thirty-two local-area overlay" is the whole floor and
    /// the overlay cell is the map cell. Centring it on the party instead
    /// walks off the map whenever the party is not at `(16, 16)`, and the
    /// off-map cells draw nothing.
    ///
    /// Measured 2026-09-18 (`qa/paired/town-death-vision.tsv`, beats `first`
    /// and `second`): the original's overlay content fills `(32,32)..
    /// (159,159)` - the published rectangle, 128 by 128 - and this engine's
    /// filled `(60,60)..(159,159)`, exactly 28 pixels short on each leading
    /// edge. The party was at `(9, 9)`, so seven cells of the window fell off
    /// the left and top at four pixels each: 28.
    ///
    /// The overworld keeps the window. It is 256 by 256, so a local view of
    /// it is a window by necessity, and the party is at its centre.
    fn surface_view_tile_at(&self, cell_x: usize, cell_y: usize) -> u8 {
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let side = LOCAL_VIEW_OVERLAY_SIDE as isize;
        let x = px - side / 2 + cell_x as isize;
        let y = py - side / 2 + cell_y as isize;
        match self.area {
            Area::Town { .. } => {
                let (x, y) = (cell_x as isize, cell_y as isize);
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    return 0;
                }
                if let Some(object) = self.object_at_current_floor(x as usize, y as usize) {
                    object.tile
                } else {
                    let tile = self.grid[y as usize * 32 + x as usize];
                    self.animation.resolve_static_tile(tile)
                }
            }
            Area::World { .. } => {
                let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                if let Some(object) = self.world_object_at(wx, wy) {
                    object.tile
                } else {
                    let tile = self.grid[world_cell_index(wx, wy)];
                    self.animation.resolve_static_tile(tile)
                }
            }
            Area::Dungeon { .. } => 0,
        }
    }

    pub fn render_dungeon_view_overlay(&self, level: u8, depth: TileGraphicsDepth) -> TileViewport {
        self.render_dungeon_view_overlay_for_mode(level, depth, ViewOverlayMode::GemView)
    }

    /// `view.md §6.3`: "the value being read is the display adapter
    /// identifier, not a peer-spell flag. The dungeon map renderer has no
    /// peer-spell branch." The mode is accepted so callers can stay uniform
    /// with the surface overlay, but every mode paints the same dungeon map
    /// for a given adapter `depth`.
    pub fn render_dungeon_view_overlay_for_mode(
        &self,
        level: u8,
        depth: TileGraphicsDepth,
        _mode: ViewOverlayMode,
    ) -> TileViewport {
        self.render_dungeon_view_overlay_with_fonts(level, depth, None)
    }

    /// `view.md §6.2` / `dungeon-mode.md §12.3`: with the text and runic
    /// fonts supplied, every font class is blitted from its selected 8x8
    /// one-bit glyph, opaquely; only the fountain and the energy field
    /// stay vector drawings.
    pub fn render_dungeon_view_overlay_with_fonts(
        &self,
        level: u8,
        depth: TileGraphicsDepth,
        fonts: Option<DungeonViewFonts<'_>>,
    ) -> TileViewport {
        let cells = DUNGEON_GEM_VIEW_GRID_SIDE;
        let scale = DUNGEON_GEM_VIEW_CELL_PIXELS;
        let width = cells * scale;
        let mut viewport = TileViewport {
            depth,
            cells_wide: cells,
            cells_high: cells,
            width,
            height: width,
            pixels: vec![0; width * width],
        };
        let glyphs = self.dungeon_vision_glyphs(level);
        for cell_y in 0..cells {
            for cell_x in 0..cells {
                let index = cell_y * cells + cell_x;
                draw_dungeon_view_glyph(
                    &mut viewport,
                    cell_x,
                    cell_y,
                    scale,
                    glyphs[index],
                    fonts,
                );
            }
        }
        viewport
    }

    /// See [`PlayState::render_dungeon_view_overlay_for_mode`]: the dungeon
    /// map renderer has no peer-spell branch, so `_mode` does not select a
    /// pen.
    pub fn render_dungeon_view_glyph_cell_for_mode(
        depth: TileGraphicsDepth,
        glyph: Option<DungeonMinimapGlyph>,
        _mode: ViewOverlayMode,
    ) -> TileViewport {
        let scale = DUNGEON_GEM_VIEW_CELL_PIXELS;
        let mut viewport = TileViewport {
            depth,
            cells_wide: 1,
            cells_high: 1,
            width: scale,
            height: scale,
            pixels: vec![0; scale * scale],
        };
        draw_dungeon_view_glyph(&mut viewport, 0, 0, scale, glyph, None);
        viewport
    }

    pub fn dungeon_vision_glyphs(&self, level: u8) -> Vec<Option<DungeonMinimapGlyph>> {
        let side = DUNGEON_GEM_VIEW_GRID_SIDE;
        let (party_cell_x, party_cell_y) = DUNGEON_GEM_VIEW_PARTY_CELL;
        // The grid side is even, so the party cell is not a centre:
        // offsets run `-party_cell` through `side - party_cell - 1`,
        // i.e. eleven cells left/above and ten right/below.
        let min_dx = -(party_cell_x as isize);
        let max_dx = (side - party_cell_x - 1) as isize;
        let min_dy = -(party_cell_y as isize);
        let max_dy = (side - party_cell_y - 1) as isize;
        let mut visible = vec![false; side * side];
        let mut queue = VecDeque::with_capacity(DUNGEON_GEM_VIEW_FRONTIER_CAPACITY);

        let party_index = party_cell_y * side + party_cell_x;
        visible[party_index] = true;
        queue.push_back((0isize, 0isize));

        while let Some((sx, sy)) = queue.pop_front() {
            let world_x = (self.player.x as isize + sx).rem_euclid(DUNGEON_SIDE as isize) as usize;
            let world_y = (self.player.y as isize + sy).rem_euclid(DUNGEON_SIDE as isize) as usize;
            if (sx != 0 || sy != 0)
                && !dungeon_minimap_expands(self.dungeon_cell(level, world_x, world_y))
            {
                continue;
            }

            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let next_x = sx + dx;
                    let next_y = sy + dy;
                    if next_x < min_dx || next_x > max_dx || next_y < min_dy || next_y > max_dy {
                        continue;
                    }
                    let scratch_x = (next_x + party_cell_x as isize) as usize;
                    let scratch_y = (next_y + party_cell_y as isize) as usize;
                    let index = scratch_y * side + scratch_x;
                    if !visible[index] {
                        visible[index] = true;
                        queue.push_back((next_x, next_y));
                        debug_assert!(
                            queue.len() <= DUNGEON_GEM_VIEW_FRONTIER_CAPACITY,
                            "dungeon-mode.md §12.2 bounds the V-View flood frontier at                              {DUNGEON_GEM_VIEW_FRONTIER_CAPACITY} pending cells;                              reached {}",
                            queue.len()
                        );
                    }
                }
            }
        }

        let mut glyphs = vec![None; side * side];
        glyphs[party_index] = Some(DUNGEON_MINIMAP_PARTY_GLYPH);
        for scratch_y in 0..side {
            for scratch_x in 0..side {
                let index = scratch_y * side + scratch_x;
                if !visible[index] || index == party_index {
                    continue;
                }
                let dx = scratch_x as isize - party_cell_x as isize;
                let dy = scratch_y as isize - party_cell_y as isize;
                let world_x =
                    (self.player.x as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as usize;
                let world_y =
                    (self.player.y as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as usize;
                glyphs[index] = dungeon_minimap_glyph(self.dungeon_cell(level, world_x, world_y));
            }
        }
        glyphs
    }

    pub fn dungeon_vision_map(&self, level: u8) -> String {
        let side = DUNGEON_GEM_VIEW_GRID_SIDE;
        let glyphs = self.dungeon_vision_glyphs(level);
        let mut out = String::new();
        for scratch_y in 0..side {
            for scratch_x in 0..side {
                let index = scratch_y * side + scratch_x;
                out.push(render_dungeon_minimap_glyph_code(glyphs[index]));
            }
            out.push('\n');
        }
        out
    }

    pub fn dungeon_forward_view(&self, level: u8) -> String {
        let mut out = String::from("First-person dungeon view:\n");
        out.push_str(&format!(
            "0: here {}\n",
            self.describe_dungeon_offset(level, 0, 0)
        ));

        let Some(left) = self.player.facing.turn_left_cardinal() else {
            out.push_str("view requires a cardinal facing direction\n");
            return out;
        };
        let Some(right) = self.player.facing.turn_right_cardinal() else {
            out.push_str("view requires a cardinal facing direction\n");
            return out;
        };

        let (fdx, fdy) = self.player.facing.delta();
        let (ldx, ldy) = left.delta();
        let (rdx, rdy) = right.delta();
        let mut obscured = false;
        for band in 1..=DUNGEON_VIEW_DEPTH {
            if obscured {
                out.push_str(&format!("{band}: obscured by front wall\n"));
                continue;
            }

            let band = band as isize;
            let ahead_dx = fdx * band;
            let ahead_dy = fdy * band;
            out.push_str(&format!(
                "{band}: ahead {}; left {}; right {}\n",
                self.describe_dungeon_offset(level, ahead_dx, ahead_dy),
                self.describe_dungeon_offset(level, ahead_dx + ldx, ahead_dy + ldy),
                self.describe_dungeon_offset(level, ahead_dx + rdx, ahead_dy + rdy)
            ));
            obscured = self.dungeon_offset_blocks_view(level, ahead_dx, ahead_dy);
        }

        out
    }

    pub fn describe_dungeon_offset(&self, level: u8, dx: isize, dy: isize) -> String {
        let x = self.player.x as isize + dx;
        let y = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&x) || !(0..DUNGEON_SIDE as isize).contains(&y) {
            "the dungeon boundary".to_string()
        } else {
            dungeon_look_description(self.dungeon_cell(level, x as usize, y as usize)).to_string()
        }
    }

    pub fn dungeon_offset_blocks_view(&self, level: u8, dx: isize, dy: isize) -> bool {
        let x = self.player.x as isize + dx;
        let y = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&x) || !(0..DUNGEON_SIDE as isize).contains(&y) {
            return true;
        }

        !is_dungeon_walkable(self.dungeon_cell(level, x as usize, y as usize))
    }

    pub fn surface_view_map(&self) -> String {
        let mut out = String::new();
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let side = 32isize;
        let origin_x = px - side / 2;
        let origin_y = py - side / 2;
        match self.area {
            Area::Town { .. } => {
                // The floor is thirty-two by thirty-two, so the overlay is
                // the floor. See `surface_view_tile_at`, where centring this
                // on the party walked off the map.
                let (origin_x, origin_y) = (0isize, 0isize);
                for y in origin_y..origin_y + side {
                    for x in origin_x..origin_x + side {
                        if x == px && y == py {
                            out.push('@');
                        } else if (0..32).contains(&x) && (0..32).contains(&y) {
                            if let Some(object) =
                                self.object_at_current_floor(x as usize, y as usize)
                            {
                                out.push(render_surface_view_class(surface_view_class(
                                    object.tile,
                                )));
                            } else {
                                let tile = self.grid[y as usize * 32 + x as usize];
                                let tile = self.animation.resolve_static_tile(tile);
                                out.push(render_surface_view_class(surface_view_class(tile)));
                            }
                        } else {
                            out.push(' ');
                        }
                    }
                    out.push('\n');
                }
            }
            Area::World { .. } => {
                for y in origin_y..origin_y + side {
                    for x in origin_x..origin_x + side {
                        let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                        let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                        if wx == self.player.x && wy == self.player.y {
                            out.push('@');
                        } else if let Some(object) = self.world_object_at(wx, wy) {
                            out.push(render_surface_view_class(surface_view_class(object.tile)));
                        } else {
                            let tile = self.grid[world_cell_index(wx, wy)];
                            let tile = self.animation.resolve_static_tile(tile);
                            out.push(render_surface_view_class(surface_view_class(tile)));
                        }
                    }
                    out.push('\n');
                }
            }
            Area::Dungeon { .. } => {}
        }
        out
    }

    #[cfg(test)]
    pub fn look_facing(&mut self) -> MoveOutcome {
        self.look_facing_with_table(None)
    }

    pub fn look_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.look_direction_with_game_dir(self.player.facing, game_dir)
    }

    pub fn look_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let look_table = load_look_table(game_dir)?;
        let outcome =
            self.look_direction_with_resources(direction, Some(&look_table), Some(game_dir))?;
        self.charge_look_command_turn();
        Ok(outcome)
    }

    /// `RETRACTIONS.md` R463 (`cleak/u5-spec#261`): "The incorrect premise was
    /// **\"Look does not consume a turn.\"** The original resident dispatcher
    /// returns acted for L-Look, including direction cancellation."
    ///
    /// The acted result is what reaches the ordinary town epilogue - "the
    /// one-minute clock call, underfoot/status work, then the eligible
    /// object/NPC passes and contact routing" - and that contact pass is why
    /// the Minoc guard can raise its charity demand "after the first eligible
    /// Look while the party remains stationary", which is the cadence
    /// difference `#261` was filed for.
    ///
    /// `view.md §3`: the fountain and wishing-well handlers keep the command
    /// open on their own prompts - the drinker selector, the coin question,
    /// the wish - and only then "return to the command loop", and
    /// `town-mode.md §7` runs the epilogue "When the dispatcher returns". So
    /// a Look that opened one of those prompts leaves the charge pending for
    /// the prompt's close; charging it here ran the minute, the starvation
    /// warning and the walkers under the open question (playtest P19-03).
    fn charge_look_command_turn(&mut self) {
        if matches!(self.area, Area::Town { .. } | Area::World { .. }) {
            let prompt_open = self.active_wishing_well.is_some()
                || self.active_party_selector.is_some_and(|session| {
                    matches!(session.target, PartySelectorTarget::FountainDrink { .. })
                });
            if prompt_open {
                self.pending_look_prompt_turn = true;
            } else {
                self.advance_turn();
            }
        }
    }

    /// Spend the Look turn [`Self::charge_look_command_turn`] left pending,
    /// once the fountain or well prompt it opened has closed.
    pub fn settle_pending_look_prompt_turn(&mut self, game_dir: &Path) -> io::Result<()> {
        let prompt_open = self.active_wishing_well.is_some()
            || self.active_party_selector.is_some_and(|session| {
                matches!(session.target, PartySelectorTarget::FountainDrink { .. })
            });
        if prompt_open || !std::mem::take(&mut self.pending_look_prompt_turn) {
            return Ok(());
        }
        let turn_before = self.turn;
        self.advance_turn();
        self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn look_facing_with_table(&mut self, look_table: Option<&LookTable>) -> MoveOutcome {
        self.look_direction_with_resources(self.player.facing, look_table, None)
            .expect("look without a game dir cannot perform file-backed look context")
    }

    #[cfg(test)]
    pub fn look_direction_with_table(
        &mut self,
        direction: Direction,
        look_table: Option<&LookTable>,
    ) -> MoveOutcome {
        self.look_direction_with_resources(direction, look_table, None)
            .expect("look without a game dir cannot perform file-backed look context")
    }

    pub fn look_facing_with_resources(
        &mut self,
        look_table: Option<&LookTable>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.look_direction_with_resources(self.player.facing, look_table, game_dir)
    }

    pub fn look_direction_with_resources(
        &mut self,
        direction: Direction,
        look_table: Option<&LookTable>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Dungeon { .. } => Ok(self.look_dungeon()),
            Area::Town { .. } => {
                let (dx, dy) = direction.delta();
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if !self.surface_look_target_visible(x, y) {
                    self.message.clear();
                    return Ok(MoveOutcome::Observed);
                }
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    // `view.md §3` step 3 reads the target through the shared
                    // live-map tile query, and `town-mode.md §15`: asked for
                    // "any coordinate outside 0..31 on either axis while a
                    // location floor is loaded, the accessor returns the
                    // loaded floor's (31,31) cell instead of failing ...
                    // Every caller that asks the accessor for an
                    // out-of-range coordinate ... takes the same
                    // substitution". This arm used to print nothing at all
                    // (playtest K19). No town actor has an off-grid
                    // coordinate, so the per-map object row cannot match;
                    // the terrain description of the substituted cell is
                    // what Look reports.
                    let tile = self.grid[TOWN_GRID_SIDE * TOWN_GRID_SIDE - 1];
                    let description = self.look_description(tile, look_table, TOWN_GRID_SIDE - 1);
                    self.present_look_terrain_description(tile, &description);
                    return Ok(MoveOutcome::Observed);
                }
                let x = x as usize;
                let y = y as usize;
                // `view.md §3` entry-dispatch row 2: "Live tile `0x29`
                // (the crystal-sphere tile)" is tested immediately after
                // the row-1 visibility gate and before the row-3 shared
                // preamble and the row-4 per-map object lookup, because
                // "the vision case is decided before anything is
                // printed". The tested byte is the **live terrain
                // layer**, never an active-object descriptor.
                if death_vision_look_tile(self.grid[y * 32 + x]) {
                    return Ok(self.apply_death_vision_look(x, y));
                }
                // `view.md §3` entry-dispatch row 4: "the per-map object case
                // wins over the sign case when both would match", and a
                // per-map object always prints its object description.
                if let Some(object) = self.blocking_object_at(x, y) {
                    // `cleak/u5-spec#194` (black-box): the stock game prints
                    // `Thou dost see` and the description on the next row.
                    self.message = if look_table.is_some() {
                        format!(
                            "{LOOK_RESULT_PREFIX}\n{}",
                            self.look_object_description(object.tile, look_table)
                        )
                    } else {
                        format!("{LOOK_RESULT_PREFIX}\nan actor tile {}", object.tile)
                    };
                    return Ok(MoveOutcome::Observed);
                }
                // Row 5: the sign path is selected by the live terrain tile,
                // and only then is `SIGNS.DAT` consulted.
                if sign_or_wanted_poster_object_class(self.grid[y * 32 + x]) {
                    let sign =
                        self.sign_message_at_for_current_area(game_dir, y as u8, x as u8)?;
                    // `view.md §5` (audit A012): a `0x0D` holds the rest of
                    // the sign behind a key.
                    self.present_sign_message(sign);
                    return Ok(MoveOutcome::Observed);
                }
                let (x, y) = self.resolve_look_redirect_tiles(x, y);
                let tile = self.grid[y * 32 + x];
                // `view.md §3` terrain-description path row 2: live tile
                // `0x59` is a **telescope** and enters the §4.2 sky renderer
                // instead of printing any description text. The row sits
                // above the wishing well (row 3) and the fountains (row 4),
                // and the table is not scene-scoped: "Only three telescopes
                // are placed in shipped data, all indoors: in Moonglow, in
                // Skara Brae, and in West Britanny", so the town-family arm
                // is the only arm that can ever reach it in ordinary play.
                if tile == TELESCOPE_LOOK_TRIGGER_TILE {
                    self.look_through_telescope();
                    return Ok(MoveOutcome::Observed);
                }
                if surface_wishing_well_look_tile(tile) {
                    // Measured: the well takes the ordinary `Thou dost see`
                    // preamble and then its **own** description, not the
                    // `LOOK2.DAT` record for the tile. See
                    // [`WISHING_WELL_LOOK_DESCRIPTION`].
                    return Ok(
                        self.start_wishing_well_prompt(direction, WISHING_WELL_LOOK_DESCRIPTION)
                    );
                }
                if surface_town_fountain_look_tile(tile) {
                    return Ok(self.start_surface_fountain_drink_prompt(direction));
                }
                let description = self.look_description(tile, look_table, x);
                self.present_look_terrain_description(tile, &description);
                Ok(MoveOutcome::Observed)
            }
            Area::World { .. } => {
                let (dx, dy) = direction.delta();
                let raw_x = self.player.x as isize + dx;
                let raw_y = self.player.y as isize + dy;
                if !self.surface_look_target_visible(raw_x, raw_y) {
                    self.message.clear();
                    return Ok(MoveOutcome::Observed);
                }
                let x = raw_x.rem_euclid(WORLD_SIDE as isize) as usize;
                let y = raw_y.rem_euclid(WORLD_SIDE as isize) as usize;
                // `view.md §3` entry-dispatch row 2 — see the town arm
                // above. The live terrain byte is tested ahead of the
                // per-map object row.
                if death_vision_look_tile(self.grid[world_cell_index(x, y)]) {
                    return Ok(self.apply_death_vision_look(x, y));
                }
                if let Some(object) = self.world_object_at(x, y) {
                    self.message = if look_table.is_some() {
                        format!(
                            "{LOOK_RESULT_PREFIX}\n{}",
                            self.look_object_description(object.tile, look_table)
                        )
                    } else {
                        format!("{LOOK_RESULT_PREFIX}\nan object tile {}", object.tile)
                    };
                    return Ok(MoveOutcome::Observed);
                }
                // `view.md §3` entry-dispatch row 5, see the town arm.
                if sign_or_wanted_poster_object_class(self.grid[world_cell_index(x, y)]) {
                    let sign =
                        self.sign_message_at_for_current_area(game_dir, y as u8, x as u8)?;
                    self.present_sign_message(sign);
                    return Ok(MoveOutcome::Observed);
                }
                let (x, y) = self.resolve_look_redirect_tiles(x, y);
                let tile = self.grid[world_cell_index(x, y)];
                if tile == TELESCOPE_LOOK_TRIGGER_TILE {
                    self.look_through_telescope();
                    return Ok(MoveOutcome::Observed);
                }
                if surface_wishing_well_look_tile(tile) {
                    return Ok(
                        self.start_wishing_well_prompt(direction, WISHING_WELL_LOOK_DESCRIPTION)
                    );
                }
                if surface_town_fountain_look_tile(tile) {
                    return Ok(self.start_surface_fountain_drink_prompt(direction));
                }
                let description = self.look_description(tile, look_table, x);
                // `view.md §3`: "the shared \"thou dost see\" preamble is
                // printed here" with the description on the next row, and
                // `commands.md §8.1` has the look family never print "tile
                // ids, coordinates, active-object slot numbers, terrain-class
                // names". The town and dungeon arms above already do this; the
                // overworld arm was still emitting the terminal harness's
                // inline form, `You see: mountains at (240, 72) on
                // BRITANNIA.`, where a paired capture of Deceit's exit reads
                // `Thou dost see` / `mountains` (`cleak/u5-engine#5`).
                self.present_look_terrain_description(tile, &description);
                Ok(MoveOutcome::Observed)
            }
        }
    }

    /// Print a terrain-path look result (`view.md §3` row 5) under the shared
    /// `Thou dost see` preamble.
    ///
    /// `view.md §3`: "The shared line-spacing cleanup belongs to the
    /// plain-description path only ... A tile that matched an appender row
    /// returns as soon as its suffix is printed and never reaches the
    /// cleanup", even when the row appends nothing - "So a clock tile emits
    /// its description and time and nothing further." That cleanup is the
    /// feed closing the look row (`text-output.md §10.4`), and no stored
    /// description carries one, so an appender result leaves its row open
    /// and the next poll's leading feed closes it with no blank row between
    /// (playtest P16-03). The row state is stated here, not inferred from
    /// the text.
    fn present_look_terrain_description(&mut self, tile: u8, description: &str) {
        self.message = format!("{LOOK_RESULT_PREFIX}\n{description}");
        // `text-output.md §6`: "a row-filling string still ends at column
        // zero of the following row, so the next command prompt still
        // spends one blank row" - an appender result whose last row is
        // exactly full (`dungeon Covetous`) is closed by the emitter's own
        // wrap, not left open (playtest P20-05).
        if look_terrain_tile_takes_appender_row(tile)
            && crate::wrapped_final_row_columns(description) < crate::MESSAGE_WINDOW_WIDTH
        {
            self.flush_message_slot();
            self.leave_last_transcript_row_open();
        }
    }

    /// `view.md §3` terrain-description path row 1: live tiles `0xE0`,
    /// `0xE1` and `0xE2` redirect the look target one cell north, east and
    /// west, re-read it and restart the terrain table; "The redirect can
    /// chain, so a run of redirect tiles resolves to one final cell". The
    /// overworld wraps like every other overworld step. The spec does not
    /// say what a town redirect off the 32-by-32 grid does, so that step
    /// (and a cycle, which no shipped map is known to hold) stops on the
    /// current cell rather than inventing a target.
    pub(crate) fn resolve_look_redirect_tiles(&self, x: usize, y: usize) -> (usize, usize) {
        let (mut x, mut y) = (x, y);
        for _ in 0..(WORLD_SIDE * WORLD_SIDE) {
            let (dx, dy): (isize, isize) = match self.surface_tile_at(x, y) {
                LOOK_REDIRECT_NORTH_TILE => (0, -1),
                LOOK_REDIRECT_EAST_TILE => (1, 0),
                LOOK_REDIRECT_WEST_TILE => (-1, 0),
                _ => break,
            };
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            match self.area {
                Area::World { .. } => {
                    x = nx.rem_euclid(WORLD_SIDE as isize) as usize;
                    y = ny.rem_euclid(WORLD_SIDE as isize) as usize;
                }
                _ => {
                    if !(0..32).contains(&nx) || !(0..32).contains(&ny) {
                        break;
                    }
                    x = nx as usize;
                    y = ny as usize;
                }
            }
        }
        (x, y)
    }

    pub fn look_surface_fountain_with_drinker(
        &mut self,
        direction: Direction,
        member_index: usize,
    ) -> MoveOutcome {
        let Some(tile) = self.surface_look_target_tile(direction) else {
            // An internal invariant: the drink prompt only opens on a
            // fountain the look already matched, and the picker bounds the
            // member index, so no shipped path reaches this. The original
            // prints nothing here, so neither does this.
            self.message.clear();
            return MoveOutcome::Observed;
        };
        if !surface_town_fountain_look_tile(tile) {
            self.message.clear();
            return MoveOutcome::Observed;
        }

        let Some(member) = self.party.get(member_index).copied() else {
            self.message.clear();
            return MoveOutcome::Observed;
        };
        let Some(status) = character_status_for_byte(member.status) else {
            self.message.clear();
            return MoveOutcome::Observed;
        };
        if town_fountain_drink_accepts(status) {
            // The prompt carries no trailing space, so the reply opens its
            // own line rather than completing the prompt's.
            self.message = FOUNTAIN_DRINK_REFRESHED.to_string();
        } else {
            // `view.md §3`, since `cleak/u5-spec#197` published the
            // literals: "A Dead or Asleep member prints
            // `Incapacitated!` and a blank row". The engine printed
            // nothing here while the line was unpublished.
            self.message = FOUNTAIN_DRINK_INCAPACITATED.to_string();
        }
        MoveOutcome::Observed
    }

    pub fn resolve_wishing_well_wish(
        &mut self,
        direction: Direction,
        typed_wish: &str,
    ) -> MoveOutcome {
        let Some((x, y)) = self.surface_look_target_position(direction) else {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        };
        let tile = self.surface_tile_at(x, y);
        if !surface_wishing_well_look_tile(tile) {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        }
        let Some(wish) = wishing_well_wish(typed_wish) else {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        };
        let grant_scene = match self.area {
            Area::Town { scene, .. } => wishing_well_grant_scene(scene.byte),
            _ => false,
        };
        if !grant_scene {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        }
        let _ = wish;
        // `view.md §3`: a matching wish in a granting scene prints `Poof!`
        // and then creates the horse "one cell east of the well caller's X
        // coordinate at the caller's Y and floor". The spec has no occupancy
        // refusal, and the allocator evicts rather than fails
        // (`active-objects.md §4`), so an occupied cell no longer turns the
        // grant into `No effect...` (playtest P21-05). A cell off the map's
        // east edge, which no shipped well reaches, creates nothing but keeps
        // the `Poof!` line.
        let grant_x = self.player.x.saturating_add(1);
        let grant_y = self.player.y;
        if grant_x < 32
            && let Some(z) = self.current_floor()
        {
            let _ = self
                .allocate_active_object_slot(horse_purchase_active_object(grant_x, grant_y, z));
        }

        self.mark_visibility_dirty();
        // Measured: the typed wish echoes upper-cased on its own row and
        // the grant prints [`WISHING_WELL_GRANT_LINE`] under it.
        self.message = format!(
            "{}\n{WISHING_WELL_GRANT_LINE}",
            typed_wish.to_ascii_uppercase()
        );
        MoveOutcome::Observed
    }

    /// `view.md §3` entry-dispatch row 2: the death-vision case "uses the
    /// shared conditional acting-member selector; an abort retains that
    /// selector's feedback but adds no vision result". An override or a
    /// single eligible member is picked silently; two or more eligible
    /// members produce `Player: ` (the earlier no-prompt reading is
    /// withdrawn, R408).
    pub fn apply_death_vision_look(&mut self, x: usize, y: usize) -> MoveOutcome {
        match self.shared_acting_member_selection(false) {
            ActingMemberSelection::Selected(slot) => {
                self.apply_death_vision_look_for_member(x, y, slot)
            }
            ActingMemberSelection::Prompt => {
                self.start_party_selector(PartySelectorTarget::DeathVision { x, y })
            }
            ActingMemberSelection::NoneAble => {
                // The selector's own feedback, as the Search site answers
                // the same state; no vision result follows.
                self.message = super::chunk_03::PARTY_SELECTOR_CANCEL_REPLY.to_string();
                MoveOutcome::Observed
            }
        }
    }

    pub fn apply_death_vision_look_for_member(
        &mut self,
        x: usize,
        y: usize,
        member_index: usize,
    ) -> MoveOutcome {
        let intelligence = self
            .party_intelligence
            .get(member_index)
            .copied()
            .unwrap_or(self.avatar_stats.intelligence);
        let roll = self.random_range_u8(DEATH_VISION_ROLL_LOW, DEATH_VISION_ROLL_HIGH);
        if intelligence > roll {
            let text_map = self.surface_view_map();
            self.active_view_overlay = Some(ViewOverlay {
                title: String::new(),
                text_map,
                kind: ViewOverlayKind::Surface,
                mode: ViewOverlayMode::SurfaceLook,
            });
            self.message = DEATH_VISION_STRANGE_LINE.to_string();
            // The literal's own feed opened the row the modal view reads its
            // dismissing key on. Measured 2026-09-12
            // (`qa/paired/town-death-vision.tsv`, beats `first` and `second`,
            // both reading `offset+1`): the original's marker row sits
            // directly under `Strange vision!`, and this engine spent §10.4's
            // blank between them.
            self.view_overlay_row_opened_by_result = true;
        } else {
            let _ = (x, y);
            self.active_view_overlay = None;
            self.view_overlay_row_opened_by_result = false;
            self.message = DEATH_VISION_DEATH_LINE.to_string();
            // `view.md §3`: "Otherwise print `Death vision!\n` and apply
            // **one HP of damage** to that member with the ordinary hit
            // feedback and stats redraw. HP reaching zero sets the member
            // Dead and clears an active-member override naming that
            // member." The shared party-damage helper is that path.
            if member_index < self.party.len() {
                self.apply_shared_party_damage(member_index, 1);
            }
        }
        MoveOutcome::Observed
    }

    fn surface_look_target_tile(&self, direction: Direction) -> Option<u8> {
        let (x, y) = self.surface_look_target_position(direction)?;
        Some(self.surface_tile_at(x, y))
    }

    fn surface_look_target_position(&self, direction: Direction) -> Option<(usize, usize)> {
        // The well and fountain follow-ups re-resolve the cell the Look
        // chose, so they take the same `view.md §3` redirect step.
        let (x, y) = match self.area {
            Area::Town { .. } => self.adjacent_position(direction)?,
            Area::World { .. } => {
                let (dx, dy) = direction.delta();
                let x = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
                let y = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
                (x, y)
            }
            Area::Dungeon { .. } => return None,
        };
        Some(self.resolve_look_redirect_tiles(x, y))
    }

    fn surface_look_target_visible(&self, x: isize, y: isize) -> bool {
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        match self.area {
            Area::Town { .. } => {
                if self.surface_visibility_pitch_dark() {
                    return false;
                }
                self.town_cell_visible_with_light_threshold(
                    px,
                    py,
                    x,
                    y,
                    SURFACE_LOOK_VISIBILITY_RADIUS,
                    self.surface_visibility_light_threshold(),
                )
            }
            Area::World { .. } => {
                if self.world_visibility_pitch_dark() {
                    return false;
                }
                self.world_cell_visible_with_light_threshold(
                    px,
                    py,
                    x,
                    y,
                    SURFACE_LOOK_VISIBILITY_RADIUS,
                    self.world_visibility_light_threshold(),
                )
            }
            Area::Dungeon { .. } => false,
        }
    }

    fn surface_tile_at(&self, x: usize, y: usize) -> u8 {
        match self.area {
            Area::Town { .. } => self.grid[y * 32 + x],
            Area::World { .. } => self.grid[world_cell_index(x, y)],
            Area::Dungeon { .. } => unreachable!("surface helper is not used in dungeon mode"),
        }
    }

    /// `target_x` is the looked-at cell's map X, which selects the `0xDF`
    /// dungeon name (`view.md §3` row 5c).
    pub fn look_description(
        &self,
        tile: u8,
        look_table: Option<&LookTable>,
        target_x: usize,
    ) -> String {
        // `formats/look2-dat.md §5`: "Use the offset table directly; do not
        // derive descriptions from tile or object ranges." A sentinel row
        // prints its own selected string, as the original LOOK2 path does
        // (§4); a missing row substitutes the domain's sentinel (§6).
        let base = look2_selected_string(look_table, tile as usize, 0);

        // `view.md §3` rows 5a-5c "Append" to the printed base description:
        // no separator and no brackets of their own. The shipped LOOK2 base
        // strings for these rows end in their own space, so adding one here
        // printed `dungeon  Doom`, and the clock time was wrapped in
        // parentheses no spec text carries (playtest J02/J07).
        if matches!(tile, 0xfa | 0xfb) {
            format!(
                "{base}{}:{:02} {}",
                self.clock.display_hour(),
                self.clock.minute,
                self.clock.am_pm_suffix()
            )
        } else if tile == ETERNAL_FLAME_LOOK_TILE {
            // `view.md §3` terrain-description row 5b: "Live tile
            // `0xDE` | Append a virtue word chosen by the current
            // scene: scene `30` appends Truth, scene `31` appends Love,
            // scene `32` appends Courage. In any other scene the base
            // description is printed with no appended word." The row is
            // an appender row even when it appends nothing, so it
            // returns here rather than falling through.
            let scene_byte = match self.area {
                Area::Town { scene, .. } => scene.byte,
                _ => SCENE_OVERWORLD,
            };
            match eternal_flame_word_for_scene(scene_byte) {
                Some(word) => format!("{base}{word}"),
                None => base,
            }
        } else if tile == WORD_OF_POWER_SEALED_TILE {
            // `view.md §3` terrain-description row 5c: "Append a dungeon
            // name chosen by the target cell's map X coordinate ... Any
            // other X appends nothing." Keyed on X alone, in every
            // surface/town scene; an appender row even when it appends
            // nothing.
            match sealed_dungeon_name_for_x(target_x) {
                Some(name) => format!("{base}{name}"),
                None => base,
            }
        } else {
            // `formats/look2-dat.md §5` / `view.md §3` rows 5a..5c list the
            // only appenders; there is no shrine appender, and
            // `catalogs/gazetteer.md §7`: "The `0x88..0x8F` band does not
            // encode the eight surface shrines."
            base
        }
    }

    pub fn look_object_description(&self, object_id: u8, look_table: Option<&LookTable>) -> String {
        // `formats/look2-dat.md §3`: the object domain is resolved before
        // indexing, and its sentinel (`x`) is printed as selected, never
        // replaced by a terrain-range class name (§5).
        look2_selected_string(
            look_table,
            LOOK2_DAT_TERRAIN_ENTRIES + object_id as usize,
            LOOK2_DAT_TERRAIN_ENTRIES,
        )
    }

    pub fn sign_message_at(
        &self,
        game_dir: Option<&Path>,
        scene: u8,
        z: u8,
        y: u8,
        x: u8,
    ) -> io::Result<Option<crate::tlk_runner::TlkRenderedText>> {
        if self.yew_wanted_poster_message(scene, z, y, x).is_some() {
            return Ok(Some(yew_wanted_poster_rendered(&self.party_names)));
        }
        let Some(game_dir) = game_dir else {
            return Ok(None);
        };
        let Some(bytes) = read_optional_disk_file(&game_dir.join(SIGNS_DAT_FILE))? else {
            return Ok(None);
        };
        if bytes.len() < SIGNS_DAT_SCENE_DIRECTORY_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{SIGNS_DAT_FILE} too short: {} bytes, need at least {SIGNS_DAT_SCENE_DIRECTORY_BYTES} for the scene directory",
                    bytes.len()
                ),
            ));
        }
        // `formats/signs-dat.md §5` (u5-spec#302 Q3, R623): the 2000-byte
        // window from the scene's directory offset is scanned across later
        // scenes' records, comparing only `z`, `x`, `y`; the first match is
        // printed (step 6, R598). Steps 3 and 7: a zero directory entry, or
        // a window with no match, prints the §6 placard - the clean contract
        // ends the scan at the window's end.
        let Some(record) = find_sign_in_window(&bytes, scene, z, y, x) else {
            return Ok(Some(sign_no_record_placard_rendered()));
        };
        // `formats/signs-dat.md §6`: the stream is "already authored" and
        // a renderer should "[a]void proportional-font wrapping, implicit
        // line breaking". Each authored row is laid out cell by cell and
        // reaches the next row through the per-cell emitter's own edge wrap
        // (`text-output.md §5`); the word-wrap printer broke them mid-row
        // (playtest J05). Each cell keeps the font its byte's high bit
        // selects (§4, R600).
        //
        // §4 step 2 / §9 (u5-spec#314 Q1): each macro byte prints its
        // resident fragment, read from the user's own `DATA.OVL`.
        let fragments = load_sign_macro_fragments(game_dir)?;
        Ok(Some(sign_stream_rows_rendered(
            &sign_payload_rendered_with_fragments(&record.payload, fragments.as_ref()),
        )))
    }

    /// Put a sign into the message window. `formats/signs-dat.md §6`: "Pause
    /// on the embedded pause code" - the sign prints up to its first `0x0D`
    /// and the rest waits behind a key (see [`crate::text_hold`]).
    ///
    /// `view.md §3` entry dispatch: row 3 prints the shared "thou dost see"
    /// preamble for rows 4, 5 and 6 alike, and row 5 then emits "a line
    /// break, then render[s] the sign or poster". The preamble "is a line
    /// feed, `Thou dost see`, then a line feed" and "the sign path's own line
    /// break is a second, separate line feed", so a blank row stands between
    /// `Thou dost see` and the sign rows (u5-spec#300 Q13). No `Sign:` label
    /// is published (playtest J05).
    fn present_sign_message(&mut self, sign: Option<crate::tlk_runner::TlkRenderedText>) {
        let mut rendered =
            crate::tlk_runner::TlkRenderedText::plain(format!("{LOOK_RESULT_PREFIX}\n\n"));
        if let Some(sign) = sign {
            rendered.push_rendered(&sign);
        }
        let (first, rest) = crate::text_hold::split_rendered_at_holds(&rendered);
        let row_open = !first.text.is_empty() && !first.text.ends_with('\n');
        // Commit the first page now, through the same slot flush the turn
        // boundary would run, then give the sign rows back the per-cell font
        // the flush flattened (`formats/signs-dat.md §4`, R600).
        self.message = first.text.clone();
        self.flush_message_slot();
        self.restore_flushed_rows_fonts(&first);
        // `formats/signs-dat.md §4` step 4 and §6: after a record the
        // formatter prints a line feed if the cursor is off column 0, and
        // every placard row carries its own feed, so the sign leaves the
        // cursor at column 0. Set explicitly for this caller.
        self.message_row_open_mid_line = false;
        self.begin_text_hold(rest, row_open);
    }

    /// Re-apply `rendered`'s per-cell fonts to the transcript rows a plain
    /// slot flush has just pushed for it, newest row first, stopping at the
    /// first row whose text does not match.
    fn restore_flushed_rows_fonts(&mut self, rendered: &crate::tlk_runner::TlkRenderedText) {
        let rows: Vec<&[crate::tlk_runner::TlkRenderedGlyph]> = rendered
            .rendered_lines()
            .filter(|row| !row.is_empty())
            .collect();
        let mut entries = self
            .message_transcript
            .iter_mut()
            .rev()
            .filter(|entry| !entry.text.is_empty());
        for row in rows.into_iter().rev() {
            let Some(entry) = entries.next() else {
                break;
            };
            if entry.text.as_bytes() != row.iter().map(|glyph| glyph.byte).collect::<Vec<_>>() {
                break;
            }
            entry.glyphs = row.to_vec();
        }
    }

    fn yew_wanted_poster_message(&self, scene: u8, z: u8, y: u8, x: u8) -> Option<String> {
        if !(scene == 4 && z == 0 && x == 17 && y == 21) {
            return None;
        }
        Some(yew_wanted_poster_rows(&self.party_names).join("\n"))
    }

    pub fn sign_message_at_for_current_area(
        &self,
        game_dir: Option<&Path>,
        y: u8,
        x: u8,
    ) -> io::Result<Option<crate::tlk_runner::TlkRenderedText>> {
        match self.area {
            Area::Town { scene, floor } => {
                self.sign_message_at(game_dir, scene.byte, floor as u8, y, x)
            }
            Area::World { plane } => {
                self.sign_message_at(game_dir, SCENE_OVERWORLD, plane.save_floor() as u8, y, x)
            }
            Area::Dungeon { .. } => Ok(None),
        }
    }

    pub fn talk_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.talk_facing_with_game_dir_and_keyword(game_dir, None)
    }

    pub fn talk_facing_with_game_dir_and_keyword(
        &mut self,
        game_dir: &Path,
        keyword: Option<&str>,
    ) -> io::Result<MoveOutcome> {
        self.talk_direction_with_game_dir_and_keyword(self.player.facing, game_dir, keyword)
    }

    /// `shops.md §2`, in the published order: a **parity gate runs first**,
    /// then the open-for-business gate.
    ///
    /// *Parity* (added 2026-09-12, issue #262, `RETRACTIONS.md` R479): "The
    /// shared dispatcher reaches the shop path only when the NPC's cached
    /// reached waypoint index is **odd**; at an even index it prints the
    /// merchant brush-off below without consulting the open-for-business check
    /// at all." `conversation.md §2` owns the same bit on the guard path.
    ///
    /// *Open for business*: "Both the NPC's cached reached waypoint and the
    /// waypoint selected by the current world hour must be **waypoint 1**."
    ///
    /// "The two rules agree on shipped schedules, where the indices are `0`,
    /// `1` and `2`, but an implementation that runs only the open-for-business
    /// check will diverge on any higher index." They agree at *every* index
    /// here, because both refusals are the same brush-off and the
    /// open-for-business test is equality with an odd number: an even index
    /// fails both, an odd index other than one passes the parity gate and
    /// fails the equality. The parity clause is therefore stated rather than
    /// coded, and this comment is the record of why. `conversation.md §2`'s
    /// behaviour-4-to-1 waypoint rewrite
    /// ([`Self::talk_dispatcher_stand_down`]) runs before both and edits
    /// only the behaviour byte, which neither test reads.
    ///
    /// A slot this engine cannot resolve - no runtime NPC at the cell, which
    /// the harness fixtures produce - is treated as open, so unit fixtures
    /// that seat a trigger without a schedule keep working.
    /// `conversation.md §2` step 5, the dispatcher's first decision: "If that
    /// waypoint's behaviour is the approach-and-converse mode (... value
    /// `4`), the dispatcher first **rewrites that waypoint's behaviour to the
    /// bounded-wander mode** (value `1`, a persisted schedule edit: the guard
    /// stands down for the rest of the period and the edit travels with the
    /// save) and then dispatches on the dialog index." The waypoint is the
    /// reached one, and the rewrite "applies to shopkeepers as well as guards,
    /// before a shop can open or refuse" and before any of the dispatcher's own
    /// lines. `shops.md §2` states the same edit for explicit Talk and
    /// automatic contact alike. Returns whether the rewrite happened, which
    /// skips the guard gate.
    pub(crate) fn talk_dispatcher_stand_down(&mut self, x: usize, y: usize) -> bool {
        let Some(slot) = self.talk_record_at(x, y).map(|npc| npc.slot) else {
            return false;
        };
        let Some(index) = self.npcs.iter().position(|npc| npc.slot == slot) else {
            return false;
        };
        let at = NPC_SCHEDULE_AI_OFFSET + self.npcs[index].cached_wp;
        let Some(byte) = self.npcs[index].schedule.get(at).copied() else {
            return false;
        };
        if npc_ai_behavior(byte) != Some(NpcAiBehavior::ApproachAndAttack) {
            return false;
        }
        self.npcs[index].schedule[at] = 1;
        self.record_town_npc_mutation(index);
        true
    }

    fn shop_keeper_is_open_for_business(&self, x: usize, y: usize) -> bool {
        const SHOP_WORKING_WAYPOINT: usize = 1;
        let Some(npc) = self.talk_record_at(x, y) else {
            return true;
        };
        npc.cached_wp == SHOP_WORKING_WAYPOINT
            && waypoint_for_hour(&npc.schedule, self.clock.hour) == SHOP_WORKING_WAYPOINT
    }

    pub fn talk_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        self.talk_direction_with_game_dir_and_keyword(direction, game_dir, None)
    }

    pub fn talk_direction_with_game_dir_and_keyword(
        &mut self,
        direction: Direction,
        game_dir: &Path,
        keyword: Option<&str>,
    ) -> io::Result<MoveOutcome> {
        let Area::Town { scene, .. } = self.area else {
            self.message = "Funny, no response!".to_string();
            // `commands.md §3`: `T` does not forward its handler's value, so
            // the outdoor refusal keeps the default "acted" and the overworld
            // runs its per-turn epilogue (playtest J09). The dungeon arm is
            // charged by its own loop.
            if matches!(self.area, Area::World { .. }) && !self.combat_active {
                self.advance_turn();
            }
            return Ok(MoveOutcome::Blocked);
        };
        // `conversation.md §2` step 1 (u5-spec#300 Q12, R596): there is no
        // "can talk now" liveness gate; provisions, party status, combat and
        // conversation state are never tested.
        if let Some((dialog_id, target_x, target_y)) = self.talk_target_in_direction(direction)
            && matches!(
                dialog_id,
                TOWN_NPC_COWERING_DIALOG_ID | TOWN_NPC_BRUSHOFF_DIALOG_ID
            )
        {
            if self.talk_entry_gates_apply()
                && let Some(refusal) =
                    talk_status_tile_refusal(self.talk_status_tile_at(target_x, target_y))
            {
                self.message = format!("\n{refusal}");
                return Ok(self.consume_ordinary_town_talk());
            }
            self.talk_dispatcher_stand_down(target_x, target_y);
            return Ok(self
                .talk_alarm_sentinel_at(dialog_id, target_x, target_y)
                .expect("published alarm sentinel was preclassified"));
        }
        let dialogue = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let raw_blob = parse_tlk_raw(&game_dir.join(format!("{}.TLK", scene.family.stem())))
            .unwrap_or_default();
        self.common_word_dictionary = load_common_word_dictionary_optional(game_dir)?;
        let shoppe_renderer =
            crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir).ok();
        Ok(self.talk_direction_with_dialogue_and_keyword_raw_inner(
            direction,
            &dialogue,
            &raw_blob,
            keyword,
            shoppe_renderer.as_ref(),
            Some(game_dir),
        ))
    }

    pub fn facing_talk_target(&self) -> Option<(u8, usize, usize)> {
        self.talk_target_in_direction(self.player.facing)
    }

    /// `conversation.md §2` step 4: the byte the Talk status-tile gate
    /// compares against `0x9D` (mirror) and `0xAB` (bed).
    ///
    /// "The test object is a map tile, not an NPC sprite. Both ids are
    /// furniture ids in the terrain-description domain of `LOOK2.DAT`, and the
    /// byte comes from the same live-map tile query that movement and Look
    /// use. The gate fires because the NPC's schedule has parked it on a bed
    /// cell or a mirror cell, not because the NPC has a distinct sleeping or
    /// praying appearance. An implementation that stores a per-NPC 'asleep'
    /// flag and tests that instead will diverge."
    ///
    /// The cell is the *resolved* cell — the faced cell, or the cell one step
    /// further along the same direction when the faced cell was talk-through —
    /// which is exactly what [`PlayState::talk_target_in_direction`] returns.
    pub fn talk_status_tile_at(&self, x: usize, y: usize) -> u8 {
        self.grid[y * 32 + x]
    }

    /// The Look description of whatever Talk resolved onto, if the
    /// cached `LOOK2.DAT` table is present. Shares
    /// [`Self::look_object_description`] with the Look command so the two
    /// cannot drift.
    pub fn talk_target_description(&self, x: usize, y: usize) -> Option<String> {
        let table = self.look_table.as_ref()?;
        let object = self.blocking_object_at(x, y)?;
        Some(self.look_object_description(object.tile, Some(table)))
    }

    pub fn talk_target_in_direction(&self, direction: Direction) -> Option<(u8, usize, usize)> {
        let (dx, dy) = direction.delta();
        let x = self.player.x as isize + dx;
        let y = self.player.y as isize + dy;
        if !(0..32).contains(&x) || !(0..32).contains(&y) {
            return None;
        }

        let x = x as usize;
        let y = y as usize;
        if let Some(target) = self.talk_target_at(x, y) {
            return Some(target);
        }
        if !is_talk_through_tile(self.grid[y * 32 + x]) {
            return None;
        }

        let x = x as isize + dx;
        let y = y as isize + dy;
        if !(0..32).contains(&x) || !(0..32).contains(&y) {
            return None;
        }
        self.talk_target_at(x as usize, y as usize)
    }

    /// `conversation.md §2` step 5, "No-owner Talk": "If no occupied,
    /// non-idle NPC record is linked to the faced active object, Talk does not
    /// stop with `Nobody's here!`: it runs the dispatcher with an unrelated
    /// adjacent record that reads as all zero, so the bare `No response!` is
    /// printed for any sprite." An object with no NPC record therefore
    /// resolves with dialog index zero.
    ///
    /// Which record owns the object is published too (added 2026-10-16,
    /// issue #216): "The owning record is the lowest-numbered occupied record
    /// linked to the faced object with a nonzero type and state; the faced
    /// object is the highest-numbered active object at that cell." A record
    /// standing on the cell but linked to another object does not own it -
    /// that is the reporter's Minoc guard (`blackthorn.md §7a`,
    /// `saved-gam.md §12`).
    ///
    /// Engine boundary: a record with no object link at all (only the
    /// harness builds those; town entry links every placed record) is still
    /// found by its position, as before.
    fn talk_target_at(&self, x: usize, y: usize) -> Option<(u8, usize, usize)> {
        if let Some(npc) = self.talk_record_at(x, y) {
            return Some((npc.dialog_id, x, y));
        }
        self.talk_faced_object_slot(x, y).map(|_| (0, x, y))
    }

    /// `conversation.md §2`: "the faced object is the highest-numbered active
    /// object at that cell" (slot 0, the party, is never a Talk target).
    fn talk_faced_object_slot(&self, x: usize, y: usize) -> Option<usize> {
        self.active_objects
            .iter()
            .enumerate()
            .skip(1)
            .rev()
            .find(|(_, object)| self.object_occupies(**object, x, y))
            .map(|(slot, _)| slot)
    }

    /// The NPC record the Talk dispatcher runs with at a resolved cell:
    /// "the lowest-numbered occupied record linked to the faced object with a
    /// nonzero type and state" (`conversation.md §2`, issue #216). `None` when
    /// no record owns the faced object - the dispatcher then reads "an
    /// unrelated adjacent record that reads as all zero", so no waypoint,
    /// guard-sprite, shop or regime test may consult a record merely standing
    /// on the cell. Every later lookup on the Talk path goes through this, so
    /// the dialogue word and the record those tests read are the same one.
    ///
    /// Engine boundary: a record with no object link at all (only the harness
    /// builds those; town entry links every placed record) is still found by
    /// its position, as is any record when no object stands on the cell.
    pub(crate) fn talk_record_at(&self, x: usize, y: usize) -> Option<&RuntimeNpc> {
        let Some(faced) = self.talk_faced_object_slot(x, y) else {
            return self.npc_at_current_floor(x, y);
        };
        self.npcs
            .iter()
            .filter(|npc| {
                npc.active_object == Some(faced)
                    && npc.type_byte != 0
                    && npc.state != NPC_STATE_EMPTY
            })
            .min_by_key(|npc| npc.slot)
            .or_else(|| {
                self.npc_at_current_floor(x, y)
                    .filter(|npc| npc.active_object.is_none())
            })
    }

    #[cfg(test)]
    pub fn talk_facing_with_dialogue(
        &mut self,
        dialogue: &HashMap<u16, Vec<String>>,
    ) -> MoveOutcome {
        self.talk_facing_with_dialogue_and_keyword(dialogue, None)
    }

    pub fn talk_facing_with_dialogue_and_keyword(
        &mut self,
        dialogue: &HashMap<u16, Vec<String>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        self.talk_direction_with_dialogue_and_keyword(self.player.facing, dialogue, keyword)
    }

    /// The legacy dialogue-map entry point. Production talk goes through the
    /// conversation runner (`talk_facing_with_game_dir`); nothing outside the
    /// tests reaches this chain, so the lines it composes are harness output
    /// rather than text the game prints.
    pub fn talk_direction_with_dialogue_and_keyword(
        &mut self,
        direction: Direction,
        dialogue: &HashMap<u16, Vec<String>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        self.talk_direction_with_dialogue_and_keyword_inner(direction, dialogue, keyword)
    }

    pub fn talk_direction_with_dialogue_and_keyword_inner(
        &mut self,
        direction: Direction,
        dialogue: &HashMap<u16, Vec<String>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        if !matches!(self.area, Area::Town { .. }) {
            self.message = "Funny, no response!".to_string();
            return MoveOutcome::Blocked;
        }
        // `conversation.md §2` step 1: no liveness gate (u5-spec#300 Q12, R596).

        let Some((dialog_id, target_x, target_y)) = self.talk_target_in_direction(direction) else {
            self.message = crate::TALK_NOBODY_HERE_LINE.to_string();
            return self.consume_ordinary_town_talk();
        };
        // `conversation.md §2` step 4 status-tile filter: the mirror
        // (`0x9D`) and bed (`0xAB`) gate reads the **live map tile occupying
        // the resolved cell**, not a per-NPC sprite or asleep value.
        if let Some(refusal) =
            talk_status_tile_refusal(self.talk_status_tile_at(target_x, target_y))
        {
            self.message = format!("\n{refusal}");
            return self.consume_ordinary_town_talk();
        }
        let stood_down = self.talk_dispatcher_stand_down(target_x, target_y);
        if let Some(outcome) = self.talk_alarm_sentinel_at(dialog_id, target_x, target_y) {
            return outcome;
        }

        if let Some((_role, _family)) = talk_shop_trigger(dialog_id) {
            // `shops.md §2` open-for-business gate: "Both the NPC's cached
            // reached waypoint and the waypoint selected by the current world
            // hour must be **waypoint 1**." The cache records arrival, so a
            // keeper who has not reached the working waypoint refuses even
            // after opening time, and one still there after closing refuses
            // too. It applies to all eight triggers, horse traders included,
            // and takes precedence over the transport refusal below.
            //
            // Measured 2026-09-07 at North Britanny's stable
            // (`qa/paired/nb-stable.tsv`), which refuses at 13:00 with
            // exactly the published lines. This engine used to open every
            // shop whose trigger byte it saw (`cleak/u5-engine#15`,
            // `cleak/u5-spec#214`).
            if !self.shop_keeper_is_open_for_business(target_x, target_y) {
                self.message = SHOP_CLOSED_REFUSAL.to_string();
                return self.consume_ordinary_town_talk();
            }
            if self.player.transport.is_horse() && dialog_id != 0x83 {
                self.message = SHOP_MOUNTED_REFUSAL.to_string();
                return self.consume_ordinary_town_talk();
            }
            let scene_byte = match self.area {
                Area::Town { scene, .. } => Some(scene.byte),
                _ => None,
            };
            if let Some(session) =
                crate::shop_session::shop_session_for_talk_context(dialog_id, scene_byte)
            {
                // `shops.md §8.C` "Horse traders": no free active-object slot
                // and no suitable adjacent placement cell "both print `The
                // stables are closed.\n` before any greeting, then exit."
                if matches!(
                    &session,
                    crate::shop_session::ActiveShopSession::HorseTrader(_)
                ) && crate::input_dispatch::horse_trader_stables_closed(self)
                {
                    self.message = format!(
                        "\n{}",
                        crate::input_dispatch::HORSE_TRADER_STABLES_CLOSED_LINE
                    );
                    return self.consume_ordinary_town_talk();
                }
                if matches!(&session, crate::shop_session::ActiveShopSession::Tavern(_)) {
                    self.tavern_secondary_drink_count = 0;
                }
                // `shops.md §8.1`: the post-item prompt's suffix reports
                // whether a transaction has completed "in this visit", so the
                // flag is per-visit and resets as the session opens.
                self.arms_transaction_completed = false;
                if matches!(
                    &session,
                    crate::shop_session::ActiveShopSession::Innkeeper(_)
                ) {
                    self.clear_active_effect_slot();
                }
                // `town-mode.md §7` step 4, as on the asset-backed entry
                // below: the Talk turn is spent when the shop arm returns
                // (playtest J16).
                self.pending_talk_shop_turn = !self.talk_entered_by_contact;
                // Both Talk entries go through one greeting builder now. This
                // one is the dialogue-map path and carries no game directory,
                // so it takes the builder's no-assets fallback; the
                // asset-backed path renders the `SHOPPE.DAT` record.
                let message =
                    self.format_talk_shop_opening_message(dialog_id, &session, None, None);
                self.active_shop = Some(session);
                self.message = message;
                return MoveOutcome::Talked;
            }
        }
        // `conversation.md §2` step 5, corrected 2026-09-12 for issue #262
        // (`RETRACTIONS.md` R479): "if the NPC's live sprite is the guard
        // sprite, the NPC answers `The guard offers` on one row and
        // `no response!` on the next, unless *both* its current waypoint index
        // is **odd** *and* its dialog index is non-zero, in which case it
        // dispatches normally." The test is the parity of the reached waypoint
        // index, not equality with waypoint one; the two agree at the indices
        // `0..2` shipped schedules use and diverge above them.
        //
        // The approach-and-converse waypoint behaviour is checked first: that
        // arm stands the guard down and dispatches on the dialog index instead.
        if !stood_down
            && let Some(npc) = self.talk_record_at(target_x, target_y)
            && npc.type_byte == crate::tlk_control_codes::TALK_GUARD_SPRITE
            && !crate::talk_guard_sprite_dispatches(npc.cached_wp, dialog_id as u8)
        {
            self.message = crate::talk_non_speaker_refusal_for_sprite(Some(
                crate::tlk_control_codes::TALK_GUARD_SPRITE,
            ));
            return self.consume_ordinary_town_talk();
        }
        // `conversation.md §2` step 5: the reserved regime index "enters the
        // regime handler of `systems/blackthorn.md` Section 7a from both
        // explicit Talk and automatic conversation contact, **subject to the
        // gates above**" - so the guard gate runs first. Measured 2026-09-12
        // (`qa/paired/blackthorn-palace-password.tsv`, beat `talk`): the
        // original refuses the palace guard with `The guard offers` /
        // `no response!` where this engine opened the password demand.
        if dialog_id == BLACKTHORN_GUARD_DEMAND_DIALOG_ID {
            let consume_turn = !self.talk_entered_by_contact;
            return match self.begin_blackthorn_guard_demand(target_x, target_y, consume_turn, None) {
                Ok(outcome) => outcome,
                Err(_) => MoveOutcome::Blocked,
            };
        }
        if matches!(
            npc_dialog_id_kind(dialog_id),
            NpcDialogIdKind::NoDialogue | NpcDialogIdKind::HighSpecial
        ) {
            // `conversation.md §2`'s guard gate: the guard sprite answers
            // the stored `The guard offers no response!` literal and every
            // other non-speaker answers the bare `No response!`. The line "is
            // not composed from the NPC's Look description", which is what
            // this engine used to do off a single guard capture.
            // A stood-down NPC skipped the guard gate, so its zero word
            // prints the bare line whatever its sprite (§2's decision tree,
            // arm 1).
            self.message = crate::talk_non_speaker_refusal_for_sprite(
                self.talk_record_at(target_x, target_y)
                    .map(|npc| npc.type_byte)
                    .filter(|_| !stood_down),
            );
            return self.consume_ordinary_town_talk();
        }

        let Some(fields) = dialogue.get(&(dialog_id as u16)) else {
            self.message = format!("Dialogue id {dialog_id} is unresolved for this scene."); // audit: not a player-facing line
            return MoveOutcome::Blocked;
        };
        if fields.len() < 3 {
            self.message = format!("Dialogue id {dialog_id} has no complete talk envelope."); // audit: not a player-facing line
            return MoveOutcome::Blocked;
        }

        let name = fields
            .first()
            .filter(|name| !name.is_empty())
            .map(String::as_str)
            .unwrap_or("someone");
        let description = fields
            .get(1)
            .filter(|description| !description.is_empty())
            .map(String::as_str)
            .unwrap_or("no description");
        let greeting = fields
            .get(2)
            .filter(|greeting| !greeting.is_empty())
            .map(String::as_str)
            .unwrap_or("...");

        self.advance_talk_turn();
        if let Some(keyword) = keyword.and_then(non_empty_talk_keyword) {
            if fields.len() < 5 {
                self.message = format!("Dialogue id {dialog_id} has no complete talk envelope."); // audit: not a player-facing line
                return MoveOutcome::Talked;
            }
            // The session's own top-level matcher (`conversation.md §6`):
            // a rebuke word answers the rebuke, not an NPC keyword.
            let response = if crate::conversation_session::tlk_keyword_lookup(None, fields, keyword)
                == crate::conversation_session::TlkKeywordLookup::Rebuke
            {
                crate::TLK_RESERVED_REBUKE_MESSAGE
            } else {
                talk_keyword_response(fields, keyword)
                    .filter(|response| !response.is_empty())
                    .unwrap_or(TLK_NO_KEYWORD_MATCH_MESSAGE)
            };
            let (response, actions) = talk_response_text_and_actions(response);
            self.apply_talk_action_grants(&actions);
            // The same shape the production path prints: the reply alone.
            let _ = name;
            self.message = response.to_string();
        } else {
            let (greeting, actions) = talk_response_text_and_actions(greeting);
            self.apply_talk_action_grants(&actions);
            // `conversation.md §9`, published for `cleak/u5-spec#262`: the
            // entry is "A blank row, `You see_` plus the NPC's Description
            // entry, a blank row, the Greeting in double quotes ..., a blank
            // row, then `Your interest?\n:`". This string-backed fallback runs
            // when a scene has no raw `.TLK` blobs; it printed the description
            // bare, dropped the quote wrapper and stopped before the input row.
            self.message = format!(
                "\n{TLK_OPENING_DESCRIPTION_PREFIX}{description}\n\n\"{greeting}\"\n\n{TLK_KEYWORD_PROMPT}"
            );
        }
        MoveOutcome::Talked
    }

    /// Talk dispatch wrapper that uses the byte-runner against the raw
    /// blob bytes when they are available, falling back to the existing
    /// string-based path otherwise. The richer renderer expands engine
    /// control bytes (printable text, action dispatch, IF/ELSE, GOTO
    /// labels) per `systems/conversation.md` §7 and merges active-scene
    /// branch-flag effects emitted by the stream.
    pub fn talk_facing_with_dialogue_and_keyword_raw(
        &mut self,
        dialogue: &HashMap<u16, Vec<String>>,
        raw_blob: &HashMap<u16, Vec<Vec<u8>>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        self.talk_direction_with_dialogue_and_keyword_raw(
            self.player.facing,
            dialogue,
            raw_blob,
            keyword,
        )
    }

    pub fn talk_direction_with_dialogue_and_keyword_raw(
        &mut self,
        direction: Direction,
        dialogue: &HashMap<u16, Vec<String>>,
        raw_blob: &HashMap<u16, Vec<Vec<u8>>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        self.talk_direction_with_dialogue_and_keyword_raw_inner(
            direction, dialogue, raw_blob, keyword, None, None,
        )
    }

    fn talk_direction_with_dialogue_and_keyword_raw_inner(
        &mut self,
        direction: Direction,
        dialogue: &HashMap<u16, Vec<String>>,
        raw_blob: &HashMap<u16, Vec<Vec<u8>>>,
        keyword: Option<&str>,
        shoppe_renderer: Option<&crate::shoppe_bark::ShoppeTextRenderer>,
        game_dir: Option<&Path>,
    ) -> MoveOutcome {
        if !matches!(self.area, Area::Town { .. }) {
            self.message = "Funny, no response!".to_string();
            return MoveOutcome::Blocked;
        }
        // `conversation.md §2` step 1: no liveness gate (u5-spec#300 Q12, R596).

        let Some((dialog_id, target_x, target_y)) = self.talk_target_in_direction(direction) else {
            self.message = crate::TALK_NOBODY_HERE_LINE.to_string();
            return self.consume_ordinary_town_talk();
        };
        let conversation_npc_slot = self
            .talk_record_at(target_x, target_y)
            .map(|npc| npc.slot);

        // `conversation.md §2` step 4: the status-tile gate reads the live map
        // tile at the resolved cell, the same query movement and Look use.
        if self.talk_entry_gates_apply()
            && let Some(refusal) =
                talk_status_tile_refusal(self.talk_status_tile_at(target_x, target_y))
        {
            self.message = format!("\n{refusal}");
            return self.consume_ordinary_town_talk();
        }
        let stood_down = self.talk_dispatcher_stand_down(target_x, target_y);
        if let Some(outcome) = self.talk_alarm_sentinel_at(dialog_id, target_x, target_y) {
            return outcome;
        }

        if let Some((_role, family)) = talk_shop_trigger(dialog_id) {
            // Same `shops.md §2` gate as the direction path above; this is the
            // keyword-carrying entry the visual shell uses, and leaving it out
            // is why the first fix looked like it had not taken.
            if !self.shop_keeper_is_open_for_business(target_x, target_y) {
                self.message = SHOP_CLOSED_REFUSAL.to_string();
                return self.consume_ordinary_town_talk();
            }
            if self.player.transport.is_horse() && dialog_id != 0x83 {
                self.message = SHOP_MOUNTED_REFUSAL.to_string();
                return self.consume_ordinary_town_talk();
            }
            let scene_byte = match self.area {
                Area::Town { scene, .. } => Some(scene.byte),
                _ => None,
            };
            if let Some(session) =
                crate::shop_session::shop_session_for_talk_context(dialog_id, scene_byte)
            {
                // `shops.md §8.C` "Horse traders": no free active-object slot
                // and no suitable adjacent placement cell "both print `The
                // stables are closed.\n` before any greeting, then exit."
                if matches!(
                    &session,
                    crate::shop_session::ActiveShopSession::HorseTrader(_)
                ) && crate::input_dispatch::horse_trader_stables_closed(self)
                {
                    self.message = format!(
                        "\n{}",
                        crate::input_dispatch::HORSE_TRADER_STABLES_CLOSED_LINE
                    );
                    return self.consume_ordinary_town_talk();
                }
                if matches!(&session, crate::shop_session::ActiveShopSession::Tavern(_)) {
                    self.tavern_secondary_drink_count = 0;
                }
                // `shops.md §8.1`: the post-item prompt's suffix reports
                // whether a transaction has completed "in this visit", so the
                // flag is per-visit and resets as the session opens.
                self.arms_transaction_completed = false;
                if matches!(
                    &session,
                    crate::shop_session::ActiveShopSession::Innkeeper(_)
                ) {
                    self.clear_active_effect_slot();
                }
                // `town-mode.md §7` step 4: the epilogue runs "when the
                // dispatcher returns", and the Talk dispatcher runs the whole
                // shop arm; `shops.md §4.1` reads the hour "fresh from the
                // world clock on every render". So the turn is spent when
                // the session closes, not before the greeting (playtest J16).
                self.pending_talk_shop_turn = !self.talk_entered_by_contact;
                let message = self.format_talk_shop_opening_message(
                    dialog_id,
                    &session,
                    Some(family),
                    shoppe_renderer,
                );
                self.active_shop = Some(session);
                self.message = message;
                return MoveOutcome::Talked;
            }
        }
        // `conversation.md §2` step 5 (`RETRACTIONS.md` R479), the same gate
        // the string-backed path above applies: a guard-sprite NPC answers the
        // two-row refusal unless its reached waypoint index is odd *and* its
        // dialog index is non-zero, and step 5 has the reserved regime index
        // enter Blackthorn's handler "**subject to the gates above**".
        //
        // This is the asset-backed path - the one the game and every paired
        // run take - and it had no such gate at all, so R479 was only ever in
        // force on the fallback. Measured 2026-09-12
        // (`qa/paired/blackthorn-palace-password.tsv`, beat `talk`): the
        // original refuses the palace guard where this engine opened the
        // password demand.
        if !stood_down
            && let Some(npc) = self.talk_record_at(target_x, target_y)
            && npc.type_byte == crate::tlk_control_codes::TALK_GUARD_SPRITE
            && !crate::talk_guard_sprite_dispatches(npc.cached_wp, dialog_id as u8)
        {
            self.message = crate::talk_non_speaker_refusal_for_sprite(Some(
                crate::tlk_control_codes::TALK_GUARD_SPRITE,
            ));
            return self.consume_ordinary_town_talk();
        }
        if dialog_id == BLACKTHORN_GUARD_DEMAND_DIALOG_ID {
            let consume_turn = !self.talk_entered_by_contact;
            // `town-mode.md §14`: explicit `T` is one of the three routings
            // into the same arrest sequence, which "branches on the current
            // location before it prints anything" - so a Badge-less palace
            // refusal here is the audience, not the surrender question. The
            // asset-backed Talk passed no game directory, so the arrest fell
            // back to the challenge (playtest P24-08).
            return match self.begin_blackthorn_guard_demand(
                target_x,
                target_y,
                consume_turn,
                game_dir,
            ) {
                Ok(outcome) => outcome,
                Err(_) => MoveOutcome::Blocked,
            };
        }
        if matches!(
            npc_dialog_id_kind(dialog_id),
            NpcDialogIdKind::NoDialogue | NpcDialogIdKind::HighSpecial
        ) {
            // `conversation.md §2`'s guard gate: the guard sprite answers
            // the stored `The guard offers no response!` literal and every
            // other non-speaker answers the bare `No response!`. The line "is
            // not composed from the NPC's Look description", which is what
            // this engine used to do off a single guard capture.
            // A stood-down NPC skipped the guard gate, so its zero word
            // prints the bare line whatever its sprite (§2's decision tree,
            // arm 1).
            self.message = crate::talk_non_speaker_refusal_for_sprite(
                self.talk_record_at(target_x, target_y)
                    .map(|npc| npc.type_byte)
                    .filter(|_| !stood_down),
            );
            return self.consume_ordinary_town_talk();
        }

        let Some(fields) = dialogue.get(&(dialog_id as u16)) else {
            self.message = format!("Dialogue id {dialog_id} is unresolved for this scene."); // audit: not a player-facing line
            return MoveOutcome::Blocked;
        };
        if fields.len() < 3 {
            self.message = format!("Dialogue id {dialog_id} has no complete talk envelope."); // audit: not a player-facing line
            return MoveOutcome::Blocked;
        }
        let raw_fields = raw_blob.get(&(dialog_id as u16));
        let keyword = keyword.and_then(non_empty_talk_keyword);

        let name = fields
            .first()
            .filter(|name| !name.is_empty())
            .map(String::as_str)
            .unwrap_or("someone");
        let description = fields
            .get(1)
            .filter(|description| !description.is_empty())
            .map(String::as_str)
            .unwrap_or("no description");

        let scene_for_flags = match self.area {
            Area::Town { scene, .. } => Some(scene),
            _ => None,
        };

        if keyword.is_none() {
            if let Some(raw_fields) = raw_fields {
                let session = crate::conversation_session::ConversationSession::new(
                    raw_fields.clone(),
                    fields.clone(),
                );
                self.active_conversation = Some(Box::new(session));
                self.active_conversation_npc_slot = conversation_npc_slot;
                // `conversation.md §7.6` / §10: the branch-flag bit `0x8C`
                // tests and `0x88` sets is the speaking NPC's roster slot,
                // supplied by the engine and never by the script.
                let npc_slot = self.active_conversation_npc_slot.map(|slot| slot as u8);
                if let Some(session) = self.active_conversation.as_mut() {
                    session.set_npc_slot(npc_slot);
                }
                let (rendered, suspended) = self.active_conversation_opening_rendered();
                let opening = self.finish_conversation_opening(&rendered, suspended);
                self.emit_tlk_message(opening);
                // `town-mode.md §7` step 4: the epilogue runs "When the
                // dispatcher returns", and the Talk dispatcher runs the whole
                // conversation (`conversation.md §2`). A conversation still
                // open owes its turn to the moment it closes (playtest
                // P14-10); one that stopped in its opening spends it now.
                if self.active_conversation.is_some() {
                    self.pending_talk_conversation_turn = !self.talk_entered_by_contact;
                } else {
                    self.advance_talk_turn();
                }
                return MoveOutcome::Talked;
            }
        }
        self.advance_talk_turn();

        // Compose the inputs for the byte-runner once per call; the
        // avatar name is the first party member's name (or "Avatar"
        // when the roster has not been seeded yet).
        let avatar_name = self
            .party_names
            .first()
            .map(|name| {
                let trimmed: Vec<u8> = name.iter().take_while(|b| **b != 0).copied().collect();
                String::from_utf8_lossy(&trimmed).into_owned()
            })
            .unwrap_or_else(|| "Avatar".to_string());
        let branch_flags = scene_for_flags
            .map(|scene| self.talk_branch_slot_for_scene(scene))
            .unwrap_or(0);
        let dictionary_owned = self.common_word_dictionary.clone();
        let dictionary_refs = common_word_dictionary_refs_or_published(dictionary_owned.as_ref());
        let inputs = crate::tlk_runner::TlkRunInputs {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: Some(&dictionary_refs),
            gold_available: Some(self.gold),
            npc_slot: self.active_conversation_npc_slot.map(|slot| slot as u8),
            ask_who_response: 0,
            yield_on_pause: false,
            yield_on_ask: false,
            // `conversation.md §7.3` (R520): "Every response stream ... goes
            // through the same byte runner, so both codes behave identically
            // on every path." This legacy keyword-carrying path marks its
            // `0x83`/`0x8F` holds like the session does, and the reply is
            // printed through the hold-aware emitter below.
            mark_holds: true,
        };

        // Resolve which field(s) to run through the byte runner.
        let run_field = |idx: usize| -> Option<crate::tlk_runner::TlkRunOutput> {
            raw_fields
                .and_then(|raw| raw.get(idx))
                .map(|bytes| crate::tlk_runner::run_tlk_stream(bytes, &inputs))
        };

        let mut applied_grants: Vec<crate::tlk_control_codes::TlkActionDispatchVerb> = Vec::new();
        let mut applied_payments: Vec<crate::conversation_session::ConversationGoldPayment> =
            Vec::new();
        let mut applied_signal_flags: Vec<u8> = Vec::new();
        let mut applied_alarms: u8 = 0;
        let mut applied_flags: u32 = 0;
        let mut applied_standing: Option<u8> = None;

        if let Some(keyword) = keyword {
            if fields.len() < 5 {
                self.message = format!("Dialogue id {dialog_id} has no complete talk envelope."); // audit: not a player-facing line
                return MoveOutcome::Talked;
            }
            // `conversation.md §6` steps 3 to 5 through the conversation
            // session's own matcher, raw records included, so the walk stops
            // at the first label declaration exactly as the interactive
            // prompt's does (§6 step 5, u5-spec#299 Q20 / Q22).
            let lookup = crate::conversation_session::tlk_keyword_lookup(
                raw_fields.map(Vec::as_slice),
                fields,
                keyword,
            );
            let response_field_index = match lookup {
                crate::conversation_session::TlkKeywordLookup::Field(idx) => Some(idx),
                _ => None,
            };
            let response_text = if lookup == crate::conversation_session::TlkKeywordLookup::Rebuke {
                crate::TLK_RESERVED_REBUKE_MESSAGE.to_string()
            } else if let Some(idx) = response_field_index {
                if let Some(output) = run_field(idx) {
                    applied_grants.extend(output.action_grants.iter().copied());
                    applied_signal_flags.extend(output.signal_flags.iter().copied());
                    applied_alarms = applied_alarms.saturating_add(
                        crate::conversation_session::tlk_run_alarm_count(&output),
                    );
                    applied_payments.extend(output.events.iter().filter_map(|event| match event {
                        crate::tlk_runner::TlkRunEvent::GoldPayment { amount, accepted } => {
                            Some(crate::conversation_session::ConversationGoldPayment {
                                amount: *amount,
                                accepted: *accepted,
                            })
                        }
                        _ => None,
                    }));
                    applied_flags |= output.branch_flags_set;
                    applied_standing = output.moral_standing.or(applied_standing);
                    if output.text.is_empty() {
                        TLK_NO_KEYWORD_MATCH_MESSAGE.to_string()
                    } else {
                        output.text
                    }
                } else {
                    talk_keyword_response(fields, keyword)
                        .filter(|response| !response.is_empty())
                        .unwrap_or(TLK_NO_KEYWORD_MATCH_MESSAGE)
                        .to_string()
                }
            } else {
                TLK_NO_KEYWORD_MATCH_MESSAGE.to_string()
            };
            let (legacy_text, legacy_actions) = talk_response_text_and_actions(&response_text);
            self.apply_tlk_action_grants(&applied_grants);
            self.apply_tlk_payments_and_standing(&applied_payments, applied_standing);
            self.record_tlk_signal_flags(&applied_signal_flags);
            self.apply_tlk_alarm_sweeps(applied_alarms);
            self.apply_talk_action_grants(&legacy_actions);
            if let Some(scene) = scene_for_flags {
                self.merge_talk_branch_flags(scene, applied_flags);
            }
            // Measured (`qa/paired/castle-talk.tsv`): a reply is the NPC's
            // own text and nothing else - the sibling branch below already
            // prints it that way, and the original never names the speaker
            // on the reply row.
            let _ = name;
            if crate::text_hold::text_has_hold_markers(&legacy_text) {
                // The first page prints now; each later page waits behind
                // its pause or wait-key (`PlayState::emit_tlk_message`).
                self.emit_tlk_message(crate::tlk_runner::TlkRenderedText::plain(legacy_text));
            } else {
                self.message = legacy_text;
            }
        } else {
            let greeting_text = if let Some(output) = run_field(2) {
                applied_grants.extend(output.action_grants.iter().copied());
                applied_signal_flags.extend(output.signal_flags.iter().copied());
                applied_alarms = applied_alarms
                    .saturating_add(crate::conversation_session::tlk_run_alarm_count(&output));
                applied_payments.extend(output.events.iter().filter_map(|event| match event {
                    crate::tlk_runner::TlkRunEvent::GoldPayment { amount, accepted } => {
                        Some(crate::conversation_session::ConversationGoldPayment {
                            amount: *amount,
                            accepted: *accepted,
                        })
                    }
                    _ => None,
                }));
                applied_flags |= output.branch_flags_set;
                applied_standing = output.moral_standing.or(applied_standing);
                if output.text.is_empty() {
                    "...".to_string()
                } else {
                    output.text
                }
            } else {
                fields
                    .get(2)
                    .filter(|greeting| !greeting.is_empty())
                    .map(String::clone)
                    .unwrap_or_else(|| "...".to_string())
            };
            let (legacy_text, legacy_actions) = talk_response_text_and_actions(&greeting_text);
            self.apply_tlk_action_grants(&applied_grants);
            self.apply_tlk_payments_and_standing(&applied_payments, applied_standing);
            self.record_tlk_signal_flags(&applied_signal_flags);
            self.apply_tlk_alarm_sweeps(applied_alarms);
            self.apply_talk_action_grants(&legacy_actions);
            if let Some(scene) = scene_for_flags {
                self.merge_talk_branch_flags(scene, applied_flags);
            }
            self.message = format!("Talked to {name}: {description}. {legacy_text} Your interest?");
        }
        MoveOutcome::Talked
    }

    fn format_talk_shop_opening_message(
        &mut self,
        dialog_id: u8,
        session: &crate::shop_session::ActiveShopSession,
        family: Option<&str>,
        shoppe_renderer: Option<&crate::shoppe_bark::ShoppeTextRenderer>,
    ) -> String {
        if crate::shoppe_records::talk_entry_uses_shared_preamble(dialog_id) {
            if let Some(renderer) = shoppe_renderer {
                let ordinal = self.random_range_u8(0, 3);
                if let Some(record_id) = crate::shoppe_records::shared_shop_bark_record(
                    dialog_id,
                    crate::shoppe_records::SharedShopBarkKind::Preamble,
                    ordinal,
                ) {
                    // `shops.md §4.1` / `§8.0`: `#` takes the shop's display
                    // name and `$` takes the *vendor's* name. They are two
                    // different resident tables sharing one shop-instance
                    // row, so the shop label must not stand in for the
                    // shopkeeper.
                    let label = session.shop_label();
                    let scene_byte = match self.area {
                        Area::Town { scene, .. } => Some(scene.byte),
                        _ => None,
                    };
                    let vendor = scene_byte
                        .and_then(|scene| shop_vendor_name_for_scene(dialog_id, scene))
                        .unwrap_or(label);
                    let ctx = crate::shoppe_bark::ShoppeBarkContext {
                        vendor_name: vendor,
                        shop_name: label,
                        hour: self.clock.hour,
                        ..Default::default()
                    };
                    if let Ok(rendered) = renderer.render_record(record_id, &ctx) {
                        // `shops.md §8.B`: "**All seven non-arms entries**
                        // first emit an opening double quote, render one
                        // record from their shared entry row, then place the
                        // input continuation according to the resulting
                        // window-local cursor column ... The record supplies
                        // the greeting/question and its closing quote."
                        //
                        // The engine printed the record bare: Cove's healer
                        // opened `I bid thee welcome` where the stock game
                        // opened `"I bid thee welcome`, and its answer row
                        // never appeared at all.
                        // `shops.md §8.0`: the Talk dispatcher "first emits
                        // the same conversation entry newline used for all
                        // Talk dispatches" before handing control to the shop
                        // overlay, so the greeting opens a row below the
                        // command echo the way a conversation reply does.
                        let greeting = format!("\n\"{rendered}");
                        let continuation = crate::shoppe_records::shop_entry_input_continuation(
                            crate::wrapped_final_row_columns(&greeting),
                        );
                        let rendered = format!("{greeting}{continuation}");
                        // Measured 2026-09-07/08 at four shops - the Paws
                        // tavern, the North Britanny inn, Cove's healer and
                        // Cove's herbalist (`qa/paired/paws-tavern.tsv`,
                        // `nb-inn.tsv`, `cove-healer.tsv`,
                        // `cove-herbalist.tsv`): entry prints the greeting
                        // record and nothing else, and the shop then waits on
                        // a `:` row where `Y` echoes as `:Yes`. The engine
                        // used to append an invented key summary
                        // (`Rest (R), Leave (L), Pick up (P), or Space.`).
                        return rendered;
                    }
                }
            }
        }

        // `shops.md §8.B`, "Arms entry, in order": arms does not use the
        // shared entry greeting - it opens on its own resident welcome line,
        // "`\"Good @, and welcome to #!\"\n`, with token expansion". The
        // attribution and greeting follow the stage-2 pause, in
        // `handle_arms_shop_key_input`.
        //
        // Without this the arms entry fell through to the no-assets fallback
        // below and printed `Weaponsmith / Armourer is now open.` in ordinary
        // play, which is how it read at Britain's weaponsmith with a full
        // asset directory loaded.
        if dialog_id == crate::shoppe_records::SHOP_DIALOG_ID_ARMS {
            return crate::input_dispatch::arms_welcome_line(self.clock.hour, session.shop_label());
        }

        // `shops.md §8` gives every other shop kind a `SHOPPE.DAT` entry
        // greeting; this is the fallback for when that record cannot be
        // rendered. It used to append `Dispatch family: {family}.`, which
        // is engine-internal routing vocabulary with no counterpart in the
        // original - a paired capture of the Iolo's Bows counter showed it
        // on screen. The routing family is still taken as an argument
        // because callers pass it for other purposes; it is simply never
        // printed.
        let _ = family;
        // Only reached when `SHOPPE.DAT` cannot be read at all - unit
        // fixtures with no assets. The original always has a record here.
        let label = session.shop_label();
        format!("{label} is now open.")
    }

    /// Apply the byte-runner's recorded [`TlkActionDispatchVerb`] grants
    /// to the live runtime counters per `conversation.md §7.6`.
    /// Apply one run's `0x85` payments and its raise/lower-standing writes.
    ///
    /// The runner reports standing as the absolute value its own writes
    /// reached, counted from the standing it was handed - which is the
    /// standing *before* any payment in the same run. Writing that value
    /// after the payments therefore discarded a qualifying payment's
    /// `karma.md §4.1` milestone: Ava's bonus record (the payment, then five
    /// `0x89`) reset the counter but ended at `+5`, not `+6` (playtest G04).
    /// The runner's writes are applied first and the milestone on top of
    /// them. The milestone and `0x89` are capped `+1` steps, so for a record
    /// that only raises standing the result is exactly stream order; only a
    /// lower-standing write pinned at the floor could tell the two apart.
    ///
    /// The open session carries the selector its streams last wrote into
    /// its later streams and turns, so it is handed the result here: a
    /// milestone the session never saw would otherwise be overwritten again
    /// by the next `0x89` / `0x8A` in the same conversation.
    pub fn apply_tlk_payments_and_standing(
        &mut self,
        payments: &[crate::conversation_session::ConversationGoldPayment],
        standing: Option<u8>,
    ) {
        self.apply_tlk_moral_standing(standing);
        self.apply_tlk_gold_payments(payments);
        let live = self.moral_standing;
        if let Some(session) = self.active_conversation.as_mut() {
            session.sync_moral_standing(live);
        }
    }

    /// `conversation.md §7.4` / `karma.md §4`: assign the shared
    /// moral-standing selector after a TLK stream ran `0x89`
    /// STANDING-UP or `0x8A` STANDING-DOWN. The runner already applied
    /// the published capped-add / capped-subtract writers, so this is an
    /// assignment; `None` means the stream ran neither code and the
    /// selector is untouched.
    pub fn apply_tlk_moral_standing(&mut self, standing: Option<u8>) {
        if let Some(standing) = standing {
            self.moral_standing = standing;
        }
    }

    pub fn apply_tlk_action_grants(
        &mut self,
        grants: &[crate::tlk_control_codes::TlkActionDispatchVerb],
    ) {
        use crate::tlk_control_codes::TlkActionDispatchVerb;
        for grant in grants {
            match grant {
                TlkActionDispatchVerb::RaiseFood => {
                    self.food = self.food.saturating_add(1).min(PARTY_FOOD_CAP);
                }
                TlkActionDispatchVerb::RaiseGold => {
                    self.gold = self.gold.saturating_add(1).min(PARTY_GOLD_CAP);
                }
                TlkActionDispatchVerb::RaiseKeys => {
                    self.keys = self.keys.saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
                TlkActionDispatchVerb::RaiseGems => {
                    self.gems = self.gems.saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
                TlkActionDispatchVerb::RaiseTorches => {
                    self.torches = self.torches.saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
                // `conversation.md §7.6`: `F` is "`+1` (cap `99`)" like every
                // counter letter; "Counter letters never assign a fixed final
                // value".
                TlkActionDispatchVerb::SetGrappleGate => {
                    self.climbing_gear = self.climbing_gear.saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
                TlkActionDispatchVerb::RaiseCarpets => {
                    let slot = &mut self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX];
                    *slot = (*slot).saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
                TlkActionDispatchVerb::SetSextantCarried => {
                    self.special_items[SPECIAL_ITEM_SEXTANT_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE;
                }
                TlkActionDispatchVerb::SetSpyglassCarried => {
                    self.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE;
                }
                TlkActionDispatchVerb::SetBlackBadgeCarried => {
                    self.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE;
                }
                TlkActionDispatchVerb::RaiseSkullKeys => {
                    let slot = &mut self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX];
                    *slot = (*slot).saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
            }
        }
    }

    /// Apply accepted conversation gold payments emitted by TLK `0x85`.
    ///
    /// Per `conversation.md §7.6` and `karma.md §4.1`, affordability
    /// alone accepts a payment and debits the gold. The turn-aged saved
    /// cooldown is only tested/reset when the speaking live NPC belongs
    /// to the one qualifying actor class. A qualifying threshold payment
    /// raises moral standing by one, plus two when the post-debit purse
    /// is empty. Payments never increment the cooldown.
    pub fn apply_tlk_gold_payments(
        &mut self,
        payments: &[crate::conversation_session::ConversationGoldPayment],
    ) {
        let qualifying_speaker = self
            .active_conversation_npc_slot
            .and_then(|npc_slot| self.npcs.iter().find(|npc| npc.slot == npc_slot))
            .filter(|npc| {
                npc.active_object
                    .and_then(|slot| self.active_objects.get(slot))
                    .is_some_and(|object| !object.is_empty())
            })
            .is_some_and(|npc| npc.type_byte == TLK_GOLD_PAYMENT_KARMA_SPEAKER_CLASS);
        for payment in payments {
            if !payment.accepted || self.gold < payment.amount {
                continue;
            }
            self.gold -= payment.amount;
            if qualifying_speaker && self.toll_progress >= TOLL_PROGRESS_MILESTONE {
                self.toll_progress = 0;
                self.moral_standing = apply_karma_action(
                    self.moral_standing,
                    KarmaAction::TollMilestone {
                        left_party_with_zero_gold: self.gold == 0,
                    },
                );
            }
        }
    }

    /// Merge a byte-runner-produced set-flag mask into the active scene's
    /// branch-flag slot.
    pub fn merge_talk_branch_flags(&mut self, scene: Scene, mask: u32) {
        if mask == 0 {
            return;
        }
        let slot = self.talk_branch_flags.entry(scene.byte).or_insert(0);
        *slot |= mask;
    }

    /// Open a multi-turn conversation session for the facing NPC.
    /// Returns the rendered greeting text. Returns `None` when there
    /// is no NPC, the NPC is a shop trigger, or the area is not a
    /// town-class scene.
    pub fn open_conversation_session(
        &mut self,
        dialogue: &HashMap<u16, Vec<String>>,
        raw_blob: &HashMap<u16, Vec<Vec<u8>>>,
    ) -> Option<String> {
        if !matches!(self.area, Area::Town { .. }) {
            return None;
        }
        // `conversation.md §2` step 1 has no liveness gate (u5-spec#300 Q12,
        // R596). Refusing to open a second session over a live one is this
        // engine's own session invariant, not a game rule.
        if self.active_conversation.is_some() {
            return None;
        }
        let (dialog_id, target_x, target_y) = self.facing_talk_target()?;
        if matches!(
            dialog_id,
            TOWN_NPC_COWERING_DIALOG_ID | TOWN_NPC_BRUSHOFF_DIALOG_ID
        ) {
            let text = town_npc_dispatcher_sentinel_line(dialog_id)
                .expect("cowering and scare indices are dispatcher sentinels");
            self.active_conversation = None;
            self.active_conversation_npc_slot = None;
            self.message = text.clone();
            if dialog_id == TOWN_NPC_BRUSHOFF_DIALOG_ID {
                self.talk_scare_hand_off(target_x, target_y);
            }
            return Some(text);
        }
        if dialog_id == 0 || talk_shop_trigger(dialog_id).is_some() {
            return None;
        }
        let fields = dialogue.get(&(dialog_id as u16))?;
        let raw = raw_blob.get(&(dialog_id as u16))?;
        let session =
            crate::conversation_session::ConversationSession::new(raw.clone(), fields.clone());
        self.active_conversation = Some(Box::new(session));
        self.active_conversation_npc_slot = self
            .talk_record_at(target_x, target_y)
            .map(|npc| npc.slot);
        // `conversation.md §7.6` / §10: the branch-flag bit `0x8C`
        // tests and `0x88` sets is the speaking NPC's roster slot,
        // supplied by the engine and never by the script.
        let npc_slot = self.active_conversation_npc_slot.map(|slot| slot as u8);
        if let Some(session) = self.active_conversation.as_mut() {
            session.set_npc_slot(npc_slot);
        }
        let (rendered, suspended) = self.active_conversation_opening_rendered();
        let opening = self.finish_conversation_opening(&rendered, suspended);
        let text = opening.text.clone();
        self.emit_tlk_message(opening);
        Some(text)
    }

    /// Render the active conversation's greeting and put it in
    /// `state.message`. Returns the rendered text.
    pub fn advance_active_conversation_greeting(&mut self) -> String {
        self.advance_active_conversation_greeting_with_seed(None)
    }

    /// [`Self::advance_active_conversation_greeting`] with the stranger
    /// coin's host-clock seed supplied, so tests can pick the branch.
    pub(crate) fn advance_active_conversation_greeting_with_seed(
        &mut self,
        stranger_host_seed: Option<u16>,
    ) -> String {
        let (rendered, suspended) =
            self.active_conversation_opening_rendered_with_seed(stranger_host_seed);
        let opening = self.finish_conversation_opening(&rendered, suspended);
        let text = opening.text.clone();
        self.emit_tlk_message(opening);
        text
    }

    /// Close the opening: the prompt that follows it, or - when an opener
    /// entry reported a `0x82` stop - nothing at all. `conversation.md §9`
    /// step 2 (u5-spec#299 Q1, R560): after an opener stop "the conversation
    /// skips the keyword prompt, `BYE` and the Bye entry, and runs only the
    /// final cleanup"; the Britain wizard's stranger branch leaves the cursor
    /// at the start of the row after `Good day!"` and the next key is a
    /// command. The same close the keyword path runs on `ended`.
    fn finish_conversation_opening(
        &mut self,
        rendered: &crate::tlk_runner::TlkRenderedText,
        suspended: bool,
    ) -> crate::tlk_runner::TlkRenderedText {
        let ended = self
            .active_conversation
            .as_ref()
            .is_some_and(|session| session.has_ended());
        if !ended {
            let prompt = self.open_conversation_prompt(suspended);
            return conversation_opening_rendered(rendered, prompt);
        }
        if let Some(session) = self.active_conversation.as_mut() {
            session.acknowledge_close();
        }
        self.active_conversation = None;
        self.active_conversation_npc_slot = None;
        let mut opening = conversation_opening_stopped_rendered(rendered);
        if self.run_final_conversation_cleanup() {
            append_falsehood_stolen_goods_line(&mut opening);
        }
        opening
    }

    /// Which prompt closes the opening: `§7`'s ASK-WHO name prompt when
    /// the Description entry suspended on its `0x88`, a label block's
    /// scoped `You respond-` prompt when a Description or Greeting GOTO's
    /// head reached its NUL (`§7.7` step 2, u5-spec#299 Q20: "This is
    /// **not** `Your interest?`"), `§6`'s keyword prompt otherwise.
    fn open_conversation_prompt(&self, suspended: bool) -> &'static str {
        let ask_who = self
            .active_conversation
            .as_ref()
            .is_some_and(|session| session.awaiting_ask_who());
        let scoped = self
            .active_conversation
            .as_ref()
            .is_some_and(|session| session.awaiting_scoped_keyword());
        if suspended && ask_who {
            crate::TLK_ASK_WHO_PROMPT
        } else if scoped {
            crate::tlk_control_codes::TLK_SCOPED_PROMPT
        } else {
            TLK_KEYWORD_PROMPT
        }
    }

    fn active_conversation_opening_rendered(
        &mut self,
    ) -> (crate::tlk_runner::TlkRenderedText, bool) {
        self.active_conversation_opening_rendered_with_seed(None)
    }

    /// Testable form of the conversation opener. A supplied seed is installed
    /// only for a stranger; acquainted NPCs do not sample or mutate the PRNG.
    pub fn active_conversation_greeting_rendered_with_host_seed(
        &mut self,
        host_seed: u16,
    ) -> crate::tlk_runner::TlkRenderedText {
        self.active_conversation_opening_rendered_with_seed(Some(host_seed))
            .0
    }

    /// Run `§9`'s opening: the Description entry, then - unless that
    /// suspended on its own `0x88` - the greeting or stranger branch.
    /// Returns the rendered opening and whether it suspended.
    fn active_conversation_opening_rendered_with_seed(
        &mut self,
        stranger_host_seed: Option<u16>,
    ) -> (crate::tlk_runner::TlkRenderedText, bool) {
        let avatar_name = self
            .party_names
            .first()
            .map(|name| {
                let trimmed: Vec<u8> = name.iter().take_while(|b| **b != 0).copied().collect();
                String::from_utf8_lossy(&trimmed).into_owned()
            })
            .unwrap_or_else(|| "Avatar".to_string());
        let branch_flags = match self.area {
            Area::Town { scene, .. } => self.talk_branch_slot_for_scene(scene),
            _ => 0,
        };
        let party_name_bytes: Vec<Vec<u8>> = self
            .party_names
            .iter()
            .take(self.party.len())
            .map(|name| name.iter().take_while(|b| **b != 0).copied().collect())
            .collect();
        let party_member_names: Vec<&[u8]> = party_name_bytes.iter().map(Vec::as_slice).collect();
        let dictionary_owned = self.common_word_dictionary.clone();
        let dictionary_refs = common_word_dictionary_refs_or_published(dictionary_owned.as_ref());
        let ctx = crate::conversation_session::ConversationContext {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: Some(&dictionary_refs),
            gold_available: Some(self.gold),
            party_member_names: &party_member_names,
        };
        let mut rendered = crate::tlk_runner::TlkRenderedText::default();
        // §9 step 2 first: the Description entry is a byte stream, and a
        // shipped NPC can put the whole opening in it and leave the
        // Greeting entry empty (`cleak/u5-spec#198`). If it suspends on
        // its own `0x88` the greeting must not run on top of it.
        let mut description = crate::tlk_runner::TlkRenderedText::default();
        let mut suspended = false;
        if let Some(session) = self.active_conversation.as_mut() {
            let output = session.present_description(&ctx);
            description = output.rendered_text();
            // u5-spec#299 Q1: a Description that stops (`0x82`) ends the
            // conversation and one that reaches `0xFF` opens the keyword
            // prompt there; either way the acquaintance test, reseed, coin
            // flip and greeting below never run (`conversation.md §9` step 2).
            suspended = session.opening_suspended() || session.has_ended();
            self.apply_tlk_action_grants(&output.action_grants);
            self.apply_tlk_payments_and_standing(&output.gold_payments, output.moral_standing);
            self.record_tlk_signal_flags(&output.signal_flags);
            self.apply_tlk_alarm_sweeps(output.alarm_sweeps);
            if let Area::Town { scene, .. } = self.area {
                self.merge_talk_branch_flags(scene, output.branch_flags_set);
            }
        }
        if suspended {
            return (description, true);
        }
        let knows_party = self
            .active_conversation_npc_slot
            .and_then(|slot| u8::try_from(slot).ok())
            .is_none_or(|slot| talk_branch_flag_is_set(branch_flags, slot));
        let stranger_introduces = if knows_party {
            false
        } else {
            self.prng_state = stranger_host_seed.unwrap_or_else(host_clock_prng_seed_now);
            // `conversation.md §9` step 3 (u5-spec#304 Q2): one draw over
            // `0..1`, the opener's only one; "On result `1` the NPC
            // introduces itself", on `0` nothing follows the description.
            self.random_range_u8(0, 1) != 0
        };
        let mut suspended_after_opening = false;
        if let Some(session) = self.active_conversation.as_mut() {
            let output = if knows_party {
                session.present_greeting(&ctx)
            } else {
                session.present_stranger_opening(&ctx, stranger_introduces)
            };
            // `conversation.md §9` step 3: the self-introduction "runs the
            // Name entry ... through the byte runner", and a shipped Name
            // entry can end on the introduce-yourself guard (`0x8C 0xFF`
            // then `0x88`). With the bit clear that ASK-WHO runs in full, so
            // the opening closes on its name prompt, not `Your interest?`
            // (playtest F04, Gruman).
            suspended_after_opening = session.awaiting_ask_who();
            rendered = output.rendered_text();
            self.apply_tlk_action_grants(&output.action_grants);
            self.apply_tlk_payments_and_standing(&output.gold_payments, output.moral_standing);
            self.record_tlk_signal_flags(&output.signal_flags);
            self.apply_tlk_alarm_sweeps(output.alarm_sweeps);
            if let Area::Town { scene, .. } = self.area {
                self.merge_talk_branch_flags(scene, output.branch_flags_set);
            }
        }
        let mut opening = description;
        if !rendered.text.trim().is_empty() {
            // `conversation.md §9` step 3: both arms speak inside the quote
            // wrapper - the known-NPC arm "opens the quote wrapper and runs
            // the Greeting entry", and the stranger arm "prints the `I am
            // called` lead-in and runs the **Name** entry ... then closes
            // the quote". The description ahead of it is narration and
            // stays bare. A paired capture of a walked-in castle
            // conversation reads `You see a pretty young girl.`, a blank
            // row, then `"I am called Ava"`. `cleak/u5-engine#5`.
            let trimmed = rendered.trimmed();
            opening.push_plain("\n\n");
            if trimmed.text.trim_start().starts_with('"') {
                opening.push_rendered(&trimmed);
            } else {
                opening.push_plain("\"");
                opening.push_rendered(&trimmed);
                // `conversation.md §9`: "The renderer drops a quote that
                // would immediately follow another quote", so an entry that
                // ends with its own closing quote is not doubled.
                // Playtest E02 (`"I am called Eb""`).
                if !trimmed.text.ends_with('"') {
                    opening.push_plain("\"");
                }
            }
            // `conversation.md §9` step 3 runs the entry "through the byte
            // runner", where `0x8D` is a forced literal newline (§7.4), and
            // §7.6 gives ASK-WHO no feed of its own: when the entry suspends
            // on its `0x88`, the row breaks the stream printed before it are
            // the only ones between the closing quote and the question.
            // Trimming them ran the question onto the name row -
            // `Trian"What is` / `thy name?"` (playtest G02; Trian, Woolfe,
            // Gruman, Eb, Treanna and Joshua share the Name shape `<name>"`,
            // two `0x8D`, `0x8C 0xFF`, `0x88`).
            if suspended_after_opening {
                let feeds = rendered
                    .glyphs
                    .iter()
                    .rev()
                    .take_while(|glyph| glyph.byte.is_ascii_whitespace())
                    .filter(|glyph| glyph.byte == b'\n')
                    .count();
                for _ in 0..feeds {
                    opening.push_plain("\n");
                }
            }
        }
        (opening, suspended_after_opening)
    }

    /// Submit one typed keyword line to the active conversation.
    /// Returns the rendered response text and a flag indicating
    /// whether the session has ended.
    pub fn submit_active_conversation_keyword(&mut self, line: &str) -> (String, bool) {
        // `conversation.md §6` step 2: the free-text reader "accepts up to
        // fifteen characters", and `input.md §8`'s reader echoes only the
        // bytes it accepted. Anything typed past the cap was never taken,
        // so it is neither echoed nor submitted. Playtest E04.
        let capped_line: String;
        let line = if line.trim_end_matches(['\r', '\n']).chars().count() > TLK_INPUT_MAX_LEN {
            capped_line = line.chars().take(TLK_INPUT_MAX_LEN).collect();
            capped_line.as_str()
        } else {
            line
        };
        let join_keyword = line.trim().eq_ignore_ascii_case("join");
        let join_candidate = self
            .active_conversation
            .as_ref()
            .and_then(|session| session.npc_name());
        let avatar_name = self
            .party_names
            .first()
            .map(|name| {
                let trimmed: Vec<u8> = name.iter().take_while(|b| **b != 0).copied().collect();
                String::from_utf8_lossy(&trimmed).into_owned()
            })
            .unwrap_or_else(|| "Avatar".to_string());
        let branch_flags = match self.area {
            Area::Town { scene, .. } => self.talk_branch_slot_for_scene(scene),
            _ => 0,
        };
        let party_name_bytes: Vec<Vec<u8>> = self
            .party_names
            .iter()
            .take(self.party.len())
            .map(|name| name.iter().take_while(|b| **b != 0).copied().collect())
            .collect();
        let party_member_names: Vec<&[u8]> = party_name_bytes.iter().map(Vec::as_slice).collect();
        let dictionary_owned = self.common_word_dictionary.clone();
        let dictionary_refs = common_word_dictionary_refs_or_published(dictionary_owned.as_ref());
        let ctx = crate::conversation_session::ConversationContext {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: Some(&dictionary_refs),
            gold_available: Some(self.gold),
            party_member_names: &party_member_names,
        };
        let mut text = String::new();
        let mut rendered = crate::tlk_runner::TlkRenderedText::default();
        let mut ended = false;
        let mut asked_party_name = None;
        let mut ask_party_name_prompted = false;
        // The line was typed onto the prompt's own `:` row, and the
        // original keeps it there. Commit it before the response is
        // emitted, so the transcript reads `:JOB` and then the answer.
        let open_prompt = self.open_prompt_line();
        // `conversation.md §6` empty-input shortcut: pressing Enter on an
        // empty line "prints this line and runs the NPC's `Bye` entry". The
        // line is `BYE\n\n`, and its lack of a leading feed puts the word on
        // the prompt's own row - measured
        // (`qa/paired/dwelling-talk-after-entry.tsv`, beat `bye`): the
        // original reads `:BYE`, where this engine left the `:` row empty and
        // opened a fresh row for the word.
        //
        // `conversation.md §6` step 3: "The shortcut belongs to this prompt
        // alone. The ASK-WHO name prompt ... never reaches the farewell path:
        // an empty line there prints the dismissive acknowledgement", so
        // neither the `BYE` echo nor the spent-feed strip applies there.
        // Only a truly empty line is empty; a lone space is typed input
        // (`conversation.md §7.6`).
        //
        // `conversation.md §7.7` step 2 (u5-spec#299 Q20, R575): the scoped
        // `You respond-` prompt is not that prompt either - an empty line
        // there re-asks with `"What didst thou say?` and prints no `BYE`.
        let empty_line = line.is_empty()
            && !self.active_conversation.as_ref().is_some_and(|session| {
                session.awaiting_ask_who() || session.awaiting_scoped_keyword()
            });
        if let Some(prompt) = open_prompt.as_deref() {
            // §6's capture shows the keyword echoed in capitals.
            let echo = if empty_line {
                TLK_EMPTY_INPUT_BYE_ECHO.to_string()
            } else {
                line.to_ascii_uppercase()
            };
            self.commit_typed_free_text_line(prompt, &echo);
        }
        if let Some(session) = self.active_conversation.as_mut() {
            let output = session.submit_keyword(line, &ctx);
            text = output.text.clone();
            rendered = output.rendered_text();
            if empty_line {
                // The word itself is now on the prompt row above, and
                // committing it there closed that row, so the first of the
                // shortcut line's two feeds has already been spent. Only the
                // second - the one that makes the blank row under it - is
                // left to emit. Measured
                // (`qa/paired/dwelling-talk-after-entry.tsv`, beat `bye`):
                // one blank row between `:BYE` and the Bye entry, not two.
                let spent = TLK_EMPTY_INPUT_BYE_ECHO.len() + 1;
                if let Some(rest) = text.strip_prefix(TLK_EMPTY_INPUT_BYE_ECHO) {
                    text = rest.strip_prefix('\n').unwrap_or(rest).to_string();
                }
                rendered = rendered.without_leading_bytes(spent);
            }
            ended = output.ended;
            asked_party_name = output.asked_party_name;
            ask_party_name_prompted = session.awaiting_ask_party_name();
            self.apply_tlk_action_grants(&output.action_grants);
            self.apply_tlk_payments_and_standing(&output.gold_payments, output.moral_standing);
            self.record_tlk_signal_flags(&output.signal_flags);
            self.apply_tlk_alarm_sweeps(output.alarm_sweeps);
            if let Area::Town { scene, .. } = self.area {
                self.merge_talk_branch_flags(scene, output.branch_flags_set);
            }
        }
        let join_prompted_for_roster_companion = ask_party_name_prompted
            && join_candidate
                .as_deref()
                .is_some_and(|candidate| self.conversation_join_candidate_available(candidate));
        if join_keyword || join_prompted_for_roster_companion {
            self.active_conversation_join_candidate = join_candidate;
        }
        if let Some(answer_slot) = asked_party_name {
            if let Some(candidate) = self.active_conversation_join_candidate.take() {
                // `conversation.md §7.6` (`0x84` row): the recruit swaps the
                // roster record, bumps the party size and removes the NPC;
                // its only published outputs are the full-party refusal and
                // the no-match diagnostic. The helper's `<name> joined.`
                // summary is not game text, so nothing is printed for a
                // successful recruit (playtest F06).
                let _ = self.apply_conversation_join_candidate(&candidate, answer_slot);
            }
        } else if !join_keyword && !join_prompted_for_roster_companion {
            self.active_conversation_join_candidate = None;
        }
        if ended {
            if let Some(session) = self.active_conversation.as_mut() {
                session.acknowledge_close();
            }
            self.active_conversation = None;
            self.active_conversation_npc_slot = None;
            if self.run_final_conversation_cleanup() {
                append_falsehood_stolen_goods_line(&mut rendered);
                text = rendered.text.clone();
            }
        }
        debug_assert_eq!(rendered.text, text);
        self.emit_tlk_message(rendered);
        (text, ended)
    }

    /// Return a save-roster view with the live active-party prefix
    /// overlaid onto the preserved inactive records.
    pub fn synced_party_roster(&self) -> Vec<PartyRosterRecord> {
        let mut roster = if self.party_roster.is_empty() {
            party_roster_from_active(
                &self.party,
                &self.party_names,
                &self.party_experience,
                &self.party_stay_counters,
                &self.party_strengths,
                &self.party_intelligence,
                &self.party_equipment,
            )
        } else {
            self.party_roster.clone()
        };

        if roster.len() < self.party.len() {
            roster.resize_with(self.party.len(), || PartyRosterRecord {
                member: PartyMember {
                    slot: 0,
                    class_byte: b'A',
                    status: b'G',
                    climb_stat: DEFAULT_CLIMB_STAT,
                    mana: 0,
                    hp: DEFAULT_PARTY_HP,
                    max_hp: DEFAULT_PARTY_MAX_HP,
                    level: 1,
                },
                name: [0; SAVE_CHARACTER_NAME_LEN],
                // `formats/saved-gam.md §3.1` publishes only two gender
                // byte values (`0x0B` male, `0x0C` female) and says nothing
                // about a record the engine synthesises with no save byte
                // behind it. Defaulting to the male value is an unpublished
                // engine choice, not a spec contract; `systems/shops.md
                // §8.1` puts it on the "otherwise" branch of the arms tail
                // either way. A record that came from a save keeps its own
                // byte via the re-sync arm below.
                gender: SAVE_GENDER_MALE_BYTE,
                experience: 0,
                stay_counter: 0,
                strength: AVATAR_STAT_MAX,
                intelligence: AVATAR_STAT_MAX,
                equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
                defense: 0,
                inn_marker: crate::INN_REGISTRY_EMPTY_MARKER,
            });
        }

        for index in 0..self.party.len() {
            if self.party_names.get(index).is_none() && roster.get(index).is_some() {
                continue;
            }
            let mut member = self.party[index];
            member.slot = index as u8;
            let previous = roster.get(index).cloned();
            // `formats/saved-gam.md §3.3` (u5-spec#299 Q14): the defense byte
            // and the inn marker move with their character, so they are
            // carried by member identity exactly as the gender byte is.
            let carried = crate::party::party_roster_record_for_active_slot(
                &self.party_roster,
                index,
                self.party_names.get(index),
            );
            let defense = carried.map_or(0, |record| record.defense);
            let inn_marker = carried.map_or(crate::INN_REGISTRY_EMPTY_MARKER, |record| {
                record.inn_marker
            });
            roster[index] = PartyRosterRecord {
                member,
                // `formats/saved-gam.md §3.1`: the gender byte has no parallel
                // active-party vector to be overlaid from, so it is carried by
                // member identity rather than by slot index — the inn's
                // leave/pick-up helpers and New Order shift `party_names` and
                // friends without shifting `party_roster`, and a slot-indexed
                // carry would hand a departed member's byte to whoever moved
                // up into the slot. See `party_roster_carried_gender`.
                gender: crate::party::party_roster_carried_gender(
                    &self.party_roster,
                    index,
                    self.party_names.get(index),
                ),
                name: self
                    .party_names
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.name))
                    .unwrap_or([0; SAVE_CHARACTER_NAME_LEN]),
                experience: self
                    .party_experience
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.experience))
                    .unwrap_or(0),
                stay_counter: self
                    .party_stay_counters
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.stay_counter))
                    .unwrap_or(0),
                strength: self
                    .party_strengths
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.strength))
                    .unwrap_or_else(|| {
                        if index == 0 {
                            self.avatar_stats.strength
                        } else {
                            AVATAR_STAT_MAX
                        }
                    }),
                intelligence: self
                    .party_intelligence
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.intelligence))
                    .unwrap_or_else(|| {
                        if index == 0 {
                            self.avatar_stats.intelligence
                        } else {
                            AVATAR_STAT_MAX
                        }
                    }),
                equipment: self
                    .party_equipment
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.equipment))
                    .unwrap_or([EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]),
                defense,
                inn_marker,
            };
        }
        roster
    }

    pub fn sync_active_party_from_roster_len(&mut self, active_len: usize) {
        let active_len = active_len
            .min(SAVE_PARTY_SIZE_MAX as usize)
            .min(self.party_roster.len());
        self.party.clear();
        self.party_names.clear();
        self.party_experience.clear();
        self.party_stay_counters.clear();
        self.party_strengths.clear();
        self.party_intelligence.clear();
        self.party_equipment.clear();

        for (index, record) in self.party_roster.iter().take(active_len).enumerate() {
            let mut member = record.member;
            member.slot = index as u8;
            self.party.push(member);
            self.party_names.push(record.name);
            self.party_experience.push(record.experience);
            self.party_stay_counters.push(record.stay_counter);
            self.party_strengths.push(record.strength);
            self.party_intelligence.push(record.intelligence);
            self.party_equipment.push(record.equipment);
        }

        if let Some(avatar) = self.party_roster.first() {
            self.avatar_stats = AvatarStats {
                strength: avatar.strength,
                dexterity: avatar.member.climb_stat,
                intelligence: avatar.intelligence,
            };
        }
        if self.active_player.is_some_and(|index| index >= active_len) {
            self.active_player = None;
        }
    }

    /// `conversation.md §7.6` (table row `0x84` RECRUIT-SPEAKER): "The engine
    /// takes the speaker's *own* name from the Name entry of the loaded blob,
    /// and matches its opening characters — case-insensitively, with bit 7
    /// stripped — against the reserve portion of the sixteen-slot character
    /// roster, **scanned from the last slot downwards**. On a match the matched
    /// roster record is swapped into the active-party insertion slot, that
    /// record's inn-lodging marker is cleared, and the party-size byte is
    /// incremented; the engine then removes the NPC from the live scene."
    ///
    /// Only the reserve portion (`active_len ..` up to the sixteen-slot roster
    /// bound) is eligible, and the scan runs downwards from the last slot, so a
    /// duplicate name resolves to the *highest* reserve slot rather than the
    /// first one encountered.
    ///
    /// Boundary: `conversation.md §7.6` also says "If no reserve record matches
    /// the speaker's name the engine prints its no-match diagnostic", but the
    /// text of that diagnostic is not published for this path (§6 step 7's
    /// `I cannot help thee with that.` is the *keyword*-scan diagnostic), so no
    /// line is emitted here rather than inventing one.
    ///
    /// The `asked_party_name` swap arm below is withdrawn behaviour — §7.6 now
    /// refuses at the cap and ejects nobody, and
    /// `ConversationSession::absorb_recruit_speaker` already refuses before
    /// reaching this helper — but it is still pinned by a test outside this
    /// file, so it is left unreachable-from-play rather than deleted here.
    pub fn apply_conversation_join_candidate(
        &mut self,
        candidate_name: &str,
        asked_party_name: u8,
    ) -> Option<String> {
        let active_len = self.party.len().min(SAVE_PARTY_SIZE_MAX as usize);
        self.party_roster = self.synced_party_roster();
        let reserve_end = self.party_roster.len().min(SAVE_ROSTER_SLOT_COUNT);
        let target_index = (active_len..reserve_end)
            .rev()
            .find(|index| party_roster_name_matches(&self.party_roster[*index], candidate_name))?;

        let joined_name = party_name_to_string(&self.party_roster[target_index].name)
            .unwrap_or_else(|| candidate_name.trim().to_string());
        if active_len < SAVE_PARTY_SIZE_MAX as usize {
            let joining_name = self.party_roster[target_index].name;
            // §7.6: "the matched roster record is swapped into the
            // active-party insertion slot" - a swap with that slot's
            // occupant, not a shift of the reserve records between
            // (playtest Q15). Lodged guests are whole roster records, so
            // the two swapped records carry their registry slots with them.
            self.party_roster.swap(active_len, target_index);
            self.swap_inn_registry_slots(active_len, target_index);
            // §7.6: "that record's inn-lodging marker is cleared" - the
            // record's own byte `0x1F` (`formats/saved-gam.md` §3.3), which
            // this engine also mirrors in its registry entry for the
            // character - the same clear that `systems/shops.md` §8.4 pickup
            // performs when a lodged companion returns to the party.
            self.clear_inn_lodging_marker_for_name(&joining_name);
            self.sync_active_party_from_roster_len(active_len + 1);
            // §7.6: "the engine then removes the NPC from the live scene."
            self.remove_recruited_speaker_from_scene();
            return Some(format!("{joined_name} joined."));
        }

        let replace_index = usize::from(asked_party_name).checked_sub(1)?;
        if replace_index == 0 || replace_index >= active_len {
            return None;
        }

        let joining = self.party_roster.remove(target_index);
        let leaving = self.party_roster.remove(replace_index);
        let leaving_name = party_name_to_string(&leaving.name)
            .unwrap_or_else(|| format!("party member {}", replace_index.saturating_add(1)));
        self.party_roster.insert(replace_index, joining);
        self.party_roster.insert(active_len, leaving);
        if self.active_player == Some(replace_index) {
            self.active_player = None;
        }
        self.sync_active_party_from_roster_len(active_len);
        Some(format!("{joined_name} joined; {leaving_name} left."))
    }

    pub fn conversation_join_candidate_available(&self, candidate_name: &str) -> bool {
        self.party_roster
            .iter()
            .any(|record| party_roster_name_matches(record, candidate_name))
    }

    /// `conversation.md §7.6`: a `0x84` recruit clears the joined record's
    /// inn-lodging marker. `formats/saved-gam.md` §3.3 (u5-spec#299 Q14,
    /// `RETRACTIONS.md` R569) makes that marker the record's own last byte
    /// (`+0x1F`); clearing it to zero is the same de-lodging write
    /// `systems/shops.md` §8.4 pickup performs ("clears the returned record's
    /// marker to zero after moving the guest back into the travelling
    /// party"). Nothing else about the record is touched here.
    fn clear_inn_lodging_marker_for_name(&mut self, name: &[u8; SAVE_CHARACTER_NAME_LEN]) {
        for guest in self.inn_registry.iter_mut() {
            if guest.name == *name {
                guest.scene_marker = 0;
            }
        }
        for record in self.party_roster.iter_mut() {
            if record.name == *name {
                record.inn_marker = crate::INN_REGISTRY_EMPTY_MARKER;
            }
        }
    }

    /// `conversation.md §7.6`: "the engine then removes the NPC from the live
    /// scene." Live removal only — §7.6 says nothing about recording the slot
    /// in the per-scene permanent removal mask of `town-mode.md` §4, so the
    /// slot is freed without marking it permanently gone.
    fn remove_recruited_speaker_from_scene(&mut self) {
        let Some(npc_slot) = self.active_conversation_npc_slot else {
            return;
        };
        let Some(index) = self.npcs.iter().position(|npc| npc.slot == npc_slot) else {
            return;
        };
        if let Some(object_slot) = self.npcs[index].active_object {
            self.free_active_object_slot(object_slot);
        }
        self.npcs.remove(index);
        self.mark_visibility_dirty();
    }

    /// Return the active scene's shared conversation cleanup sentinel.
    pub fn shared_town_conversation_sentinel(&self) -> u8 {
        if !matches!(self.area, Area::Town { .. }) {
            return CONVERSATION_SHARED_NO_SLOT_SENTINEL;
        }
        self.resident_shadowlord
            .map(|slot| slot as u8)
            .unwrap_or(CONVERSATION_SHARED_NO_SLOT_SENTINEL)
    }

    /// Apply numeric TLK `0x86` action-dispatch arguments to the generic
    /// one-conversation signal band. The public contract increments each
    /// selected slot by one through the capped byte helper.
    pub fn record_tlk_signal_flags(&mut self, flags: &[u8]) {
        for flag in flags {
            let index = usize::from(*flag);
            if let Some(slot) = self.conversation_signal_flags.get_mut(index) {
                *slot = slot.saturating_add(1).min(TLK_GENERIC_SIGNAL_CAP);
            }
        }
    }

    /// `conversation.md §7.5` / §10 "Town alarm" (u5-spec#298 Q14, R531):
    /// each `0x8B` ALARM "runs the town alarm sweep owned by
    /// `systems/town-mode.md` Section 14; conversation passes no arguments and
    /// reads no result". The sweep is the town roster's, so outside a town
    /// there is nothing for it to rewrite.
    pub fn apply_tlk_alarm_sweeps(&mut self, count: u8) {
        let Area::Town { scene, floor } = self.area else {
            return;
        };
        for _ in 0..count {
            self.town_alarm_sweep(scene, floor, None);
        }
    }

    /// Run the Shadowlord of Falsehood's post-conversation theft. Returns
    /// whether the theft ran.
    ///
    /// `conversation.md §10` (added 2026-10-25, u5-spec#300 Q20): the cleanup
    /// prints the stolen-goods line [`TLK_FALSEHOOD_STOLEN_GOODS_LINE`]; the
    /// callers append it to the conversation's closing text when this returns
    /// `true` (see [`append_falsehood_stolen_goods_line`]).
    pub fn run_final_conversation_cleanup(&mut self) -> bool {
        self.run_final_conversation_cleanup_with_seed(host_clock_prng_seed_now())
    }

    /// Deterministic-seed form of [`Self::run_final_conversation_cleanup`].
    /// The production caller samples the host clock after the warning line and
    /// sound; tests can inject the same resulting 12-bit seed directly.
    pub fn run_final_conversation_cleanup_with_seed(&mut self, host_seed: u16) -> bool {
        if self.resident_shadowlord != Some(SHADOWLORD_FALSEHOOD_INDEX) {
            return false;
        }
        self.emit_sound_effect(SoundEffect::ActionSnap);

        // `prng.md §3`: this sample precedes inventory inspection even when
        // the selected branch below is a deterministic high-to-low scan.
        self.prng_state = host_seed;
        if self.keys != 0 || self.gems != 0 || self.torches != 0 {
            // `conversation.md §10` item 1 (u5-spec#304 Q3): "one draw from
            // the inclusive range `0..2`: `0` picks keys, `1` gems, `2`
            // torches. If the party has none of the picked item it draws
            // again from `0..2`, one draw per attempt".
            loop {
                let stock = match self.random_range_u8(0, 2) {
                    0 => &mut self.keys,
                    1 => &mut self.gems,
                    _ => &mut self.torches,
                };
                if *stock != 0 {
                    *stock -= 1;
                    break;
                }
            }
        } else if decrement_stock_high_to_low(&mut self.equipment_stock).is_some() {
        } else if decrement_stock_high_to_low(&mut self.scroll_stock).is_some() {
        } else if decrement_stock_high_to_low(&mut self.potion_stock).is_some() {
        } else {
            let debit = u16::from(self.random_range_u8(
                CONVERSATION_CLEANUP_GOLD_DEBIT_MIN,
                CONVERSATION_CLEANUP_GOLD_DEBIT_MAX,
            ));
            self.gold = self.gold.saturating_sub(debit);
        }
        self.mark_visibility_dirty();
        true
    }

    pub fn apply_talk_action_grants(&mut self, actions: &[char]) {
        for action in actions {
            match *action {
                'F' => {
                    self.climbing_gear =
                        self.climbing_gear.saturating_add(1).min(PARTY_BYTE_STOCK_CAP)
                }
                'H' => {
                    self.special_items[SPECIAL_ITEM_SEXTANT_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
                }
                'I' => {
                    self.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
                }
                'J' => {
                    self.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
                }
                _ => {}
            }
        }
    }

    /// `commands.md §3` / `conversation.md §2`: every town `T` result
    /// except the guard-demand arrest discriminator is an ordinary
    /// acted command. A missing target, status-tile refusal, funny-look
    /// stub, or liveness refusal therefore still consumes the town's
    /// one-minute turn and schedule pass.
    fn consume_ordinary_town_talk(&mut self) -> MoveOutcome {
        self.advance_talk_turn();
        MoveOutcome::Blocked
    }

    /// `shops.md §2` "Contact and turn accounting": "Explicit Talk, including
    /// either refusal, has the ordinary acted outcome and receives normal town
    /// clock/scheduler processing. Automatic entry is part of the triggering
    /// command's existing post-action pass and does not create an additional
    /// Talk command or schedule tick."
    fn advance_talk_turn(&mut self) {
        if !self.talk_entered_by_contact {
            self.advance_turn();
        }
    }

    fn talk_alarm_sentinel_at(
        &mut self,
        dialog_id: u8,
        target_x: usize,
        target_y: usize,
    ) -> Option<MoveOutcome> {
        let response = town_npc_dispatcher_sentinel_line(dialog_id)?;
        match dialog_id {
            TOWN_NPC_COWERING_DIALOG_ID => {}
            TOWN_NPC_BRUSHOFF_DIALOG_ID => {}
            _ => return None,
        }
        // `conversation.md §2` step 5 (u5-spec#300 Q19): the damage pass and
        // the flight helper run inside Talk, ahead of the turn epilogue, so
        // their draws and the flight rewrite precede its scheduler work.
        if dialog_id == TOWN_NPC_BRUSHOFF_DIALOG_ID {
            self.talk_scare_hand_off(target_x, target_y);
        }
        self.advance_talk_turn();
        self.message = response;
        Some(MoveOutcome::Talked)
    }

    /// `conversation.md §2` step 5, the scare index row (added 2026-10-25,
    /// u5-spec#300 Q19, `RETRACTIONS.md` R601): after `"Begone,` ⏎
    /// `vermin!"`, "the whole-party damage pass, then the forced-flight
    /// helper"; "each party slot below the party count whose status is not
    /// Dead takes its own uniform `1..8`" - the `overworld.md §6.2.4` pass.
    ///
    /// "On automatic conversation contact the flight helper receives the
    /// contacting NPC. **On an explicit typed Talk it does not receive the
    /// talked-to NPC's number**, and which record, if any, it rewrites there
    /// is not established"; static reading suggests the talked-to NPC is never
    /// rewritten. Conservatively, an explicit Talk rewrites no record.
    fn talk_scare_hand_off(&mut self, target_x: usize, target_y: usize) {
        let applied = self.apply_outdoor_impact_party_damage();
        for entry in applied {
            self.push_diagnostic(format!(
                "scare brush-off: party slot {} took {} HP ({} HP left)",
                entry.slot, entry.applied, entry.hp_after
            ));
        }
        if !self.talk_entered_by_contact {
            return;
        }
        let target_slot = self
            .talk_record_at(target_x, target_y)
            .map(|npc| npc.slot);
        if let Some(index) = self
            .npcs
            .iter()
            .position(|npc| Some(npc.slot) == target_slot)
        {
            let _ = self.npcs[index].force_town_flight();
            self.record_town_npc_mutation(index);
        }
    }

    pub(crate) fn begin_blackthorn_guard_demand(
        &mut self,
        target_x: usize,
        target_y: usize,
        consume_turn: bool,
        game_dir: Option<&std::path::Path>,
    ) -> io::Result<MoveOutcome> {
        let Area::Town { scene, floor } = self.area else {
            return Ok(self.consume_ordinary_town_talk());
        };
        let npc_slot = self
            .talk_record_at(target_x, target_y)
            .map_or(0, |npc| npc.slot);
        let arrest = TownArrestPrompt {
            scene_byte: scene.byte,
            floor,
            npc_slot,
        };
        let living = self
            .party
            .iter()
            .filter(|member| party_member_counts_as_living(member.status))
            .count()
            .min(u16::MAX as usize) as u16;
        let badge_active = self.active_effect_tag == Some(BLACK_BADGE_ACTIVE_EFFECT_TAG)
            && self.active_effect_counter != 0;

        if consume_turn {
            self.advance_turn();
        }
        match begin_blackthorn_guard_demand(scene.byte, badge_active, living) {
            BlackthornGuardDemandStart::Prompt(prompt) => {
                self.active_blackthorn_guard_demand =
                    Some(ActiveBlackthornGuardDemand { prompt, arrest });
                self.message = prompt.message();
                Ok(MoveOutcome::Talked)
            }
            BlackthornGuardDemandStart::Refused => {
                // Measured 2026-09-08 at Minoc's gate
                // (`qa/paired/minoc-refuse.tsv`): a refused demand goes
                // straight into the published arrest exchange - `"Thou art
                // under arrest!"`, a blank row, `"Wilt thou come quietly?"`
                // and a `:` row - with no sentence of the engine's own in
                // between. `blackthorn.md` §7a says exactly this; the engine
                // was printing its own summary instead.
                //
                // §7a also says "Palace contact without the worn Badge
                // returns failure before printing a demand at all. The
                // caller then owns arrest or capture under `town-mode.md`
                // Section 14", and §14's arrest branches on location before
                // printing - so inside Blackthorn's castle this refusal is
                // the audience, not the challenge. Caught at `bt-audience`'s
                // `ask1` beat by `qa/tools/paired_compare.py`
                // (`cleak/u5-engine#21`): the stock side was already asking
                // the first mantra where this engine printed `"Thou art
                // under arrest!"`.
                self.message.clear();
                // `conversation.md §2`, "The dispatcher's own lines": the
                // branch lines all follow "the dispatcher's own leading line
                // feed", which is printed before the dispatch on the dialogue
                // index - the Minoc and tribute demands below that same
                // `0xFF` dispatch carry it (measured 2026-09-12). A palace
                // refusal prints no demand, but the feed is already out: the
                // stock game shows two blank rows, not one, between the
                // walk's last echo and `Thou art subdued and blindfolded!`
                // (`bt-audience`, `bt-capture-pacing`, 2026-10-27), where
                // this engine went straight to the capture's own feed. Only
                // the audience path is reached with assets; the asset-less
                // dialogue-map fallback keeps its surrender prompt as it was.
                if game_dir.is_some() {
                    self.emit_message_line(crate::commands::TALK_DISPATCHER_LEADING_FEED);
                }
                if self.town_post_action_cleanup_skipped {
                    return Ok(MoveOutcome::Used);
                }
                self.open_town_arrest(arrest, game_dir)
            }
        }
    }

    pub fn resolve_blackthorn_guard_demand_input(
        &mut self,
        key: char,
        suffix: &str,
        game_dir: Option<&std::path::Path>,
    ) -> Option<MoveOutcome> {
        let active = self.active_blackthorn_guard_demand?;
        let mut input = String::new();
        input.push(key);
        input.push_str(suffix);
        if matches!(active.prompt, BlackthornGuardDemandPrompt::PalacePassword) {
            // `input.md §8`: Escape "clears the whole current line but does
            // **not** submit or terminate it", and the Enter that ends the
            // line is not part of it.
            if key == '\u{1b}' && suffix.is_empty() {
                return Some(MoveOutcome::PromptDeclined);
            }
            if let Some(cleared) = input.rfind('\u{1b}') {
                input = input[cleared + 1..].to_string();
            }
            input.retain(|ch| !matches!(ch, '\r' | '\n'));
            // `blackthorn.md §7a` branch 1: "the typed answer begins
            // immediately at the left edge of that next row", and the reader
            // echoes each accepted character (`input.md §8`) up to the
            // fourteen it takes, upper-cased like every general typed string
            // (`text-output.md §10.6`). The answered row stays in the
            // transcript above the outcome; the engine replaced the whole
            // window with `"Pass, friend!"` and the word was never shown
            // (playtest P12-01). Return is "not echoed", so the row is left
            // open.
            let echo = input
                .chars()
                .take(crate::BLACKTHORN_GUARD_PASSWORD_INPUT_MAX)
                .collect::<String>()
                .to_ascii_uppercase();
            if !echo.is_empty() {
                self.emit_message_line(echo);
                self.message_row_open_mid_line = true;
            }
        }
        // `blackthorn.md §7a` branch 2: the `:` answer row "echoes `Yes`
        // or `No!`". The password branch echoed the typed word above.
        if !matches!(active.prompt, BlackthornGuardDemandPrompt::PalacePassword) {
            match key.to_ascii_lowercase() {
                'y' => self.commit_prompt_reply(":", TOWN_ARREST_YES_REPLY),
                'n' => self.commit_prompt_reply(":", TOWN_ARREST_NO_REPLY),
                _ => {}
            }
        }
        match resolve_blackthorn_guard_demand(active.prompt, &input, self.gold) {
            BlackthornGuardDemandResolution::AwaitingInput => {
                self.message = active.prompt.message();
                Some(MoveOutcome::PromptDeclined)
            }
            BlackthornGuardDemandResolution::PaidOrPassed { gold } => {
                self.gold = gold;
                self.active_blackthorn_guard_demand = None;
                // `blackthorn.md §7a`: branch 1's match "answers `"Pass,
                // friend!"`" - that one is published. Branches 2 and 3
                // publish no acknowledgement at all; §7a says only that
                // "paid/passed is the ordinary outcome", so the engine
                // prints nothing rather than inventing one, the same way
                // it does for the unpublished arrest and guards'
                // challenges. It previously invented `Thy charitable gift
                // is accepted.` and `Thy tribute is accepted.`
                self.message = match active.prompt {
                    BlackthornGuardDemandPrompt::PalacePassword => "\"Pass, friend!\"".to_string(),
                    BlackthornGuardDemandPrompt::MinocCharity
                    | BlackthornGuardDemandPrompt::Tribute { .. } => String::new(),
                };
                Some(MoveOutcome::Talked)
            }
            BlackthornGuardDemandResolution::Refused { gold } => {
                self.gold = gold;
                self.active_blackthorn_guard_demand = None;
                // Same §7a / §14 hand-off as the immediate refusal above.
                self.message.clear();
                // `town-mode.md §14`: explicit `T`'s failure "passes the
                // arrest discriminator to cleanup", and "No arrest can begin
                // while Negate Time is active: that gate skips post-action
                // cleanup, the arrest's only host ... Under Quickness the
                // same holds on the alternate turns whose gate skips the
                // walkers and cleanup". This turn's epilogue already ran
                // its gates, so a skipped cleanup means no arrest at all;
                // the engine arrested anyway and then ran the jail wait on a
                // frozen clock (playtest P24-09).
                if self.town_post_action_cleanup_skipped {
                    return Some(MoveOutcome::Used);
                }
                Some(self.open_town_arrest(active.arrest, game_dir).ok()?)
            }
        }
    }

    /// `conversation.md §2` step 5: "The dispatcher in step 5 is also called
    /// by the NPC's automatic conversation-contact event, without steps 1–4."
    /// Contact therefore takes neither the step-1 direction prompt nor the
    /// step-4 mirror/bed status-tile gate. The step-3 lookup is kept only as
    /// this engine's way of resolving the contacting NPC: the producer is
    /// orthogonally adjacent by construction (`npc-schedules.md §9.2`) and the
    /// direction is derived from it, so the lookup lands on that NPC.
    pub(crate) fn talk_entry_gates_apply(&self) -> bool {
        !self.talk_entered_by_contact
    }

    pub fn talk_branch_slot_for_scene(&self, scene: Scene) -> u32 {
        self.talk_branch_flags
            .get(&scene.byte)
            .copied()
            .unwrap_or(0)
    }

    pub fn talk_branch_flag_is_set_for_scene(&self, scene: Scene, bit_index: u8) -> bool {
        talk_branch_flag_is_set(self.talk_branch_slot_for_scene(scene), bit_index)
    }

    pub fn set_talk_branch_flag_for_scene(&mut self, scene: Scene, bit_index: u8) -> bool {
        if talk_branch_flag_mask(bit_index) == 0 {
            return false;
        }
        let slot = self.talk_branch_flags.entry(scene.byte).or_insert(0);
        set_talk_branch_flag(slot, bit_index)
    }

    pub fn active_talk_branch_flag_is_set(&self, bit_index: u8) -> bool {
        let Area::Town { scene, .. } = self.area else {
            return false;
        };
        self.talk_branch_flag_is_set_for_scene(scene, bit_index)
    }

    pub fn set_active_talk_branch_flag(&mut self, bit_index: u8) -> bool {
        let Area::Town { scene, .. } = self.area else {
            return false;
        };
        self.set_talk_branch_flag_for_scene(scene, bit_index)
    }
}

/// Lay an authored `SIGNS.DAT` stream out as message-window rows the way
/// the per-cell emitter does (`text-output.md §5`): every printed cell
/// advances the cursor, and the cell that fills the row's last column carries
/// the cursor onto the next row. A line feed returns the column to the left
/// edge and steps down one row - after an edge wrap that is a second row
/// step. Hold markers occupy no cell.
#[cfg(test)]
pub fn sign_stream_rows(stream: &str) -> String {
    sign_stream_rows_rendered(&crate::tlk_runner::TlkRenderedText::plain(stream)).text
}

/// [`sign_stream_rows`] over font-carrying cells: the inserted edge-wrap
/// breaks are ordinary line feeds and every cell keeps its font.
pub fn sign_stream_rows_rendered(
    stream: &crate::tlk_runner::TlkRenderedText,
) -> crate::tlk_runner::TlkRenderedText {
    use crate::tlk_runner::{TlkRenderedGlyph, TlkRenderedText};
    let mut out = TlkRenderedText::default();
    let mut push = |glyph: TlkRenderedGlyph| {
        out.text.push(char::from(glyph.byte));
        out.glyphs.push(glyph);
    };
    let mut column = 0usize;
    for &glyph in &stream.glyphs {
        if glyph.byte == crate::text_hold::TEXT_HOLD_WAIT_KEY_MARKER {
            push(glyph);
            continue;
        }
        if glyph.byte == b'\n' {
            if column == crate::MESSAGE_WINDOW_WIDTH {
                push(TlkRenderedGlyph::ordinary(b'\n'));
            }
            push(glyph);
            column = 0;
            continue;
        }
        if column == crate::MESSAGE_WINDOW_WIDTH {
            push(TlkRenderedGlyph::ordinary(b'\n'));
            column = 0;
        }
        push(glyph);
        column += 1;
    }
    out
}

/// `formats/signs-dat.md §6` "The no-record placard" (u5-spec#300 Q13): six
/// sixteen-cell rows in the runic font, each ending with its own line feed.
pub fn sign_no_record_placard_rendered() -> crate::tlk_runner::TlkRenderedText {
    use crate::tlk_runner::{TlkGlyphFont, TlkRenderedGlyph, TlkRenderedText};
    let mut out = TlkRenderedText::default();
    for row in SIGN_NO_RECORD_PLACARD_ROWS {
        for byte in row.bytes() {
            out.text.push(char::from(byte));
            out.glyphs.push(TlkRenderedGlyph {
                byte,
                font: TlkGlyphFont::Runic,
            });
        }
        out.text.push('\n');
        out.glyphs.push(TlkRenderedGlyph::ordinary(b'\n'));
    }
    out
}

pub fn yew_wanted_poster_rows(names: &[[u8; SAVE_CHARACTER_NAME_LEN]]) -> Vec<String> {
    let mut rows = Vec::with_capacity(9);
    rows.push("abbbbbbbbbbbbbc".to_string());
    rows.push("g   Wanted:   g".to_string());
    rows.push("g             g".to_string());
    for slot in 0..3 {
        rows.push(yew_wanted_poster_name_row(names.get(slot)));
    }
    rows.push("g             g".to_string());
    rows.push("gDead or Aliveg".to_string());
    rows.push("deeeeeeeeeeeeeef".to_string());
    rows
}

/// [`yew_wanted_poster_rows`] with its frame drawn as a frame.
///
/// `formats/signs-dat.md §6`: "The border letters above are the resident
/// sign-border glyph fragments used by the original text layer, not prose",
/// and the no-record placard is given "in the runic font, in the same
/// code-point notation as the poster above" - so `a`..`g` on the border are
/// runic-font frame cells, the same ones the placard and framed `SIGNS.DAT`
/// records draw (playtest S09). The spec does not say which font the
/// `Wanted:`, name and `Dead or Alive` cells use; they keep the text font.
pub fn yew_wanted_poster_rendered(
    names: &[[u8; SAVE_CHARACTER_NAME_LEN]],
) -> crate::tlk_runner::TlkRenderedText {
    use crate::tlk_runner::{TlkGlyphFont, TlkRenderedGlyph, TlkRenderedText};
    let rows = yew_wanted_poster_rows(names);
    let last_row = rows.len() - 1;
    let mut out = TlkRenderedText::default();
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.text.push('\n');
            out.glyphs.push(TlkRenderedGlyph::ordinary(b'\n'));
        }
        let last_cell = row.len() - 1;
        for (column, byte) in row.bytes().enumerate() {
            let border = index == 0 || index == last_row || column == 0 || column == last_cell;
            out.text.push(char::from(byte));
            out.glyphs.push(TlkRenderedGlyph {
                byte,
                font: if border {
                    TlkGlyphFont::Runic
                } else {
                    TlkGlyphFont::Ordinary
                },
            });
        }
    }
    out
}

fn yew_wanted_poster_name_row(name: Option<&[u8; SAVE_CHARACTER_NAME_LEN]>) -> String {
    let mut row = [b' '; 15];
    row[0] = b'g';
    row[14] = b'g';
    if let Some(name) = name.and_then(|name| party_name_to_string(name)) {
        let start = 7usize.saturating_sub(name.len() / 2);
        for (offset, byte) in name.bytes().take(13usize.saturating_sub(start)).enumerate() {
            row[start + offset] = byte;
        }
    }
    String::from_utf8(row.to_vec()).expect("poster rows are ASCII")
}

/// `conversation.md §7.6`: the `0x84` RECRUIT-SPEAKER compare matches the
/// speaker's "opening characters — case-insensitively, with bit 7 stripped —
/// against the reserve portion of the sixteen-slot character roster". §11's
/// derivation note names it as "the case-insensitive bit-7-stripping
/// string-equality routine used by the JOIN-name compare".
///
/// The end boundary below - the roster record must either end where the
/// speaker's name ends or carry a literal space there - was taken from §6 step
/// 5's keyword rule, which u5-spec#299 Q20 retracts for keywords (R577: the
/// keyword match is the shared text matcher with no end check). §7.6 gives the
/// recruit compare only as "opening characters" of a string-equality routine,
/// so this engine keeps its conservative reading for it: `LORD` matches a
/// roster `LORD BRITISH` but `GWEN` does not match `GWENNO`.
///
/// It is the speaker's *opening characters* that are compared, so a roster
/// name that opens the speaker's longer Name entry matches as well. §7.7.1
/// (u5-spec#299 Q20) has `KEEP.TLK` 1 recruit on the original by `SHAD`, `Y`,
/// `Y`, and that NPC's Name entry is the name followed by a title, where the
/// roster record holds the bare name.
fn party_roster_name_matches(record: &PartyRosterRecord, needle: &str) -> bool {
    let speaker = conversation_name_compare_bytes(needle.as_bytes());
    if speaker.is_empty() {
        return false;
    }
    let roster = conversation_name_compare_bytes(&record.name);
    if roster.is_empty() {
        return false;
    }
    if speaker.starts_with(&roster) {
        return true;
    }
    if roster.len() < speaker.len() || roster[..speaker.len()] != speaker[..] {
        return false;
    }
    matches!(roster.get(speaker.len()), None | Some(b' '))
}

/// Normalise one side of the §7.6 JOIN-name compare: stop at the record's NUL
/// terminator (obfuscated or plain), strip bit 7, fold to upper case, and drop
/// surrounding padding spaces.
fn conversation_name_compare_bytes(raw: &[u8]) -> Vec<u8> {
    let end = raw
        .iter()
        .position(|byte| byte & 0x7f == 0)
        .unwrap_or(raw.len());
    let folded: Vec<u8> = raw[..end]
        .iter()
        .map(|byte| (byte & 0x7f).to_ascii_uppercase())
        .collect();
    let start = folded
        .iter()
        .position(|byte| *byte != b' ')
        .unwrap_or(folded.len());
    let stop = folded
        .iter()
        .rposition(|byte| *byte != b' ')
        .map_or(start, |index| index + 1);
    folded[start..stop].to_vec()
}

pub fn decrement_stock_high_to_low(stock: &mut [u8]) -> Option<usize> {
    for index in (0..stock.len()).rev() {
        if stock[index] != 0 {
            stock[index] -= 1;
            return Some(index);
        }
    }
    None
}

pub fn scroll_label(index: usize) -> &'static str {
    SCROLL_SPELL_LABELS.get(index).copied().unwrap_or("Unknown")
}

fn pending_action_for_use_request(request: UseItemRequest) -> Option<UsePendingAction> {
    match request {
        UseItemRequest::Potion {
            index,
            target: None,
        } => Some(UsePendingAction::PotionTarget {
            index,
            highlight: 0,
        }),
        UseItemRequest::Scroll {
            index: SCROLL_WIND_CHANGE_INDEX,
            direction: None,
            ..
        } => Some(UsePendingAction::ScrollWindDirection {
            index: SCROLL_WIND_CHANGE_INDEX,
        }),
        UseItemRequest::Scroll {
            index: SCROLL_RESURRECTION_INDEX,
            target: None,
            ..
        } => Some(UsePendingAction::ScrollResurrectionTarget {
            highlight: 0,
            index: SCROLL_RESURRECTION_INDEX,
        }),
        // Measured: the skull key asks `Direction-` before it acts.
        UseItemRequest::SkullKey => Some(UsePendingAction::SkullKeyDirection),
        _ => None,
    }
}

fn shadowlord_shard_special_item_index(index: usize) -> Option<usize> {
    match index {
        SHADOWLORD_FALSEHOOD_INDEX => Some(SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX),
        SHADOWLORD_HATRED_INDEX => Some(SPECIAL_ITEM_SHARD_HATRED_INDEX),
        SHADOWLORD_COWARDICE_INDEX => Some(SPECIAL_ITEM_SHARD_COWARDICE_INDEX),
        _ => None,
    }
}

impl PlayState {
    /// Step the shared `On who:` party-member prompt by one key.
    ///
    /// See [`UseTargetStep`] for the published rule this follows.
    pub(crate) fn step_use_target_selector(
        &self,
        key: char,
        suffix: &str,
        highlight: usize,
    ) -> UseTargetStep {
        let last = self.party.len().saturating_sub(1);
        // Return or Space commits the indicated row.
        if matches!(key, '\r' | '\n' | ' ') {
            return UseTargetStep::Committed(highlight.min(last));
        }
        // The four direction keys move the indicator.
        if let Some(forward) = party_selector_direction_step(key) {
            let next = if forward {
                highlight.saturating_add(1).min(last)
            } else {
                highlight.saturating_sub(1)
            };
            return UseTargetStep::Moved(next);
        }
        // A digit repositions it, bounded by the party size; an out-of-range
        // digit leaves the indicator where it is.
        if let Some(target) = pending_use_party_target(key, suffix) {
            if target < self.party.len() {
                return UseTargetStep::Moved(target);
            }
            return UseTargetStep::Moved(highlight);
        }
        UseTargetStep::Waiting
    }
}

fn pending_use_party_target(key: char, suffix: &str) -> Option<usize> {
    let mut value = String::new();
    value.push(key);
    value.push_str(suffix);
    parse_inline_party_index(&value)
}

fn pending_use_cardinal_direction(key: char, suffix: &str) -> Option<Direction> {
    std::iter::once(key)
        .chain(suffix.chars())
        .find_map(Direction::from_play_key)
        .filter(|direction| direction.opposite_cardinal().is_some())
}

/// `view.md §4` party-marker pen: slot 1, white (15) on EGA/Tandy; the
/// driver's pen mask reduces it to 3 on CGA.
const SURFACE_VIEW_MARKER_XOR_PEN: u8 = 15;

/// `view.md §4`: the party-marker box is XOR-drawn "on every fourth idle
/// poll" while the local view waits, so it blinks; §3 calls the same step
/// "re-draws the party marker every fourth idle pass", so the box is up
/// when the view is first shown. Polls 0..=3 show it, the fourth poll's XOR
/// erases it for polls 4..=7, the eighth restores it, and so on. (A
/// frontend with no wait loop never polls and so shows it steadily.)
const fn surface_view_marker_visible(idle_polls: u32) -> bool {
    (idle_polls / 4) % 2 == 0
}

fn draw_surface_view_cell(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    class: u8,
    tile: u8,
    player_marker: bool,
    mode: ViewOverlayMode,
) {
    if player_marker {
        // `view.md §4`: "a 4-by-4 box at the party's own cell is XOR-drawn
        // with the slot-1 pen [...] so each pixel's colour index is inverted
        // against white rather than set to a fixed colour".
        let limit = viewport.depth.pixel_limit();
        let pen = SURFACE_VIEW_MARKER_XOR_PEN % limit;
        for local_y in 0..scale {
            for local_x in 0..scale {
                let x = cell_x * scale + local_x;
                let y = cell_y * scale + local_y;
                if x < viewport.width && y < viewport.height {
                    let pixel = &mut viewport.pixels[y * viewport.width + x];
                    *pixel = (*pixel ^ pen) % limit;
                }
            }
        }
        return;
    }

    let color = surface_view_class_color(class, mode);
    match class {
        0x00 => {}
        0x01 => draw_surface_view_sparse_checker(viewport, cell_x, cell_y, scale, color),
        0x02 | 0x0F => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
        0x03 => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
        0x04 => {
            draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 0, color);
            draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale - 1, color);
        }
        0x05 => {
            for y in (scale / 2).saturating_sub(1)..=(scale / 2) {
                for x in 1..scale.saturating_sub(1) {
                    set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
                }
            }
        }
        0x06 => draw_view_overlay_box(viewport, cell_x, cell_y, scale, color),
        0x07 => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
        0x08 => {
            for y in 0..scale {
                for x in 0..scale {
                    if (x < scale / 2 && y < scale / 2) || (x >= scale / 2 && y >= scale / 2) {
                        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
                    }
                }
            }
        }
        0x09 => {
            draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 0, color);
            draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 2, color);
            // `view.md §4` (corrected 2026-10-12, issue #283; `RETRACTIONS.md`
            // R515): the first plot is `(2,1)`, not `(1,2)`.
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 1, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 3, color);
        }
        0x0A => {
            draw_surface_view_water_corners(viewport, cell_x, cell_y, scale, tile, mode);
        }
        0x0B => {
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 0, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 2, color);
        }
        0x0C => {
            // `view.md §4`: deep water is not the old table-mapped no-op.
            // It draws exactly one modal-terrain micro-blit at `(2,2)`.
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 2, color);
        }
        0x0D => {
            let fixed = surface_view_class_color(0x02, mode);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 1, 0, fixed);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 3, 1, fixed);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 2, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 3, color);
        }
        0x0E => {
            for y in 0..scale {
                set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 1, y, color);
                set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, y, color);
            }
        }
        0x10 => draw_surface_view_road(viewport, cell_x, cell_y, scale, tile, mode),
        0x5A => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
        _ => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
    }
}

/// `view.md §6.3`: "Earlier revisions of this section described a magic
/// peer-view tint branch inside the dungeon map renderer, and an alternate
/// tinted tile source for some wall classes. Both are withdrawn: the value
/// being read is the display adapter identifier, not a peer-spell flag. The
/// dungeon map renderer has no peer-spell branch."
///
/// So this painter takes no [`ViewOverlayMode`]: V-View, the gem map, the
/// peer spell and the X-Ray spell all paint the identical dungeon map. The
/// only thing the original varied here is the display adapter, which this
/// engine models as [`TileGraphicsDepth`] on the viewport, and only the
/// high-colour (EGA/Tandy) pens are in the v1 target.
fn draw_dungeon_view_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    glyph: Option<DungeonMinimapGlyph>,
    fonts: Option<DungeonViewFonts<'_>>,
) {
    let Some(glyph) = glyph else {
        return;
    };
    let door_color = dungeon_view_door_color();
    let wall_color = dungeon_view_wall_color();
    let highlight = DUNGEON_VIEW_HIGHLIGHT_COLOR;
    // `view.md §6.2`: "Every published glyph identifier in the
    // class-to-glyph table is an index into whichever of the two fonts that
    // class selects, and each cell is drawn opaquely in a
    // foreground/background pair."
    if let (Some(fonts), DungeonMinimapGlyph::Font { index, font }) = (fonts, glyph) {
        let source = match font {
            DungeonMinimapFont::Text => fonts.text,
            DungeonMinimapFont::Runic => fonts.runic,
        };
        let foreground = dungeon_view_font_glyph_foreground(index);
        blit_dungeon_view_font_glyph(viewport, cell_x, cell_y, scale, source, index, foreground);
        return;
    }
    // `dungeon-mode.md §12.3`: two classes "are not font characters at
    // all but small vector drawings". Their geometry is §12.5's and is
    // drawn directly rather than through a glyph index.
    let glyph = match glyph {
        DungeonMinimapGlyph::Fountain => {
            draw_dungeon_fountain_glyph(viewport, cell_x, cell_y, scale);
            return;
        }
        DungeonMinimapGlyph::EnergyField => {
            draw_dungeon_energy_field_glyph(viewport, cell_x, cell_y, scale);
            return;
        }
        DungeonMinimapGlyph::Font { index, .. } => index,
    };
    match glyph {
        // §12.4 party marker: arrowhead glyph 0x60 at the centre cell.
        0x60 => {
            for offset in 0..scale {
                set_view_overlay_pixel(viewport, cell_x, cell_y, scale, offset, scale / 2, 15);
                set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, offset, 15);
            }
        }
        0x18 => draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale / 2, 7),
        0x2e => draw_dungeon_ladder_glyph(viewport, cell_x, cell_y, scale, true, false, highlight),
        0x2d => draw_dungeon_ladder_glyph(viewport, cell_x, cell_y, scale, false, true, highlight),
        0x2f => draw_dungeon_ladder_glyph(viewport, cell_x, cell_y, scale, true, true, highlight),
        0x70 => {
            fill_view_overlay_cell(viewport, cell_x, cell_y, scale, 6);
            draw_view_overlay_box(viewport, cell_x, cell_y, scale, highlight);
        }
        // §12.4 exact byte 0x68: the up-and-down arrow, and the only
        // published owner of glyph 0x12. The fountain class used to
        // collide with it here.
        0x12 => draw_dungeon_up_and_down_arrow_glyph(viewport, cell_x, cell_y, scale, 7),
        0x19 => draw_dungeon_pit_glyph(viewport, cell_x, cell_y, scale, highlight),
        0x71 => draw_dungeon_trap_glyph(viewport, cell_x, cell_y, scale, 12),
        0x72 => draw_dungeon_trap_glyph(viewport, cell_x, cell_y, scale, 14),
        0x73 => draw_dungeon_door_glyph(viewport, cell_x, cell_y, scale, door_color),
        0x74 => draw_view_overlay_box(viewport, cell_x, cell_y, scale, wall_color),
        0x75 => {
            draw_view_overlay_box(viewport, cell_x, cell_y, scale, wall_color);
            draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale / 2, highlight);
        }
        0x76 => {
            draw_view_overlay_box(
                viewport,
                cell_x,
                cell_y,
                scale,
                dungeon_view_extra_wall_color(),
            );
            set_view_overlay_pixel(
                viewport,
                cell_x,
                cell_y,
                scale,
                scale / 2,
                scale / 2,
                highlight,
            );
        }
        0x77 => {
            draw_dungeon_door_glyph(viewport, cell_x, cell_y, scale, door_color);
            draw_view_overlay_box(viewport, cell_x, cell_y, scale, door_color);
        }
        0x7f => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, wall_color),
        _ => set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, scale / 2, 7),
    }
}

/// The foreground pen of a font-drawn minimap class. `dungeon-mode.md
/// §12.4` publishes the glyph, the font and that the cell is opaque, but not
/// the palette indices of the pens (only that `0xC?`/`0xD?` use a
/// "background pen" and that `0xD?`'s depends on the adapter), so these are
/// the engine's existing stand-in colours per class, unchanged.
const fn dungeon_view_font_glyph_foreground(index: u8) -> u8 {
    match index {
        0x18 | 0x12 => 7,
        0x71 => 12,
        0x72 => 14,
        0x73 | 0x77 => dungeon_view_door_color(),
        0x74 | 0x75 | 0x7f => dungeon_view_wall_color(),
        0x76 => dungeon_view_extra_wall_color(),
        0x70 => 6,
        _ => DUNGEON_VIEW_HIGHLIGHT_COLOR,
    }
}

/// Opaque one-bit blit of one 8x8 font glyph: set bits take `foreground`,
/// clear bits the black background, most significant bit leftmost.
fn blit_dungeon_view_font_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    font: &FixedCellFont,
    index: u8,
    foreground: u8,
) {
    for row in 0..scale.min(8) {
        let bits = font.glyph_row(index, row).unwrap_or(0);
        for column in 0..scale.min(8) {
            let set = bits & (0x80 >> column) != 0;
            let color = if set { foreground } else { 0 };
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, column, row, color);
        }
    }
}

/// Diagnostic ASCII stand-in for one minimap cell. The letters are this
/// engine's own text rendering of the map, not published art; what the
/// published table fixes is that each class gets its **own** output, so
/// the fountain and the energy field need codes of their own rather than
/// borrowing the arrow glyphs they used to collide with.
fn render_dungeon_minimap_glyph_code(glyph: Option<DungeonMinimapGlyph>) -> char {
    let Some(glyph) = glyph else {
        return ' ';
    };
    match glyph {
        DungeonMinimapGlyph::Fountain => 'f',
        DungeonMinimapGlyph::EnergyField => '=',
        DungeonMinimapGlyph::Font { index, .. } => match index {
            0x60 => '@',
            0x18 => '.',
            0x2e => '<',
            0x2d => '>',
            0x2f => 'H',
            0x70 => '$',
            0x12 => 'I',
            0x19 => 'o',
            0x71 => 'v',
            0x72 => '!',
            0x73 => '+',
            0x74 | 0x75 | 0x76 | 0x7f => '#',
            0x77 => '+',
            _ => '?',
        },
    }
}

/// `dungeon-mode.md §12.5`: the fountain basin is drawn in "the bright
/// foreground pen", which is also the pen the ladder, pit, chest-box and
/// wall-centre highlights use.
const DUNGEON_VIEW_HIGHLIGHT_COLOR: u8 = 15;

/// The dungeon minimap pens below take no [`ViewOverlayMode`]. `view.md
/// §6.3` and `dungeon-mode.md §12.4` both withdraw the peer-spell tint
/// branch that used to select them: "the value they were reading is the
/// **display-adapter identifier**, not a peer-spell flag ... V-View has no
/// peer-spell branch of its own." Any surviving variation belongs to the
/// display adapter ([`TileGraphicsDepth`]), and CGA/Hercules raster output
/// is outside the v1 clean-recreation target, so one high-colour pen set
/// serves every mode.
const fn dungeon_view_door_color() -> u8 {
    11
}

const fn dungeon_view_wall_color() -> u8 {
    8
}

/// `dungeon-mode.md §12.4`: the `0xD?` arch wall is "drawn with a
/// background pen; which pen depends on the display adapter" - the adapter,
/// not the view mode.
const fn dungeon_view_extra_wall_color() -> u8 {
    13
}

fn draw_dungeon_ladder_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    up: bool,
    down: bool,
    color: u8,
) {
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale / 2, color);
    if up {
        draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 0, color);
    }
    if down {
        draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale - 1, color);
    }
    if up && down {
        draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale / 2, color);
    }
}

/// `dungeon-mode.md §12.5` fountain vector drawing. The basin strokes go
/// down first in the bright foreground pen, then the pen switches to a
/// brighter blue for the jet and the four spray dots. All ranges are
/// inclusive and relative to the cell's pixel origin.
///
/// There is no view-mode branch: `view.md §6.3` withdraws the peer-view
/// tint that used to pick a second basin/jet pair here.
fn draw_dungeon_fountain_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
) {
    let basin = DUNGEON_VIEW_HIGHLIGHT_COLOR;
    let jet = 9;
    let mut plot = |x: usize, y: usize, color: u8| {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
    };
    // Basin: lower lip, middle lip, then the two feet.
    for x in 1..=6 {
        plot(x, 4, basin);
    }
    for x in 2..=5 {
        plot(x, 5, basin);
    }
    for x in 1..=2 {
        plot(x, 6, basin);
    }
    for x in 5..=6 {
        plot(x, 6, basin);
    }
    // Jet and spray.
    plot(2, 1, jet);
    plot(5, 1, jet);
    plot(1, 2, jet);
    plot(6, 2, jet);
    for x in 3..=4 {
        plot(x, 2, jet);
        plot(x, 3, jet);
    }
}

/// `dungeon-mode.md §12.5` energy-field vector drawing: eight full-width
/// horizontal runs covering **all eight** rows of the cell, in four
/// two-row colour bands. Each band's pen is a
/// `display-driver.md §2` user-interface colour-table slot biased into
/// the bright half of the palette — band A slot 4, band B slot 0, band C
/// slot 2, band D slot 3 — resolved through the same table the rest of
/// the interface uses so the low-colour drivers inherit their own values.
/// The drawing reads no sub-type, so all four field flavours look
/// identical on the map.
fn draw_dungeon_energy_field_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
) {
    let high_colour = viewport.depth.pixel_limit() > 4;
    for (band, slot) in DUNGEON_ENERGY_FIELD_BAND_SLOTS.iter().enumerate() {
        let color = crate::display_driver::ui_colour_slot_bright(*slot, high_colour);
        for row in 0..2 {
            let y = band * 2 + row;
            for x in 1..=6 {
                set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
            }
        }
    }
}

/// `dungeon-mode.md §12.5`: the user-interface colour-table slot each of
/// the energy field's four two-row bands takes its pen from, in band
/// order A, B, C, D.
const DUNGEON_ENERGY_FIELD_BAND_SLOTS: [usize; 4] = [4, 0, 2, 3];

/// `dungeon-mode.md §12.4` exact byte `0x68`: the up-and-down-arrow text
/// glyph `0x12`. Drawn as a vertical shaft with a head at each end.
fn draw_dungeon_up_and_down_arrow_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale / 2, color);
    let mid = scale / 2;
    for offset in 1..=1 {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, mid - offset, offset, color);
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, mid + offset, offset, color);
        set_view_overlay_pixel(
            viewport,
            cell_x,
            cell_y,
            scale,
            mid - offset,
            scale - 1 - offset,
            color,
        );
        set_view_overlay_pixel(
            viewport,
            cell_x,
            cell_y,
            scale,
            mid + offset,
            scale - 1 - offset,
            color,
        );
    }
}

fn draw_dungeon_pit_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    draw_view_overlay_box(viewport, cell_x, cell_y, scale, color);
    set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, scale / 2, color);
}

fn draw_dungeon_trap_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    draw_view_overlay_diagonals(viewport, cell_x, cell_y, scale, color);
    set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, scale / 2, color);
}

fn draw_dungeon_door_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale / 2, color);
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, (scale / 2) + 1, color);
    draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale / 2, color);
}

/// `view.md §4`, corrected 2026-10-12 (`cleak/u5-spec#283`; `RETRACTIONS.md`
/// R514/R517): there are no "source banks" and no mode-dependent pen swap.
/// Every class selects one of `display-driver.md` §2's seven UI colour
/// slots (some biased into the bright half of the EGA/Tandy palette by
/// adding eight); the hand-written per-class EGA indices this used to carry
/// had no citation, because at the time there was nothing to cite
/// (`cleak/u5-engine#40`). The adapter, not `mode`, is the only thing that
/// could vary this - and on EGA/Tandy, the v1 target, it never does: every
/// caller (View, Peer, X-Ray, Look) draws the same colours. `mode` stays a
/// parameter only because callers still pass one; it is not read here.
fn surface_view_class_color(class: u8, _mode: ViewOverlayMode) -> u8 {
    match class {
        0x01 | 0x02 | 0x09 => 10, // slot 3 + 8, light green: secondary terrain
        0x03 => 4,                // slot 0, dark red: frame fill
        0x04..=0x07 | 0x0E | 0x5A => 15, // slot 1, white: frame line
        0x08 => 14,               // slot 5, yellow: dedicated rough terrain
        // slot 2 + 8, light blue: water pen. `0xA`/`0xD` also use the
        // slot 3 + 8 secondary pen per corner/pair; see
        // `draw_surface_view_water_corners` and the `0x0D` arm above.
        0x0A..=0x0C | 0x0D | 0x0F => 9,
        _ => 7,
    }
}

fn fill_view_overlay_cell(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    for y in 0..scale {
        for x in 0..scale {
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
        }
    }
}

fn draw_view_overlay_box(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 0, color);
    draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale - 1, color);
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, 0, color);
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale - 1, color);
}

fn draw_view_overlay_diagonals(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    for offset in 0..scale {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, offset, offset, color);
        set_view_overlay_pixel(
            viewport,
            cell_x,
            cell_y,
            scale,
            scale - 1 - offset,
            offset,
            color,
        );
    }
}

fn draw_surface_view_sparse_checker(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    for (x, y) in [(1, 0), (1, 2), (3, 1), (3, 3)] {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
    }
}

fn surface_view_river_tile(tile: u8) -> bool {
    matches!(tile, 0x60..=0x69 | 0x6C..=0x6F)
}

/// `view.md §4` river-family corner table: a set bit is a light-blue (B)
/// corner, in the corner order `(1,0)`, `(3,1)`, `(1,2)`, `(3,3)` = bits
/// `0x01`, `0x02`, `0x04`, `0x08`. The table is the authority: `0x60` and
/// `0x66` share one pattern with different low nibbles, so the mask is per
/// tile, not the raw `tile & 0x0F`. Non-river members are all B.
const fn surface_view_river_shoreline_mask(tile: u8) -> u8 {
    match tile {
        0x60 | 0x66 => 0x0A, // G B G B
        0x61 | 0x65 => 0x06, // G B B G
        0x62 | 0x6F => 0x02, // G B G G
        0x63 | 0x6C => 0x08, // G G G B
        0x64 => 0x0C,        // G G B B
        0x67 | 0x68 => 0x0E, // G B B B
        0x69 => 0x07,        // B B B G
        0x6D => 0x04,        // G G B G
        0x6E => 0x01,        // B G G G
        _ => 0x0F,
    }
}

fn draw_surface_view_water_corners(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    tile: u8,
    mode: ViewOverlayMode,
) {
    let modal = surface_view_class_color(0x0A, mode);
    let secondary = surface_view_class_color(0x02, mode);
    let shoreline_mask = surface_view_river_shoreline_mask(tile);
    for (bit, x, y) in [(0x01, 1, 0), (0x02, 3, 1), (0x04, 1, 2), (0x08, 3, 3)] {
        let color = if surface_view_river_tile(tile) && shoreline_mask & bit == 0 {
            secondary
        } else {
            modal
        };
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
    }
}

const fn surface_view_road_connection_mask(tile: u8) -> u8 {
    match tile {
        0x20 => 0x01 | 0x04, // north-south
        0x21 => 0x02 | 0x08, // east-west
        0x22 => 0x01 | 0x02, // north-east
        0x23 => 0x02 | 0x04, // east-south
        0x24 => 0x04 | 0x08, // south-west
        0x25 => 0x08 | 0x01, // west-north
        0x26 => 0x0F,
        _ => 0,
    }
}

fn draw_surface_view_road(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    tile: u8,
    mode: ViewOverlayMode,
) {
    // `view.md §4`: the road opens with the class-1 secondary checker,
    // then lays down the slot-0 (dark red) centre body and connection
    // stubs. Corrected 2026-10-12 (issue #283; `RETRACTIONS.md` R516): the
    // stubs are slot 0 too, not the frame-line pen - the body and every
    // stub share one colour.
    draw_surface_view_sparse_checker(
        viewport,
        cell_x,
        cell_y,
        scale,
        surface_view_class_color(0x01, mode),
    );
    let fill = surface_view_class_color(0x03, mode);
    for y in 1..=2 {
        for x in 1..=2 {
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, fill);
        }
    }

    let mask = surface_view_road_connection_mask(tile);
    if mask & 0x1 != 0 {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 1, 0, fill);
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 0, fill);
    }
    if mask & 0x2 != 0 {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 3, 1, fill);
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 3, 2, fill);
    }
    if mask & 0x4 != 0 {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 1, 3, fill);
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 3, fill);
    }
    if mask & 0x8 != 0 {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 1, fill);
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 2, fill);
    }

    let blank_notch = match tile {
        0x22 => Some((1, 2)),
        0x23 => Some((1, 1)),
        0x24 => Some((2, 1)),
        0x25 => Some((2, 2)),
        _ => None,
    };
    if let Some((x, y)) = blank_notch {
        // The final stamp is the blank source over the centre quarter
        // diagonally opposite the elbow, not a coloured orientation mark.
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, 0);
    }
}

fn draw_view_overlay_hline(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    y: usize,
    color: u8,
) {
    for x in 0..scale {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
    }
}

fn draw_view_overlay_vline(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    x: usize,
    color: u8,
) {
    for y in 0..scale {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
    }
}

fn set_view_overlay_pixel(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    local_x: usize,
    local_y: usize,
    color: u8,
) {
    let x = cell_x * scale + local_x;
    let y = cell_y * scale + local_y;
    if x < viewport.width && y < viewport.height {
        let limit = viewport.depth.pixel_limit();
        viewport.pixels[y * viewport.width + x] = color % limit;
    }
}

fn conversation_opening_rendered(
    opening: &crate::tlk_runner::TlkRenderedText,
    prompt: &str,
) -> crate::tlk_runner::TlkRenderedText {
    // `text-output.md §10.4`: the opening prints under the
    // `Talk-<direction>` echo, which completed its own row, so its leading
    // line feed leaves one blank row between them - the shape the Look
    // result and the Talk refusals already carry. A paired capture of a
    // walked-in castle conversation reads `>Talk-North`, a blank row,
    // then `You see a pretty young girl.` (`cleak/u5-engine#5`).
    let mut rendered = crate::tlk_runner::TlkRenderedText::plain("\n");
    rendered.push_plain(TLK_OPENING_DESCRIPTION_PREFIX);
    // A blank row separates the opening from the prompt that closes it,
    // the same shape §6's response framing carries. Two line feeds,
    // because `text-output.md §5`'s feed is a combined CR+LF.
    //
    // `conversation.md §7.3`'s own example is this path: a dwelling rat's
    // description ends `"Squeek..."`, two feeds, a `0x83` PAUSE, then the
    // `0x88` prompt. The pause leaves a hold marker (`crate::text_hold`),
    // which is not whitespace, so trimming the opening stopped at it and
    // the padding below then fed two more rows on top of the stream's own
    // two - `dwelling-talk-after-entry`'s `talk` beat read three blank rows
    // above `"What is thy name?"` where the original shows one. The tail
    // is therefore read through the markers: its feeds count towards the
    // two and stay in front of the hold, where the stream put them.
    let leading = opening
        .glyphs
        .iter()
        .take_while(|glyph| glyph.byte.is_ascii_whitespace())
        .count();
    let (body, tail) = opening_body_and_hold_tail(&opening.without_leading_bytes(leading));
    rendered.push_rendered(&body);
    // `conversation.md §7.7` HEAD: a Description GOTO's label handler "emit[s]
    // the quote, then run[s] the head", and a head that opens on ASK-WHO puts
    // the handler's own opening quote straight after it - which the byte
    // runner drops, "because the stored-text path ... does not update that
    // record" of the last printable quote (§7.6 "Which quotes are the
    // stream's"). So the question continues the HEAD quote on its row, with
    // no padding between: the stream's own feeds before the GOTO already made
    // the blank row. Only when the quote is the stream's last byte: feeds or
    // a hold after it are the stream's own and keep the padding rules below.
    // Measured 2026-10-20 (`town-talk-speaker`, beat `talk`,
    // the Britain wizard of §9 step 2): the stock reads the description, a
    // blank row, then `"What is thy`; this engine drew a lone `"` row and a
    // blank above a second opening quote.
    if prompt == crate::TLK_ASK_WHO_PROMPT && opening.text.ends_with('"') {
        rendered.push_plain(prompt.strip_prefix('"').unwrap_or(prompt));
        return rendered;
    }
    let mut feeds = 0usize;
    for glyph in tail {
        if glyph.byte == b'\n' {
            if feeds < 2 {
                feeds += 1;
                rendered.text.push('\n');
                rendered.glyphs.push(glyph);
            }
        } else {
            rendered.text.push(char::from(glyph.byte));
            rendered.glyphs.push(glyph);
        }
    }
    for _ in feeds..2 {
        rendered.push_plain("\n");
    }
    rendered.push_plain(prompt);
    rendered
}

/// `conversation.md §10` (added 2026-10-25, u5-spec#300 Q20): "the single
/// stored line `Something was stolen!`, with no leading spaces, which begins
/// and ends with its own line feed, so one blank row separates it from the
/// conversation's last row and the cursor is left at column 0 of the row
/// below it".
pub const TLK_FALSEHOOD_STOLEN_GOODS_LINE: &str = "\nSomething was stolen!\n";

/// Append [`TLK_FALSEHOOD_STOLEN_GOODS_LINE`] to a conversation's closing text.
/// The closing text already ends at column 0 under one blank row (the feed
/// pair this engine closes every conversation with), so the line's own
/// leading feed is that blank row: the trailing feeds are trimmed back to the
/// conversation's last row, the stored line follows its closing feed, and the
/// same closing blank row follows the line.
fn append_falsehood_stolen_goods_line(rendered: &mut crate::tlk_runner::TlkRenderedText) {
    while rendered.glyphs.last().is_some_and(|glyph| glyph.byte == b'\n') {
        rendered.glyphs.pop();
        rendered.text.pop();
    }
    rendered.push_plain("\n");
    rendered.push_plain(TLK_FALSEHOOD_STOLEN_GOODS_LINE);
    rendered.push_plain("\n");
}

/// An opening whose entry reported a `0x82` stop (u5-spec#299 Q1): the same
/// lead-in as [`conversation_opening_rendered`], then the stream exactly as
/// it stopped - no padding to a blank row and no prompt, because the opener
/// "returns and nothing further is printed" (`conversation.md §9` step 2).
fn conversation_opening_stopped_rendered(
    opening: &crate::tlk_runner::TlkRenderedText,
) -> crate::tlk_runner::TlkRenderedText {
    let mut rendered = crate::tlk_runner::TlkRenderedText::plain("\n");
    rendered.push_plain(TLK_OPENING_DESCRIPTION_PREFIX);
    let leading = opening
        .glyphs
        .iter()
        .take_while(|glyph| glyph.byte.is_ascii_whitespace())
        .count();
    rendered.push_rendered(&opening.without_leading_bytes(leading));
    rendered
}

/// Split an opening into its body and the trailing run that follows the
/// last printed glyph. The tail keeps only the line feeds and hold markers
/// that come before the last hold; whitespace after it, and any other
/// whitespace in the run, is dropped the way trimming dropped it before
/// holds existed.
fn opening_body_and_hold_tail(
    opening: &crate::tlk_runner::TlkRenderedText,
) -> (
    crate::tlk_runner::TlkRenderedText,
    Vec<crate::tlk_runner::TlkRenderedGlyph>,
) {
    let is_hold = |byte: u8| crate::text_hold::TextHoldKind::from_marker(byte).is_some();
    let body_end = opening
        .glyphs
        .iter()
        .rposition(|glyph| !glyph.byte.is_ascii_whitespace() && !is_hold(glyph.byte))
        .map_or(0, |index| index + 1);
    let tail_glyphs = &opening.glyphs[body_end..];
    let last_hold = tail_glyphs.iter().rposition(|glyph| is_hold(glyph.byte));
    let tail = match last_hold {
        Some(last) => tail_glyphs[..=last]
            .iter()
            .filter(|glyph| glyph.byte == b'\n' || is_hold(glyph.byte))
            .copied()
            .collect(),
        None => Vec::new(),
    };
    let glyphs = opening.glyphs[..body_end].to_vec();
    let text = glyphs.iter().map(|glyph| char::from(glyph.byte)).collect();
    (crate::tlk_runner::TlkRenderedText { text, glyphs }, tail)
}

#[cfg(test)]
mod shop_vendor_name_tests {
    use super::shop_vendor_name_for_scene;

    /// `shops.md §8.0` publishes 46 reachable vendor names across the eight
    /// per-kind tables. The vendor column is a *second* resident table sharing
    /// the shop-instance row with the display-name column, so it must never be
    /// derived from the shop's own name.
    #[test]
    fn published_vendor_names_cover_all_forty_six_reachable_shop_rows() {
        let published: [(u8, u8, &str); 46] = [
            (0x81, 2, "Gwenneth"),
            (0x81, 3, "Nomaan"),
            (0x81, 4, "Ronan"),
            (0x81, 5, "Shenstone"),
            (0x81, 6, "Paul"),
            (0x81, 17, "Max"),
            (0x81, 24, "Kitiara"),
            (0x81, 26, "Steve"),
            (0x81, 32, "Thol"),
            (0x82, 1, "Sam"),
            (0x82, 2, "Tika"),
            (0x82, 3, "Nicole"),
            (0x82, 4, "Duclas"),
            (0x82, 8, "Felicity"),
            (0x82, 19, "Jaymes"),
            (0x82, 22, "Dr. Cat"),
            (0x82, 24, "Nikki"),
            (0x82, 30, "Rob"),
            (0x83, 6, "Hettar"),
            (0x83, 20, "Theoan"),
            (0x83, 22, "Ferru"),
            (0x84, 3, "Bantral"),
            (0x84, 5, "Captain Blyth"),
            (0x84, 21, "Master Hawkins"),
            (0x84, 24, "Jones"),
            (0x85, 1, "Nilrem"),
            (0x85, 4, "Madam Pendra"),
            (0x85, 7, "Toama"),
            (0x85, 23, "Enlor"),
            (0x85, 30, "Virden"),
            (0x86, 8, "Braunam"),
            (0x86, 22, "Danfits"),
            (0x86, 24, "Daem"),
            (0x87, 5, "Regina"),
            (0x87, 6, "Leila"),
            (0x87, 7, "Temptious"),
            (0x87, 21, "Milan"),
            (0x87, 23, "Jessica"),
            (0x87, 30, "Faye"),
            (0x87, 31, "Jessip"),
            (0x88, 2, "Donya"),
            (0x88, 3, "Gremnor"),
            (0x88, 7, "Rogi"),
            (0x88, 20, "Terbor"),
            (0x88, 22, "Lorien"),
            (0x88, 24, "Ransack"),
        ];

        for (dialog_id, scene, vendor) in published {
            assert_eq!(
                shop_vendor_name_for_scene(dialog_id, scene),
                Some(vendor),
                "shops.md §8.0 vendor for trigger {dialog_id:#04x} scene {scene}"
            );
        }
    }

    #[test]
    fn horse_trader_scene_thirty_row_stays_unreachable() {
        // `shops.md §8.0`: the shipped horse-trader table holds a fourth row
        // for The Lycaeum whose vendor is Simplon, but "no `0x83` trigger
        // exists anywhere in the shipped rosters for scene `30`, so the row is
        // unreachable in ordinary play ... implementations should publish and
        // reach the three rows above and must not treat scene `30` as a fourth
        // stable."
        assert_eq!(shop_vendor_name_for_scene(0x83, 30), None);
    }

    #[test]
    fn a_scene_absent_from_a_kinds_table_resolves_to_no_vendor() {
        // `shops.md §8.0`: the shipped search leaves the row one past the end
        // and reads a neighbouring kind's data; "a clean implementation should
        // reject the trigger and leave the conversation alone", so this
        // lookup must not fall back to row zero.
        assert_eq!(shop_vendor_name_for_scene(0x81, 1), None);
        assert_eq!(shop_vendor_name_for_scene(0x88, 32), None);
        assert_eq!(shop_vendor_name_for_scene(0x89, 2), None);
    }
}

/// What one key did at the shared `On who:` party-member prompt.
///
/// `inventory.md §4`: "**A digit moves the indicator; it does not commit.**
/// `1` through `6`, bounded by the party size, reposition the inverted row
/// exactly as the direction keys do and leave the prompt open. Only Return or
/// Space commits the indicated row, Escape cancels" - and "This is one shared
/// routine, so the rule is the same for Z-stats, R-Ready, New Order, the
/// fountain, Search and every other caller." The `On who:` prompt the potion
/// and the Resurrection scroll share is such a caller.
///
/// This engine committed on the digit instead, so it consumed one key fewer
/// than the original and the scripted Return that should have committed fell
/// through to the world loop and echoed `What`. Measured 2026-09-10
/// (`use-potions/red`), where the original shows no such line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseTargetStep {
    /// A digit or direction key repositioned the indicator.
    Moved(usize),
    /// An unrecognised key; the prompt stays open and nothing is redrawn.
    Waiting,
    /// Return or Space committed the indicated row.
    Committed(usize),
}

#[cfg(test)]
mod conversation_opening_hold_tests {
    use super::conversation_opening_rendered;
    use crate::tlk_runner::TlkRenderedText;

    /// `conversation.md §7.3`'s example: the dwelling rat's description
    /// ends `"Squeek..."`, two feeds, a `0x83` PAUSE, then the `0x88`
    /// prompt. The stream's own two feeds already leave the one blank row
    /// above the prompt, so the hold marker must not stop the opening from
    /// seeing them - `dwelling-talk-after-entry`'s `talk` beat read three
    /// blank rows there after the hold layer landed, against one in the
    /// original.
    #[test]
    fn a_pause_after_the_openings_own_feeds_adds_no_rows() {
        let pause = crate::text_hold::TextHoldKind::TimedPause.marker_str();
        let opening = TlkRenderedText::plain(format!("a giant rat!\n\n\"Squeek...\"\n\n{pause}"));
        let rendered = conversation_opening_rendered(&opening, crate::TLK_ASK_WHO_PROMPT);
        assert_eq!(
            rendered.text,
            format!(
                "\nYou see a giant rat!\n\n\"Squeek...\"\n\n{pause}{}",
                crate::TLK_ASK_WHO_PROMPT
            )
        );
        assert_eq!(rendered.glyphs.len(), rendered.text.len());
        assert_eq!(
            crate::text_hold::strip_hold_markers(&rendered.text),
            conversation_opening_rendered(
                &TlkRenderedText::plain("a giant rat!\n\n\"Squeek...\"\n\n"),
                crate::TLK_ASK_WHO_PROMPT
            )
            .text,
            "once the hold is over the window reads as it did without one"
        );
    }

    /// `conversation.md §7.7` HEAD + §7.6: a Description GOTO into a head
    /// that opens on ASK-WHO leaves the HEAD quote as the stream's last byte,
    /// and the handler's own opening quote is dropped after it - one quote,
    /// no padding (`town-talk-speaker`, beat `talk`). A quote followed by the
    /// stream's own feeds keeps the ordinary shape.
    #[test]
    fn an_ask_who_after_the_head_quote_continues_it() {
        let rendered = conversation_opening_rendered(
            &TlkRenderedText::plain("a fixture wizard.\n\n\""),
            crate::TLK_ASK_WHO_PROMPT,
        );
        assert_eq!(
            rendered.text,
            "\nYou see a fixture wizard.\n\n\"What is thy name?\"\n\nYou respond-\n:"
        );
        assert_eq!(rendered.glyphs.len(), rendered.text.len());
        let fed = conversation_opening_rendered(
            &TlkRenderedText::plain("a giant rat!\n\n\"Squeek...\"\n\n"),
            crate::TLK_ASK_WHO_PROMPT,
        );
        assert_eq!(
            fed.text,
            format!("\nYou see a giant rat!\n\n\"Squeek...\"\n\n{}", crate::TLK_ASK_WHO_PROMPT)
        );
    }

    /// A pause with no feeds in front of it still gets the blank row, and
    /// feeds after the last hold are trimmed as before.
    #[test]
    fn the_blank_row_is_padded_after_a_bare_pause_and_trailing_feeds_are_trimmed() {
        let pause = crate::text_hold::TextHoldKind::TimedPause.marker_str();
        let bare = conversation_opening_rendered(
            &TlkRenderedText::plain(format!("a rat.{pause}")),
            crate::TLK_KEYWORD_PROMPT,
        );
        assert_eq!(
            bare.text,
            format!("\nYou see a rat.{pause}\n\n{}", crate::TLK_KEYWORD_PROMPT)
        );
        let plain = conversation_opening_rendered(
            &TlkRenderedText::plain("a rat.\n\n\n\n"),
            crate::TLK_KEYWORD_PROMPT,
        );
        assert_eq!(
            plain.text,
            format!("\nYou see a rat.\n\n{}", crate::TLK_KEYWORD_PROMPT)
        );
    }
}
