# Releasing

Two GitHub Actions workflows live in `.github/workflows/`:

- `ci.yml` runs on every branch push and pull request.
- `release.yml` builds and publishes the player packages when a `v*` tag is
  pushed, and can be run by hand.

Packaging logic shared by the workflow and local runs is in
`packaging/package.sh`; the per-platform `RUNNING.txt`, the macOS
`Info.plist` and the unsigned-macOS note are in `packaging/`.

## CI (`ci.yml`)

| Job | Runner | Needs | What it does |
|---|---|---|---|
| Format | `ubuntu-latest` | | Checks that `RUST_TOOLCHAIN` in both workflows equals `channel` in `rust-toolchain.toml`, then runs `cargo fmt --all -- --check`. |
| Build warnings | `ubuntu-latest` | Format | Runs the Python self-tests of the warning-line gate and of `paired_suite.py`'s exports (parity lock, load latency), then `qa/tools/check_no_build_warnings.sh`: builds the workspace and `-p u5-tui --features visual` (all targets) with `RUSTFLAGS=-Dwarnings`, and fails on any `warning:` line, which also catches build-script `cargo:warning` lines. |
| Clippy | `ubuntu-latest` | Build warnings | `cargo clippy --locked --workspace --all-targets -- -D warnings`, then the same for `-p u5-tui --features visual`. |
| Test (Linux) | `ubuntu-latest` | Clippy | Installs the Bevy build dependencies, runs `cargo test --release --workspace --locked`, then builds the windowed binary (`-p u5-tui --features visual --bins`). |
| Build (Windows) | `windows-latest` | Format | Builds the windowed binary with the static C runtime, as the release does. |

The gates run in the order format, warning lines, clippy, test, so the cheap
failures come first. The build jobs cache `~/.cargo` and `target/` with
`Swatinem/rust-cache`. A newer push to the same branch cancels a running CI
build.

To run the gates locally before pushing:

```sh
cargo fmt --all -- --check
qa/tools/check_no_build_warnings.sh
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked -p u5-tui --features visual --all-targets -- -D warnings
cargo test --release --workspace --locked
```

CI has no original game files. Tests that need them are compiled only with the
`original-asset-tests` feature, or skip themselves when `U5_TEST_ASSET_DIR` is
unset, so CI never sets either. To run them locally, see "Running the tests"
in the README.

The workflows build by package and feature (`-p u5-tui --features visual`),
not by crate list, so adding workspace crates needs no workflow change unless a
new crate brings a new system library (see below).

## Release (`release.yml`)

| Job | Runner | Output |
|---|---|---|
| Resolve version | `ubuntu-latest` | Works out the tag, the version (the tag without `v`) and whether to publish. For tags it also checks that `[workspace.package] version` in `Cargo.toml` equals the tag. |
| Verify | `ubuntu-latest` | Nothing kept. Runs the format check, both clippy runs and `cargo test --release --workspace --locked` on the code being released. Every platform build `needs:` it, so a tag that fails them builds and publishes nothing. |
| Linux x86_64 | `ubuntu-22.04` | `u5-engine-<version>-linux-x86_64.tar.gz`. Built on 22.04 so the binary needs only glibc 2.35. |
| Windows x86_64 | `windows-latest` (MSVC) | `u5-engine-<version>-windows-x86_64.zip`, built with `+crt-static` so no Visual C++ redistributable is needed. |
| macOS universal | `macos-15` | `u5-engine-<version>-macos-universal.zip`: one `U5 Engine.app` whose binary is `lipo`-merged from the `aarch64-apple-darwin` and `x86_64-apple-darwin` builds (minimum macOS 11). Signed and notarized when the Apple secrets exist. |
| Publish release | `ubuntu-latest` | Writes `SHA256SUMS` and attaches it and the three packages to the GitHub release for the tag (`softprops/action-gh-release`), creating the release if needed. |

Each package holds the binary (or app), `README.md`, `LICENSE` when the
repository has one, and `RUNNING.txt`. If a `u5-engine-gui` binary exists (see
"Windows console window" below) the Linux and Windows packages include it too.

Only the publish job gets `contents: write`; everything else runs with
read-only repository access.

### Before a release candidate

CI has no game files, so these gates run on game-dev-server, on a fresh
`game-dev-u5 prepare-profile` copy, before each release candidate:

- the route smoke, `u5-engine --route-smoke --route-smoke-manifest OUT
  GAME_DIR`: the manifest's sha256 must still start `22254c83f2af767b`
  unless a change to the game was meant;
- both visual suites (`--visual-frame-suite`, `--visual-route-suite`),
  compared with the last release's manifests;
- the toggle matrix, `python3 qa/tools/toggle_matrix.py`: every enhancement
  configuration equals the all-off game (see `docs/settings.md`);
