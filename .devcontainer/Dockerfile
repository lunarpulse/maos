# Use an ARG to define the Ubuntu variant
ARG VARIANT="latest"
FROM debian:$VARIANT

# Build arguments
ARG USER="vscode"
ARG UID=1000
ARG GID=1000

ARG RUST_VERSION
ARG PYTHON_VERSION
ARG VENV_PYTHON_VERSION
# Set environment variables
ENV USER=$USER
ENV UID=$UID
ENV GID=$GID

ENV RUST_VERSION=$RUST_VERSION
ENV PYTHON_VERSION=$PYTHON_VERSION
ENV VENV_PYTHON_VERSION=$VENV_PYTHON_VERSION

ENV HOME=/home/$USER

USER root

# Set CARGO_HOME and RUSTUP_HOME environment variables
ENV CARGO_HOME=/home/$USER/.cargo
ENV RUSTUP_HOME=/home/$USER/.rustup

# Install basic dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config libssl-dev \
    git jq ripgrep\
    # software-properties-common \
    wget curl\
    unzip tar \
    zsh sudo \
    mold postgresql-client uuid-runtime \
    procps psmisc util-linux
    
# Create the user with the specified UID and GID
RUN groupadd --gid $GID $USER \
    && useradd --uid $UID --gid $GID -m $USER \
    && echo "$USER ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/$USER

# Create directories and set permissions
RUN mkdir -p $CARGO_HOME $RUSTUP_HOME \
    && chown -R $USER:$USER $CARGO_HOME $RUSTUP_HOME

# Install Python for the user
# RUN add-apt-repository "deb http://deb.debian.org/debian $VARIANT-backports main"

# Install Python $PYTHON_VERSION dependencies
RUN apt-get update && apt-get install -y python${PYTHON_VERSION} python${PYTHON_VERSION}-dev python${PYTHON_VERSION}-venv python3-pip

