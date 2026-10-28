#!/bin/bash

set -e

# Check for dry-run flag
DRY_RUN=false
if [[ "$1" == "--dry-run" ]] || [[ "$1" == "-n" ]]; then
    DRY_RUN=true
    echo "🔍 DRY RUN MODE: Will only check prerequisites and show what would be installed"
fi

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to log with timestamp
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

# Function to run command or show what would run in dry-run mode
run_cmd() {
    if [[ "$DRY_RUN" == "true" ]]; then
        echo "  [DRY RUN] Would run: $*"
    else
        "$@"
    fi
}

log "🚀 Starting rust development environment setup..."

add_to_shell_config() {
  local var_export="$1"
  local config_file="$2"
  if ! grep -qxF "$var_export" "$config_file" 2>/dev/null; then
    echo "$var_export" >> "$config_file"
    echo "Added $var_export to $config_file"
  else
    echo "$var_export already exists in $config_file"
  fi
}

# Update Rust and install components
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain $RUST_VERSION \
    && sudo echo 'export PATH="$PATH:$HOME/.cargo/bin"' >> $HOME/.zshrc

. "$HOME/.cargo/env"
export PATH="$PATH:$HOME/.cargo/bin"
add_to_shell_config 'export PATH="$PATH:$HOME/.cargo/bin"' "$HOME/.zshrc"

echo "🦀 Checking Rust toolchain..."
# Check if update is needed
CURRENT_VERSION=$(rustc --version 2>/dev/null | awk '{print $2}' || echo "none")
LATEST_VERSION=$(rustup check 2>/dev/null | grep "stable" | head -1 | awk '{print $2}' | sed 's/-.*//' || echo "unknown")

if [ "$CURRENT_VERSION" != "$LATEST_VERSION" ] && [ "$LATEST_VERSION" != "unknown" ]; then
    echo "📦 Updating Rust toolchain from $CURRENT_VERSION to $LATEST_VERSION..."
    rustup update stable
    rustup default stable
else
    echo "✅ Rust toolchain is up to date ($CURRENT_VERSION)"
fi

# Check and install Rust components if missing
echo "🔧 Checking Rust components..."
for component in rust-analyzer rust-src rustfmt clippy; do
    if ! rustup component list --installed | grep -q "^$component"; then
        echo "  Installing $component..."
        rustup component add $component
    else
        echo "  ✓ $component already installed"
    fi
done

# Install additional Rust tools if not present or outdated
echo "🔧 Checking additional Rust development tools..."
TOOLS="cargo-watch cargo-edit cargo-expand cargo-outdated"
for tool in $TOOLS; do
    TOOL_NAME=$(echo $tool | sed 's/cargo-//')
    if ! cargo install --list | grep -q "^$tool "; then
        echo "  Installing $tool..."
        cargo install $tool
    else
        INSTALLED_VER=$(cargo install --list | grep "^$tool " | awk '{print $2}' | tr -d '():')
        LATEST_VER=$(cargo search $tool --limit 1 2>/dev/null | head -1 | awk '{print $3}' | tr -d '"=' || echo "")
        if [ -n "$LATEST_VER" ] && [ "$INSTALLED_VER" != "$LATEST_VER" ]; then
            echo "  Updating $tool from $INSTALLED_VER to $LATEST_VER..."
            cargo install $tool --force
        else
            echo "  ✓ $tool is up to date ($INSTALLED_VER)"
        fi
    fi
done

# ── sccache ────────────────────────────────────────────────────────────────
# 컨테이너마다 CARGO_TARGET_DIR 이 /mnt/build/cargo-<slug> 로 분리돼 있어,
# 컨테이너 사이의 컴파일 결과 재사용은 sccache 캐시(/mnt/build/sccache)만 제공한다.
# 바이너리는 이미지에 굽지 않고 공유 마운트에 둔다 (static-pie musl → 배포판 무관).
SCCACHE_BIN="${SCCACHE_BIN:-/mnt/build/bin/sccache}"
SCCACHE_VERSION="${SCCACHE_VERSION:-v0.18.0}"
echo "📦 Checking sccache..."
if [ -x "$SCCACHE_BIN" ]; then
    echo "  ✓ $SCCACHE_BIN ($("$SCCACHE_BIN" --version 2>/dev/null))"
elif [ -w "$(dirname "$(dirname "$SCCACHE_BIN")")" ] || [ -w "$(dirname "$SCCACHE_BIN")" ]; then
    A="sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl"
    echo "  Installing sccache ${SCCACHE_VERSION} → $SCCACHE_BIN"
    run_cmd mkdir -p "$(dirname "$SCCACHE_BIN")" /tmp/sccache-dl
    (cd /tmp/sccache-dl \
        && curl -sSLO "https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/${A}.tar.gz" \
        && curl -sSLO "https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/${A}.tar.gz.sha256" \
        && echo "$(awk '{print $1}' "${A}.tar.gz.sha256")  ${A}.tar.gz" | sha256sum -c - \
        && tar xzf "${A}.tar.gz" \
        && install -m 755 "${A}/sccache" "$SCCACHE_BIN") \
        || echo "  ⚠ sccache 설치 실패 — RUSTC_WRAPPER 없이 계속"
else
    echo "  ⚠ $SCCACHE_BIN 없음 + /mnt/build 쓰기 불가 — sccache 건너뜀"
fi
if [ -x "$SCCACHE_BIN" ]; then
    export RUSTC_WRAPPER="$SCCACHE_BIN"
    export SCCACHE_DIR="${SCCACHE_DIR:-/mnt/build/sccache-maos}"
    add_to_shell_config "export RUSTC_WRAPPER=\"$SCCACHE_BIN\"" "$HOME/.zshrc"
    add_to_shell_config "export SCCACHE_DIR=\"$SCCACHE_DIR\"" "$HOME/.zshrc"
    add_to_shell_config "export SCCACHE_CACHE_SIZE=\"${SCCACHE_CACHE_SIZE:-12G}\"" "$HOME/.zshrc"
    "$SCCACHE_BIN" --show-stats 2>/dev/null | head -6 || true
fi
