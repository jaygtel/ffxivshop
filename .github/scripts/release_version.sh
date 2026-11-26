#!/usr/bin/env bash
# Reads the version to release from Cargo.toml ([workspace.package]) and
# checks it can be released: it's a version number, Cargo.lock agrees, it
# hasn't been released yet, and it's newer than the latest release. Run by
# the Release workflow (.github/workflows/release.yml), which uses what it
# writes to GITHUB_OUTPUT.
set -euo pipefail

fail() {
    echo "::error::$1"
    exit 1
}

version="$(awk '
    /^\[workspace\.package\]/ { in_section = 1; next }
    /^\[/ { in_section = 0 }
    in_section && /^version *= *"/ { gsub(/^version *= *"|".*$/, ""); print; exit }
' Cargo.toml)"

[[ $version =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z.-]+)?$ ]] ||
    fail "The version in Cargo.toml ([workspace.package]) is \"$version\", which isn't a version number like 1.2.0 or 1.2.0-rc.1."

# Cargo.lock records each package's version too, and builds with --locked
# stop if it's behind.
for package in ffxiv-shop shop-tools shop-orders; do
    locked="$(awk -v line="name = \"$package\"" '$0 == line { getline; gsub(/^version = "|"$/, ""); print; exit }' Cargo.lock)"
    [ "$locked" = "$version" ] ||
        fail "Cargo.lock has $package at ${locked:-no version}, not $version. Run cargo update --workspace and commit Cargo.lock."
done

tag="v$version"
core="${version%%-*}"

# Every release tag so far, pre-releases included.
mapfile -t tags < <(
    git ls-remote --tags --refs origin 'v*' |
        sed 's|.*refs/tags/||' |
        grep -E '^v[0-9]+\.[0-9]+\.[0-9]+(-.+)?$' |
        sort -rV
)
for existing in "${tags[@]}"; do
    [ "$existing" != "$tag" ] || fail "$tag has been released already. Raise the version in Cargo.toml first."
done

# The latest stable release, which this one has to be newer than.
latest=""
for existing in "${tags[@]}"; do
    case $existing in
        *-*) ;;
        *) latest="${existing#v}"; break ;;
    esac
done
if [ -n "$latest" ]; then
    newer="$(printf '%s\n%s\n' "$latest" "$core" | sort -V | tail -n 1)"
    if [ "$core" = "$latest" ] || [ "$newer" != "$core" ]; then
        fail "$tag isn't newer than v$latest, the latest release. Raise the version in Cargo.toml first."
    fi
fi

prerelease=false
[[ $version == *-* ]] && prerelease=true

{
    echo "version=$version"
    echo "tag=$tag"
    echo "prerelease=$prerelease"
} >> "$GITHUB_OUTPUT"
echo "Making $tag (pre-release: $prerelease)" >> "$GITHUB_STEP_SUMMARY"
