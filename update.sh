#!/bin/sh

set -eu

usage() {
    echo "Usage: $0 [--dry-run] [REF]"
    echo "Update Luau from mlua-rs/luau (default ref: master)."
}

die() {
    echo "error: $*" >&2
    exit 1
}

ref=master
ref_set=false
dry_run=false
for arg in "$@"; do
    case "$arg" in
        --dry-run) dry_run=true ;;
        -h|--help) usage; exit 0 ;;
        -*) die "unknown option: $arg" ;;
        *)
            [ "$ref_set" = false ] || die "expected at most one ref"
            ref=$arg
            ref_set=true
            ;;
    esac
done

cd "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repository=https://github.com/mlua-rs/luau.git
components="Ast Bytecode CodeGen Common Compiler Config Inliner Require VM"
set --
for component in $components; do
    set -- "$@" "luau/$component"
done
changes=$(git status --porcelain --untracked-files=all --ignored -- "$@")
[ -z "$changes" ] ||
    die "commit or stash changes and move ignored files out of vendored components first"

temporary=$(mktemp -d "${TMPDIR:-/tmp}/luau-update.XXXXXX")
replaced=
cleanup() {
    status=$?
    trap - 0 HUP INT TERM
    if [ -n "$replaced" ]; then
        echo "Update interrupted; restoring previous sources." >&2
        for component in $replaced; do
            rm -rf "luau/$component" &&
                cp -R "$temporary/backup/$component" "luau/$component" || {
                    echo "Restore failed; backup retained at $temporary/backup" >&2
                    exit 1
                }
        done
    fi
    rm -rf "$temporary"
    exit "$status"
}
trap cleanup 0
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

echo "Fetching $repository ($ref)..."
git init --quiet "$temporary/source"
git -C "$temporary/source" fetch --quiet --depth=100 "$repository" "$ref"
commit=$(git -C "$temporary/source" rev-parse FETCH_HEAD)
# The fork has no release tags. Include fixes after the latest release sync.
git -C "$temporary/source" log --first-parent --format=%s "$commit" > "$temporary/history"
version=$(sed -n 's|^Sync to upstream/release/\([0-9][0-9]*\)\([[:space:]].*\)\{0,1\}$|\1|p' "$temporary/history" | sed -n '1p')
[ -n "$version" ] || die "no Luau release sync found in the fetched 100 commits"
git -C "$temporary/source" checkout --quiet "$commit" -- $components
for component in $components; do
    for part in include src; do
        [ -d "$temporary/source/$component/$part" ] ||
            die "missing upstream directory: $component/$part"
    done
done

echo "Luau 0.$version at $commit"
if [ "$dry_run" = true ]; then
    echo "Dry run: sources were not changed."
    exit 0
fi

# Stage and validate before replacing anything; preserve Custom and local files.
mkdir "$temporary/backup"
for component in $components; do
    cp -R "luau/$component" "$temporary/backup/$component"
done
for component in $components; do
    replaced="$replaced $component"
    rm -rf "luau/$component"
    cp -R "$temporary/source/$component" "luau/$component"
done
replaced=

echo "Done."
