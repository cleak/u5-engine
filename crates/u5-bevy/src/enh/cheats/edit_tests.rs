//! Edit party and stores (design 7.2.6, 7.5 Phase 3; build plan M8-03):
//! the draft's ranges, clamping and typed entry, the status rules, `[L]`,
//! the `!` marker, Apply and Undo all, the layout, and the face through
//! the real input driver and `cheat_apply`.

use std::collections::BTreeSet;

use bevy::prelude::*;
use u5_runtime::cheats::{CheatEdit, CheatRefusal, MemberEdit, StoredStatus, apply_cheat_edits};
use u5_runtime::enhancements::{FeatureToggles, Toggle};
use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::{
    AvatarStats, EQUIPMENT_EMPTY, EQUIPMENT_SLOT_COUNT, PartyMember, PlayState,
    SAVE_CHARACTER_NAME_LEN,
};

use super::*;

fn member(slot: u8, class_byte: u8, status: u8, hp: u16, max_hp: u16, mana: u8) -> PartyMember {
    PartyMember {
        slot,
        class_byte,
        status,
        climb_stat: 20,
        mana,
        hp,
        max_hp,
        level: 3,
    }
}

/// Avatar, Bard (Poisoned), Mage (Sleeping), Fighter (Dead) and Ranger
/// (Ashes, with an out-of-range STR of 45).
fn party(mut state: PlayState) -> PlayState {
    state.party = vec![
        member(0, b'A', b'G', 50, 240, 3),
        member(1, b'B', b'P', 60, 180, 1),
        member(2, b'M', b'S', 70, 150, 0),
        member(3, b'F', b'D', 0, 150, 0),
        member(4, b'R', b'A', 0, 120, 0),
    ];
    let len = state.party.len();
    let mut names = vec![[0u8; SAVE_CHARACTER_NAME_LEN]; len];
    for (index, name) in names.iter_mut().enumerate() {
        name[..2].copy_from_slice(&[b'M', b'0' + index as u8]);
    }
    state.party_names = names;
    state.party_experience = vec![100, 200, 300, 400, 500];
    state.party_stay_counters = vec![0; len];
    state.avatar_stats = AvatarStats {
        strength: 20,
        dexterity: 20,
        intelligence: 24,
    };
    state.party_strengths = vec![20, 21, 22, 23, 45];
    state.party_intelligence = vec![24, 16, 27, 10, 12];
    state.party_combat_defense = vec![0; len];
    state.party_equipment = vec![[EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]; len];
    state
}

fn town() -> PlayState {
    party(test_state(open_grid(), 5, 5))
}

/// The cursor on `field`.
fn at(draft: &mut EditDraft, field: Field) {
    let index = field_order().iter().position(|f| *f == field).unwrap();
    let now = field_order()
        .iter()
        .position(|f| *f == draft.field())
        .unwrap();
    draft.move_field(index as i32 - now as i32);
    assert_eq!(draft.field(), field);
}

/// Types `value` into the field under the cursor.
fn type_value(draft: &mut EditDraft, value: u32) -> EditOutcome {
    assert_eq!(draft.enter(), EditOutcome::Quiet, "typing starts");
    for byte in value.to_string().bytes() {
        draft.type_digit(byte - b'0');
    }
    draft.enter()
}

fn range_line(min: u32, max: u32) -> EditOutcome {
    EditOutcome::Say(format!("Range {min}..{max}."))
}

/// Every numeric field of the Avatar and the stores.
fn numeric_fields() -> Vec<Field> {
    field_order()
        .into_iter()
        .filter(|f| *f != Field::Status)
        .collect()
}

