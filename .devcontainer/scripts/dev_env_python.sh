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

log "🚀 Starting python development environment setup..."

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

log "✅ Python Development environment setup complete!"
