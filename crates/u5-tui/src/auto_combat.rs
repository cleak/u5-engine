//! Auto-combat's terminal half (Phase B design 6.2.6, 6.3.5; build plan
//! M7-04).
//!
//! The terminal shell has no hotkeys. With `combat.auto` on and unlocked,
//! an `@auto` control line engages or disengages roster positions:
//!
//! - `@auto`: the member whose command prompt is open;
//! - `@auto party`: every roster position;
//! - `@auto 1,3`: roster positions 1 and 3 (the stats panel's numbers);
//! - `@auto until`: the whole party until the fight is done;
//! - `@auto off`: nobody.
//!
//! In the interactive loop auto then answers every prompt it owns; in a
//! play script the token runs the autopilot, one key at a time, until it
//! hands back. Every key is an ordinary 1.0.1 input, so
//! `--auto-combat-log` can write the run as a plain script (or the stdin
//! lines) that replays it with the feature off.
//!
//! With the feature off or locked there is no [`TerminalAutoCombat`], so
//! `@auto` is not special: it is the `@` key, as in 1.0.1, and no log file
//! is created.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use u5_runtime::PlayState;
use u5_runtime::auto_combat::{
    AutoEngagement, AutoFightRuntime, AutoKey, AutoPlan, AutoPrompt, CombatSettings, HandBack,
    auto_prompt, observe_after_auto_key, plan_next_auto_key, record_print_before_auto_key,
};
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockState, ResolveInput, ResolvedEnhancements, resolve,
};

use crate::play_loop::{ScriptTokenHooks, ScriptTokenIntercept};

/// The control token's word.
pub const AUTO_TOKEN: &str = "@auto";

/// Roster positions auto can be handed (the stats panel's 1-6).
const ROSTER_POSITIONS: usize = 6;

/// What one `@auto` line asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoControl {
    /// `@auto`: the prompted member.
    Prompted,
    /// `@auto party`.
    Party,
    /// `@auto 1,3`: these roster positions (index 0 is position 1).
    Members([bool; ROSTER_POSITIONS]),
    /// `@auto until`.
    Until,
    /// `@auto off`.
    Off,
    /// `@auto` followed by anything else: taken, with a note, and ignored.
    Unknown,
}

/// Parses an `@auto` line; `None` when the line is not one (a different
/// `@` token, or the `@` key).
pub fn parse_auto_control(line: &str) -> Option<AutoControl> {
    let line = line.trim();
    let rest = line.strip_prefix(AUTO_TOKEN)?;
    if !(rest.is_empty() || rest.starts_with(char::is_whitespace)) {
        return None;
    }
    let rest = rest.trim().to_ascii_lowercase();
    Some(match rest.as_str() {
        "" => AutoControl::Prompted,
        "party" => AutoControl::Party,
        "until" => AutoControl::Until,
        "off" => AutoControl::Off,
        list => parse_members(list).map_or(AutoControl::Unknown, AutoControl::Members),
    })
}

/// `1,3` (commas or spaces) as roster positions 1-6.
fn parse_members(list: &str) -> Option<[bool; ROSTER_POSITIONS]> {
    let mut members = [false; ROSTER_POSITIONS];
    let mut any = false;
    for item in list.split([',', ' ']).filter(|item| !item.is_empty()) {
        let position: usize = item.parse().ok()?;
        if !(1..=ROSTER_POSITIONS).contains(&position) {
            return None;
        }
        members[position - 1] = true;
        any = true;
    }
    any.then_some(members)
}

/// The `[combat]` settings of a terminal run (design 6.3.5): the terminal
/// modes resolve from the command line only, with no stored file (R4.4),
/// and the parity lock gives the defaults, so `combat.auto` is off.
pub fn terminal_combat_settings(cli: &[CliSetting], lock: LockState) -> CombatSettings {
    terminal_resolved(cli, lock).combat()
}

fn terminal_resolved(cli: &[CliSetting], lock: LockState) -> ResolvedEnhancements {
    resolve(ResolveInput {
        stored: None,
        cli,
        lock,
        caps: HostCapabilities::ALL,
    })
}

/// How `--auto-combat-log` writes its run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoLogForm {
    /// A `--play-script` value: every command that ran, `;`-separated,
    /// with each `@auto` token replaced by the keys it typed.
    Script,
    /// The stdin lines of an interactive run, one per line, with the auto
    /// keys as the lines that type them.
    Stdin,
}

/// The line an interactive player types for `key`: the script token,
/// except Pass, which a terminal types as a Space (the `pass` word is a
/// script-only name).
pub fn auto_key_stdin_line(key: AutoKey) -> &'static str {
    match key {
        AutoKey::Pass => " ",
        key => key.script_token(),
    }
}

/// The note a hand-back prints, with the 6.2.5 toast's fixed words.
pub fn hand_back_note(reason: HandBack, state: &PlayState) -> String {
    match reason {
        HandBack::Interrupted => "Auto off: key pressed".to_string(),
        HandBack::Victory => "Victory - auto off".to_string(),
        HandBack::Hurt(roster) => format!("Auto off: {} is hurt", member_name(state, roster)),
        HandBack::NoTarget => "Auto off: no target in sight".to_string(),
        HandBack::Stuck => "Auto off: stuck".to_string(),
        HandBack::ActionCap => "Auto off: action limit".to_string(),
        HandBack::UnexpectedPrompt => "Auto off: unexpected prompt".to_string(),
        HandBack::FightOver => "Auto off".to_string(),
    }
}

