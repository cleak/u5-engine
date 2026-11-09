//! The enhancement catalogue: every toggle and valued setting, as data
//! (Phase B design 1.3.2, 1.4.1 and 1.4.2).
//!
//! The catalogue is the single list of setting keys. One `toggles!`
//! invocation lists every boolean toggle; the [`Toggle`] enum, the
//! [`TOGGLE_KEYS`] table and the [`TOGGLES`] rows are generated from it, so
//! a toggle cannot exist without a full row. Valued settings are
//! [`OptionSpec`] rows in [`OPTIONS`].
//!
//! Each row carries:
//!
//! - its **effect class** ([`EffectClass`]): (a) presentation, (b) input
//!   driven or (c) state altering, the user's three classes of
//!   2026-11-01. A setting of the foundation itself has no class (`-` in
//!   the design's tables);
//! - its **apply point** ([`ApplyAt`]): when a change made while the game
//!   runs takes effect. File and command-line edits always apply at the
//!   next launch;
//! - its **tier** ([`Tier`]): what a preset may turn on. Deviation and
//!   cheat rows are never set by a preset, with the one user-approved
//!   exception [`PRESET_DEVIATION_EXCEPTION`] (U6);
//! - its **shipping phase** ([`Phase`]). A row whose phase is not in
//!   [`SHIPPED_PHASES`] is hidden: no page lists it, the resolver keeps a
//!   stored value but uses the default, and every preset skips it (U13).
//!
//! This module holds only chapter 1's own rows (`[enhancements]` and
//! `[keys]`). Each feature's first package adds its section's rows here;
//! the presets already name those keys, and skip them until the rows
//! exist and ship.
//!
//! Classic, the default, is every key at its default, which is
//! [`FeatureToggles::CLASSIC`]: no toggle on.
//!
//! [`settings_table`] renders the catalogue as the table in
//! `docs/settings.md`; a test keeps the two equal.

use super::source::Capability;

/// The three effect classes (user requirement, 2026-11-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectClass {
    /// (a) reads state and never writes it.
    Presentation,
    /// (b) acts only through keys a player could type.
    InputDriven,
    /// (c) changes game state. Cheats only.
    StateAltering,
}

/// When a change made while the game runs takes effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyAt {
    /// Next frame boundary.
    Frame,
    /// Next idle command prompt.
    IdlePrompt,
    /// Next launch only.
    NextLaunch,
}

/// What a preset may turn on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Feature,
    /// Non-faithful view, key shadow or pacing change.
    Deviation,
    Cheat,
}

/// A label shown next to a row wherever settings are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tag {
    /// Shows more of the world than the original's 11x11 window.
    WiderView,
    /// May keep or show what the player has not earned (U12). A row with
    /// this tag is never in a preset.
    SpoilerRisk,
    /// Would replace a key the game reads (reserved rows, U3).
    ReplacesKey,
}

impl Tag {
    /// The plain-language tag text.
    pub fn label(self) -> &'static str {
        match self {
            Tag::WiderView => "wider view",
            Tag::SpoilerRisk => "spoiler risk",
            Tag::ReplacesKey => "replaces a key",
        }
    }
}

/// The design phase that ships a row's feature (U13): the chapter number
/// and the phase name that chapter uses (`P2`, `G2`, `Phase 4d`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Phase {
    pub chapter: u8,
    pub name: &'static str,
}

impl Phase {
    pub const fn new(chapter: u8, name: &'static str) -> Self {
        Self { chapter, name }
    }

    /// Whether this phase has shipped in this build.
    pub fn is_shipped(self) -> bool {
        SHIPPED_PHASES.contains(&self)
    }
}

/// Chapter 1, P2: the catalogue, presets and resolver.
const FOUNDATION_P2: Phase = Phase::new(1, "P2");
/// Chapter 1, P5: launch plumbing and the input router (`[keys]`).
const FOUNDATION_P5: Phase = Phase::new(1, "P5");
/// Chapter 1, P6b: Frozen overlays, optional and later (U1).
const FOUNDATION_P6B: Phase = Phase::new(1, "P6b");

/// Every phase that has shipped in this build. Shipping a phase is adding
/// it here, in the package that completes it; its rows then become
/// visible and presets start to set them.
pub const SHIPPED_PHASES: &[Phase] = &[FOUNDATION_P2];

/// A soft dependency (design 1.4.5): what a feature prefers to have, and
/// what it does when that is absent. No feature requires another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoftDep {
    pub prefers: &'static str,
    pub fallback: &'static str,
}