- the cast-blink paired smoke, then the full paired DOSBox sweep with the
  standard flags. The release bar is two clean checks in a row: no crash,
  hang, softlock or major issue, and no real paired difference.

Changes players will notice go in [release-notes.md](release-notes.md);
paste the section for the tag into the release text.

### Cutting a release

1. Make sure `main` is green in CI.
2. Set `version` under `[workspace.package]` in `Cargo.toml` to the new
   version (e.g. `1.1.0`), run `cargo build` so `Cargo.lock` is updated, and
   commit.
3. Tag and push:

   ```sh
   git tag -a v1.1.0 -m "u5-engine 1.1.0"
   git push origin main v1.1.0
   ```

4. Watch the Release workflow in the Actions tab. When it finishes, the
   release page for `v1.1.0` has the three packages and `SHA256SUMS`, with
   auto-generated notes. Edit the notes on GitHub if you like.

A tag with a suffix (`v1.1.0-rc1`) is published as a pre-release.

### Running the release workflow by hand

Actions → **Release** → **Run workflow**, or with the GitHub CLI:

```sh
# Dry run: build every platform from main, publish nothing.
gh workflow run release.yml --ref main

# Rebuild an existing tag and (re)upload its assets, e.g. after adding the
# Apple secrets. Files with the same name are replaced.
gh workflow run release.yml --ref main -f tag=v1.0.0
```

A dry run uploads the packages, `SHA256SUMS` and the draft release notes as
workflow artifacts (`release-dry-run`) on the run's summary page. When `tag`
is given the workflow checks out that tag, so the code built is the tagged
code, whatever branch you started it from.

## macOS signing and notarization

Without the secrets below, the macOS job still builds the app, gives it an
ad-hoc signature, adds `MACOS-UNSIGNED.txt` (how to open an unsigned app) to
the zip, prints a warning in the run, and adds a note to the release text.

With them, the app is signed with your Developer ID certificate using the
hardened runtime, submitted to Apple's notary service with `notarytool`, and
the ticket is stapled to the app, so it opens with no warning.

### Secrets