/// The roster position's name, as the stats panel shows it.
fn member_name(state: &PlayState, roster: usize) -> String {
    state
        .party_names
        .get(roster)
        .map(|name| {
            let end = name
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(name.len());
            String::from_utf8_lossy(&name[..end]).into_owned()
        })
        .unwrap_or_else(|| format!("member {}", roster + 1))
}

/// The `--auto-combat-log` file.
#[derive(Debug)]
struct AutoCombatLog {
    file: File,
    form: AutoLogForm,
}

impl AutoCombatLog {
    fn write(&mut self, entry: &str) {
        let entry = entry.trim_end_matches(['\r', '\n']);
        let result = match self.form {
            AutoLogForm::Script => write!(self.file, "{entry};"),
            AutoLogForm::Stdin => writeln!(self.file, "{entry}"),
        };
        if let Err(error) = result.and_then(|()| self.file.flush()) {
            eprintln!("auto-combat log: {error}");
        }
    }
}

/// Auto-combat's terminal state: the settings, who is handed over, the
/// fight's counters and the log. It exists only while `combat.auto` is on
/// and unlocked.
#[derive(Debug)]
pub struct TerminalAutoCombat {
    settings: CombatSettings,
    engaged: AutoEngagement,
    fight: AutoFightRuntime,
    /// The key planned and handed to the shell, until it has run.
    pending: Option<AutoKey>,
    /// The last hand-back, for the caller and the tests.
    last_hand_back: Option<HandBack>,
    keys: usize,
    log: Option<AutoCombatLog>,
}

impl TerminalAutoCombat {
    /// Auto-combat for a terminal run, when `settings` turn it on; the
    /// resolver has already applied the parity lock (it gives `auto =
    /// false`), so a locked run has none and creates no log.
    pub fn new(settings: CombatSettings) -> Option<Self> {
        settings.auto.then(|| Self {
            settings,
            engaged: AutoEngagement::default(),
            fight: AutoFightRuntime::default(),
            pending: None,
            last_hand_back: None,
            keys: 0,
            log: None,
        })
    }

    /// [`new`](Self::new), writing `--auto-combat-log` to `path`. The file
    /// is created only when the feature is on.
    pub fn with_log(
        settings: CombatSettings,
        path: Option<&Path>,
        form: AutoLogForm,
    ) -> io::Result<Option<Self>> {
        let Some(mut auto) = Self::new(settings) else {
            return Ok(None);
        };
        if let Some(path) = path {
            auto.log = Some(AutoCombatLog {
                file: File::create(path)?,
                form,
            });
        }
        Ok(Some(auto))
    }

    /// Whether `line` is auto-combat's control line.
    pub fn owns(line: &str) -> bool {
        parse_auto_control(line).is_some()
    }

    pub fn engagement(&self) -> AutoEngagement {
        self.engaged
    }

    pub fn last_hand_back(&self) -> Option<HandBack> {
        self.last_hand_back
    }

    /// Auto keys typed in this run.
    pub fn keys_typed(&self) -> usize {
        self.keys
    }

    /// This fight's counters.
    pub fn fight(&self) -> &AutoFightRuntime {
        &self.fight
    }

