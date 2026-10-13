#!/usr/bin/env bash
# devcontainer 호스트 준비 — 컨테이너 생성 **전에 호스트에서** 실행된다
# (devcontainer.json 의 initializeCommand. VS Code 와 Zed 모두 실행한다 — 실측 확인).
#
# 하는 일은 셋뿐이다:
#   1. 빌드 루트 결정 (/mnt/build, 없으면 ~/.cache/devbuild) + 필요한 하위 디렉터리 생성
#   2. sccache 확보 → 없으면 RUSTC_WRAPPER 를 **빈 값**으로 (cargo 는 빈 값을 unset 취급;
#      존재하지 않는 경로를 넣으면 모든 빌드가 os error 2 로 죽는다)
#   3. 위 결과를 .devcontainer/.env 의 관리 블록에 기록 (compose 치환은 프로젝트 .env 만 읽는다)
#
# 이 스크립트를 돌리지 않아도 컨테이너는 뜬다 — compose 기본값이 그대로 동작한다.
# 돌리면 sccache 가 켜지고 빌드 디렉터리가 미리 준비될 뿐이다.
#
# 마운트는 두 개로 유지한다: /mnt/build 와 ~/.omp.
#   ~/.omp 는 **설정 + 에이전트 인증** 공유용이다(컨테이너마다 로그인하지 않기 위해).
#   세션은 compose 의 PI_CODING_AGENT_SESSION_DIR 로 컨테이너 안에 남는다.
#   ~/.config/* 나 ~/.local/state/* 같은 **중첩 경로는 바인드하지 않는다** — 도커가 중간
#   디렉터리를 root 소유로 만들어 버려서, Zed 의 remote server 가 ~/.local/share 를
#   못 만들고 "Failed to init server paths" 로 죽는 사고가 있었다.
#
# 환경변수: DEV_BUILD_ROOT_OVERRIDE (빌드 루트 강제), SCCACHE_VERSION
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="$HERE/.env"
BEGIN_MARK='# >>> host-prepare.sh (생성됨 — 블록 안은 수동 편집 금지) >>>'
END_MARK='# <<< host-prepare.sh <<<'
SCCACHE_VERSION="${SCCACHE_VERSION:-v0.18.0}"

say(){ printf '  %s\n' "$*"; }
echo "host-prepare: $HERE"

# ── 0) .env 보장 ────────────────────────────────────────────────────────────
if [ ! -f "$ENV_FILE" ]; then
  if [ -f "$HERE/.env_example" ]; then cp "$HERE/.env_example" "$ENV_FILE"; say ".env 생성 (.env_example 복사)"
  else : > "$ENV_FILE"; say ".env 생성 (빈 파일)"; fi
fi

# ── 1) 빌드 루트 ────────────────────────────────────────────────────────────
if [ -n "${DEV_BUILD_ROOT_OVERRIDE:-}" ]; then BUILD_ROOT="$DEV_BUILD_ROOT_OVERRIDE"
elif [ -d /mnt/build ] && [ -w /mnt/build ];  then BUILD_ROOT=/mnt/build
else BUILD_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/devbuild"; fi
mkdir -p "$BUILD_ROOT"
[ -w "$BUILD_ROOT" ] || { echo "FAIL: 빌드 루트에 쓸 수 없다: $BUILD_ROOT" >&2; exit 1; }
say "빌드 루트: $BUILD_ROOT  (컨테이너 안에서는 항상 /mnt/build)"

# compose/devcontainer.json 이 참조하는 /mnt/build 하위 경로를 실제로 만든다.
while IFS= read -r p; do
  [ -n "$p" ] || continue
  [ "$p" = "/mnt/build/bin/sccache" ] && continue   # 디렉터리가 아니라 파일
  mkdir -p "$BUILD_ROOT/${p#/mnt/build/}"
done < <(cat "$HERE"/docker-compose.yml "$HERE"/devcontainer*.json 2>/dev/null \
         | grep -oE '/mnt/build/[A-Za-z0-9._/-]+' | sort -u)
mkdir -p "$BUILD_ROOT/sccache" "$BUILD_ROOT/bin"

# ── 2) sccache (선택) ───────────────────────────────────────────────────────
SCCACHE_HOST="$BUILD_ROOT/bin/sccache"
if [ ! -x "$SCCACHE_HOST" ]; then
  A="sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl"; T="$(mktemp -d)"
  if (cd "$T" \
        && curl -fsSLO "https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/${A}.tar.gz" \
        && curl -fsSLO "https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/${A}.tar.gz.sha256" \
        && echo "$(awk '{print $1}' "${A}.tar.gz.sha256")  ${A}.tar.gz" | sha256sum -c - >/dev/null \
        && tar xzf "${A}.tar.gz" && install -m 755 "${A}/sccache" "$SCCACHE_HOST") 2>/dev/null
  then say "sccache 설치: $SCCACHE_HOST"; else say "경고: sccache 설치 실패 — 캐시 없이 빌드"; fi
  rm -rf "$T"