/// One boolean toggle's catalogue row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToggleSpec {
    pub toggle: Toggle,
    /// `section.name`, the only spelling the resolver accepts.
    pub key: &'static str,
    /// Plain-language label for every settings page.
    pub label: &'static str,
    pub class: EffectClass,
    pub apply: ApplyAt,
    pub tier: Tier,
    pub ships: Phase,
    pub tags: &'static [Tag],
    /// Extra command-line flags, each with a `--no-` form.
    pub cli_aliases: &'static [&'static str],
    pub soft_deps: &'static [SoftDep],
    /// What the host must offer; on a host without it the resolver forces
    /// the toggle off with a note (1.3.14).
    pub needs: &'static [Capability],
}

impl ToggleSpec {
    /// Whether this row is listed and settable in this build (U13).
    pub fn is_shipped(&self) -> bool {
        self.ships.is_shipped()
    }
}

/// Generates [`Toggle`], [`TOGGLE_KEYS`] and [`TOGGLES`] from one list.
/// Every row must name every [`ToggleSpec`] field except `toggle` and
/// `key`, which the macro fills in.
macro_rules! toggles {
    ($($name:ident = $key:literal { $($field:ident: $value:expr),* $(,)? }),* $(,)?) => {
        /// One variant per boolean toggle, in catalogue order. The
        /// discriminant is the toggle's bit in [`FeatureToggles`].
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Toggle { $($name),* }

        /// Every toggle with its key, in catalogue order.
        pub const TOGGLE_KEYS: &[(Toggle, &str)] = &[$((Toggle::$name, $key)),*];

        /// Every toggle's full row, in catalogue order.
        pub const TOGGLES: &[ToggleSpec] = &[$(ToggleSpec {
            toggle: Toggle::$name,
            key: $key,
            $($field: $value),*
        }),*];
    };
}

toggles! {
    EnhancementsFreezeOverlays = "enhancements.freeze_overlays" {
        label: "Pause the world while a menu or map is open",
        class: EffectClass::Presentation,
        apply: ApplyAt::Frame,
        tier: Tier::Deviation,
        ships: FOUNDATION_P6B,
        tags: &[],
        cli_aliases: &[],
        soft_deps: &[],
        needs: &[],
    },
}

// One bit per toggle in a `u128`.
const _: () = assert!(TOGGLES.len() <= 128);

impl Toggle {
    /// This toggle's catalogue row.
    pub fn spec(self) -> &'static ToggleSpec {
        &TOGGLES[self as usize]
    }

    pub fn key(self) -> &'static str {
        self.spec().key
    }

    /// The toggle named by `key`, if the catalogue has one.
    pub fn from_key(key: &str) -> Option<Toggle> {
        TOGGLE_KEYS
            .iter()
            .find(|(_, k)| *k == key)
            .map(|(toggle, _)| *toggle)
    }
}

/// One bit per toggle. Classic is all zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureToggles(u128);

impl FeatureToggles {
    pub const CLASSIC: Self = Self(0);

    pub fn on(self, t: Toggle) -> bool {
        (self.0 >> t as u8) & 1 == 1
    }

    pub fn set(&mut self, t: Toggle, v: bool) {
        let bit = 1u128 << t as u8;
        if v { self.0 |= bit } else { self.0 &= !bit }
    }

    pub fn is_classic(self) -> bool {
        self.0 == 0
    }

    pub fn iter_on(self) -> impl Iterator<Item = Toggle> {
        TOGGLE_KEYS
            .iter()
            .map(|(t, _)| *t)
            .filter(move |t| self.on(*t))
    }

    /// Bit `index`, the toggle at that position of the catalogue the
    /// resolver ran against (for the built-in one, `Toggle as u8`).
    #[cfg(test)]
    pub(crate) fn bit(self, index: usize) -> bool {
        (self.0 >> index) & 1 == 1
    }

    pub(crate) fn set_bit(&mut self, index: usize) {
        self.0 |= 1u128 << index;
    }
}

/// The value type of a valued setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionKind {
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    /// One of the listed words.
    Enum(&'static [&'static str]),
    /// `#rrggbb`.
    Color,
    /// A file path; empty means none.
    Path,
    /// A list drawn from the listed words.
    List(&'static [&'static str]),
    /// A key chord such as `F12` or `Ctrl+Shift+A` (`[keys]`, 1.2.5). The
    /// engine's input router parses it; this crate only stores the text.
    Chord,
}

/// A valued setting's default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionDefault {
    Bool(bool),
    Int(i64),
    /// Enum word, colour, path or chord text.
    Str(&'static str),
    List(&'static [&'static str]),
}