Add these under the repository's Settings → Secrets and variables → Actions →
**New repository secret**:

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE_P12_BASE64` | Your **Developer ID Application** certificate and private key, exported as `.p12` and base64-encoded (see below). |
| `APPLE_CERTIFICATE_PASSWORD` | The password you set when exporting the `.p12`. |
| `APPLE_SIGNING_IDENTITY` | The certificate's full name, e.g. `Developer ID Application: Jane Doe (AB12CD34EF)`. |
| `APPLE_ID` | The Apple ID email of the developer account. |
| `APPLE_TEAM_ID` | The 10-character Team ID (Apple Developer → Membership details). |
| `APPLE_APP_PASSWORD` | An app-specific password for that Apple ID, made at [account.apple.com](https://account.apple.com) → Sign-In and Security → App-Specific Passwords. |

The first three sign the app; the last three notarize it. If only the first
three are set, the app is signed but not notarized, which Gatekeeper still
blocks for downloaded apps, so set all six.

Or with the GitHub CLI:

```sh
base64 -i DeveloperID.p12 | gh secret set APPLE_CERTIFICATE_P12_BASE64 -R cleak/u5-engine
gh secret set APPLE_CERTIFICATE_PASSWORD -R cleak/u5-engine
gh secret set APPLE_SIGNING_IDENTITY     -R cleak/u5-engine
gh secret set APPLE_ID                   -R cleak/u5-engine
gh secret set APPLE_TEAM_ID              -R cleak/u5-engine
gh secret set APPLE_APP_PASSWORD         -R cleak/u5-engine
```

(`gh secret set` without a value prompts for it, so it doesn't land in your
shell history.)

### Getting and exporting the certificate

You need a paid Apple Developer Program membership. Only the **Account
Holder** can create Developer ID certificates.

1. On a Mac, open Keychain Access → Certificate Assistant → **Request a
   Certificate From a Certificate Authority**. Enter your email, choose
   "Saved to disk", and save the `.certSigningRequest`.
2. At [developer.apple.com](https://developer.apple.com/account/resources/certificates/list)
   → Certificates → **+** → **Developer ID Application**, upload the request
   and download the `.cer`. Double-click it to add it to your login keychain.
   (Xcode → Settings → Accounts → Manage Certificates → **+** → Developer ID
   Application does the same in one step.)
3. In Keychain Access → login → My Certificates, find
   "Developer ID Application: …". Expand it to check the private key is
   underneath, select the certificate, then File → **Export Items…** →
   format "Personal Information Exchange (.p12)". Set a strong password.
4. Find the exact identity string for `APPLE_SIGNING_IDENTITY`:

   ```sh
   security find-identity -v -p codesigning
   ```

5. Base64-encode the `.p12` for `APPLE_CERTIFICATE_P12_BASE64`
   (`base64 -i DeveloperID.p12 | pbcopy`), add the secrets, then delete the
   `.p12` and the base64 text from your disk and clipboard.

The workflow imports the certificate into a temporary keychain with a random
password and deletes the keychain at the end of the job.

To check a notarized download on a Mac:

```sh
spctl --assess --type execute --verbose=4 "U5 Engine.app"
xcrun stapler validate "U5 Engine.app"
```

## Windows console window

`u5-engine.exe` is a console-subsystem program: it is also the terminal game
and the QA/CLI tool, and it must keep working from `cmd`/PowerShell (reading
the keyboard, printing, and making the shell wait for it to exit).
Double-clicking a console program opens a console window beside the game
window.

The fix is a second, GUI-subsystem binary for players, `u5-engine-gui.exe`,
which starts the windowed game exactly as `u5-engine` with no arguments does.
Both workflows already build every `u5-tui` binary (`--bins`) and package
`u5-engine-gui` when it exists. Until it lands, the Windows package has only
`u5-engine.exe`, and double-clicking it shows a console window as well.

The change, in `crates/u5-tui`:

```toml
# Cargo.toml
[[bin]]
name = "u5-engine-gui"
path = "src/bin/u5-engine-gui.rs"
required-features = ["visual"]
```

```rust
// src/bin/u5-engine-gui.rs
// GUI subsystem on Windows, so double-clicking opens no console window.
// Elsewhere it behaves exactly like `u5-engine` with no arguments.
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() -> std::io::Result<()> {
    u5_tui::run_default_launch()
}
```

`run_default_launch` stands for whatever library function the no-argument
path of `main.rs` calls (the windowed game plus the setup wizard). It has to
live in the `u5_tui` library (or a crate it depends on), not in `main.rs`, so
both binaries can call it. A GUI-subsystem process has no stdout or stderr, so
startup errors in that path should be shown in a message box (for example
`rfd::MessageDialog`) or written to a log file rather than printed.

Putting `windows_subsystem = "windows"` on `u5-engine` itself is not an
option: a GUI-subsystem program started from `cmd` returns the prompt at once
and cannot share the console's keyboard input, which breaks terminal play and
the CLI tools. A single-binary alternative is to keep the console subsystem and
call `FreeConsole()` at startup when `GetConsoleProcessList` reports that this
process is the console's only user (meaning Explorer created the console for
it). That works but flashes a console window briefly on every double-click.

## System dependencies

| Where | Packages |
|---|---|
| Linux build (CI and release) | `pkg-config libasound2-dev libudev-dev libx11-dev libxkbcommon-dev libwayland-dev` |
| Linux runtime | ALSA, libudev (linked); X11 or Wayland, xkbcommon and a Vulkan driver (loaded at startup); the file-chooser portal for the first-run folder picker. See the README. |
| Windows / macOS | None beyond the runner's toolchain. |

The native folder picker is expected to use `rfd` with its default
`xdg-portal` backend, which talks to the desktop portal over D-Bus in pure Rust
and needs no GTK headers. If `rfd`'s `gtk3` feature is ever enabled instead,
add `libgtk-3-dev` to the apt lines in both workflows and `libgtk-3-0` to the
runtime list.

## Maintenance notes

- **Toolchain.** `rust-toolchain.toml` pins the exact Rust release (with
  `rustfmt` and `clippy`) for local builds and CI alike. To move to a newer
  release, change `channel` there and `RUST_TOOLCHAIN` in both workflows in
  one commit, and fix whatever new format or clippy findings it brings; the
  CI format job fails if the three disagree.
- **Action pins.** Every action is pinned by full commit SHA with its
  release in a trailing comment. `.github/dependabot.yml` opens a weekly
  pull request that moves the pins and comments together; review the
  action's changes before merging it. `dtolnay/rust-toolchain` has no
  releases, so it is pinned to a `master` commit and always given
  `toolchain:` explicitly.
- **Rebuilding an old tag.** The Verify job runs on the tagged code. A tag
  from before these gates (1.0.1 and earlier) has no `rust-toolchain.toml`
  and does not pass the format and clippy checks, so rebuild such a tag
  from a branch that still has the old workflow.
- `ubuntu-22.04` is the oldest GitHub-hosted runner. When GitHub retires it,
  move the Linux release job to the next oldest runner (raising the glibc
  floor, which `packaging/linux/RUNNING.txt` and the README state) or build
  inside an older container.
- To check the Linux packaging locally:

  ```sh
  cargo build --release --locked -p u5-tui --features visual --bins
  packaging/package.sh linux 0.0.0-local target/release dist
  ```

- Lint workflow changes with [actionlint](https://github.com/rhysd/actionlint)
  (it runs shellcheck on the `run:` scripts when shellcheck is installed).
