#!/usr/bin/env bash
# Deploys a published release of Aether & Allure on your server.
#
#   ./deploy.sh v1.2.0    builds and starts release v1.2.0
#   ./deploy.sh           builds and starts the release that's running again,
#                         after changing .env, say
#
# Keep it in the folder that holds your .env (see Hosting on your server in
# the wiki). It only ever builds a published, stable release, never a copy of
# the repository: it refuses drafts and pre-releases, downloads the release's
# bundle from GitHub, checks it against the release's SHA256SUMS, unpacks it
# into releases/, and builds and starts it from there with your .env. If the
# build fails, the site that's running carries on as it was.
#
# Each release keeps its folder and its images (tagged with its version), so
# going back to an earlier one is ./deploy.sh with its version. `current`
# points at the release that's running.

set -euo pipefail

repo="jaygtel/ffxivshop"
# Only changed for testing this script.
api="${DEPLOY_GITHUB_API:-https://api.github.com}"
downloads="${DEPLOY_GITHUB_DOWNLOADS:-https://github.com}"

say() { printf '%s\n' "$*"; }
fail() {
    printf 'deploy.sh: %s\n' "$*" >&2
    exit 1
}

for tool in curl sha256sum tar cmp docker; do
    command -v "$tool" > /dev/null || fail "$tool is needed, and isn't installed."
done
docker compose version > /dev/null 2>&1 || fail "Docker's Compose plugin is needed (docker compose)."

home="$(cd "$(dirname "$0")" && pwd)"
self="$home/$(basename "$0")"
cd "$home"

[ $# -le 1 ] || fail "usage: ./deploy.sh [version]"
version="${1:-}"
if [ -z "$version" ]; then
    [ -L current ] || fail "Nothing has been deployed here yet: give a version, like ./deploy.sh v1.0.0"
    version="$(basename "$(readlink current)")"
    say "Building $version again, the release that's running."
fi
case $version in
    [0-9]*) version="v$version" ;;
esac
[[ $version =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
    fail "$version isn't a stable release. Releases look like v1.2.0, and pre-releases (like v1.2.0-rc.1) are never deployed."

release="releases/$version"
if [ ! -d "$release" ]; then
    # Drafts can't be seen from here at all, so a draft is "not found" too.
    info="$(curl -fsL -H 'Accept: application/vnd.github+json' "$api/repos/$repo/releases/tags/$version")" ||
        fail "There's no published release called $version on GitHub."
    grep -Eq '"draft": *false' <<< "$info" || fail "$version is still a draft."
    grep -Eq '"prerelease": *false' <<< "$info" ||
        fail "$version is a pre-release, and pre-releases are never deployed."
    if grep -Eq '"immutable": *false' <<< "$info"; then
        fail "$version was published before releases were locked, so its files could have changed since."
    fi

    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT
    bundle="aether-allure-$version.tar.gz"
    from="$downloads/$repo/releases/download/$version"
    say "Downloading $version..."
    curl -fsSL --retry 3 -o "$work/$bundle" "$from/$bundle" ||
        fail "Couldn't download $bundle from the release."
    curl -fsSL --retry 3 -o "$work/SHA256SUMS" "$from/SHA256SUMS" ||
        fail "Couldn't download SHA256SUMS from the release."
    (cd "$work" && grep -E "^[0-9a-f]{64}  $bundle\$" SHA256SUMS | sha256sum --check --strict --status -) ||
        fail "$bundle doesn't match its checksum in the release's SHA256SUMS. Nothing was changed."
    # With GitHub's CLI installed and signed in, also check GitHub's record of
    # where the bundle was made.
    if command -v gh > /dev/null && gh auth status > /dev/null 2>&1; then
        gh attestation verify "$work/$bundle" --repo "$repo" > /dev/null ||
            fail "GitHub has no record of $bundle being made by this repository's Release workflow."
        say "GitHub's record of where it was made checks out."
    fi
    tar -xzf "$work/$bundle" -C "$work"
    mkdir -p releases
    mv "$work/aether-allure-$version" "$release"
    say "Checked it, and unpacked it into $release."
fi

# The first time, start .env from the release's template.
if [ ! -f .env ]; then
    cp "$release/.env.example" .env
    chmod 600 .env
    say "Made .env from the release's .env.example. Fill it in (see Settings and secrets in the wiki), then run ./deploy.sh $version again."
    exit 0
fi

previous="$(readlink current 2> /dev/null || true)"

say "Building and starting $version..."
export RELEASE="$version"
compose=(docker compose --project-directory "$release" --file "$release/compose.yaml" --env-file "$home/.env")
"${compose[@]}" up --detach --build --remove-orphans
ln -sfn "$release" current
"${compose[@]}" ps

say ""
say "$version is running."
if [ -n "$previous" ] && [ "$previous" != "$release" ]; then
    if ! cmp -s "$previous/deploy/traefik/aether-allure.yml" "$release/deploy/traefik/aether-allure.yml"; then
        say "deploy/traefik/aether-allure.yml has changed since $(basename "$previous"): copy it to Traefik's dynamic folder again."
    fi
    say "To go back: ./deploy.sh $(basename "$previous")"
fi
if ! cmp -s "$self" "$release/deploy/deploy.sh"; then
    say "This release comes with a different deploy.sh. To use it: cp $release/deploy/deploy.sh $self"
fi