/// One valued setting's catalogue row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OptionSpec {
    /// `section.name`, the only spelling the resolver accepts.
    pub key: &'static str,
    /// Plain-language label for every settings page.
    pub label: &'static str,
    pub kind: OptionKind,
    pub default: OptionDefault,
    /// `None` for a setting of the foundation itself (`-` in 1.4.1).
    pub class: Option<EffectClass>,
    pub apply: ApplyAt,
    /// The tier of the row's ordinary values.
    pub tier: Tier,
    /// Values that are deviation tier although the row is not, such as
    /// `automap.glyphs = gem-faithful`. A preset never sets one.
    pub deviation_values: &'static [&'static str],
    pub ships: Phase,
    pub tags: &'static [Tag],
    pub cli_aliases: &'static [&'static str],
    /// The toggle this setting switches on when it leaves its default,
    /// for a valued setting that is also a feature switch (such as
    /// `display.scale_mode` other than `fit`). `None` for a sub-option or
    /// preference.
    pub toggle: Option<Toggle>,
    /// The toggle this setting is a sub-option of, inert while that toggle
    /// is off (such as `journal.spoiler_tier` under `journal.page`). Every
    /// valued setting with an effect class maps to a toggle or has a
    /// parent, so [`FeatureToggles::is_classic`] sees every setting that
    /// changes anything (1.3.5).
    pub parent: Option<Toggle>,
    /// What the host must offer; on a host without it the resolver keeps
    /// the default with a note (1.3.14). A chord naming an F-key also
    /// needs [`Capability::FunctionKeys`], whatever this lists.
    pub needs: &'static [Capability],
}

impl OptionSpec {
    /// Whether this row is listed and settable in this build (U13).
    pub fn is_shipped(&self) -> bool {
        self.ships.is_shipped()
    }

    /// The tier of `value` for this row.
    pub fn value_tier(&self, value: &str) -> Tier {
        if self.deviation_values.contains(&value) {
            Tier::Deviation
        } else {
            self.tier
        }
    }
}

/// The preset names, in the order `enhancements.preset` lists them.
const PRESET_WORDS: &[&str] = &["classic", "recommended", "everything"];

/// A foundation `[keys]` row (1.2.5). Bindings exist only while their
/// feature is on; the router (P5) owns parsing and collision checks.
const fn key_row(key: &'static str, label: &'static str, chord: &'static str) -> OptionSpec {
    OptionSpec {
        key,
        label,
        kind: OptionKind::Chord,
        default: OptionDefault::Str(chord),
        class: None,
        apply: ApplyAt::Frame,
        tier: Tier::Feature,
        deviation_values: &[],
        ships: FOUNDATION_P5,
        tags: &[],
        cli_aliases: &[],
        toggle: None,
        parent: None,
        needs: &[Capability::Keyboard],
    }
}

/// Every valued setting, in catalogue order.
pub const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        key: "enhancements.schema",
        label: "Settings file format version",
        kind: OptionKind::Int { min: 1, max: 1 },
        default: OptionDefault::Int(1),
        class: None,
        apply: ApplyAt::NextLaunch,
        tier: Tier::Feature,
        deviation_values: &[],
        ships: FOUNDATION_P2,
        tags: &[],
        cli_aliases: &[],
        toggle: None,
        parent: None,
        needs: &[],
    },
    OptionSpec {
        key: "enhancements.preset",
        label: "Preset",
        kind: OptionKind::Enum(PRESET_WORDS),
        default: OptionDefault::Str("classic"),
        class: None,
        apply: ApplyAt::IdlePrompt,
        tier: Tier::Feature,
        deviation_values: &[],
        ships: FOUNDATION_P2,
        tags: &[],
        cli_aliases: &["--preset"],
        toggle: None,
        parent: None,
        needs: &[],
    },
    key_row("keys.menu", "Open the menu", "F12"),
    key_row("keys.fullscreen", "Switch fullscreen", "F11"),
    key_row("keys.map", "Open the map", "Tab"),
    key_row("keys.minimap", "Show or hide the minimap", "Ctrl+M"),
    key_row("keys.journal", "Open the journal", "Ctrl+J"),
    key_row("keys.column", "Show or hide the side column", "Ctrl+P"),
    key_row("keys.auto_member", "Auto-combat for this member", "Ctrl+A"),
    key_row(
        "keys.auto_party",
        "Auto-combat for the party",
        "Ctrl+Shift+A",
    ),
    key_row(
        "keys.auto_until_done",
        "Auto-combat until the fight ends",
        "Ctrl+F",
    ),
    key_row("keys.cheats", "Open the cheats page", "Ctrl+F12"),
];

