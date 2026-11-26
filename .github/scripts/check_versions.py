#!/usr/bin/env python3
"""Looks for newer versions of what Dependabot can't update by itself.

Dependabot (.github/dependabot.yml) updates the crates in Cargo.lock within
what Cargo.toml allows, the Rust version, the base images and the actions.
This checks the rest, every Monday from .github/workflows/versions.yml:

- the build tools pinned in the Dockerfile: Trunk and wasm-opt (binaryen),
  with the SHA-256 of each download, and leptosfmt;
- crates whose newest version is outside what Cargo.toml allows, so moving to
  it means changing Cargo.toml, and usually the code.

It writes a report in Markdown to the file it's given and, on GitHub, sets
the step output `found` to true or false. It only reads: the workflow opens,
updates or closes the issue.

    python3 .github/scripts/check_versions.py report.md
"""

import hashlib
import json
import os
import re
import sys
import time
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

USER_AGENT = "aether-allure-versions-check (https://github.com/jaygtel/ffxivshop)"
# The Dockerfile's names for each CPU, as in its download URLs and ARGs.
CPUS = {"x86_64": "AMD64", "aarch64": "ARM64"}


def open_url(url, accept):
    headers = {"User-Agent": USER_AGENT, "Accept": accept}
    token = os.environ.get("GH_TOKEN")
    if token and url.startswith("https://api.github.com/"):
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    for attempt in range(3):
        try:
            return urllib.request.urlopen(request, timeout=120)
        except urllib.error.HTTPError as error:
            if error.code < 500 or attempt == 2:
                raise
        except urllib.error.URLError:
            if attempt == 2:
                raise
        time.sleep(5)
    raise AssertionError("unreachable")


def get_json(url):
    with open_url(url, "application/json") as response:
        return json.load(response)


def get_text(url):
    with open_url(url, "application/octet-stream") as response:
        return response.read().decode()


def sha256_of(url):
    digest = hashlib.sha256()
    with open_url(url, "application/octet-stream") as response:
        while chunk := response.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def version_tuple(text):
    return tuple(int(part) for part in re.findall(r"\d+", text))


# ── Build tools in the Dockerfile ───────────────────────────────────────────


def dockerfile_args(path):
    args = {}
    for line in Path(path).read_text().splitlines():
        match = re.match(r"^ARG ([A-Z0-9_]+)=(\S+)$", line.strip())
        if match:
            args[match.group(1)] = match.group(2)
    return args


def latest_release(repo):
    return get_json(f"https://api.github.com/repos/{repo}/releases/latest")


def checksum_lines(prefix, release, downloads):
    """ARG lines with each download's SHA-256, and a note on how it was checked.

    `downloads` maps each CPU to (file name, URL). Where the release publishes
    a NAME.sha256 file beside a download, the worked-out checksum is compared
    with it.
    """
    assets = {asset["name"]: asset["browser_download_url"] for asset in release["assets"]}
    lines, compared, mismatched = [], 0, []
    for cpu, (name, url) in downloads.items():
        actual = sha256_of(url)
        published_url = assets.get(f"{name}.sha256")
        if published_url:
            compared += 1
            if get_text(published_url).split()[0].lower() != actual:
                mismatched.append(name)
        lines.append(f"ARG {prefix}_SHA256_{CPUS[cpu]}={actual}")
    if mismatched:
        note = (
            "**Do not use these yet:** "
            + ", ".join(f"`{name}`" for name in mismatched)
            + " does not match the checksum published with the release."
        )
    elif compared == len(downloads):
        note = "The checksums were worked out from the downloads, and match the ones published with the release."
    else:
        note = "The checksums were worked out from the downloads. The release has no checksums of its own to compare them with."
    return lines, note