#[test]
fn every_field_at_both_ends_and_one_step_outside() {
    let state = town();
    for field in numeric_fields() {
        let mut draft = EditDraft::of(&state);
        at(&mut draft, field);
        let (min, max) = draft.range(field).unwrap();
        // Typed: both ends taken, one step outside refused.
        assert_eq!(type_value(&mut draft, max), EditOutcome::Quiet, "{field:?}");
        assert_eq!(draft.value(field), Some(max));
        assert_eq!(type_value(&mut draft, max + 1), range_line(min, max));
        assert_eq!(draft.value(field), Some(max), "{field:?} kept");
        assert_eq!(type_value(&mut draft, min), EditOutcome::Quiet);
        assert_eq!(draft.value(field), Some(min));
        if min > 0 {
            assert_eq!(type_value(&mut draft, min - 1), range_line(min, max));
            assert_eq!(draft.value(field), Some(min));
        }
        // Arrows and pages clamp at the ends.
        draft.step(-1);
        draft.step(-10);
        assert_eq!(draft.value(field), Some(min), "{field:?}");
        type_value(&mut draft, max);
        draft.step(1);
        draft.step(10);
        assert_eq!(draft.value(field), Some(max), "{field:?}");
        // The runtime takes both ends of what the draft allows.
        let mut game = state.clone();
        match apply_cheat_edits(&mut game, &draft.edits()) {
            Ok(_) | Err(CheatRefusal::NothingToDo) => {}
            Err(refusal) => panic!("{field:?} at max: {refusal:?}"),
        }
        let mut draft = EditDraft::of(&state);
        at(&mut draft, field);
        type_value(&mut draft, min);
        let mut game = state.clone();
        match apply_cheat_edits(&mut game, &draft.edits()) {
            Ok(_) | Err(CheatRefusal::NothingToDo) => {}
            Err(refusal) => panic!("{field:?} at min: {refusal:?}"),
        }
    }
}

#[test]
fn the_ranges_follow_the_member() {
    let state = town();
    let mut draft = EditDraft::of(&state);
    assert_eq!(draft.range(Field::Str), Ok((1, 30)));
    assert_eq!(draft.range(Field::Hp), Ok((1, 240)));
    assert_eq!(draft.range(Field::Mp), Ok((0, 24)), "Avatar: INT");
    assert_eq!(draft.range(Field::Xp), Ok((0, 9999)));
    assert_eq!(draft.range(Field::Food), Ok((0, 9999)));
    assert_eq!(draft.range(Field::Reagent(7)), Ok((0, 99)));
    draft.pick_member(1);
    assert_eq!(draft.range(Field::Mp), Ok((0, 8)), "Bard: INT / 2");
    draft.pick_member(3);
    assert!(draft.range(Field::Mp).is_err(), "a fighter has no target");
    assert!(draft.range(Field::Hp).is_err(), "the dead have no HP range");
    at(&mut draft, Field::Hp);
    assert!(matches!(draft.step(1), EditOutcome::Say(_)));
    assert!(matches!(draft.enter(), EditOutcome::Say(_)));
    assert!(draft.edits().is_empty());
    // MP follows the INT the draft sets.
    draft.pick_member(0);
    at(&mut draft, Field::Int);
    type_value(&mut draft, 10);
    assert_eq!(draft.range(Field::Mp), Ok((0, 10)));
}

#[test]
fn dead_zeroes_hp_and_good_on_the_dead_sets_max() {
    let state = town();
    let mut draft = EditDraft::of(&state);
    at(&mut draft, Field::Status);
    assert_eq!(draft.status_key(StoredStatus::Dead), EditOutcome::Quiet);
    assert_eq!(draft.members()[0].hp, 0);
    draft.pick_member(3);
    assert_eq!(draft.status_key(StoredStatus::Good), EditOutcome::Quiet);
    assert_eq!(draft.members()[3].hp, 150);
    let edits = draft.edits();
    assert_eq!(
        edits,
        vec![
            CheatEdit::Member {
                slot: 0,
                fields: MemberEdit {
                    status: Some(StoredStatus::Dead),
                    ..MemberEdit::default()
                }
            },
            CheatEdit::Member {
                slot: 3,
                fields: MemberEdit {
                    status: Some(StoredStatus::Good),
                    ..MemberEdit::default()
                }
            },
        ]
    );
    let mut game = state.clone();
    apply_cheat_edits(&mut game, &edits).unwrap();
    assert_eq!((game.party[0].status, game.party[0].hp), (b'D', 0));
    assert_eq!((game.party[3].status, game.party[3].hp), (b'G', 150));
    assert_eq!(draft.refreshed(&game).members(), draft.members());
}

