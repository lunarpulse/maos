#!/bin/bash

# Get project name from directory containing .devcontainer
PROJECT_DIR=$(basename $(dirname $(dirname $(dirname  $(realpath $0)))))
ENV_FILE=".devcontainer/.env"
ENV_EXAMPLE=".devcontainer/.env_example"

update_project_name() {
    local file=$1
    # PROJECT_NAME 은 최초 1회만 찍는다. 디렉터리 이름이 겹치는 리포가 있어서
    # (Rust/docs-mcp-server vs Work/Sides/docs-mcp-server) 덮어쓰면 named volume 이 공유된다.
    if grep -q "^PROJECT_NAME=" "$file"; then
        :   # 이미 있으면 그대로 둔다
    else
        echo "PROJECT_NAME=$PROJECT_DIR" >> "$file"
    fi
}

create_env_file() {
    if [ -f "$ENV_EXAMPLE" ]; then
        cp "$ENV_EXAMPLE" "$ENV_FILE"
    else
        touch "$ENV_FILE"
        echo "PROJECT_NAME=$PROJECT_DIR" > "$ENV_FILE"
    fi
}

# Main logic
if [ -f "$ENV_FILE" ]; then
    update_project_name "$ENV_FILE"
else
    create_env_file
    [ -f "$ENV_EXAMPLE" ] && update_project_name "$ENV_FILE"
fi
