//! The journal's no-spoiler suite across its three surfaces (foundation
//! 1.4.7, design 5.6; build plan M5-06): the page, the side panel and the
//! offline report, on the fresh save at tiers 1 and 2 with the page and the
//! panel both on.
//!
//! - no dungeon, Shadowlord, regalia, tool, companion, town or Word of
//!   Power name, no total and no Endgame section;
//! - only Virtues and People cycle, every virtue reads `-` and the header
//!   reads "Nothing recorded yet";
//! - one event at a time adds only its own row's words, and a Shadowlord
//!   slot of 1..8 or an unrecruited roster record adds nothing;
//! - tiers 3 and 4 carry `SPOILERS` in the page title and the report's
//!   first line.
//!
//! The surfaces are read from the same snapshot the game builds, from a
//! fresh SAVED.GAM image (the factory seed with the Avatar named) and from
//! the live shell.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bevy::input::ButtonState;
use bevy::window::{Window, WindowResolution};
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, JOURNAL_P3, JOURNAL_P3B, JournalPanelSide, LockState,
    ResolveInput, resolve_as_shipped,
};
use u5_runtime::quest_journal::CompanionPlace;
use u5_runtime::quest_journal::report::format_text;
use u5_runtime::test_fixtures::{
    journal_undiscovered_names, open_grid, saved_game_seed_bytes, test_state,
};
use u5_runtime::{
    SAVE_AVATAR_NAME_OFFSET, SAVE_CHARACTER_RECORD_LEN, SAVE_CLIMBING_GEAR_OFFSET,
    SAVE_INN_REGISTRY_OFFSET, SAVE_PARTY_SIZE_OFFSET, SAVE_ROSTER_OFFSET,
    SAVE_SHADOWLORD_HIDEOUTS_OFFSET, SAVE_SHRINE_ORDAINED_MASK_OFFSET, SAVE_SPECIAL_ITEM_OFFSET,
    SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET, SPECIAL_ITEM_CROWN_LB_INDEX,
};

use super::*;
use crate::enh::equivalence::{Scene, Shell};
use crate::enh::side::{PANEL_BODY_ROWS, PanelContent, PanelKind, SideLayout, SidePanelProvider};

const LOW_TIERS: [SpoilerTier; 2] = [SpoilerTier::Progress, SpoilerTier::Goals];

// ---------------------------------------------------------------------------
// The three surfaces of one snapshot.

/// The fresh save: the factory seed, the Avatar named and alone.
fn fresh_save() -> Vec<u8> {
    let mut bytes = saved_game_seed_bytes(0, 0, 10, 10);
    assert_eq!(SAVE_AVATAR_NAME_OFFSET, SAVE_ROSTER_OFFSET);
    bytes[SAVE_ROSTER_OFFSET..SAVE_ROSTER_OFFSET + 6].copy_from_slice(b"Avatar");
    bytes[SAVE_PARTY_SIZE_OFFSET] = 1;
    bytes
}

fn snapshot(bytes: &[u8]) -> QuestSnapshot {
    QuestSnapshot::from_save_bytes(bytes).unwrap()
}

/// The page's text in every view of `model`: each section of the cycle and
/// each cursor position, with the title and the key help.
fn page_texts(model: &JournalModel) -> Vec<String> {
    let mut title = text::TITLE.to_string();
    if model.spoilers {
        title.push(' ');
        title.push_str(text::SPOILERS);
    }
    let mut texts = vec![title];
    for section in &model.sections {
        for cursor in 0..section.rows.len().max(1) {
            let mut view = PageView::new(Some(model), Some(section));
            view.cursor = cursor;
            view.scroll = (cursor + 1).saturating_sub(VISIBLE_ROWS);
            texts.push(key_help(&view).to_string());
            texts.extend(page_cells(&view).into_iter().map(|cell| cell.text));
        }
    }
    texts
}

/// The side panel's lines (its title included), as the provider gives
/// them.
fn panel_texts(snap: &QuestSnapshot) -> Vec<String> {
    let mut lines = vec![PANEL_TITLE.to_string()];
    lines.extend(SidePanelModel::from_snapshot(snap).lines());
    lines
}

fn report_texts(model: &JournalModel) -> Vec<String> {
    format_text(model).lines().map(str::to_string).collect()
}