#[test]
fn ashes_is_shown_but_cannot_be_chosen() {
    let state = town();
    let mut draft = EditDraft::of(&state);
    at(&mut draft, Field::Status);
    let mut seen = BTreeSet::new();
    for _ in 0..8 {
        draft.step(1);
        seen.insert(draft.members()[0].status);
    }
    for _ in 0..8 {
        draft.step(-1);
        seen.insert(draft.members()[0].status);
    }
    assert_eq!(seen, BTreeSet::from(*b"GPSD"));
    draft.undo_all();
    draft.pick_member(4);
    assert!(matches!(draft.step(1), EditOutcome::Say(_)));
    for status in StoredStatus::ALL {
        assert!(matches!(draft.status_key(status), EditOutcome::Say(_)));
    }
    assert_eq!(draft.members()[4].status, b'A');
    assert!(draft.edits().is_empty());
    let texts: Vec<String> = draft.runs().into_iter().map(|run| run.text).collect();
    assert!(
        texts
            .iter()
            .any(|text| text.starts_with("Status") && text.ends_with('A')),
        "{texts:?}"
    );
}

#[test]
fn status_cannot_be_edited_in_combat() {
    let mut state = town();
    state.combat_active = true;
    let mut draft = EditDraft::of(&state);
    at(&mut draft, Field::Status);
    for delta in [1, -1, 10] {
        assert_eq!(
            draft.step(delta),
            EditOutcome::Say("Not during combat.".into())
        );
    }
    assert!(matches!(
        draft.status_key(StoredStatus::Dead),
        EditOutcome::Say(_)
    ));
    assert!(draft.edits().is_empty());
    // Other fields still edit in combat.
    at(&mut draft, Field::Str);
    draft.step(1);
    assert_eq!(draft.edits().len(), 1);
}

#[test]
fn recompute_level_at_the_published_points() {
    for (xp, level) in [(0, 1), (99, 1), (100, 2), (6399, 7), (6400, 8), (9999, 8)] {
        let state = town();
        let mut draft = EditDraft::of(&state);
        at(&mut draft, Field::Xp);
        type_value(&mut draft, xp);
        assert!(matches!(draft.recompute_level(), EditOutcome::Say(_)));
        let max = 30 * u16::from(level);
        let shown = &draft.members()[0];
        assert_eq!((shown.level, shown.max_hp, shown.hp), (level, max, max));
        let edits = draft.edits();
        assert!(
            edits.contains(&CheatEdit::RecomputeLevel { slot: 0 }),
            "{edits:?}"
        );
        let mut game = state.clone();
        apply_cheat_edits(&mut game, &edits).unwrap();
        assert_eq!(game.party[0].level, level, "XP {xp}");
        assert_eq!(game.party[0].max_hp, max, "XP {xp}");
        assert_eq!(game.party[0].hp, max);
        assert_eq!(game.party_experience[0], xp as u16);
    }
    // A typed HP after [L] is sent after it.
    let state = town();
    let mut draft = EditDraft::of(&state);
    draft.recompute_level();
    at(&mut draft, Field::Hp);
    type_value(&mut draft, 5);
    let mut game = state.clone();
    apply_cheat_edits(&mut game, &draft.edits()).unwrap();
    assert_eq!((game.party[0].max_hp, game.party[0].hp), (60, 5));
}

