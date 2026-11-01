#!/usr/bin/env bash
# Release packaging for u5-engine. Used by .github/workflows/release.yml and
# runnable locally. Run from the repository root.
#
#   packaging/package.sh linux        VERSION BIN_DIR OUT_DIR
#       -> OUT_DIR/u5-engine-VERSION-linux-x86_64.tar.gz
#
#   packaging/package.sh macos-bundle VERSION BINARY STAGE_DIR
#       -> STAGE_DIR/U5 Engine.app (unsigned; sign it before macos-zip)
#
#   packaging/package.sh macos-zip    VERSION STAGE_DIR OUT_DIR [NOTE_FILE]
#       -> OUT_DIR/u5-engine-VERSION-macos-universal.zip
#
# BIN_DIR holds the built binaries (u5-engine, plus u5-engine-gui when that
# binary exists). README.md and LICENSE are copied from the repository root;
# a missing LICENSE is reported, not fatal.
set -euo pipefail

APP_NAME="U5 Engine"
BINARIES=(u5-engine u5-engine-gui)

die() { echo "package.sh: $*" >&2; exit 1; }

copy_docs() {
    local dest=$1 running=$2
    cp README.md "$dest/README.md"
    cp "$running" "$dest/RUNNING.txt"
    if [[ -f LICENSE ]]; then
        cp LICENSE "$dest/LICENSE"
    else
        echo "package.sh: warning: no LICENSE file in the repository root" >&2
    fi
}

cmd=${1:-}
case "$cmd" in
linux)
    [[ $# -eq 4 ]] || die "usage: linux VERSION BIN_DIR OUT_DIR"
    version=$2 bin_dir=$3 out_dir=$4
    name="u5-engine-${version}-linux-x86_64"
    stage=$(mktemp -d)
    trap 'rm -rf "$stage"' EXIT
    mkdir -p "$stage/$name" "$out_dir"
    found=0
    for bin in "${BINARIES[@]}"; do
        if [[ -f "$bin_dir/$bin" ]]; then
            install -m 0755 "$bin_dir/$bin" "$stage/$name/$bin"
            found=1
        fi
    done
    [[ $found -eq 1 && -f "$stage/$name/u5-engine" ]] || die "no u5-engine binary in $bin_dir"
    copy_docs "$stage/$name" packaging/linux/RUNNING.txt
    tar -C "$stage" --owner=0 --group=0 --numeric-owner -czf "$out_dir/$name.tar.gz" "$name"
    echo "$out_dir/$name.tar.gz"
    ;;
macos-bundle)
    [[ $# -eq 4 ]] || die "usage: macos-bundle VERSION BINARY STAGE_DIR"
    version=$2 binary=$3 stage=$4
    [[ -f "$binary" ]] || die "missing binary $binary"
    app="$stage/$APP_NAME.app"
    rm -rf "$app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    install -m 0755 "$binary" "$app/Contents/MacOS/u5-engine"
    # Bundle versions must be numeric (major.minor.patch); drop any suffix.
    short_version=${version%%[-+]*}
    sed -e "s/__VERSION__/${version}/g" -e "s/__SHORT_VERSION__/${short_version}/g" \
        packaging/macos/Info.plist > "$app/Contents/Info.plist"
    printf 'APPL????' > "$app/Contents/PkgInfo"
    if command -v plutil >/dev/null; then
        plutil -lint "$app/Contents/Info.plist" >/dev/null
    fi
    echo "$app"
    ;;
macos-zip)
    [[ $# -ge 4 ]] || die "usage: macos-zip VERSION STAGE_DIR OUT_DIR [NOTE_FILE]"
    version=$2 stage=$3 out_dir=$4 note=${5:-}
    name="u5-engine-${version}-macos-universal"
    [[ -d "$stage/$APP_NAME.app" ]] || die "missing $stage/$APP_NAME.app"
    pkg=$(mktemp -d)
    trap 'rm -rf "$pkg"' EXIT
    mkdir -p "$pkg/$name" "$out_dir"
    # ditto keeps the bundle's signature, extended attributes and stapled ticket.
    ditto "$stage/$APP_NAME.app" "$pkg/$name/$APP_NAME.app"
    copy_docs "$pkg/$name" packaging/macos/RUNNING.txt
    if [[ -n "$note" ]]; then
        cp "$note" "$pkg/$name/$(basename "$note")"
    fi
    out_abs=$(cd "$out_dir" && pwd)
    (cd "$pkg" && ditto -c -k --sequesterRsrc --keepParent "$name" "$out_abs/$name.zip")
    echo "$out_dir/$name.zip"
    ;;
*)
    die "unknown command '${cmd}' (expected linux, macos-bundle or macos-zip)"
    ;;
esac