/// Every surface's text for `snap` at `tier`, labelled.
fn surfaces(snap: &QuestSnapshot, tier: SpoilerTier) -> [(&'static str, Vec<String>); 3] {
    let model = derive(snap, tier);
    [
        ("page", page_texts(&model)),
        ("panel", panel_texts(snap)),
        ("report", report_texts(&model)),
    ]
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
        .filter(|word| !word.is_empty())
}

/// The 1.4.7 audit of one surface's text at tier 1 or 2 on a fresh save.
fn audit_fresh(label: &str, texts: &[String]) {
    let names = journal_undiscovered_names();
    for text in texts {
        for name in &names {
            assert!(
                !text.contains(name.as_str()),
                "{label}: {name:?} in {text:?}"
            );
        }
        // No total: no "n/8", no "n of 8", no count at all on a fresh save.
        assert!(!text.contains('/'), "{label}: {text:?}");
        assert!(!words(text).any(|word| word == "of"), "{label}: {text:?}");
        // The numbers a fresh save shows: the Avatar's standing, as Ctrl+K
        // prints it (its own cell, or its report line), and the report's
        // tier in its first line.
        let standing = text.contains(text::KARMA_LABEL)
            || text.trim() == "0"
            || text.starts_with(text::REPORT_TITLE);
        assert!(
            standing || !text.chars().any(|c| c.is_ascii_digit()),
            "{label}: a count in {text:?}"
        );
        assert!(!text.contains("Endgame"), "{label}: {text:?}");
        assert!(!text.contains(text::SPOILERS), "{label}: {text:?}");
    }
}

#[test]
fn the_fresh_save_names_nothing_on_any_surface_at_tiers_1_and_2() {
    let snap = snapshot(&fresh_save());
    for tier in LOW_TIERS {
        let model = derive(&snap, tier);
        assert_eq!(model.header, text::NOTHING_RECORDED, "{tier:?}");
        let ids: Vec<SectionId> = model.sections.iter().map(|s| s.id).collect();
        assert_eq!(ids, [SectionId::Virtues, SectionId::People], "{tier:?}");
        let virtues = model.section(SectionId::Virtues).unwrap();
        assert_eq!(virtues.rows.len(), 8);
        for row in &virtues.rows {
            assert_eq!(row.mark.glyph, '-', "{tier:?}");
        }
        for (surface, texts) in surfaces(&snap, tier) {
            audit_fresh(&format!("{tier:?} {surface}"), &texts);
        }
        // The page's own cells: the header, and a `-` beside every virtue.
        let view = PageView::new(Some(&model), Some(virtues));
        let cells = page_cells(&view);
        assert!(
            cells
                .iter()
                .any(|cell| cell.row == HEADER_ROW && cell.text == text::NOTHING_RECORDED)
        );
        for virtue in u5_runtime::ShrineVirtue::ALL {
            let row = cells
                .iter()
                .find(|cell| cell.text == virtue.name())
                .unwrap_or_else(|| panic!("{tier:?}: {virtue:?} on the page"))
                .row;
            assert!(
                cells.iter().any(|cell| cell.row == row && cell.text == "-"),
                "{tier:?}: {virtue:?} reads -"
            );
        }
        // The report: the same header, every virtue marked `-`.
        let report = format_text(&model);
        assert!(report.contains(text::NOTHING_RECORDED));
        for virtue in u5_runtime::ShrineVirtue::ALL {
            let line = report
                .lines()
                .find(|line| words(line).any(|word| word == virtue.name()))
                .unwrap();
            assert!(line.trim_start().starts_with('-'), "{line:?}");
        }
    }
    // The panel: the same words, every virtue marked `-`.
    let panel = panel_texts(&snap);
    assert!(panel.iter().any(|line| line.starts_with("Nothing")));
    for virtue in u5_runtime::ShrineVirtue::ALL {
        let line = panel
            .iter()
            .find(|line| line.starts_with(virtue.name()))
            .unwrap();
        assert!(line.trim_end().ends_with('-'), "{line:?}");
    }
}

#[test]
fn tiers_3_and_4_say_spoilers_on_the_page_and_in_the_report() {
    let snap = snapshot(&fresh_save());
    for tier in SpoilerTier::ALL {
        let model = derive(&snap, tier);
        let page = page_texts(&model);
        let report = format_text(&model);
        let first = report.lines().next().unwrap();
        assert_eq!(page[0].contains(text::SPOILERS), tier.is_spoiler());
        assert_eq!(first.contains(text::SPOILERS), tier.is_spoiler());
        assert_eq!(
            report.matches(text::SPOILERS).count(),
            usize::from(tier.is_spoiler()),
            "{tier:?}: only the first line"
        );
        // The panel never shows more than tier 2: it has no tier.
        assert!(
            !panel_texts(&snap)
                .iter()
                .any(|l| l.contains(text::SPOILERS))
        );
    }
}

// ---------------------------------------------------------------------------
// One event at a time.

fn record(slot: usize) -> usize {
    SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN
}

/// The fresh save with one event written into it, as the game writes it.
fn with(apply: impl Fn(&mut Vec<u8>)) -> Vec<u8> {
    let mut bytes = fresh_save();
    apply(&mut bytes);
    bytes
}

fn name(bytes: &mut [u8], slot: usize, name: &[u8]) {
    bytes[record(slot)..record(slot) + name.len()].copy_from_slice(name);
}

/// The words a surface shows that the fresh one does not.
fn added_words(base: &[String], now: &[String]) -> BTreeSet<String> {
    let had: BTreeSet<&str> = base.iter().flat_map(|t| words(t)).collect();
    now.iter()
        .flat_map(|t| words(t))
        .filter(|word| !had.contains(word))
        .map(str::to_string)
        .collect()
}

/// The words of the rows `event` adds or changes in its model, of the
/// sections it adds (title and headings), and of the header.
fn own_words(base: &JournalModel, event: &JournalModel) -> BTreeSet<String> {
    let mut own = BTreeSet::new();
    let mut add = |text: &str| own.extend(words(text).map(str::to_string));
    // The page's Shard column words, which only a Shadowlord row shows.
    add(text::SHARD_CARRIED_CELL);
    add(text::SHARD_DESTROYED_CELL);
    add(&event.header);
    for section in &event.sections {
        let old = base.section(section.id);
        if old.is_none() {
            add(section.id.title());
            for (_, head) in super::headings(section.id, event.tier) {
                add(head);
            }
        }
        for row in &section.rows {
            let before = old.and_then(|old| old.rows.iter().find(|r| r.key == row.key));
            if before != Some(row) {
                for text in row.strings() {
                    add(text);
                }
            }
        }
    }
    own
}

#[test]
fn one_event_adds_only_its_own_row_on_every_surface() {
    let base_bytes = fresh_save();
    let base = snapshot(&base_bytes);
    // (event, its bytes, the panel's own words for it)
    let events: Vec<(&str, Vec<u8>, &[&str])> = vec![
        (
            "ordained at one shrine",
            with(|b| b[SAVE_SHRINE_ORDAINED_MASK_OFFSET] = 0b10),
            &["done", "in", "progress", "0", "1"],
        ),
        (
            "one seal opened",
            with(|b| b[SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET + 3] = 0x80),
            &["Seals", "open", "1"],
        ),
        (
            "one Shadowlord vanquished",
            with(|b| b[SAVE_SHADOWLORD_HIDEOUTS_OFFSET + 1] = 0xFF),
            &["Vanquished", "1"],
        ),
        (
            "a shard",
            with(|b| b[SAVE_SPECIAL_ITEM_OFFSET + 8] = 1),
            &[],
        ),
        (
            "the Crown",
            with(|b| b[SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_CROWN_LB_INDEX] = 1),
            &[],
        ),
        (
            "a tool carried",
            with(|b| b[SAVE_CLIMBING_GEAR_OFFSET] = 1),
            &[],
        ),
        (
            "a companion lodged",
            with(|b| {
                name(b, 1, b"Shamino");
                b[SAVE_INN_REGISTRY_OFFSET + SAVE_CHARACTER_RECORD_LEN] = 2;
            }),
            &[],
        ),
    ];
    for tier in LOW_TIERS {
        let base_model = derive(&base, tier);
        let base_surfaces = surfaces(&base, tier);
        for (event, bytes, panel_own) in &events {
            let snap = snapshot(bytes);
            let model = derive(&snap, tier);
            let mut own = own_words(&base_model, &model);
            // The Crown's panel line is its regalia row's label.
            own.extend(panel_own.iter().map(|w| (*w).to_string()));
            for ((surface, before), (_, after)) in base_surfaces.iter().zip(surfaces(&snap, tier)) {
                let added = added_words(before, &after);
                let stray: Vec<&String> = added.difference(&own).collect();
                assert!(
                    stray.is_empty(),
                    "{tier:?} {event} {surface}: {stray:?} is not the event's own"
                );
                if *surface != "panel" {
                    assert!(
                        !added.is_empty(),
                        "{tier:?} {event} {surface}: nothing added"
                    );
                }
            }
        }
    }
}

/// A Shadowlord at large (slot 1..8) and a roster record nobody recruited
/// or lodged add nothing to any surface at any tier below 3.
#[test]
fn a_shadowlord_at_large_or_an_unrecruited_record_adds_nothing() {
    let base = snapshot(&fresh_save());
    let mut quiet: Vec<(String, Vec<u8>)> = Vec::new();
    for slot in 0..3 {
        for town in 1..=8u8 {
            quiet.push((
                format!("Shadowlord {slot} at {town}"),
                with(|b| b[SAVE_SHADOWLORD_HIDEOUTS_OFFSET + slot] = town),
            ));
        }
    }
    quiet.push((
        "an unrecruited record".into(),
        with(|b| {
            name(b, 2, b"Iolo");
            name(b, 5, b"Dupre");
        }),
    ));
    for tier in LOW_TIERS {
        let base_surfaces = surfaces(&base, tier);
        for (label, bytes) in &quiet {
            let after = surfaces(&snapshot(bytes), tier);
            for ((surface, a), (_, b)) in base_surfaces.iter().zip(after.iter()) {
                assert_eq!(a, b, "{tier:?} {label} {surface}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The live shell.

fn launch(tier: u8) -> VisualLaunchConfig {
    let cli = [
        CliSetting::Enable("journal.page".into()),
        CliSetting::Enable("journal.side_panel".into()),
        CliSetting::Set {
            key: "journal.spoiler_tier".into(),
            value: tier.to_string(),
        },
    ];
    let enhancements = resolve_as_shipped(
        ResolveInput {
            stored: None,
            cli: &cli,
            lock: LockState::Off,
            caps: HostCapabilities::ALL,
        },
        &[JOURNAL_P3, JOURNAL_P3B],
    );
    assert!(enhancements.notes.is_empty(), "{:?}", enhancements.notes);
    VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    }
}

fn key(shell: &mut Shell, keys: &[(KeyCode, ButtonState)]) {
    use crate::enh::equivalence::{Owner, Timeline};
    let mut timeline = Timeline::new(1);
    for (code, state) in keys {
        timeline.event(0, *code, *state, false, Owner::Feature);
    }
    shell.step(timeline.frame(0));
}

fn tap(shell: &mut Shell, code: KeyCode) {
    key(shell, &[(code, ButtonState::Pressed)]);
    key(shell, &[(code, ButtonState::Released)]);
}

/// The shell from a fresh game with the page and the panel on: the panel
/// shown beside the frame, the page opened with Ctrl+J and cycled with
/// Right, audited at every stop; the live model is the fresh save's.
#[test]
fn the_live_page_and_panel_spoil_nothing_on_a_fresh_game() {
    let state = test_state(open_grid(), 5, 5);
    let snap = QuestSnapshot::from_play_state(&state);
    assert_eq!(snap.ordained_mask | snap.codex_mask, 0);
    assert!(
        snap.companions
            .iter()
            .all(|c| c.place == CompanionPlace::Party)
    );
    for tier in [1, 2] {
        let mut shell = Shell::new(state.clone(), &Scene::synthetic(), &launch(tier), |_| {});
        shell.app.world_mut().spawn(Window {
            resolution: WindowResolution::new(1920.0, 1080.0),
            ..default()
        });
        for _ in 0..5 {
            shell.step(&[]);
        }
        let granted = shell
            .app
            .world()
            .resource::<SideLayout>()
            .granted
            .iter()
            .any(|(kind, _)| *kind == PanelKind::Journal);
        assert!(granted, "tier {tier}: the panel is shown");
        let provider =
            JournalPanelProvider::new(JournalPanelSide::Auto, Arc::new(AtomicBool::new(false)));
        let Some(PanelContent::Cells(rows)) = provider.snapshot(Some(&shell.visual().state)) else {
            panic!("tier {tier}: no panel content");
        };
        let mut panel = vec![String::from_utf8_lossy(rows.title()).trim_end().to_string()];
        panel.extend(
            (0..PANEL_BODY_ROWS)
                .map(|at| String::from_utf8_lossy(rows.row(at)).trim_end().to_string()),
        );
        audit_fresh(&format!("tier {tier} live panel"), &panel);

        key(
            &mut shell,
            &[
                (KeyCode::ControlLeft, ButtonState::Pressed),
                (KeyCode::KeyJ, ButtonState::Pressed),
            ],
        );
        key(
            &mut shell,
            &[
                (KeyCode::KeyJ, ButtonState::Released),
                (KeyCode::ControlLeft, ButtonState::Released),
            ],
        );
        let world = shell.app.world();
        assert_eq!(
            world.resource::<OverlayState>().open(),
            Some(OverlayId::Journal),
            "tier {tier}: open"
        );
        let ui = world.resource::<QuestJournalUi>();
        let model = ui.model().unwrap().clone();
        assert_eq!(model, derive(&snap, model.tier));
        assert_eq!(model.tier.number(), tier);
        assert_eq!(ui.cycle(), [SectionId::Virtues, SectionId::People]);
        let mut seen = Vec::new();
        for _ in 0..3 {
            let ui = shell.app.world().resource::<QuestJournalUi>();
            seen.push(ui.section_id());
            let mut texts: Vec<String> =
                page_cells(&ui.view()).into_iter().map(|c| c.text).collect();
            texts.push(key_help(&ui.view()).to_string());
            audit_fresh(
                &format!("tier {tier} live page {:?}", ui.section_id()),
                &texts,
            );
            tap(&mut shell, KeyCode::ArrowRight);
        }
        assert_eq!(
            seen,
            [SectionId::Virtues, SectionId::People, SectionId::Virtues]
        );
        audit_fresh(&format!("tier {tier} report"), &report_texts(&model));
    }
}