#[test]
fn slot_zero_edits_reach_all_four_mirrors() {
    let state = town();
    let mut draft = EditDraft::of(&state);
    for (field, value) in [(Field::Str, 11), (Field::Dex, 12), (Field::Int, 13)] {
        at(&mut draft, field);
        type_value(&mut draft, value);
    }
    let mut game = state.clone();
    apply_cheat_edits(&mut game, &draft.edits()).unwrap();
    assert_eq!(
        game.avatar_stats,
        AvatarStats {
            strength: 11,
            dexterity: 12,
            intelligence: 13
        }
    );
    assert_eq!(game.party_strengths[0], 11);
    assert_eq!(game.party[0].climb_stat, 12);
    assert_eq!(game.party_intelligence[0], 13);
}

#[test]
fn an_untouched_out_of_range_value_is_marked_and_preserved() {
    let mut state = town();
    state.food = 12_000;
    let mut draft = EditDraft::of(&state);
    draft.pick_member(4);
    assert!(draft.out_of_range(Field::Str), "STR 45");
    assert!(draft.out_of_range(Field::Food), "food 12000");
    assert!(!draft.out_of_range(Field::Dex));
    let marks = draft.runs().iter().filter(|run| run.text == "!").count();
    assert_eq!(marks, 2);
    // Another field edited: the marked ones are not sent.
    at(&mut draft, Field::Dex);
    draft.step(-1);
    let mut game = state.clone();
    apply_cheat_edits(&mut game, &draft.edits()).unwrap();
    assert_eq!(game.party_strengths[4], 45);
    assert_eq!(game.food, 12_000);
    assert_eq!(game.party[4].climb_stat, 19);
    // Editing a marked field clamps it into range.
    at(&mut draft, Field::Str);
    draft.step(-1);
    assert_eq!(draft.value(Field::Str), Some(30));
    at(&mut draft, Field::Food);
    draft.step(1);
    assert_eq!(draft.value(Field::Food), Some(9999));
}

#[test]
fn an_empty_apply_sends_nothing_and_undo_restores() {
    let state = town();
    let mut draft = EditDraft::of(&state);
    assert_eq!(draft.apply(), EditOutcome::Apply(Vec::new()));
    // Changed and changed back: still nothing.
    at(&mut draft, Field::Gold);
    draft.step(1);
    draft.step(-1);
    assert!(draft.edits().is_empty());
    let mut game = state.clone();
    assert_eq!(
        apply_cheat_edits(&mut game, &draft.edits()),
        Err(CheatRefusal::NothingToDo)
    );
    // Undo all throws the draft away.
    draft.step(5);
    draft.pick_member(2);
    at(&mut draft, Field::Str);
    draft.step(1);
    draft.recompute_level();
    assert!(!draft.edits().is_empty());
    draft.undo_all();
    assert!(draft.edits().is_empty());
    assert!(!draft.recompute_pending(2));
}

#[test]
fn only_the_travelling_party_is_shown() {
    let mut state = town();
    let record = u5_runtime::default_party_roster(1)[0].clone();
    state.party_roster = vec![record; 16];
    let draft = EditDraft::of(&state);
    assert_eq!(draft.members().len(), 5);
    let rows: BTreeSet<u32> = draft
        .runs()
        .iter()
        .filter(|run| (TABLE_FIRST_ROW..FIELDS_FIRST_ROW - 1).contains(&run.row))
        .map(|run| run.row)
        .collect();
    assert_eq!(rows.len(), 5);
}

