# Bevy Minimal Playable V0

This is the current Bevy playable target. It is not an exact visual-parity
release; it is the smallest coherent Bevy route through intro, save creation,
Journey Onward, exploration, combat, modal text flows, and save/load using the
shared clean runtime.

## Launch

```powershell
cargo run --features visual -- --visual-playable C:\Games\U5-Clean
```

`--visual-playable` is an alias for the Bevy intro/menu shell. From there:

- `C` creates a new U5 save and returns to the menu.
- `T` transfers a U4 save when `PARTY.SAV` is present and accepted.
- `J` loads `SAVED.GAM`/`SAVED.OOL` and enters Bevy gameplay.
- `Esc` exits the intro shell, or exits gameplay only when no modal prompt is active.

## Play Scope

The v0 scope is ordinary playability, not frame-perfect DOS presentation:

- World, town, dungeon, combat, shops, conversation, magic, rest/camp,
  Blackthorn, shrine/Codex, Shadowlord, ship/horse/skiff, and save/load paths
  route through the same `PlayState` used by terminal play.
- Bevy renders the 11x11 top-down/combat viewport, first-person dungeon view,
  fixed-cell text/status surface, modal prompts, intro/menu panels, and endgame
  surfaces.
- Text-heavy flows use the shared fixed-cell modal/status surface. Exact
  original shop/conversation wait, clear, cursor, and rectangle pacing remains
  parity work.

## V0 Smoke Gate

Run this gate before calling a change Bevy-playable:

```powershell
cargo fmt --all -- --check
cargo test -p u5-tui cli_parser_accepts_visual_playable_alias
cargo test -p u5-tui --features visual
cargo test -p u5-bevy
$env:U5_BEVY_SCREENSHOT='target\bevy-playable-smoke.png'
$env:U5_BEVY_SCREENSHOT_DELAY='90'
cargo run -p u5-tui --features visual -- --visual-playable C:\Games\U5-Clean
Remove-Item Env:\U5_BEVY_SCREENSHOT
Remove-Item Env:\U5_BEVY_SCREENSHOT_DELAY
```

The screenshot smoke should exit by itself and write a nonblank PNG. For a
broader noninteractive route check, run:

```powershell
cargo run -p u5-tui --features visual -- --visual-route-suite target\visual-route-suite C:\Games\U5-Clean
```

## Known V0 Limits

- The game uses the supplied asset/save directory directly. Use a copied clean
  asset directory when testing destructive save flows.
- The published effect boundaries have generated PC-speaker-style audio. The
  dungeon-decoration sweep retains its published discrete frequencies and
  pacing ratio; exact historical timings for calibrated waits, other
  unpublished envelopes, original modal pacing, and historical display-driver
  deltas remain outside this v0 target.
- Remaining public-spec parity gaps are tracked in `docs/completion-audit.md`
  and the open `cleak/u5-spec` issues; they do not block this playable Bevy
  shell unless they surface as crashes, trapped prompts, or impossible ordinary
  progression.