def check_trunk(args):
    pinned = args["TRUNK_VERSION"]
    release = latest_release("trunk-rs/trunk")
    newest = release["tag_name"].removeprefix("v")
    if version_tuple(newest) <= version_tuple(pinned):
        return None
    base = f"https://github.com/trunk-rs/trunk/releases/download/v{newest}"
    downloads = {}
    for cpu in CPUS:
        name = f"trunk-{cpu}-unknown-linux-gnu.tar.gz"
        downloads[cpu] = (name, f"{base}/{name}")
    lines, note = checksum_lines("TRUNK", release, downloads)
    return ("Trunk", pinned, newest, [f"ARG TRUNK_VERSION={newest}", *lines], note)


def check_binaryen(args):
    pinned = args["BINARYEN_VERSION"]
    release = latest_release("WebAssembly/binaryen")
    newest = release["tag_name"].removeprefix("version_")
    if version_tuple(newest) <= version_tuple(pinned):
        return None
    base = f"https://github.com/WebAssembly/binaryen/releases/download/version_{newest}"
    downloads = {}
    for cpu in CPUS:
        name = f"binaryen-version_{newest}-{cpu}-linux.tar.gz"
        downloads[cpu] = (name, f"{base}/{name}")
    lines, note = checksum_lines("BINARYEN", release, downloads)
    return ("wasm-opt (binaryen)", pinned, newest, [f"ARG BINARYEN_VERSION={newest}", *lines], note)


def newest_crate_version(name):
    """The newest stable version on crates.io, or None if there's none."""
    time.sleep(1)  # crates.io asks for at most one request a second
    try:
        return get_json(f"https://crates.io/api/v1/crates/{name}")["crate"]["max_stable_version"]
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise


def check_leptosfmt(args):
    pinned = args["LEPTOSFMT_VERSION"]
    newest = newest_crate_version("leptosfmt")
    if newest is None or version_tuple(newest) <= version_tuple(pinned):
        return None
    note = "leptosfmt is built from crates.io when the dev container is built, so there is no checksum to change."
    return ("leptosfmt", pinned, newest, [f"ARG LEPTOSFMT_VERSION={newest}"], note)


# ── Crates that need Cargo.toml changing ────────────────────────────────────


def parse_version(text):
    """The numbers in a version or partial version: "0.8" -> [0, 8]."""
    numbers = []
    for part in text.split("-")[0].split("+")[0].split("."):
        if part.strip() in ("*", "x", "X"):
            break
        numbers.append(int(part))
    return numbers


def comparator_allows(comparator, version):
    """Whether one comparator of a Cargo version requirement allows `version`.

    Follows Cargo's rules: a bare version is a caret requirement ("0.8" allows
    0.8.x but not 0.9), and "~", "=", wildcards and <, <=, >, >= work as in
    the Cargo book.
    """
    comparator = comparator.strip()
    if comparator in ("", "*"):
        return True
    match = re.match(r"^(\^|~|=|>=|>|<=|<)?\s*(.+)$", comparator)
    op, wanted = match.group(1) or "^", match.group(2)
    given = parse_version(wanted)
    wildcard = bool(re.search(r"(^|\.)[*xX]($|\.)", wanted))
    v = tuple(version + [0] * (3 - len(version)))[:3]
    low = tuple(given + [0] * (3 - len(given)))

    if op in (">=", ">", "<=", "<"):
        return {">=": v >= low, ">": v > low, "<=": v <= low, "<": v < low}[op]
    if not given:
        return True

    if op == "^" and not wildcard:
        if len(given) == 1 or given[0] > 0:
            high = (given[0] + 1, 0, 0)
        elif len(given) == 2 or given[1] > 0:
            high = (0, given[1] + 1, 0)
        else:
            high = (0, 0, given[2] + 1)
    elif op == "~" and not wildcard and len(given) > 1:
        high = (given[0], given[1] + 1, 0)
    else:
        # "=", "~1" and wildcards: every version that starts with the given numbers.
        if len(given) == 3:
            return v == low
        high = (given[0] + 1, 0, 0) if len(given) == 1 else (given[0], given[1] + 1, 0)
    return low <= v < high


def requirement_allows(requirement, version):
    numbers = parse_version(version)
    return all(comparator_allows(part, numbers) for part in requirement.split(","))


