# Aether & Allure: one Dockerfile for development and for the server.
#
#   tools        Rust (from rust-toolchain.toml), Trunk, wasm-bindgen and wasm-opt
#   dev          the dev container Zed opens: tools + git, leptosfmt and a user account
#   build        compiles the site with `trunk build --release`
#   site         what runs on the server: nginx serving the built site (behind Traefik)
#   shop-tools   compiles shop-tools
#   prices       what also runs on the server: shop-tools updating the market
#                prices in prices.json every week, for the site to read
#   shop-orders  compiles the orders service
#   orders       what also runs on the server: the orders service, emailing
#                the orders sent with the order panel's Send by email
#
# On the server, `docker compose up -d --build` builds and runs `site`,
# `prices` and `orders` (see compose.yaml). Zed builds `dev` (see
# .devcontainer/devcontainer.json).
# Requires BuildKit, which Docker uses by default.
#
# Nothing secret is needed to build, and .dockerignore keeps .env out of the
# build, so no image ever contains your settings. The orders service's
# Postmark token reaches only its running container (see compose.yaml).

# Only bootstraps rustup; the toolchain that's used comes from
# rust-toolchain.toml. Keep the two equal so Rust isn't downloaded twice.
ARG RUST_VERSION=1.97.0
ARG NGINX_VERSION=1.30.6
# The Debian release under Rust and under the prices image, kept the same so
# shop-tools runs where it was built. It must be trixie or newer: Trunk's
# prebuilt binary needs glibc 2.39 or newer.
ARG DEBIAN_RELEASE=trixie

# =============================================================================
FROM rust:${RUST_VERSION}-slim-${DEBIAN_RELEASE} AS tools

# Pinned tool versions and the SHA-256 of each download (the upstream release
# checksums). wasm-bindgen must match the version in Cargo.lock: after
# `cargo update` changes it, update WASM_BINDGEN_* here too.
ARG TRUNK_VERSION=0.22.1
ARG TRUNK_SHA256_AMD64=2ab8d548fd5ed9cda50906f61fd8654204e7893bd52ebbfd90533862b7d71ee1
ARG TRUNK_SHA256_ARM64=b4017d1e0bfcffb4f4bee2be37c48def21e6e6896c3d0f503c43c42f714e57ea
ARG WASM_BINDGEN_VERSION=0.2.130
ARG WASM_BINDGEN_SHA256_AMD64=9687f3bb551b9f66f37ed32a40d7913a00f7962db653054e56857bc306c3227f
ARG WASM_BINDGEN_SHA256_ARM64=cc8dd8e90aa9111758168d30c63a14b7f0276997d9d60d3e160a1fb5e7d3b919
ARG BINARYEN_VERSION=131
ARG BINARYEN_SHA256_AMD64=b5bf1f0eaf17c63ee588ff7a5954dc8f6ce2c26989051c66f24dfe9ece3e46db
ARG BINARYEN_SHA256_ARM64=ba991f677edd9a21d2bc96c0144bc8ac5b112d4d98a3eb266e075e22e557df2a

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# A bare `rustup toolchain install` installs the toolchain rust-toolchain.toml
# names, with its components and targets. The checks fail the build if the
# tools this project relies on are missing.
COPY rust-toolchain.toml /tmp/toolchain/rust-toolchain.toml
RUN set -eux; \
    cd /tmp/toolchain; \
    rustup toolchain install; \
    rustup default "$(rustup show active-toolchain | cut -d' ' -f1)"; \
    rustup target list --installed | grep -qx wasm32-unknown-unknown; \
    rustc --version; \
    cargo clippy --version; \
    rustfmt --version; \
    rust-analyzer --version; \
    cd /; \
    rm -rf /tmp/toolchain; \
    chmod -R a+w "${RUSTUP_HOME}" "${CARGO_HOME}"