    /// Applies one `@auto` line and returns the note the shell prints (the
    /// 6.2.2 confirmation words). Engagement exists only in combat.
    pub fn control(&mut self, line: &str, state: &PlayState) -> String {
        let Some(control) = parse_auto_control(line) else {
            return String::new();
        };
        if control == AutoControl::Off {
            self.disengage();
            return "Auto off".to_string();
        }
        if control == AutoControl::Unknown {
            return "Auto-combat: use @auto, @auto party, @auto 1,3, @auto until or @auto off"
                .to_string();
        }
        if !state.combat_active {
            return "Auto-combat: not in combat".to_string();
        }
        self.last_hand_back = None;
        match control {
            AutoControl::Prompted => match auto_prompt(state) {
                AutoPrompt::Command {
                    roster: Some(roster),
                    ..
                } if roster < ROSTER_POSITIONS => {
                    self.engaged.members[roster] = true;
                    format!("Auto: member {}", roster + 1)
                }
                _ => "Auto-combat: no member's prompt is open".to_string(),
            },
            AutoControl::Party => {
                self.engaged.members = [true; ROSTER_POSITIONS];
                "Auto: party".to_string()
            }
            AutoControl::Until => {
                self.engaged.members = [true; ROSTER_POSITIONS];
                self.engaged.until_done = true;
                "Auto: until done".to_string()
            }
            AutoControl::Members(members) => {
                for (engaged, asked) in self.engaged.members.iter_mut().zip(members) {
                    *engaged |= asked;
                }
                let list = members
                    .iter()
                    .enumerate()
                    .filter(|(_, asked)| **asked)
                    .map(|(position, _)| (position + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("Auto: member {list}")
            }
            AutoControl::Off | AutoControl::Unknown => unreachable!("handled above"),
        }
    }

    fn disengage(&mut self) {
        self.engaged = AutoEngagement::default();
        self.fight = AutoFightRuntime::default();
        self.pending = None;
    }

    /// The key for the prompt now open, when auto owns it, and the note to
    /// print when auto hands back. A key is logged as it is handed out;
    /// the shell must run it next, with no idle pass in between.
    pub fn next_key(&mut self, state: &PlayState) -> (Option<AutoKey>, Option<String>) {
        if !state.combat_active {
            return (None, self.fight_over(state));
        }
        if !self.engaged.any() {
            return (None, None);
        }
        match plan_next_auto_key(state, &self.settings, &self.engaged, &self.fight) {
            AutoPlan::Key(key) => {
                record_print_before_auto_key(state, &mut self.fight);
                self.pending = Some(key);
                self.keys += 1;
                if let Some(log) = self.log.as_mut() {
                    log.write(match log.form {
                        AutoLogForm::Script => key.script_token(),
                        AutoLogForm::Stdin => auto_key_stdin_line(key),
                    });
                }
                (Some(key), None)
            }
            AutoPlan::HandBack(reason) => {
                self.disengage();
                self.last_hand_back = Some(reason);
                (None, Some(hand_back_note(reason, state)))
            }
            AutoPlan::NotOurs => (None, None),
        }
    }

    /// Combat has ended: the fight's counters go, and so does engagement
    /// unless `keep_between_fights` (never for "until done").
    fn fight_over(&mut self, state: &PlayState) -> Option<String> {
        self.fight = AutoFightRuntime::default();
        self.pending = None;
        if !self.engaged.any() {
            return None;
        }
        if self.settings.keep_between_fights && !self.engaged.until_done {
            return None;
        }
        self.engaged = AutoEngagement::default();
        self.last_hand_back = Some(HandBack::FightOver);
        Some(hand_back_note(HandBack::FightOver, state))
    }

    /// After every command: an auto key is observed (6.3.2); a player's
    /// key leaves any cursor it opened to the player.
    pub fn after_command(&mut self, state: &PlayState) {
        match self.pending.take() {
            Some(key) => observe_after_auto_key(state, key, &mut self.fight),
            None => self.fight.owns_open_attempt = false,
        }
        if !state.combat_active {
            self.fight = AutoFightRuntime::default();
        }
    }

    /// A line the player's side gave the game, for the log.
    pub fn log_typed(&mut self, line: &str) {
        if let Some(log) = self.log.as_mut() {
            log.write(line);
        }
    }
}

/// Auto-combat on the play-script seam (design 1.3.12, 6.3.5): an `@auto`
/// token drives one key at a time ([`ScriptTokenIntercept::Drive`]), each
/// planned from the state the last one left. Every other token passes
/// through. It records what each token typed, so the run can be written as
/// the plain script that replays it.
#[derive(Debug)]
pub struct AutoCombatHooks {
    auto: TerminalAutoCombat,
    typed: Vec<(usize, String)>,
    notes: Vec<String>,
}

impl AutoCombatHooks {
    /// The hooks for `settings`; `None` with the feature off.
    pub fn new(settings: CombatSettings) -> Option<Self> {
        TerminalAutoCombat::new(settings).map(|auto| Self {
            auto,
            typed: Vec::new(),
            notes: Vec::new(),
        })
    }

    pub fn auto(&self) -> &TerminalAutoCombat {
        &self.auto
    }

    /// Every key typed, with the index of the token that typed it.
    pub fn typed(&self) -> &[(usize, String)] {
        &self.typed
    }

    /// The notes the shell would have printed, in order.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// `commands` as the plain script that replays this run with the
    /// feature off: each `@auto` token becomes the keys it typed.
    pub fn replay_script(&self, commands: &[String]) -> Vec<String> {
        let mut script = Vec::new();
        for (index, command) in commands.iter().enumerate() {
            if TerminalAutoCombat::owns(command) {
                script.extend(
                    self.typed
                        .iter()
                        .filter(|(at, _)| *at == index)
                        .map(|(_, key)| key.clone()),
                );
            } else {
                script.push(command.clone());
            }
        }
        script
    }
}

impl ScriptTokenHooks for AutoCombatHooks {
    fn after_command(&mut self, state: &PlayState) {
        self.auto.after_command(state);
    }

    fn intercept(
        &mut self,
        _index: usize,
        token: &str,
        state: &mut PlayState,
    ) -> ScriptTokenIntercept {
        if !TerminalAutoCombat::owns(token) {
            return ScriptTokenIntercept::PassThrough;
        }
        let note = self.auto.control(token, state);
        self.notes.push(note);
        ScriptTokenIntercept::Drive
    }

    fn next_command(&mut self, index: usize, state: &PlayState) -> Option<String> {
        let (key, note) = self.auto.next_key(state);
        self.notes.extend(note);
        let token = key?.script_token().to_string();
        self.typed.push((index, token.clone()));
        Some(token)
    }
}

#[cfg(test)]
mod tests;