/// A catalogue row of either kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogueRow {
    Toggle(&'static ToggleSpec),
    Option(&'static OptionSpec),
}

impl CatalogueRow {
    /// The row for `key`, if the catalogue has one.
    pub fn find(key: &str) -> Option<CatalogueRow> {
        if let Some(toggle) = Toggle::from_key(key) {
            return Some(CatalogueRow::Toggle(toggle.spec()));
        }
        OPTIONS
            .iter()
            .find(|option| option.key == key)
            .map(CatalogueRow::Option)
    }

    /// Every row, toggles first, each kind in catalogue order.
    pub fn all() -> impl Iterator<Item = CatalogueRow> {
        TOGGLES
            .iter()
            .map(CatalogueRow::Toggle)
            .chain(OPTIONS.iter().map(CatalogueRow::Option))
    }

    pub fn key(self) -> &'static str {
        match self {
            CatalogueRow::Toggle(spec) => spec.key,
            CatalogueRow::Option(spec) => spec.key,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CatalogueRow::Toggle(spec) => spec.label,
            CatalogueRow::Option(spec) => spec.label,
        }
    }

    pub fn ships(self) -> Phase {
        match self {
            CatalogueRow::Toggle(spec) => spec.ships,
            CatalogueRow::Option(spec) => spec.ships,
        }
    }

    pub fn is_shipped(self) -> bool {
        self.ships().is_shipped()
    }

    pub fn tags(self) -> &'static [Tag] {
        match self {
            CatalogueRow::Toggle(spec) => spec.tags,
            CatalogueRow::Option(spec) => spec.tags,
        }
    }

    pub fn cli_aliases(self) -> &'static [&'static str] {
        match self {
            CatalogueRow::Toggle(spec) => spec.cli_aliases,
            CatalogueRow::Option(spec) => spec.cli_aliases,
        }
    }

    /// The tier a preset would take on by setting this row to `value`
    /// (`None` means "on" for a toggle).
    pub fn tier_for(self, value: Option<&str>) -> Tier {
        match (self, value) {
            (CatalogueRow::Toggle(spec), _) => spec.tier,
            (CatalogueRow::Option(spec), Some(value)) => spec.value_tier(value),
            (CatalogueRow::Option(spec), None) => spec.tier,
        }
    }
}

/// The one deviation-tier key a preset sets: the user approved
/// `combat.fast_forward` in Recommended and Everything (U6, 2026-11-04;
/// kept by U9, 2026-11-06).
pub const PRESET_DEVIATION_EXCEPTION: &str = "combat.fast_forward";

/// What a preset sets a key to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetValue {
    /// A boolean toggle, on.
    On,
    /// A valued setting, set to this word.
    Choice(&'static str),
}

/// One key a preset sets. Presets set toggles only, never preferences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresetEntry {
    pub key: &'static str,
    pub value: PresetValue,
}

const fn on(key: &'static str) -> PresetEntry {
    PresetEntry {
        key,
        value: PresetValue::On,
    }
}

const fn choice(key: &'static str, value: &'static str) -> PresetEntry {
    PresetEntry {
        key,
        value: PresetValue::Choice(value),
    }
}

/// The three presets (design 1.4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Preset {
    /// Every key at its default: the 1.0.1 game.
    Classic,
    /// U2, extended by U11 and U16; the minimap is parked (U18).
    Recommended,
    /// Every feature-tier toggle, plus the U6 exception. Contains
    /// Recommended.
    Everything,
}

/// Every preset, in the order `enhancements.preset` lists them.
pub const PRESETS: &[Preset] = &[Preset::Classic, Preset::Recommended, Preset::Everything];

/// Recommended (U2, U11, U16; 1.4.2). The minimap and the three
/// exploration-memory toggles that fed it are out while it is parked
/// (U18).
const RECOMMENDED: &[PresetEntry] = &[
    on("combat.auto"),
    on("combat.fast_forward"),
    on("journal.clues"),
    choice("journal.clue_highlight", "asked"),
    on("journal.quest_tracker"),
    on("display.sprite_composite"),
];

/// Everything (1.4.2): every feature-tier toggle, options at their
/// defaults, plus `combat.fast_forward` (U6). `automap.minimap` is out
/// while it is parked (U18).
const EVERYTHING: &[PresetEntry] = &[
    choice("display.scale_mode", "sharp"),
    on("display.fullscreen_key"),
    on("menu.overlay"),
    on("menu.named_slots"),
    on("menu.title_layer"),
    on("audio.mute_on_focus_loss"),
    on("journal.page"),
    on("journal.side_panel"),
    on("journal.clues"),
    choice("journal.clue_highlight", "asked"),
    on("journal.quest_tracker"),
    on("automap.dungeon"),
    on("automap.town"),
    on("automap.overworld"),
    on("automap.fullscreen_map"),
    on("combat.auto"),
    on("combat.allies"),
    on("display.sprite_composite"),
    on("gem_map.enhanced"),
    on("combat.fast_forward"),
];

