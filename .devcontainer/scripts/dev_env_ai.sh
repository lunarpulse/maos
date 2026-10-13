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

echo "--- Installing opencode via Volta ---"
voltvolta install opencode-ai

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
# echo "🤖 Installing SuperClaude..."
# uv tool install SuperClaude
# uv tool update-shell

# Update PATH for current session
export PATH="$HOME/.local/bin:$PATH"

# Clone and setup Serena
# echo "🏠 Setting up Serena locally..."
# cd ~
# if [ ! -d "serena" ]; then
#     git clone https://github.com/oraios/serena
# fi

# === CLAUDE MCP CONFIGURATION ===
echo ""
echo "=== CLAUDE MCP CONFIGURATION ==="

# Function to add or update MCP server
configure_mcp_server() {
    local server_name="$1"
    shift
    local add_args="$@"

    # Check if server already exists
    if claude mcp list 2>/dev/null | grep -q "^${server_name}$" || \
       claude mcp list 2>/dev/null | grep -q "\"${server_name}\""; then
        echo "⚙️ MCP server '${server_name}' already exists, removing it first..."
        claude mcp remove "${server_name}" 2>/dev/null || true
    fi

    echo "⚙️ Configuring MCP server '${server_name}'..."
    if eval "claude mcp ${add_args}"; then
        echo "✅ ${server_name} MCP server configured successfully"
        return 0
    else
        echo "⚠️ Failed to configure ${server_name} MCP server, you may need to configure it manually"
        return 1
    fi
}

# Configure context7 MCP server
# configure_mcp_server "context7" "add-json context7 '{\"command\": \"npx\",\"args\": [\"-y\", \"@upstash/context7-mcp@latest\"]}'"

# Configure Serena MCP server
# configure_mcp_server "serena" "add serena -t sse \"http://0.0.0.0:9121/sse\""

# === COMPLETION ===
echo ""
echo "✨ Complete installation finished!"
echo ""
# echo "📝 Next steps:"
# echo "1. Restart your terminal to update PATH"
# echo "2. Start Serena MCP server with: uv run --directory $HOME/serena serena start-mcp-server --context ide-assistant --project /home/dev_ws --transport sse --port 9121"
# echo "3. Restart Claude Desktop to load the new MCP configuration"
# echo ""
# echo "🎯 Services will be available at:"
# echo "   - Claude CLI: Available via 'claude' command"
# echo "   - Serena MCP: http://0.0.0.0:9121/sse"
# echo ""
echo "--- Setup complete with Volta and full development environment ---"
echo "Please restart your shell or run 'source ~/.zshrc' to activate all PATH updates."
echo ""