/// Every run sits inside its box: within the box's 38 cells, on a box row
/// (never a divider), and no two runs share a cell.
#[test]
fn the_layout_fits_its_boxes() {
    let mut state = town();
    state.food = 12_000;
    state.party.push(member(5, b'T', b'G', 999, 999, 0));
    state.party_names.push(*b"LONGNAME9");
    state.party_experience.push(9999);
    state.party_strengths.push(99);
    state.party_intelligence.push(99);
    let mut drafts = Vec::new();
    for field in field_order() {
        for slot in [4, 5] {
            let mut draft = EditDraft::of(&state);
            draft.pick_member(slot);
            at(&mut draft, field);
            drafts.push(draft.clone());
            if draft.enter() == EditOutcome::Quiet {
                for digit in [9, 9, 9, 9, 9] {
                    draft.type_digit(digit);
                }
                drafts.push(draft);
            }
        }
    }
    for draft in drafts {
        let mut cells = BTreeSet::new();
        for run in draft.runs() {
            assert!(run.row >= 1 && run.row <= 22, "{run:?}");
            assert!(!EDIT_DIVIDERS.contains(&run.row), "{run:?}");
            let end = run.col + run.text.len() as u32;
            assert!(end <= 38, "{run:?} ends at {end}");
            for col in run.col..end {
                assert!(cells.insert((col, run.row)), "{run:?} overlaps");
            }
        }
    }
    for line in HELP_LINES {
        assert!(line.len() <= 36, "{line}");
    }
    assert!(EDIT_FOOTER.len() + 2 <= 38);
}

// ---------------------------------------------------------------------------
// The face through the real input driver and `cheat_apply`.

mod app {
    use super::*;
    use crate::enh::add_enhancements;
    use crate::enh::cheats::{CheatPage, CheatSession, PageMode};
    use crate::enh::overlay::OverlayId;
    use crate::enh::overlay::tests::Game;
    use crate::enh::toast::ShellBadges;

    fn toggles(edit: bool) -> FeatureToggles {
        let mut toggles = FeatureToggles::CLASSIC;
        toggles.set(Toggle::CheatsEnabled, true);
        toggles.set(Toggle::CheatsMaxGold, true);
        toggles.set(Toggle::CheatsStatEdit, edit);
        toggles
    }

    fn game(edit: bool) -> Game {
        let mut game = Game::new(false);
        let mut launch = crate::enh::VisualLaunchConfig::classic();
        launch.enhancements.toggles = toggles(edit);
        add_enhancements(&mut game.app, &launch);
        {
            let mut visual = game.visual_mut();
            let fresh = party(visual.state.clone());
            visual.state = fresh;
        }
        game.frame();
        game.frame();
        game
    }

    fn tap(game: &mut Game, key: KeyCode) {
        game.press(key);
        game.frame();
        game.release(key);
        game.frame();
    }

    fn open(game: &mut Game) {
        game.press(KeyCode::ControlLeft);
        game.press(KeyCode::F12);
        game.frame();
        game.release(KeyCode::F12);
        game.release(KeyCode::ControlLeft);
        game.frame();
        assert_eq!(game.open(), Some(OverlayId::Cheats));
    }

    fn page(game: &Game) -> &CheatPage {
        game.app.world().resource::<CheatPage>()
    }

    fn tainted(game: &Game) -> bool {
        game.app.world().resource::<CheatSession>().tainted
    }

    fn badge(game: &Game) -> bool {
        game.app
            .world()
            .get_resource::<ShellBadges>()
            .is_some_and(|badges| badges.modified)
    }

    /// Everything the editor can write (the live world keeps animating
    /// while the page is open, so the whole state is not compared).
    fn edited(game: &Game) -> String {
        let s = &game.visual().state;
        format!(
            "{:?} {:?} {:?} {:?} {:?} {} {} {} {} {} {:?} {:?}",
            s.party,
            s.avatar_stats,
            s.party_strengths,
            s.party_intelligence,
            s.party_experience,
            s.food,
            s.gold,
            s.keys,
            s.gems,
            s.torches,
            s.equipment_stock,
            s.reagents
        )
    }

