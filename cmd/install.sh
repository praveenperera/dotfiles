#!/usr/bin/env bash
# Install cmd from a GitHub release
#
#   ./install.sh                 # latest release into ~/.local/bin
#   ./install.sh v0.0.251        # a specific tag
#   CMD_INSTALL_DIR=/usr/local/bin ./install.sh
#   curl -fsSL https://raw.githubusercontent.com/praveenperera/dotfiles/master/cmd/install.sh | bash
#
# Release builds exist for Apple silicon macOS and x86_64 Linux; build other
# platforms from source with ./release

set -euo pipefail

repo="${CMD_REPO:-praveenperera/dotfiles}"
install_dir="${CMD_INSTALL_DIR:-$HOME/.local/bin}"
tag="${1:-${CMD_VERSION:-}}"

die() {
    echo "install.sh: $*" >&2
    exit 1
}

# release-cmd.yml names its archives by these suffixes, not by target triple
release_suffix() {
    case "$(uname -s)-$(uname -m)" in
        Darwin-arm64) echo macos-arm64 ;;
        Linux-x86_64) echo linux-musl ;;
        *) die "no release build for $(uname -s) $(uname -m); build from source with ./release" ;;
    esac
}

# fetch a url to stdout, or to the file in $2, failing on http errors
fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL ${2:+-o "$2"} "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "${2:--}" "$1"
    else
        die "needs curl or wget"
    fi
}

latest_tag() {
    # avoid a jq dependency; tag_name appears once in a release object
    fetch "https://api.github.com/repos/$repo/releases/latest" |
        sed -nE 's/.*"tag_name": *"([^"]+)".*/\1/p' | head -n 1
}

main() {
    local suffix archive
    suffix="$(release_suffix)"
    [ -n "$tag" ] || tag="$(latest_tag)"
    [ -n "$tag" ] || die "could not resolve the latest release of $repo"

    # global so the EXIT trap still sees it after main returns
    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT

    archive="cmd-$tag-$suffix.tar.gz"
    echo "downloading $archive from $repo" >&2
    fetch "https://github.com/$repo/releases/download/$tag/$archive" "$work/$archive" ||
        die "failed to download $archive"

    tar -xzf "$work/$archive" -C "$work"
    [ -f "$work/cmd" ] || die "$archive does not contain a cmd binary"

    # stage beside the destination, then rename, so a running cmd keeps its inode
    mkdir -p "$install_dir"
    cp "$work/cmd" "$install_dir/.cmd.new"
    chmod 755 "$install_dir/.cmd.new"
    mv -f "$install_dir/.cmd.new" "$install_dir/cmd"

    echo "installed cmd $tag to $install_dir/cmd" >&2
    case ":$PATH:" in
        *":$install_dir:"*) ;;
        *) echo "note: $install_dir is not on PATH" >&2 ;;
    esac
}

main
