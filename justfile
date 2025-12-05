# CS2 Rust Plugin Build Commands

# SteamRT path for native builds - override with STEAMRT_PATH env var
steamrt_path := env_var_or_default("STEAMRT_PATH", "~/steamrt")

# Local CS2 server path - override with CS2_SERVER_PATH env var
cs2_server_path := env_var_or_default("CS2_SERVER_PATH", justfile_directory() / "server")

# Default recipe
default: build

# Build for development (native)
build:
    cargo build
    cp target/debug/libcs2rust.so target/debug/cs2rust.so 2>/dev/null || true

# Build release (native) - won't work on SteamRT servers due to glibc
release:
    cargo build --release
    cp target/release/libcs2rust.so target/release/cs2rust.so

# Build for SteamRT (Docker) - use this for server deployment
steamrt:
    docker build -t cs2rust-builder .
    docker run --rm -v {{justfile_directory()}}:/workspace cs2rust-builder \
        sh -c "cargo build --release && cp target/release/libcs2rust.so target/release/cs2rust.so"

# Build for SteamRT using native runtime (no Docker)
# Install SteamRT3 via steamcmd: app_update 1628350
steamrt-native:
    {{steamrt_path}}/run -- cargo build --release
    cp target/release/libcs2rust.so target/release/cs2rust.so

# Run tests
test:
    cargo test

# Run clippy lints
lint:
    cargo clippy --all-targets

# Format code
fmt:
    cargo fmt

# Clean build artifacts
clean:
    cargo clean

# Package release for deployment (builds + creates zip)
package: steamrt
    rm -rf target/package
    mkdir -p target/package/addons/cs2rust/bin/linuxsteamrt64
    cp target/release/cs2rust.so target/package/addons/cs2rust/bin/linuxsteamrt64/
    cp dist/addons/cs2rust.vdf target/package/addons/
    cd target/package && zip -r ../cs2rust-linux.zip addons/
    @echo "Package created: target/cs2rust-linux.zip"

# Package using native SteamRT build (no Docker)
package-native: steamrt-native
    rm -rf target/package
    mkdir -p target/package/addons/cs2rust/bin/linuxsteamrt64
    cp target/release/cs2rust.so target/package/addons/cs2rust/bin/linuxsteamrt64/
    cp dist/addons/cs2rust.vdf target/package/addons/
    cd target/package && zip -r ../cs2rust-linux.zip addons/
    @echo "Package created: target/cs2rust-linux.zip"

# Build and copy to a CS2 server addon directory (customize path as needed)
deploy path:
    just steamrt
    mkdir -p {{path}}/addons/cs2rust/bin/linuxsteamrt64
    cp target/release/cs2rust.so {{path}}/addons/cs2rust/bin/linuxsteamrt64/
    cp dist/addons/cs2rust.vdf {{path}}/addons/

# Full local dev workflow: build, deploy, run server
dev: steamrt-native
    mkdir -p {{cs2_server_path}}/game/csgo/addons/cs2rust/bin/linuxsteamrt64
    cp target/release/cs2rust.so {{cs2_server_path}}/game/csgo/addons/cs2rust/bin/linuxsteamrt64/
    cp dist/addons/cs2rust.vdf {{cs2_server_path}}/game/csgo/addons/
    cd {{cs2_server_path}} && {{steamrt_path}}/run ./cs2.sh -dedicated +map de_dust2

# Deploy to local server only (no build, no server start)
deploy-local:
    mkdir -p {{cs2_server_path}}/game/csgo/addons/cs2rust/bin/linuxsteamrt64
    cp target/release/cs2rust.so {{cs2_server_path}}/game/csgo/addons/cs2rust/bin/linuxsteamrt64/
    cp dist/addons/cs2rust.vdf {{cs2_server_path}}/game/csgo/addons/