# Trunk, wasm-bindgen and wasm-opt: prebuilt and checksum-verified, so builds
# never download tools.
RUN set -eux; \
    arch="$(dpkg --print-architecture)"; \
    case "$arch" in \
        amd64) \
            cpu=x86_64; \
            trunk_sha="${TRUNK_SHA256_AMD64}"; \
            bindgen_target=x86_64-unknown-linux-musl; \
            bindgen_sha="${WASM_BINDGEN_SHA256_AMD64}"; \
            binaryen_sha="${BINARYEN_SHA256_AMD64}"; \
            ;; \
        arm64) \
            cpu=aarch64; \
            trunk_sha="${TRUNK_SHA256_ARM64}"; \
            bindgen_target=aarch64-unknown-linux-gnu; \
            bindgen_sha="${WASM_BINDGEN_SHA256_ARM64}"; \
            binaryen_sha="${BINARYEN_SHA256_ARM64}"; \
            ;; \
        *) \
            echo "Unsupported CPU architecture: $arch (use amd64 or arm64)" >&2; \
            exit 1; \
            ;; \
    esac; \
    download() { \
        curl --fail --location --silent --show-error --retry 3 --output "$2" "$1"; \
        echo "$3  $2" | sha256sum --check --strict -; \
    }; \
    work="$(mktemp -d)"; \
    cd "$work"; \
    \
    download "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-${cpu}-unknown-linux-gnu.tar.gz" \
        trunk.tar.gz "$trunk_sha"; \
    tar -xzf trunk.tar.gz -C /usr/local/bin trunk; \
    \
    bindgen_dir="wasm-bindgen-${WASM_BINDGEN_VERSION}-${bindgen_target}"; \
    download "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${WASM_BINDGEN_VERSION}/${bindgen_dir}.tar.gz" \
        wasm-bindgen.tar.gz "$bindgen_sha"; \
    tar -xzf wasm-bindgen.tar.gz --strip-components=1 -C /usr/local/bin \
        "${bindgen_dir}/wasm-bindgen" "${bindgen_dir}/wasm-bindgen-test-runner"; \
    \
    download "https://github.com/WebAssembly/binaryen/releases/download/version_${BINARYEN_VERSION}/binaryen-version_${BINARYEN_VERSION}-${cpu}-linux.tar.gz" \
        binaryen.tar.gz "$binaryen_sha"; \
    tar -xzf binaryen.tar.gz --strip-components=2 -C /usr/local/bin \
        "binaryen-version_${BINARYEN_VERSION}/bin/wasm-opt"; \
    \
    cd /; \
    rm -rf "$work"; \
    trunk --version; \
    wasm-bindgen --version; \
    wasm-opt --version

# =============================================================================
# The dev container. Every tool for working on the site lives here, so your
# computer only needs Docker (or Podman) and Zed.
FROM tools AS dev

ARG LEPTOSFMT_VERSION=0.1.33

