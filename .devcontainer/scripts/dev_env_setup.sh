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

log "🚀 Starting complete development environment setup..."

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

OWNER="${USERNAME:-${USER:-$(id -un)}}"
VENV_DIR="${HOME}/.venv"
REQ_VER="${VENV_PYTHON_VERSION:-}"  # e.g., 3.12

# Ensure ~/.local/bin on PATH (uv installs here)
export PATH="$HOME/.local/bin:$PATH"
add_to_shell_config 'export PATH="$HOME/.local/bin:$PATH"' "${HOME}/.bashrc"
add_to_shell_config 'export PATH="$HOME/.local/bin:$PATH"' "${HOME}/.zshrc"

# --- install uv if missing ---
if ! command -v uv >/dev/null 2>&1; then
  echo "Installing uv..."
  curl -LsSf https://astral.sh/uv/install.sh | sh
  export PATH="$HOME/.local/bin:$PATH"
fi
uv --version

# --- create venv with uv (using VENV_PYTHON_VERSION if set) ---
echo "Creating virtual environment at ${VENV_DIR} using uv..."
deactivate 2>/dev/null || true
rm -rf "${VENV_DIR}"

if [ -n "$REQ_VER" ]; then
  echo "Using Python ${REQ_VER} for the venv..."
  uv venv --python "$REQ_VER" "$VENV_DIR"
else
  echo "VENV_PYTHON_VERSION not set; creating venv with uv default..."
  uv venv "$VENV_DIR"
fi

# Ownership and permissions
if command -v sudo >/dev/null 2>&1; then
  sudo chown -R "${OWNER}:${OWNER}" "${VENV_DIR}" || true
fi
chmod -R u+rwX "${VENV_DIR}"

# Activate on shell startup
ACTIVATE_CMD="source ${VENV_DIR}/bin/activate"
add_to_shell_config "$ACTIVATE_CMD" "${HOME}/.bashrc"
add_to_shell_config "$ACTIVATE_CMD" "${HOME}/.zshrc"

# --- ensure pip exists in the venv ---
# shellcheck source=/dev/null
source "${VENV_DIR}/bin/activate"
PYBIN="${VENV_DIR}/bin/python"

if ! "${PYBIN}" -m pip --version >/dev/null 2>&1; then
  echo "Bootstrapping pip in the venv..."
  if ! "${PYBIN}" -m ensurepip --upgrade >/dev/null 2>&1; then
    echo "ensurepip not available; falling back to get-pip.py"
    curl -fsSL https://bootstrap.pypa.io/get-pip.py -o /tmp/get-pip.py
    "${PYBIN}" /tmp/get-pip.py
    rm -f /tmp/get-pip.py
  fi
fi

# Upgrade core tooling
python -m pip install --no-cache-dir --quiet --upgrade pip setuptools wheel
python -V


# Check required commands
log "🔍 Checking system prerequisites..."
for cmd in curl git bash; do
    if ! command_exists "$cmd"; then
        log "❌ Error: $cmd is not installed. Please install it first."
        exit 1
    fi
done
log "✅ All prerequisites found"

# === CLAUDE DESKTOP INSTALLATION ===
echo ""
echo "=== CLAUDE DESKTOP SETUP ==="

echo "--- Installing Volta (Node.js version manager) ---"
if ! command_exists volta; then
    curl https://get.volta.sh | bash
    export VOLTA_HOME="$HOME/.volta"
    export PATH="$VOLTA_HOME/bin:$PATH"
    # Source volta environment if available
    if [ -s "$VOLTA_HOME/load.sh" ]; then
        source "$VOLTA_HOME/load.sh"
    fi
    log "✅ Volta installed successfully"
else
    log "✅ Volta already installed"
    export VOLTA_HOME="$HOME/.volta"
    export PATH="$VOLTA_HOME/bin:$PATH"
fi

# Verify volta is working
if ! command_exists volta; then
    log "❌ Error: Volta installation failed or not in PATH"
    exit 1
fi

echo "--- Installing Node.js and npm via Volta ---"
volta install node@lts
volta install npm@latest