impl Preset {
    /// The word stored as `enhancements.preset` and given to `--preset`.
    pub fn key(self) -> &'static str {
        PRESET_WORDS[self as usize]
    }

    /// Plain-language name for settings pages.
    pub fn label(self) -> &'static str {
        match self {
            Preset::Classic => "Classic",
            Preset::Recommended => "Recommended",
            Preset::Everything => "Everything",
        }
    }

    pub fn from_key(key: &str) -> Option<Preset> {
        PRESETS.iter().copied().find(|preset| preset.key() == key)
    }

    /// Every key the preset names, shipped or not. Keys whose rows are
    /// not in the catalogue yet belong to features still to be built.
    pub const fn entries(self) -> &'static [PresetEntry] {
        match self {
            Preset::Classic => &[],
            Preset::Recommended => RECOMMENDED,
            Preset::Everything => EVERYTHING,
        }
    }

    /// The entries this build applies: those whose row exists and has
    /// shipped. Every other entry is skipped (U13).
    pub fn shipped_entries(self) -> impl Iterator<Item = (&'static PresetEntry, CatalogueRow)> {
        self.entries().iter().filter_map(|entry| {
            CatalogueRow::find(entry.key)
                .filter(|row| row.is_shipped())
                .map(|row| (entry, row))
        })
    }

    /// The toggles this preset turns on in this build.
    pub fn toggles(self) -> FeatureToggles {
        let mut toggles = FeatureToggles::CLASSIC;
        for (entry, row) in self.shipped_entries() {
            match (row, entry.value) {
                (CatalogueRow::Toggle(spec), PresetValue::On) => toggles.set(spec.toggle, true),
                (CatalogueRow::Option(spec), PresetValue::Choice(value)) => {
                    if let Some(toggle) = spec.toggle
                        && spec.default != OptionDefault::Str(value)
                    {
                        toggles.set(toggle, true);
                    }
                }
                // A value of the wrong kind for its row is a catalogue
                // error that the tests reject; apply nothing for it.
                _ => {}
            }
        }
        toggles
    }
}

/// The catalogue as the Markdown table of `docs/settings.md`, one line per
/// row, toggles first, each kind in catalogue order. Hidden rows are listed
/// with the phase that ships them.
pub fn settings_table() -> String {
    let mut table = String::from(
        "| Key | Setting | Type = default | Cls, When | Tier | Alias | Needs | Available |\n\
         |---|---|---|---|---|---|---|---|\n",
    );
    for row in CatalogueRow::all() {
        let (kind, class, apply, tier) = match row {
            CatalogueRow::Toggle(spec) => (
                "bool = false".to_string(),
                Some(spec.class),
                spec.apply,
                tier_text(spec.tier, &[]),
            ),
            CatalogueRow::Option(spec) => (
                option_kind_text(spec),
                spec.class,
                spec.apply,
                tier_text(spec.tier, spec.deviation_values),
            ),
        };
        let class = match class {
            None => "-",
            Some(EffectClass::Presentation) => "a",
            Some(EffectClass::InputDriven) => "b",
            Some(EffectClass::StateAltering) => "c",
        };
        let apply = match apply {
            ApplyAt::Frame => "F",
            ApplyAt::IdlePrompt => "P",
            ApplyAt::NextLaunch => "L",
        };
        let tier = row
            .tags()
            .iter()
            .fold(tier, |text, tag| format!("{text}, {}", tag.label()));
        let needs = match row {
            CatalogueRow::Toggle(spec) => spec.needs,
            CatalogueRow::Option(spec) => spec.needs,
        };
        let available = if row.is_shipped() {
            "yes".to_string()
        } else {
            format!(
                "later (chapter {} {})",
                row.ships().chapter,
                row.ships().name
            )
        };
        table.push_str(&format!(
            "| {} | {} | {kind} | {class}, {apply} | {tier} | {} | {} | {available} |\n",
            row.key(),
            row.label(),
            list_or_dash(row.cli_aliases().iter().copied()),
            list_or_dash(needs.iter().map(|need| need.label())),
        ));
    }
    table
}

fn list_or_dash<'a>(items: impl Iterator<Item = &'a str>) -> String {
    let items: Vec<&str> = items.collect();
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(", ")
    }
}

fn tier_text(tier: Tier, deviation_values: &[&str]) -> String {
    let mut text = match tier {
        Tier::Feature => "feature".to_string(),
        Tier::Deviation => "deviation".to_string(),
        Tier::Cheat => "cheat".to_string(),
    };
    if !deviation_values.is_empty() {
        text.push_str(&format!(" (deviation: {})", deviation_values.join(", ")));
    }
    text
}

