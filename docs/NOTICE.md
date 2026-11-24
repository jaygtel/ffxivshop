# Licences and notices

© 2026 Jason Telford. This repository holds material under different terms,
and the [BSD 3-Clause License](../LICENSE) covers only the software:

| Material | Terms |
| --- | --- |
| Rust application code | [BSD 3-Clause License](#software-bsd-3-clause-license) |
| JavaScript and development tools | [BSD 3-Clause License](#software-bsd-3-clause-license) |
| CSS implementation | [BSD 3-Clause License](#software-bsd-3-clause-license) |
| Original logos and artwork | [All rights reserved](#logos-artwork-and-branding-all-rights-reserved) |
| Original branding and visual identity | [All rights reserved](#logos-artwork-and-branding-all-rights-reserved) |
| Third-party fonts | [Their own SIL Open Font License 1.1](#fonts-sil-open-font-license) |
| Third-party game data and assets | [Their respective terms](#final-fantasy-xiv-game-data-and-assets-their-respective-terms) |

## Software: BSD 3-Clause License

Everything in this repository is under the BSD 3-Clause License in
[`LICENSE`](../LICENSE), apart from what the sections below list. That includes:

- the Rust application code: [`src/`](../src), [`build.rs`](../build.rs) and the
  Cargo files;
- the JavaScript and development tools:
  [`assets/js/intro.js`](../assets/js/intro.js), shop-tools in [`tools/`](../tools),
  the orders service in [`orders/`](../orders),
  the [`Dockerfile`](../Dockerfile), [`compose.yaml`](../compose.yaml),
  [`deploy/`](../deploy), the build check in [`.github/`](../.github), the git hook
  in [`.githooks/`](../.githooks), and the dev container and editor settings in
  [`.devcontainer/`](../.devcontainer) and [`.zed/`](../.zed);
- the CSS implementation: [`assets/css/main.css`](../assets/css/main.css).

## Logos, artwork and branding: all rights reserved

© 2026 Jason Telford. All rights reserved. The BSD 3-Clause License does not
cover any of these, and they may not be copied, changed or reused without
Jason Telford's written permission:

- the crystal emblem and favicon: [`assets/img/logo.svg`](../assets/img/logo.svg),
  [`assets/img/favicon.svg`](../assets/img/favicon.svg),
  [`assets/img/favicon.ico`](../assets/img/favicon.ico) and
  [`assets/img/apple-touch-icon.png`](../assets/img/apple-touch-icon.png);
- the hero picture, [`assets/img/hero.svg`](../assets/img/hero.svg), and the link
  preview image, [`assets/img/og-card.png`](../assets/img/og-card.png), which is
  also the README's banner;
- the emblem, corner pieces and divider drawn inline in
  [`src/components/hero.rs`](../src/components/hero.rs) (`EMBLEM`, `CORNERS` and
  `DIVIDER`), and the window corner pieces drawn in
  [`assets/css/main.css`](../assets/css/main.css) (`.window::before`);
- the order receipt email's pictures in
  [`assets/img/email/`](../assets/img/email): the emblem, the divider, the
  diamond and the frame's corner pieces, drawn from the pieces above;
- the Aether & Allure name, and the brand and visual identity made from it and
  the pieces above.

The code that draws or loads them stays under the BSD licence. If you reuse
it, put your own name, logo and artwork in their place, and don't present your
site as Aether & Allure.

The lettering in `hero.svg`, `og-card.png` and the intro is drawn from Cinzel
and Cormorant Garamond. The fonts' licence covers the fonts, not the artwork
drawn with them.

## Fonts: SIL Open Font License

The fonts in [`assets/fonts/`](../assets/fonts) are not under the BSD licence.
Each keeps its own SIL Open Font License 1.1, in the file beside it:

| Font | Files | Copyright | Licence |
| --- | --- | --- | --- |
| Cinzel | `cinzel.woff2` | Copyright 2020 The Cinzel Project Authors (https://github.com/NDISCOVER/Cinzel) | [`OFL-Cinzel.txt`](../assets/fonts/OFL-Cinzel.txt) |
| Cormorant Garamond | `cormorant-garamond-italic.woff2` | Copyright 2015 the Cormorant Project Authors (github.com/CatharsisFonts/Cormorant) | [`OFL-Cormorant-Garamond.txt`](../assets/fonts/OFL-Cormorant-Garamond.txt) |
| Alegreya Sans | `alegreya-sans-latin-*.woff2` | Copyright 2013 The Alegreya Sans Project Authors (https://github.com/huertatipografica/Alegreya-Sans) | [`OFL-Alegreya-Sans.txt`](../assets/fonts/OFL-Alegreya-Sans.txt) |

## FINAL FANTASY XIV game data and assets: their respective terms

FINAL FANTASY XIV © SQUARE ENIX CO., LTD. All Rights Reserved. FINAL FANTASY
is a registered trademark of Square Enix Holdings Co., Ltd. This is an
unofficial fan site and isn't affiliated with or endorsed by Square Enix.

The game's material belongs to Square Enix. Neither the BSD licence nor Jason
Telford's copyright covers:

- the item names, and the in-game IDs, Eorzea Database IDs, icon numbers,
  item levels and jobs that shop-tools writes into [`shop.toml`](../shop.toml);
- the names of the crafting and gathering jobs, and other game terms;
- the item icons, which visitors' browsers load from
  [XIVAPI](https://v2.xivapi.com) and which aren't kept in this repository;
- the [Eorzea Database](https://eu.finalfantasyxiv.com/lodestone/playguide/db/)
  that item names link to, and its tooltips, which load from Square Enix's
  own servers.

The site uses this material under Square Enix's FINAL FANTASY XIV Materials
Usage Policy, which Square Enix publishes for each region where accounts are
registered, such as
[Europe](https://support.eu.square-enix.com/rule.php?id=5383&la=2&tag=authc)
and
[North America](https://support.na.square-enix.com/rule.php?id=5382&la=1&tag=authc).

shop-tools downloads its item data while it runs and keeps it in Cargo's
target folder, not in this repository. The data comes from:

- [FFXIV Teamcraft](https://github.com/ffxiv-teamcraft/ffxiv-teamcraft)'s
  game-data files. The project is under the MIT License; the game data in it
  is Square Enix's.
- [Asvel's Eorzea Database ID list](https://github.com/Asvel/ffxiv-lodestone-item-id),
  released to the public domain apart from the FINAL FANTASY XIV content in
  it, which is Square Enix's.

shop-tools' `prices` command also reads current market board prices from
[Universalis](https://universalis.app) each time it runs, and keeps none of
them. Players upload those prices from the game. Universalis's own software is
under the MIT License, and it states no terms for its price data. The gil
prices the command writes, into [`shop.toml`](../shop.toml) or into the
`prices.json` the site reads on the server, are worked out from them and from
the recipes in FFXIV Teamcraft's game data.

## Other software

The Rust crates listed in [`Cargo.lock`](../Cargo.lock), and the tools that build
and serve the site (such as Trunk, wasm-bindgen, Binaryen and nginx, set in the
`Dockerfile`), keep their own licences.
