# Aether & Allure

The website for **Aether & Allure**, Sigrid Halvorsen's crafting shop in
FINAL FANTASY XIV, at [aetherandallure.com](https://aetherandallure.com).
It's written in Rust and compiled to WebAssembly with [Leptos](https://leptos.dev),
built with [Trunk](https://trunkrs.dev), and runs in a Docker container on your
own server. The code is kept on GitHub.

© 2026 Jason Telford. Released under the [BSD 3-Clause License](LICENSE).

## Editing the shop

Everything on the page comes from **`shop.toml`**. Each part is commented:

| Section        | What it controls                                                        |
| -------------- | ----------------------------------------------------------------------- |
| `[site]`       | The site address and the copyright notice in the footer                  |
| `[shop]`       | Name, owner, open/busy/closed status, crafter levels, ordering steps      |
| `[location]`   | World, data center, housing district, ward, plot, and your retainer       |
| `[contact]`    | `/tell`, friend request, message book, Discord, email, other links        |
| `[[category]]` | The tabs above the item list                                             |
| `[[item]]`     | One per listing: name, category, price in gil, ready time, HQ, notes     |

Ready times are written like `"now"`, `"45m"`, `"6h"`, `"2d"`, `"1d 12h"` or `"1w"`.
Leave out `price` to show "Ask". `price_own_mats` adds a cheaper price for buyers
who bring the materials.

If something in `shop.toml` is wrong, the build stops and says exactly what to fix.
When your own details are in, set `demo = false` to remove the "sample shop" notice.

### Adding items

Write the in-game name, category, price and ready time, then run:

```sh
cargo run -p shop-tools -- fill
```

It looks up each item's Eorzea Database link, icon, item level, level and jobs,
and writes them into `shop.toml` without touching anything you've already set
(comments and layout are kept). To search for an item first:

```sh
cargo run -p shop-tools -- find courtly lover's
```

After a game patch, add `--refresh` to download fresh item data.

## Developing in the dev container (Zed)

Everything needed to work on the site runs inside a container, so your
computer only needs two things:

- **Docker** (Docker Desktop on macOS and Windows, Docker Engine on Linux) or
  Podman. With Podman, set `"use_podman": true` in Zed's settings.
- **Zed**, recent enough to have dev container support.

### First time

1. Open this folder in Zed. When Zed asks, choose **Open in Container**. (If
   you missed the prompt, run *Project: Open Remote* from the command palette
   and pick the dev container.)
2. Zed builds the image and starts the container. The first build takes a few
   minutes; after that it starts in seconds.
3. When it's ready, a terminal shows the installed tool versions. Zed's
   terminals, tasks and rust-analyzer now all run inside the container.

### Day to day

Run tasks with *task: spawn* (`cmd-shift-r` on macOS, `alt-shift-t` on Linux and Windows):

| Task                           | What it does                                                   |
| ------------------------------ | -------------------------------------------------------------- |
| `trunk serve`                  | Serves the site at http://localhost:8080, rebuilding on save   |
| `trunk build --release`        | Builds the finished site into `dist/`                          |
| `cargo test`                   | Runs the checks                                                |
| `cargo clippy (site, wasm32)`  | Lints the site for the target it ships to                     |
| `shop-tools: fill shop.toml`   | Looks up IDs, icons and stats for new items                    |
| `shop-tools: find selected item` | Searches for the item name you've selected                   |
| `leptosfmt: format view! macros` | Formats Leptos `view!` blocks, then everything else          |

Saving a Rust file formats it with rustfmt and shows compiler and clippy
warnings. `leptosfmt` isn't run on save because it removes the braces around
`view!` attribute values; write comparisons there as `n != 0` rather than
`n > 0`.

### What's in the container

Defined by the `dev` stage of the `Dockerfile`, on Debian 13 (trixie):

- Rust 1.97.0 (from `rust-toolchain.toml`) with rustfmt, clippy,
  rust-analyzer, rust-src and the `wasm32-unknown-unknown` target
- Trunk 0.22.1, wasm-bindgen 0.2.130 (matching `Cargo.lock`) and wasm-opt
  (Binaryen 131), so builds never download tools
- leptosfmt 0.1.33, git, a C compiler, curl, nano and sudo (no password)

Downloads are checked against the publishers' SHA-256 checksums. Both Intel/AMD
(x86_64) and ARM (Apple Silicon) machines are supported.

Build output, downloaded crates and your git settings live in Docker volumes
named `aether-allure-*`, so rebuilding the container is quick and keeps them.
Build output goes to the volume rather than `target/` in this folder; the
finished site still goes to `dist/` here.

### Git

Zed's git panel works inside the container. Set your name and email once from
its terminal (they're kept when the container is rebuilt):

```sh
git config --global user.name "Your Name"
git config --global user.email "you@example.com"
```

Your SSH keys stay on your computer, so if you push over SSH, push from a
terminal on your computer (the folder is the same).

### Changing the container

Zed doesn't rebuild automatically. After editing `.devcontainer/` or the
`Dockerfile`, stop the container (`docker ps` to find it, then
`docker kill <name>`) and reopen the project in the container.

- **Port 8080 in use:** change `forwardPorts` and `TRUNK_SERVE_PORT` in
  `.devcontainer/devcontainer.json`.
- **New Rust version:** change `channel` in `rust-toolchain.toml` and
  `RUST_VERSION` in the Dockerfile to match.
- **Updated wasm-bindgen:** if `cargo update` changes its version in
  `Cargo.lock`, update `WASM_BINDGEN_VERSION` and its two checksums in the
  Dockerfile (from the `.sha256sum` files on its GitHub release). Otherwise
  Trunk downloads the matching version itself, which still works.
- **Start fresh:** `docker volume rm` the `aether-allure-*` volumes.

The container also works in VS Code and other editors that support dev
containers.

## Running it without the container

1. Install Rust from [rustup.rs](https://rustup.rs). rustup reads
   `rust-toolchain.toml` and installs the right version, components and
   WebAssembly target the first time you run `cargo` here (or run
   `rustup toolchain install`).
2. Install Trunk: `cargo install --locked trunk`
3. Run `trunk serve --open`. The site opens at http://localhost:8080 and
   reloads whenever you save `shop.toml` or the code.

`cargo test --workspace` runs the checks.

## Hosting on your server

The site runs on your server in a Docker container, behind your Traefik:

```
browser ─HTTPS─▶ Cloudflare ─HTTPS─▶ Traefik (websecure) ─HTTP─▶ aether-allure-site:8080
```

Traefik handles HTTPS with your Cloudflare Origin certificate, as it does for
your other sites. Nothing in this project requests, renews or stores
certificates. The container is nginx serving the built site. The server builds
the image straight from this repository, so it only needs Docker.

### What you need

- Docker Engine with the Compose plugin on the server
- Traefik with a file provider (dynamic YAML files), your Origin certificate on
  the `websecure` entry point, and the `traefik_proxy` Docker network
- In Cloudflare, for aetherandallure.com:
  - DNS records for `aetherandallure.com` (and `www`, which redirects to it)
    pointing at the server, proxied (orange cloud)
  - SSL/TLS encryption mode set to **Full (strict)**
  - **Always Use HTTPS** turned on (SSL/TLS → Edge Certificates), so
    `http://` visits are redirected before they reach your server
- An Origin certificate that covers `aetherandallure.com` and
  `*.aetherandallure.com`. If the one Traefik serves was made for a different
  domain, create one for this domain in Cloudflare (SSL/TLS → Origin Server)
  and add it to the `tls.certificates` list in your Traefik config, next to the
  existing one. Traefik picks the right certificate for each domain itself.

### First deploy

On the server:

```sh
git clone <your GitHub repository URL> aether-and-allure
cd aether-and-allure
docker compose up -d --build
```

The first build compiles the site and takes several minutes. The container
joins your existing `traefik_proxy` network and publishes no ports on the
server; Traefik reaches it by name through Docker's DNS. Then give Traefik the
route by copying the dynamic config into the folder its file provider reads:

```sh
cp deploy/traefik/aether-allure.yml /path/to/traefik/dynamic/
```

Traefik loads it straight away if the file provider has `watch: true`;
otherwise restart Traefik. To check:

```sh
docker compose ps                  # STATUS should end in (healthy)
curl -I https://aetherandallure.com
```

If Traefik answers with "Bad Gateway", check that the site can be reached by
name on the network:

```sh
docker run --rm --network traefik_proxy busybox wget -qO- http://aether-allure-site:8080/ | head -3
```

### The Traefik config

`deploy/traefik/aether-allure.yml` sets up:

- `aetherandallure.com` on `websecure`, sent to the container at
  `http://aether-allure-site:8080`
- `www.aetherandallure.com`, permanently redirected to
  `https://aetherandallure.com` (keeping the rest of the address)

Both routers have `tls: {}` and no `certResolver`, so Traefik serves the
certificate it already has and never contacts Let's Encrypt. If your HTTPS entry
point has a different name, change `websecure` in the file.

### Updating the site

Edit and commit as usual (for example new prices in `shop.toml`), push to
GitHub, then on the server:

```sh
git pull
docker compose up -d --build
```

Compiled dependencies are cached between builds, so updates are quick. Traefik
keeps routing to the new container by name; copy `aether-allure.yml` again only
if that file itself changed.

Every month or so, run `docker compose build --pull` before `up` to pick up
security fixes in the nginx and Rust base images, and `docker image prune` to
clear out old images. `docker compose logs -f` shows nginx's log (visitors
appear with Traefik's address there; Traefik's access log has the real ones).
`docker compose down` stops the site.

### The container

- nginx 1.30 (`nginx/nginx-unprivileged`, Alpine), set up by `deploy/nginx.conf`
- Runs as a non-root user, with a read-only filesystem and no Linux capabilities
- Listens on port 8080 inside the container only. No ports are published on
  the server: it's reachable just from the `traefik_proxy` network
- Browsers keep the build files with hashed names (`.wasm`, `.js`, `.css`) for
  a year and check everything else, including the page itself, on every visit,
  so changes show up straight away
- Has a health check, and its logs are capped at 30 MB

## Settings and secrets (`.env`)

Anything secret goes in `.env`, created from `.env.example`. The site needs no
settings or secrets today, so `.env` is optional. It is:

- ignored by git (see `.gitignore`), so it never reaches GitHub, and
- ignored by Docker (see `.dockerignore`), so it's never copied into an image.

The site itself has no secrets: it's static, and everything in `shop.toml` is
shown on the page. Keep it that way: never put passwords, tokens or keys in
`shop.toml` or the code. If you add something secret later, put it in `.env`,
add its name (without the value) to `.env.example`, and pass it to the
container in `compose.yaml`. `git status` should never list `.env`.

Your Cloudflare Origin certificate and its private key belong with Traefik's
own config. Never copy them into this repository.

## The logo and hero

`assets/logo.svg` and `assets/favicon.svg` are the A&A monogram as transparent
SVGs. They use dark letters on light backgrounds and light letters on dark ones
(following the viewer's light or dark mode), and every letter has a thin
contrasting edge, so the mark stays readable even on a background that doesn't
match. A page that inlines it can choose its colours with the `--aa-ink`,
`--aa-accent` and `--aa-edge` CSS custom properties.

`assets/favicon.ico` is a fallback copy of the favicon (16, 32 and 48 pixels, in
its light-background colours) for places that can't show SVG, such as Google
Search results, older Safari and tools that only ever ask for `/favicon.ico`.
Browsers that can show the SVG (Chrome, Edge, Firefox) still use it: the
`sizes="32x32"` on the ICO's link in `index.html` is what stops Chrome from
choosing the ICO instead. If the favicon's design changes, remake the ICO too.

`assets/hero.svg` is the preview card's design (monogram, «Master Artisan»,
the shop's name and tagline) on a transparent background. It handles light and
dark backgrounds the same way as the logo, and arranges itself for the space it
has:

- landscape, 900px and wider: monogram on the left, text on the right
- landscape, under 900px: the same arrangement, with larger text for the width
- narrower than 5:4 (phones, tablets held upright): stacked

It always sits centred in its space. The site opens with it: a full-screen,
centred hero with an **Order Now** button that scrolls down to the shop, the
shop's status and the crafting jobs. The site inlines the file, so it follows
the browser window, and sets its colours with the custom properties above plus
`--aa-muted` (the tagline) and `--aa-glow`. Opened on its own or used as an
image, it follows its own box. As an image it has no fixed size: give it a
width and the aspect ratio listed at the top of the file.

Once the hero has scrolled away, a **Back to top** button fades into the
bottom-right corner. On wide windows (about 1376px and up) the margin beside the
page has room for it, so it shows straight away. On narrower windows it would
cover the page as it scrolls past, so it waits until you reach the very bottom,
where it sits in the empty space below the footer. It's always hidden while the
hero is on screen and whenever the page is stacked (960px wide and under).

The hero's lettering is drawn as shapes, so its words are fixed, as on the
preview card. `name`, `title` and `tagline` in `shop.toml` are what screen
readers hear, so keep them matching; different wording on screen means
redrawing the hero.

The letterforms are drawn from Cinzel, Cinzel Decorative and Alegreya Sans (SIL
Open Font License). The site also serves `logo.svg`, `hero.svg`,
`favicon.svg`, `favicon.ico`, `og-card.png` (the preview image shown when the
link is shared on Discord and elsewhere) and `apple-touch-icon.png`.

## Item links, tooltips and icons

- Item names link to the official **Eorzea Database** on The Lodestone
  (`lodestone = "eu"` in `shop.toml`; `na`, `fr`, `de` and `jp` also work).
- Hovering a link shows Square Enix's official Eorzea Database tooltip
  (`tooltips = false` turns this off).
- Item icons are loaded from [XIVAPI](https://v2.xivapi.com) (`icons = false`
  shows the drawn category symbols instead).
- `shop-tools` looks items up in [FFXIV Teamcraft](https://github.com/ffxiv-teamcraft/ffxiv-teamcraft)'s
  game data and [Asvel's Eorzea Database ID list](https://github.com/Asvel/ffxiv-lodestone-item-id).

## Project layout

```
shop.toml               the shop's details: edit this
Dockerfile              the dev container and the server image (four stages)
compose.yaml            runs the site on your server
deploy/nginx.conf       the web server's settings, inside the container
deploy/traefik/         the Traefik route for aetherandallure.com
.env.example            template for .env (server settings and secrets)
.devcontainer/          dev container settings and setup script
.zed/                   Zed project settings and tasks
rust-toolchain.toml     the Rust version and components
index.html, Trunk.toml  page shell and build settings
build.rs                checks shop.toml at build time
src/model.rs            reads and checks shop.toml
src/order.rs            the visitor's order and the messages they copy
src/components/         the parts of the page
style/main.css          the look
assets/                 monogram, favicon, hero, preview images, fonts
tools/                  shop-tools, the item lookup helper
LICENSE                 BSD 3-Clause License
```

## License and credits

The code and site design are © 2026 Jason Telford, released under the
[BSD 3-Clause License](LICENSE).

Fonts: Cinzel, Cinzel Decorative and Alegreya Sans, under their own SIL Open
Font License 1.1 (see `assets/fonts`).

FINAL FANTASY XIV © SQUARE ENIX CO., LTD. All Rights Reserved. FINAL FANTASY is a
registered trademark of Square Enix Holdings Co., Ltd. This is an unofficial fan
site and isn't affiliated with or endorsed by Square Enix.