def manifest_requirements(path, root):
    """(crate, requirement, file) for every crates.io dependency in a manifest."""
    manifest = tomllib.loads(path.read_text())
    tables = [manifest.get("workspace", {}).get("dependencies", {})]
    for key in ("dependencies", "dev-dependencies", "build-dependencies"):
        tables.append(manifest.get(key, {}))
    for target in manifest.get("target", {}).values():
        for key in ("dependencies", "dev-dependencies", "build-dependencies"):
            tables.append(target.get(key, {}))
    for table in tables:
        for name, spec in table.items():
            if isinstance(spec, str):
                yield name, spec, path.relative_to(root)
            elif isinstance(spec, dict) and "version" in spec and not ({"path", "git"} & spec.keys()):
                yield spec.get("package", name), spec["version"], path.relative_to(root)


def check_crates(root):
    manifests = [root / "Cargo.toml"]
    workspace = tomllib.loads(manifests[0].read_text()).get("workspace", {})
    manifests += [root / member / "Cargo.toml" for member in workspace.get("members", [])]
    rows, newest_seen = set(), {}
    for manifest in manifests:
        for crate, requirement, file in manifest_requirements(manifest, root):
            if crate not in newest_seen:
                newest_seen[crate] = newest_crate_version(crate)
            newest = newest_seen[crate]
            if newest and not requirement_allows(requirement, newest):
                rows.add((crate, requirement, newest, str(file)))
    return sorted(rows)


# ── The report ──────────────────────────────────────────────────────────────


def report(tools, crates):
    out = [
        "Dependabot keeps the crates in `Cargo.lock`, the Rust version, the base images and "
        "the actions up to date. Every Monday this check looks for newer versions of what "
        "Dependabot cannot update by itself, and these are the ones it found.",
        "",
    ]
    if tools:
        out += [
            "## Build tools in the Dockerfile",
            "",
            "| Tool | Pinned | Newest |",
            "| --- | --- | --- |",
            *[f"| {name} | {pinned} | {newest} |" for name, pinned, newest, _, _ in tools],
            "",
        ]
        for name, _, _, lines, note in tools:
            which = "this line" if len(lines) == 1 else "these lines"
            out += [f"To update {name}, change {which} in the `Dockerfile`:", "", "```dockerfile", *lines, "```", "", note, ""]
        out += [
            "Then rebuild the dev container to try it, and open a pull request as usual: the "
            "Build check builds the server's images with the new versions.",
            "",
        ]
    if crates:
        out += [
            "## Crates that need Cargo.toml changing",
            "",
            "| Crate | Cargo.toml allows | Newest | In |",
            "| --- | --- | --- | --- |",
            *[f"| `{crate}` | `{requirement}` | {newest} | `{file}` |" for crate, requirement, newest, file in crates],
            "",
            "Moving to these usually means changing the code as well, so they are yours to plan. "
            "Everything within what `Cargo.toml` allows arrives from Dependabot.",
            "",
        ]
    server, repo, run = (os.environ.get(key) for key in ("GITHUB_SERVER_URL", "GITHUB_REPOSITORY", "GITHUB_RUN_ID"))
    when = time.strftime("%-d %B %Y", time.gmtime())
    if server and repo and run:
        out.append(f"Checked on {when} by the [Versions check]({server}/{repo}/actions/runs/{run}).")
    else:
        out.append(f"Checked on {when}.")
    return "\n".join(out) + "\n"


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: check_versions.py REPORT.md")
    root = Path.cwd()
    args = dockerfile_args(root / "Dockerfile")
    tools = [found for check in (check_trunk, check_binaryen, check_leptosfmt) if (found := check(args))]
    crates = check_crates(root)
    found = bool(tools or crates)
    text = report(tools, crates) if found else "Everything pinned by hand is up to date.\n"
    Path(sys.argv[1]).write_text(text)
    print(text)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(f"found={'true' if found else 'false'}\n")


if __name__ == "__main__":
    main()