    #[test]
    fn the_entry_exists_only_with_stat_edit() {
        let mut off = game(false);
        open(&mut off);
        tap(&mut off, KeyCode::KeyE);
        assert_eq!(page(&off).mode(), PageMode::Main);
        assert!(page(&off).edit().is_none());

        let mut on = game(true);
        open(&mut on);
        tap(&mut on, KeyCode::KeyE);
        assert_eq!(page(&on).mode(), PageMode::Edit);
        assert_eq!(page(&on).edit().unwrap().members().len(), 5);
        tap(&mut on, KeyCode::Escape);
        assert_eq!(page(&on).mode(), PageMode::Main);
        assert_eq!(on.open(), Some(OverlayId::Cheats), "Esc goes back, not out");
    }

    #[test]
    fn an_empty_apply_is_nothing_to_do_and_leaves_the_game_untainted() {
        let mut game = game(true);
        open(&mut game);
        tap(&mut game, KeyCode::KeyE);
        let before = edited(&game);
        tap(&mut game, KeyCode::KeyA);
        assert_eq!(page(&game).result(), Some("Nothing to change."));
        assert!(!tainted(&game));
        assert!(!badge(&game));
        assert_eq!(edited(&game), before);
    }

    #[test]
    fn apply_changes_the_game_once_and_undo_drops_the_draft() {
        let mut game = game(true);
        open(&mut game);
        tap(&mut game, KeyCode::KeyE);
        let turn = game.visual().state.turn;
        // STR of the Avatar: up two, typed 25 over it, then Apply.
        tap(&mut game, KeyCode::ArrowRight);
        tap(&mut game, KeyCode::ArrowRight);
        assert_eq!(
            game.visual().state.avatar_stats.strength,
            20,
            "a draft only"
        );
        tap(&mut game, KeyCode::Enter);
        tap(&mut game, KeyCode::Digit2);
        tap(&mut game, KeyCode::Digit5);
        tap(&mut game, KeyCode::Enter);
        tap(&mut game, KeyCode::KeyA);
        let state = &game.visual().state;
        assert_eq!(state.avatar_stats.strength, 25);
        assert_eq!(state.party_strengths[0], 25);
        assert_eq!(state.turn, turn, "no key reached the game");
        assert_eq!(page(&game).result(), Some("Changes applied."));
        assert!(tainted(&game));
        assert!(
            page(&game).edit().unwrap().edits().is_empty(),
            "fresh draft"
        );

        // Member 2, Undo all: nothing changes.
        tap(&mut game, KeyCode::Digit2);
        tap(&mut game, KeyCode::ArrowLeft);
        tap(&mut game, KeyCode::KeyU);
        let before = game.visual().state.party_strengths[1];
        tap(&mut game, KeyCode::KeyA);
        assert_eq!(game.visual().state.party_strengths[1], before);
        assert_eq!(page(&game).result(), Some("Nothing to change."));
    }

    #[test]
    fn a_typed_value_out_of_range_shows_the_range() {
        let mut game = game(true);
        open(&mut game);
        tap(&mut game, KeyCode::KeyE);
        tap(&mut game, KeyCode::Enter);
        tap(&mut game, KeyCode::Digit3);
        tap(&mut game, KeyCode::Digit1);
        tap(&mut game, KeyCode::Enter);
        assert_eq!(page(&game).result(), Some("Range 1..30."));
        assert!(page(&game).edit().unwrap().edits().is_empty());
    }

    #[test]
    fn switching_stat_edit_off_returns_to_the_main_face_and_refuses_apply() {
        let mut game = game(true);
        open(&mut game);
        tap(&mut game, KeyCode::KeyE);
        tap(&mut game, KeyCode::ArrowRight);
        game.app
            .insert_resource(crate::enh::Toggles(toggles(false)));
        game.frame();
        tap(&mut game, KeyCode::KeyA);
        assert_eq!(page(&game).mode(), PageMode::Main);
        assert_eq!(game.visual().state.avatar_stats.strength, 20);
    }
}
