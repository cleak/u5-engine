# Release Notes

What each release changes for players. The release workflow adds the
download notes and the list of merged changes; paste the section for the
tag above them.

## Unreleased

### Optional enhancements: the foundation

The game plays exactly as 1.0.1 by default, and nothing in this section
changes it. This release lays the groundwork for optional enhancements,
each its own setting and each off unless you turn it on. No enhancement is
available yet; the first ones arrive in later releases.

- **Settings.** Every enhancement setting is listed in
  [settings.md](settings.md), with how to change it: `config.toml`, the setup
  window's Enhancements page (`u5-engine --settings`) or the command line
  (`--preset`, `--enable`, `--disable`, `--set`). `--show-settings` prints
  what a launch would use. Unknown keys and bad values never stop the game:
  the setting keeps its default and the log says why.
- **Presets.** Classic (the default, the original game), Recommended and
  Everything. In this release every preset still gives the original game.
- **Parity lock.** `--parity-lock` or `U5_PARITY_LOCK=1` turns every
  enhancement off whatever the settings say.
- **Hotkeys** for future features are listed in
  [commands.md](commands.md#enhancement-hotkeys). None is bound while its
  feature is off, and none is a key the game reads.
- **Files beside the save.** Future features that remember things per save
  will keep them in a `.u5-engine-enh` folder next to `SAVED.GAM`
  ([sidecars.md](sidecars.md#player-sidecars)). `SAVED.GAM` itself never
  changes, and with every enhancement off the folder is never created.

Checks behind it: the route-smoke manifest and both visual-suite manifests
are unchanged from 1.0.1, two full paired DOSBox sweeps found no real
difference from it (every non-match was a random draw or capture timing that
1.0.1 shows too), and every enhancement configuration in the toggle matrix
plays exactly as the original.
