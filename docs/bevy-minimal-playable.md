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

The launcher treats the supplied game directory as a read-only asset source.
For the default `C:\Games\U5-Clean` install, and for any install whose mutable
save files are read-only, interactive play uses a persistent writable mirror in
`%LOCALAPPDATA%\u5-engine\runtime`. `U5_ENGINE_RUNTIME_DIR` overrides that
parent directory. This keeps Journey Onward, character creation, plane-overlay
mirroring, and later saves writable without changing the clean source assets.

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
- PC-speaker audio follows `systems/audio.md` (spec commit `86bee4d`) exactly:
  one voice, the published trigger inventory, the four sound families, the
  divisor rule, and the `§3` mute rule that suppresses output without changing
  cadence. Effects are synthesized per invocation from their operation list, so
  jitter-driven and PRNG-driven pitch sequences are reproduced rather than
  approximated with fixed waves. Wall-clock pacing comes from the single
  calibrated-delay-unit anchor `cleak/u5-spec#146` publishes (0.88 ms +/- 10%,
  a static derivation with a modelling band), so the residual timing
  uncertainty is the spec's own band. Original modal pacing and historical
  display-driver deltas remain outside this v0 target.
- Remaining public-spec parity gaps are tracked in `docs/completion-audit.md`
  and the open `cleak/u5-spec` issues; they do not block this playable Bevy
  shell unless they surface as crashes, trapped prompts, or impossible ordinary
  progression.