# The account you work as. On Linux and macOS, Zed changes its UID/GID to
# match yours when it builds the container, so files keep your ownership.
ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=${USER_UID}

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bash-completion \
        build-essential \
        git \
        less \
        nano \
        openssh-client \
        pkg-config \
        procps \
        sudo \
    && rm -rf /var/lib/apt/lists/*

# leptosfmt formats Leptos view! macros. It's built from source (there are no
# prebuilt Linux binaries); the crate cache is removed afterwards so the
# registry volume starts out empty.
RUN set -eux; \
    CARGO_TARGET_DIR=/tmp/leptosfmt-build cargo install --locked "leptosfmt@${LEPTOSFMT_VERSION}"; \
    rm -rf /tmp/leptosfmt-build "${CARGO_HOME}/registry" "${CARGO_HOME}/git"; \
    mkdir -p "${CARGO_HOME}/registry"; \
    leptosfmt --version; \
    # Like the base image: let any user run cargo, add crates and install tools.
    chmod -R a+w "${CARGO_HOME}" "${RUSTUP_HOME}"

# The two folders below are mounted as Docker volumes by devcontainer.json:
# build output (fast on macOS and Windows, kept between container rebuilds)
# and your git settings (so your name and email are remembered). Creating them
# here gives the volumes the right owner.
RUN set -eux; \
    groupadd --gid "${USER_GID}" "${USERNAME}"; \
    useradd --uid "${USER_UID}" --gid "${USER_GID}" --create-home --shell /bin/bash "${USERNAME}"; \
    echo "${USERNAME} ALL=(root) NOPASSWD:ALL" > "/etc/sudoers.d/${USERNAME}"; \
    chmod 0440 "/etc/sudoers.d/${USERNAME}"; \
    mkdir -p "/home/${USERNAME}/.cache/cargo-target" "/home/${USERNAME}/.config/git"; \
    touch "/home/${USERNAME}/.config/git/config"; \
    chown -R "${USERNAME}:${USERNAME}" "/home/${USERNAME}"; \
    # Debian's /etc/profile resets PATH for login shells, which would hide
    # cargo's tools. Keep the PATH set by this image instead.
    sed -i -E 's/((^|\s)PATH=)([^\$]*)$/\1${PATH:-\3}/g' /etc/profile

ENV LANG=C.UTF-8 \
    EDITOR=nano \
    # Build output goes to a volume rather than ./target in the project folder.
    CARGO_TARGET_DIR=/home/${USERNAME}/.cache/cargo-target \
    # Listen on all interfaces so the published port reaches `trunk serve`.
    TRUNK_SERVE_ADDRESS=0.0.0.0

USER ${USERNAME}
WORKDIR /home/${USERNAME}

# =============================================================================
# Compiles the site. The cache mounts keep downloaded crates and compiled
# dependencies between builds, so after editing shop.toml a rebuild on the
# server takes seconds rather than minutes.
FROM tools AS build

# The shop details shop.toml reads from .env, passed in by compose.yaml (.env
# itself is never copied into a build). build.rs picks them up from here.
ARG SHOP_DISCORD=
ARG SHOP_EMAIL=

WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    trunk build --release

# =============================================================================
# What runs on the server: nginx serving the finished site over HTTP on port
# 8080, as an unprivileged user. Traefik sits in front and handles HTTPS with
# your Cloudflare Origin certificate (see deploy/traefik/aether-allure.yml).
FROM nginx/nginx-unprivileged:${NGINX_VERSION}-alpine-slim AS site

COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html

# =============================================================================
# Compiles shop-tools for the prices image, with the same caches as `build`.
FROM tools AS shop-tools

WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked -p shop-tools \
    && cp target/release/shop-tools /usr/local/bin/shop-tools

# =============================================================================
# What also runs on the server: shop-tools working out the market prices
# when the site starts or is updated, then every week (the command is in
# compose.yaml), and writing them to /data/prices.json. The site reads that
# file from the same volume (see deploy/nginx.conf). It runs as an
# unprivileged user and only reaches out to Universalis and GitHub.
FROM debian:${DEBIAN_RELEASE}-slim AS prices

# CA certificates, so shop-tools can check the sites it talks to.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# /data is where the prices go. Its owner carries over to the volume
# mounted there, so shop-tools can write to it.
RUN set -eux; \
    groupadd --system --gid 10001 prices; \
    useradd --system --uid 10001 --gid prices --no-create-home --shell /usr/sbin/nologin prices; \
    mkdir /data; \
    chown prices:prices /data

COPY --from=shop-tools /usr/local/bin/shop-tools /usr/local/bin/shop-tools
COPY shop.toml /app/shop.toml

# shop-tools keeps Teamcraft's recipes and item names in the Cargo target
# directory; here that's /tmp (a tmpfs in compose.yaml).
ENV CARGO_TARGET_DIR=/tmp
USER prices
ENTRYPOINT ["shop-tools"]

# =============================================================================
# Compiles the orders service, with the same caches as `build`.
FROM tools AS shop-orders

WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked -p shop-orders \
    && cp target/release/shop-orders /usr/local/bin/shop-orders

# =============================================================================
# What also runs on the server: the orders service. It takes the orders sent
# with the order panel's Send by email, which the site's nginx passes on to it
# (see deploy/nginx.conf), and emails each one through Postmark, to you with a
# copy to the buyer. It takes the messages sent with the contact page's form
# the same way, and emails them to you. Its settings come from .env through
# compose.yaml. It runs as an unprivileged user and only reaches out to
# Postmark. It needs no CA certificates from Debian: it carries its own.
FROM debian:${DEBIAN_RELEASE}-slim AS orders

RUN set -eux; \
    groupadd --system --gid 10002 orders; \
    useradd --system --uid 10002 --gid orders --no-create-home --shell /usr/sbin/nologin orders

COPY --from=shop-orders /usr/local/bin/shop-orders /usr/local/bin/shop-orders
COPY shop.toml /app/shop.toml

USER orders
ENTRYPOINT ["shop-orders"]
CMD ["--listen", "0.0.0.0:8080", "--prices", "/srv/prices/prices.json", "/app/shop.toml"]