# Clean up apt package manager caches
RUN apt-get autoremove -y && apt-get clean -y && \
    rm -rf /var/lib/apt/lists/* && \
    touch /var/lib/dpkg/statoverride

# ── AI 코딩 에이전트 CLI (omp, herdr) ───────────────────────────────────────
# 설치 위치 = /opt/agent-clis/bin, **디렉터리째 개발 사용자 소유**, 설치도 개발 사용자로 한다.
#   · 홈($HOME/.local/bin)이 아닌 이유: $HOME 은 런타임에 named volume 으로 가려진다 →
#     볼륨이 이미 있으면 이미지의 새 바이너리가 보이지 않고, 리포마다 ~240MB 를 다시 받는다.
#   · /usr/local/bin 이 아닌 이유: `omp update` / `herdr update` 는 바이너리 옆에 임시 파일을
#     만들고 rename 한다. 파일만 chown 해도 디렉터리가 root 소유면 EACCES 로 실패한다(실측:
#     `open '/usr/local/bin/omp.….new'`, herdr `install directory not writable`) → sudo 가 필요했다.
#   · postCreateCommand 에 두면 컨테이너를 만들 때마다 네트워크를 타고, 오프라인이면 실패한다.
# 컨테이너 안 업데이트는 컨테이너를 재생성하면 이미지 버전으로 돌아간다(`docker compose build`).
# 인증·설정은 호스트 ~/.omp 바인드로 공유되고, 세션만 컨테이너(PI_CODING_AGENT_SESSION_DIR)에 남는다.
ARG INSTALL_AGENT_CLIS=true
ENV AGENT_CLI_DIR=/opt/agent-clis/bin
ENV PATH=$AGENT_CLI_DIR:$PATH
# ENV PATH 는 docker exec·비로그인 셸용. 로그인 셸(bash -l, sh -l)은 /etc/profile 이 PATH 를
# 다시 쓰므로 profile.d 로 한 번 더 넣는다.
RUN install -d -o $UID -g $GID /opt/agent-clis $AGENT_CLI_DIR \
    && printf 'case ":$PATH:" in *":%s:"*) ;; *) PATH="%s:$PATH"; export PATH ;; esac\n' \
         "$AGENT_CLI_DIR" "$AGENT_CLI_DIR" > /etc/profile.d/agent-clis.sh

USER $USER

# 스모크 테스트(--version)가 홈에 남기는 natives(~346MB)·캐시는 이미지에서 걷어낸다
# (런타임 ~/.omp 는 호스트 바인드라 이미지 홈의 사본은 쓸모가 없다).
RUN if [ "$INSTALL_AGENT_CLIS" = "true" ]; then \
        curl -fsSL https://omp.sh/install | PI_INSTALL_DIR=$AGENT_CLI_DIR sh \
        && curl -fsSL https://herdr.dev/install.sh | HERDR_INSTALL_DIR=$AGENT_CLI_DIR sh \
        && omp --version && herdr --version \
        && rm -rf "$HOME/.omp" "$HOME/.cache" "$HOME/.config"; \
    fi

# Graphify CLI (PyPI: graphifyy) is image-owned like omp/herdr: outside the home
# volume, installed as the dev user, and isolated in its own uv tool environment.
# UV_TOOL_* stay set at runtime so `uv tool run --from graphifyy` finds this install.
# The `sql` extra (tree-sitter-sql) lets it index .sql files; kept uniform across devcontainers.
ARG UV_VERSION=0.12.22
ARG GRAPHIFY_VERSION=0.9.79
ENV UV_TOOL_DIR=/opt/agent-clis/uv-tools \
    UV_TOOL_BIN_DIR=/opt/agent-clis/bin
RUN mkdir -p "$UV_TOOL_DIR" \
    && curl -fsSL \
         "https://astral.sh/uv/${UV_VERSION}/install.sh" \
         -o /tmp/graphify-uv-install.sh \
    && UV_UNMANAGED_INSTALL="$AGENT_CLI_DIR" \
         sh /tmp/graphify-uv-install.sh \
    && "$AGENT_CLI_DIR/uv" tool install \
         --python "/usr/bin/python${PYTHON_VERSION}" \
         "graphifyy[sql]==${GRAPHIFY_VERSION}" \
    && "$AGENT_CLI_DIR/graphify" --version \
    && "$AGENT_CLI_DIR/uv" cache clean \
    && rm /tmp/graphify-uv-install.sh

ARG ZSH_EXPORTS="export HISTFILE=/commandhistory/.zsh_history\n\
export SAVEHIST=10000\n\
export HISTSIZE=10000\n\
setopt SHARE_HISTORY\n\
setopt APPEND_HISTORY\n\
setopt INC_APPEND_HISTORY"

ARG BASH_EXPORTS="export HISTFILE=/commandhistory/.bash_history\n\
export HISTFILESIZE=10000\n\
export HISTSIZE=10000\n\
shopt -s histappend"

RUN sudo mkdir -p /commandhistory \
    && sudo touch /commandhistory/.zsh_history /commandhistory/.bash_history \
    && sudo chown -R ${USER}:${USER} /commandhistory \
    && echo "$ZSH_EXPORTS" >> "/home/$USER/.zshrc" \
    && echo "$BASH_EXPORTS" >> "/home/$USER/.bashrc"

# Install Zsh and Starship for the user
RUN sh -c "$(curl -fsSL https://starship.rs/install.sh)" -- -y \
    && echo 'eval "$(starship init zsh)"' >> $HOME/.zshrc

# ── 컨테이너 엔트리포인트 ───────────────────────────────────────────────────
# git 신원 시드(홈이 named volume 이라 호스트 ~/.gitconfig 가 안 보인다 → "Author identity
# unknown" 으로 커밋 실패). 엔트리포인트는 Zed / VS Code / 순수 docker compose 세 경로에서
# 모두 실행되는 유일한 훅이다 — postCreateCommand 는 IDE 가 만들 때만 돈다.
# COPY --chmod: 파일 소유자는 root 이므로 USER 전환 뒤에는 chmod 를 걸 수 없다.
COPY --chmod=0755 container-entrypoint.sh /usr/local/bin/devcontainer-entrypoint.sh
ENTRYPOINT ["/usr/local/bin/devcontainer-entrypoint.sh"]
CMD ["sleep", "infinity"]