echo "--- Installing Claude Code Library via Volta ---" 
# Note: This is the library, not the primary CLI tool for direct interaction.
volta install @anthropic-ai/claude-code

echo "--- Installing ccusage (Claude Code token usage analysis tool) via Volta ---"
volta install ccusage

echo "--- Testing claude CLI installation ---"
# Test if claude is available in PATH after Volta installation
if command_exists claude; then
    log "✅ Claude CLI is available in PATH"
    claude --version || true
else
    log "⚠️ Claude CLI not found in PATH, trying alternative methods"
    # Try npx method
    npx claude --version || true
fi

# Add Volta to PATH in ~/.zshrc if not already present
if [[ ":$PATH:" != *":$HOME/.volta/bin:"* ]]; then
    echo "--- Adding Volta to PATH in ~/.zshrc ---"
    echo 'export VOLTA_HOME="$HOME/.volta"' >> ~/.zshrc
    echo 'export PATH="$VOLTA_HOME/bin:$PATH"' >> ~/.zshrc
    echo "Volta path added. Please source ~/.zshrc or restart your shell."
else
    echo "Volta already in PATH. No modification needed."
fi

# Claude CLI should be available through Volta, no additional PATH needed
log "✅ Claude CLI available through Volta installation"

# === DEVELOPMENT ENVIRONMENT SETUP ===
echo ""
echo "=== DEVELOPMENT ENVIRONMENT SETUP ==="

# Install uv package manager
echo "📦 Installing uv package manager..."
if ! command_exists uv; then
    curl -LsSf https://astral.sh/uv/install.sh | sh
    # Source cargo env if available
    [ -s "$HOME/.cargo/env" ] && source "$HOME/.cargo/env" || true
    # Add uv to PATH for current session
    export PATH="$HOME/.local/bin:$PATH"
    log "✅ uv package manager installed"
else
    log "✅ uv already installed"
fi

# Install SuperClaude with uv
echo "🤖 Installing SuperClaude..."
uv tool install SuperClaude
uv tool update-shell

# Update PATH for current session
export PATH="$HOME/.local/bin:$PATH"

# Clone and setup Serena
echo "🏠 Setting up Serena locally..."
cd ~
if [ ! -d "serena" ]; then
    git clone https://github.com/oraios/serena
fi

# Update Rust and install components
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain $RUST_VERSION \
    && sudo echo 'export PATH="$PATH:$HOME/.cargo/bin"' >> $HOME/.zshrc \
    && rustup component add rust-src rust-analysis clippy
    
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

# === CLAUDE MCP CONFIGURATION ===
echo ""
echo "=== CLAUDE MCP CONFIGURATION ==="

# Configure MCP servers
echo "⚙️ Configuring MCP servers..."
claude mcp add-json context7 '{"command": "npx","args": ["-y", "@upstash/context7-mcp@latest"]}'

# Configure Serena MCP server
echo "⚙️ Configuring Serena MCP server..."
# Use claude mcp add command with SSE configuration
claude mcp add serena --type sse --url "http://0.0.0.0:9121/sse"
if [ $? -eq 0 ]; then
    echo "✅ Serena MCP server configured successfully"
else
    echo "⚠️ Failed to configure Serena MCP server, you may need to configure it manually"
fi

# === COMPLETION ===
echo ""
echo "✨ Complete installation finished!"
echo ""
echo "📝 Next steps:"
echo "1. Restart your terminal to update PATH"
echo "2. Start Serena MCP server with: uv run --directory $HOME/serena serena start-mcp-server --context ide-assistant --project /home/dev_ws --transport sse --port 9121"
echo "3. Restart Claude Desktop to load the new MCP configuration"
echo ""
echo "🎯 Services will be available at:"
echo "   - Claude CLI: Available via 'claude' command"
echo "   - Serena MCP: http://0.0.0.0:9121/sse"
echo ""
echo "--- Setup complete with Volta and full development environment ---"
echo "Please restart your shell or run 'source ~/.zshrc' to activate all PATH updates."
echo ""
echo "# Install SuperClaude"
echo "⚡ Running SuperClaude installation..."
echo "SuperClaude install --yes"