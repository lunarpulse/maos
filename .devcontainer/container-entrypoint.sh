#!/usr/bin/env bash
# devcontainer 컨테이너 엔트리포인트 — Zed / VS Code / 순수 `docker compose up` 세 경로에서
# **모두** 실행되는 유일한 훅이다. postCreateCommand 는 IDE 가 컨테이너를 만들 때만 돌고,
# `docker exec` 로 들어오는 경로에는 아무 훅도 없다.
#
# 하는 일: git 신원 시드.
#   홈은 named volume 이라 호스트 ~/.gitconfig 가 보이지 않는다. 신원이 없으면
#   `git commit` 이 "Author identity unknown / unable to auto-detect email address" 로 죽는다(실측).
#   호스트 값은 host-prepare.sh 가 .devcontainer/.env 관리 블록에 GIT_USER_NAME / GIT_USER_EMAIL
#   로 넣고, compose 의 env_file 이 그대로 컨테이너 환경에 올린다.
#
# 바인드가 아니라 **시드**인 이유:
#   · ~/.gitconfig 를 :ro 로 바인드하면 컨테이너 안에서 `git config --global …` 이 전부 실패한다.
#   · 호스트에 ~/.gitconfig 가 없는 머신에서는 도커가 같은 이름의 *디렉터리*를 만들어
#     git 이 "unable to read config file" 로 죽는다(이식성 사고).
#   · 시드는 쓰기 가능한 진짜 파일이라 컨테이너 쪽에서 덮어쓰고 추가 설정도 할 수 있다.
#
# GIT_AUTHOR_NAME/GIT_AUTHOR_EMAIL 같은 env 를 직접 주지 않는 이유:
#   값이 비면(`${VAR:-}`) git 이 "empty ident name (for <>) not allowed" 로 죽는다(실측).
#   env 는 설정 파일보다 우선하므로 사용자가 컨테이너에서 바꿀 수도 없다.
#
# 절대 컨테이너 기동을 막지 않는다: 모든 실패는 경고만 남기고 넘어간다.
set -u

seed_git_identity() {
  command -v git >/dev/null 2>&1 || return 0
  [ -n "${GIT_USER_NAME:-}" ] && [ -n "${GIT_USER_EMAIL:-}" ] || return 0
  [ -w "$HOME" ] || { echo "entrypoint: \$HOME 쓰기 불가 → git 신원 시드 생략" >&2; return 0; }

  git config --global --get user.name  >/dev/null 2>&1 || git config --global user.name  "$GIT_USER_NAME"
  git config --global --get user.email >/dev/null 2>&1 || git config --global user.email "$GIT_USER_EMAIL"
  git config --global --get init.defaultBranch >/dev/null 2>&1 \
    || git config --global init.defaultBranch "${GIT_DEFAULT_BRANCH:-main}"
}

seed_git_identity || echo "entrypoint: git 신원 시드 실패(무시하고 계속)" >&2

exec "$@"