fi
if [ -x "$SCCACHE_HOST" ]; then RUSTC_WRAPPER_VALUE=/mnt/build/bin/sccache; say "sccache: 사용"
else RUSTC_WRAPPER_VALUE=; say "sccache: 없음 → RUSTC_WRAPPER 빈 값"; fi

# ── 3) omp 설정·인증 디렉터리 ───────────────────────────────────────────────
mkdir -p "$HOME/.omp"
say "omp 설정·인증 공유: $HOME/.omp  (세션은 컨테이너 안)"

# ── 3-1) git 신원 ───────────────────────────────────────────────────────────
# 홈이 named volume 이라 호스트 ~/.gitconfig 는 컨테이너에서 보이지 않는다.
# 신원 없이 `git commit` 하면 "Author identity unknown" 으로 죽으므로 값만 넘긴다.
# 파일을 바인드하지 않는다: :ro 면 컨테이너에서 `git config --global` 이 막히고,
# 호스트에 파일이 없으면 도커가 같은 이름의 디렉터리를 만들어 git 이 아예 깨진다.
# 실제 시드는 컨테이너 엔트리포인트(container-entrypoint.sh)가 한다.
GIT_NAME="$(git config --get user.name  2>/dev/null || true)"
GIT_EMAIL="$(git config --get user.email 2>/dev/null || true)"
if [ -n "$GIT_NAME" ] && [ -n "$GIT_EMAIL" ]; then
  say "git 신원: $GIT_NAME <$GIT_EMAIL>"
else
  say "경고: 호스트에 git 신원이 없다 → 컨테이너에서 커밋 전에 git config --global 필요"
fi

# ── 3-2) SSH (fetch/pull/push) ──────────────────────────────────────────────
# 개인키를 컨테이너에 복사하지 않는다. 호스트 **ssh-agent 소켓**을 바인드해서
# 서명만 호스트에서 시킨다(암호 걸린 키도 이 방식으로만 컨테이너에서 쓸 수 있다).
# ~/.ssh 는 config·known_hosts 때문에 읽기 전용으로 같이 붙인다($HOME 바로 아래 = Zed 안전).
SSH_SOCK="${SSH_AUTH_SOCK:-${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/ssh-agent.socket}"
if [ -S "$SSH_SOCK" ]; then
  KEYS="$(SSH_AUTH_SOCK="$SSH_SOCK" ssh-add -l 2>/dev/null | grep -c . || true)"
  if [ "${KEYS:-0}" -gt 0 ]; then say "ssh-agent: $SSH_SOCK (키 ${KEYS}개)"
  else say "ssh-agent: $SSH_SOCK (키 0개 — 호스트에서 'ssh-add ~/.ssh/id_rsa' 필요, push 불가)"; fi
else
  say "경고: ssh-agent 소켓 없음 → 컨테이너에서 push 불가"
  say "      systemctl --user enable --now ssh-agent.socket && ssh-add ~/.ssh/id_rsa"
  SSH_SOCK=/dev/null          # 바인드는 유지(없는 경로를 소스로 주면 컨테이너가 안 뜬다)
fi
[ -d "$HOME/.ssh" ] || mkdir -m 700 -p "$HOME/.ssh"

# ── 4) .env 관리 블록 ───────────────────────────────────────────────────────
TMP="$(mktemp)"
sed "/^${BEGIN_MARK//\//\\/}\$/,/^${END_MARK//\//\\/}\$/d" "$ENV_FILE" > "$TMP"
printf '%s\n' "$(cat "$TMP")" > "$TMP"
{
  echo ""
  echo "$BEGIN_MARK"
  echo "# 생성: $(date '+%F %T')  호스트: $(hostname)"
  echo "USER=${USER:-$(id -un)}"
  echo "UID=$(id -u)"
  echo "GID=$(id -g)"
  echo "DEV_BUILD_ROOT=$BUILD_ROOT"
  echo "RUSTC_WRAPPER=$RUSTC_WRAPPER_VALUE"
  echo "HOST_OMP_HOME=$HOME/.omp"
  echo "GIT_USER_NAME=$GIT_NAME"
  echo "GIT_USER_EMAIL=$GIT_EMAIL"
  echo "HOST_SSH_AUTH_SOCK=$SSH_SOCK"
  echo "HOST_SSH_DIR=$HOME/.ssh"
  echo "$END_MARK"
} >> "$TMP"
mv "$TMP" "$ENV_FILE"; chmod 600 "$ENV_FILE"
say ".env 관리 블록 갱신"
echo "host-prepare: 완료"