/// `type = default` in 1.4.1's notation. `|` is escaped for the table.
fn option_kind_text(spec: &OptionSpec) -> String {
    let kind = match spec.kind {
        OptionKind::Bool => "bool".to_string(),
        OptionKind::Int { min, max } => format!("int {min}..{max}"),
        OptionKind::Enum(words) => format!("enum {}", words.join("\\|")),
        OptionKind::Color => "color".to_string(),
        OptionKind::Path => "path".to_string(),
        OptionKind::List(words) => format!("list of {}", words.join("\\|")),
        OptionKind::Chord => "chord".to_string(),
    };
    let default = match spec.default {
        OptionDefault::Bool(value) => value.to_string(),
        OptionDefault::Int(value) => value.to_string(),
        OptionDefault::Str("") => "\"\"".to_string(),
        OptionDefault::Str(text) => text.to_string(),
        OptionDefault::List(items) => format!("[{}]", items.join(", ")),
    };
    format!("{kind} = {default}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Deviation-tier keys and values named by design 1.4.1, including
    /// rows later packages add. Presets are checked against this list as
    /// well as the catalogue, so a preset cannot pick up one of these
    /// before its row exists.
    const DESIGN_DEVIATION_KEYS: &[(&str, Option<&str>)] = &[
        ("enhancements.freeze_overlays", None),
        ("display.grow_left", None),
        ("automap.glyphs", Some("gem-faithful")),
        ("automap.britannia_revealed", None),
        ("automap.reveal_all", None),
        ("automap.quest_landmarks", Some("always")),
        ("gem_map.glyphs", Some("gem-faithful")),
        ("journal.spoiler_tier", Some("3")),
        ("journal.spoiler_tier", Some("4")),
        ("journal.clue_highlight", Some("new")),
        ("journal.tracker_evidence", Some("npc")),
        ("journal.clue_misses", None),
        ("audio.start_sound_off", None),
        ("audio.start_buffer_on", None),
        ("combat.fast_forward", None),
    ];

    fn entry_value(entry: &PresetEntry) -> Option<&'static str> {
        match entry.value {
            PresetValue::On => None,
            PresetValue::Choice(value) => Some(value),
        }
    }

    #[test]
    fn defaults_equal_classic() {
        assert_eq!(FeatureToggles::default(), FeatureToggles::CLASSIC);
        assert!(FeatureToggles::CLASSIC.is_classic());
        assert_eq!(FeatureToggles::CLASSIC.iter_on().count(), 0);
        assert!(Preset::Classic.entries().is_empty());
        assert_eq!(Preset::Classic.toggles(), FeatureToggles::CLASSIC);

        let preset = CatalogueRow::find("enhancements.preset").unwrap();
        let CatalogueRow::Option(preset) = preset else {
            panic!("enhancements.preset is a valued setting");
        };
        assert_eq!(
            preset.default,
            OptionDefault::Str(Preset::Classic.key()),
            "the default preset is Classic"
        );
    }

    #[test]
    fn no_preset_holds_a_deviation_or_cheat_key() {
        for preset in PRESETS {
            for entry in preset.entries() {
                if entry.key == PRESET_DEVIATION_EXCEPTION {
                    continue;
                }
                assert!(
                    !entry.key.starts_with("cheats."),
                    "{preset:?} names cheat {}",
                    entry.key
                );
                let value = entry_value(entry);
                for (key, deviation) in DESIGN_DEVIATION_KEYS {
                    let hit = *key == entry.key && (deviation.is_none() || *deviation == value);
                    assert!(!hit, "{preset:?} sets deviation key {}", entry.key);
                }
                if let Some(row) = CatalogueRow::find(entry.key) {
                    assert_eq!(
                        row.tier_for(value),
                        Tier::Feature,
                        "{preset:?} sets {} above feature tier",
                        entry.key
                    );
                    assert!(
                        !row.tags().contains(&Tag::SpoilerRisk),
                        "{preset:?} sets spoiler-risk key {}",
                        entry.key
                    );
                }
            }
        }
    }

    #[test]
    fn the_deviation_exception_is_the_design_one() {
        assert_eq!(PRESET_DEVIATION_EXCEPTION, "combat.fast_forward");
        let exception_holders: Vec<Preset> = PRESETS
            .iter()
            .copied()
            .filter(|preset| {
                preset
                    .entries()
                    .iter()
                    .any(|entry| entry.key == PRESET_DEVIATION_EXCEPTION)
            })
            .collect();
        assert_eq!(exception_holders, [Preset::Recommended, Preset::Everything]);
    }

    #[test]
    fn presets_skip_hidden_rows() {
        for preset in PRESETS {
            for (entry, row) in preset.shipped_entries() {
                assert!(row.is_shipped(), "{preset:?} applies hidden {}", entry.key);
            }
            for toggle in preset.toggles().iter_on() {
                assert!(
                    toggle.spec().is_shipped(),
                    "{preset:?} turns on hidden {}",
                    toggle.key()
                );
            }
        }
    }

    #[test]
    fn a_hidden_toggle_is_never_applied_by_a_preset() {
        // `enhancements.freeze_overlays` ships with the optional P6b
        // (U1), so this build must keep it hidden.
        let spec = Toggle::EnhancementsFreezeOverlays.spec();
        assert!(!spec.is_shipped());
        for preset in PRESETS {
            assert!(!preset.toggles().on(Toggle::EnhancementsFreezeOverlays));
        }
    }

    #[test]
    fn preset_entries_match_their_rows() {
        for preset in PRESETS {
            let mut seen = HashSet::new();
            for entry in preset.entries() {
                assert!(seen.insert(entry.key), "{preset:?} repeats {}", entry.key);
                assert!(is_well_formed_key(entry.key), "bad key {}", entry.key);
                match (CatalogueRow::find(entry.key), entry.value) {
                    (None, _) => {}
                    (Some(CatalogueRow::Toggle(_)), PresetValue::On) => {}
                    (Some(CatalogueRow::Option(spec)), PresetValue::Choice(value)) => {
                        assert!(
                            option_accepts(spec, value),
                            "{preset:?} sets {} to {value}",
                            entry.key
                        );
                    }
                    (Some(_), value) => {
                        panic!("{preset:?} gives {} the wrong kind: {value:?}", entry.key)
                    }
                }
            }
        }
    }

    #[test]
    fn everything_contains_recommended() {
        for entry in Preset::Recommended.entries() {
            assert!(
                Preset::Everything.entries().contains(entry),
                "Everything lacks {}",
                entry.key
            );
        }
    }

    #[test]
    fn preset_words_round_trip() {
        assert_eq!(PRESETS.len(), PRESET_WORDS.len());
        for (index, preset) in PRESETS.iter().enumerate() {
            assert_eq!(*preset as usize, index);
            assert_eq!(Preset::from_key(preset.key()), Some(*preset));
            assert!(!preset.label().is_empty());
        }
        assert_eq!(Preset::from_key("custom"), None);
    }

    #[test]
    fn keys_are_unique_and_well_formed() {
        let mut keys = HashSet::new();
        let mut aliases = HashSet::new();
        for row in CatalogueRow::all() {
            assert!(is_well_formed_key(row.key()), "bad key {}", row.key());
            assert!(keys.insert(row.key()), "duplicate key {}", row.key());
            for alias in row.cli_aliases() {
                assert!(
                    alias.starts_with("--") && !alias.starts_with("--no-"),
                    "bad alias {alias}"
                );
                assert!(aliases.insert(*alias), "duplicate alias {alias}");
            }
        }
        assert_eq!(keys.len(), TOGGLES.len() + OPTIONS.len());
    }

    #[test]
    fn every_row_has_a_label_class_tier_and_apply_point() {
        for spec in TOGGLES {
            assert!(!spec.label.trim().is_empty(), "{} has no label", spec.key);
            // Class, tier and apply point are required fields of every
            // `toggles!` row; the match keeps a new variant from being
            // added without a decision here.
            match (spec.class, spec.tier, spec.apply) {
                (EffectClass::StateAltering, tier, _) => {
                    assert_eq!(tier, Tier::Cheat, "{} alters state", spec.key)
                }
                (_, _, ApplyAt::Frame | ApplyAt::IdlePrompt | ApplyAt::NextLaunch) => {}
            }
        }
        for spec in OPTIONS {
            assert!(!spec.label.trim().is_empty(), "{} has no label", spec.key);
            let foundation = spec.key.starts_with("enhancements.") || spec.key.starts_with("keys.");
            assert_eq!(
                spec.class.is_none(),
                foundation,
                "only foundation settings have no effect class: {}",
                spec.key
            );
            if spec.class == Some(EffectClass::StateAltering) {
                assert_eq!(spec.tier, Tier::Cheat, "{} alters state", spec.key);
            }
            for value in spec.deviation_values {
                assert!(option_accepts(spec, value), "{} lists {value}", spec.key);
            }
        }
    }

    #[test]
    fn option_defaults_fit_their_kinds() {
        for spec in OPTIONS {
            let fits = match (spec.kind, spec.default) {
                (OptionKind::Bool, OptionDefault::Bool(_)) => true,
                (OptionKind::Int { min, max }, OptionDefault::Int(value)) => {
                    min <= max && (min..=max).contains(&value)
                }
                (OptionKind::Enum(words), OptionDefault::Str(word)) => words.contains(&word),
                (OptionKind::Color, OptionDefault::Str(text)) => is_colour(text),
                (OptionKind::Path, OptionDefault::Str(_)) => true,
                (OptionKind::List(words), OptionDefault::List(items)) => {
                    items.iter().all(|item| words.contains(item))
                }
                (OptionKind::Chord, OptionDefault::Str(text)) => !text.is_empty(),
                _ => false,
            };
            assert!(fits, "{} default does not fit its kind", spec.key);
            if let OptionDefault::Str(word) = spec.default {
                assert_eq!(
                    spec.value_tier(word),
                    spec.tier,
                    "{} defaults to a deviation value",
                    spec.key
                );
            }
        }
    }

    #[test]
    fn foundation_rows_match_the_design() {
        let keys: Vec<&str> = OPTIONS
            .iter()
            .filter(|spec| spec.key.starts_with("keys."))
            .map(|spec| spec.key)
            .collect();
        assert_eq!(keys.len(), 10, "1.2.5 lists ten [keys] actions");

        let mut chords = HashSet::new();
        for spec in OPTIONS.iter().filter(|spec| spec.key.starts_with("keys.")) {
            assert_eq!(spec.kind, OptionKind::Chord);
            assert_eq!(spec.apply, ApplyAt::Frame);
            let OptionDefault::Str(chord) = spec.default else {
                panic!("{} has no chord", spec.key);
            };
            assert!(chords.insert(chord), "two defaults share {chord}");
        }

        let freeze = Toggle::from_key("enhancements.freeze_overlays").unwrap();
        assert_eq!(freeze.spec().tier, Tier::Deviation);
        assert_eq!(freeze.spec().class, EffectClass::Presentation);
        assert_eq!(freeze.spec().apply, ApplyAt::Frame);

        let Some(CatalogueRow::Option(schema)) = CatalogueRow::find("enhancements.schema") else {
            panic!("enhancements.schema is a valued setting");
        };
        assert_eq!(schema.default, OptionDefault::Int(1));
        assert_eq!(schema.apply, ApplyAt::NextLaunch);

        let Some(CatalogueRow::Option(preset)) = CatalogueRow::find("enhancements.preset") else {
            panic!("enhancements.preset is a valued setting");
        };
        assert_eq!(preset.kind, OptionKind::Enum(PRESET_WORDS));
        assert_eq!(preset.apply, ApplyAt::IdlePrompt);
        assert_eq!(preset.cli_aliases, ["--preset"]);
    }

    #[test]
    fn toggle_bits_follow_catalogue_order() {
        assert_eq!(TOGGLES.len(), TOGGLE_KEYS.len());
        for (index, spec) in TOGGLES.iter().enumerate() {
            assert_eq!(spec.toggle as usize, index);
            assert_eq!(TOGGLE_KEYS[index], (spec.toggle, spec.key));
            assert_eq!(Toggle::from_key(spec.key), Some(spec.toggle));
        }
        assert_eq!(Toggle::from_key("enhancements.unknown"), None);
    }

    #[test]
    fn feature_toggles_set_and_clear_one_bit() {
        let mut toggles = FeatureToggles::CLASSIC;
        for spec in TOGGLES {
            toggles.set(spec.toggle, true);
            assert!(toggles.on(spec.toggle));
            assert!(!toggles.is_classic());
            assert_eq!(toggles.iter_on().collect::<Vec<_>>(), [spec.toggle]);
            toggles.set(spec.toggle, false);
            assert!(!toggles.on(spec.toggle));
            assert!(toggles.is_classic());
        }
    }

    #[test]
    fn every_valued_setting_maps_to_a_toggle_or_is_a_sub_option() {
        // Without this, `audio.volume = 50` alone would leave the toggles
        // Classic and the feature's plugin out (1.3.5). Foundation rows
        // (no effect class) change nothing by themselves.
        for spec in OPTIONS.iter().filter(|spec| spec.class.is_some()) {
            assert!(
                spec.toggle.is_some() != spec.parent.is_some(),
                "{} must either switch a toggle or be a sub-option of one",
                spec.key
            );
        }
        for spec in OPTIONS.iter().filter(|spec| spec.class.is_none()) {
            assert_eq!(spec.toggle, None, "{} is a foundation row", spec.key);
        }
    }

    #[test]
    fn the_settings_table_has_one_line_per_row() {
        let table = settings_table();
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 2 + TOGGLES.len() + OPTIONS.len());
        for (line, row) in lines[2..].iter().zip(CatalogueRow::all()) {
            assert!(line.starts_with(&format!("| {} | ", row.key())), "{line}");
            assert_eq!(line.ends_with("| yes |"), row.is_shipped(), "{line}");
        }
        assert!(table.contains(
            "| enhancements.preset | Preset | enum classic\\|recommended\\|everything = classic \
             | -, P | feature | --preset | - | yes |"
        ));
    }

    #[test]
    fn tag_labels_are_plain_text() {
        for tag in [Tag::WiderView, Tag::SpoilerRisk, Tag::ReplacesKey] {
            assert!(!tag.label().is_empty());
        }
    }

    fn is_well_formed_key(key: &str) -> bool {
        let mut parts = key.split('.');
        let (Some(section), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
            return false;
        };
        let word = |part: &str| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        };
        word(section) && word(name)
    }

    fn is_colour(text: &str) -> bool {
        text.len() == 7 && text.starts_with('#') && text[1..].chars().all(|c| c.is_ascii_hexdigit())
    }

    fn option_accepts(spec: &OptionSpec, value: &str) -> bool {
        match spec.kind {
            OptionKind::Bool => value == "true" || value == "false",
            OptionKind::Int { min, max } => value
                .parse::<i64>()
                .is_ok_and(|number| (min..=max).contains(&number)),
            OptionKind::Enum(words) => words.contains(&value),
            OptionKind::Color => is_colour(value),
            OptionKind::Path | OptionKind::Chord => true,
            OptionKind::List(words) => words.contains(&value),
        }
    }
}
