# Runbook — J1 T8: 유료 2-호스트 서명 런 (`j1-crosshost-2d` AC8)

> **이 문서의 성격.** 기존 `runbook-j1-tier-2-signed-live-run.md`는 **판단 기록**입니다 — 무엇이
> 왜 틀렸는지, 어떤 주장이 금지되는지를 담습니다. 그건 계속 규범(normative)입니다.
> 이 문서는 **실행 순서**입니다: 위에서 아래로 따라가면 됩니다.
>
> **아래 모든 명령은 2026-08-22에 실제로 실행되었습니다.** 가짜 `claude` 픽스처로 끝까지 돌려
> 크로싱·워커 spawn·공유 `frame_id`·양쪽 서명 검증·`reconcile-hosts`까지 초록불을 확인했습니다.
> 검증되지 않은 유일한 부분은 **실제 과금 spawn**이며, 그건 돈이 필요해서지 미지여서가 아닙니다.

## 규범 문서 (이 런북은 이것을 대체하지 않습니다)

| 문서 | 역할 |
|---|---|
| `j1-two-host-evidence/README.md` | **입회 계약** — capture 필드, 과잉주장 tripwire, 게이트가 검사하지 **않는** 것 |
| `j1-two-host-evidence/PUBLISHED-FINGERPRINTS.md` | **런 전 커밋된 키 약정** |
| `runbook-j1-tier-2-signed-live-run.md` | 진단·판단 기록, Phase 0.0 호출 계약, 중단 조건 |
| `RELEASE-HOLDS.md` rows 13–15 | 이 런이 **주장할 수 없는** 것 |

---

## 0. 무엇이 남았는지

`j1-crosshost-2e`(2026-08-22)가 코드 블로커 6개(F1–F5, F7)를 모두 닫았습니다.
**남은 것은 코드가 아닙니다:** 두 호스트, 깨끗한 sandbox home, 과금형 API 키, 그리고 지출 결정.

과금 지점은 **1곳**(host B의 claude spawn)이고, **중단 조건은 그 뒤에 있습니다**(Phase 7.4의
`verify.py`). 그래서 **1장은 전부 무료**이며 건너뛰면 돈을 쓴 뒤에 거부됩니다.

---

## 1장 — 무료 리허설 (전부 통과해야 2장 진입)

### 1.1 빌드

```bash
cd /path/to/maos                      # 반드시 리포 루트
cargo build --release -p maos-bin -p maos-cli
```

### 1.2 릴리스 빌드가 진짜인지 판별

`MAOS_TEST_BOOT_NONCE`는 **런타임** `cfg!(debug_assertions)`로 걸려 있어
`RUSTFLAGS="-C debug-assertions=yes" cargo build --release`가 조용히 되살립니다.
`check-mock-not-in-release`는 심볼 테이블만 grep하므로 이걸 못 잡습니다.

```bash
for i in 1 2; do
  H=$(mktemp -d)
  HOME=$H MAOS_HOME=$H XDG_DATA_HOME=$H MAOS_TEST_BOOT_NONCE=424242 \
    ./target/release/maos run spirits/topologies/j1-founder-loop.toml --once >/dev/null 2>&1
  echo -n "run $i (rc=$?) distinct boot_nonce: "
  HOME=$H MAOS_HOME=$H XDG_DATA_HOME=$H \
    ./target/release/maosctl audit query --range 1d --format ndjson \
    | python3 -c 'import json,sys
seen={r["boot_nonce"] for l in sys.stdin
      for r in [json.loads(l)] if r.get("boot_nonce") is not None}
print(sorted(seen))'
done
```

- **서로 다른 값 2개, `424242` 아님 ⇒ 진짜 릴리스 빌드.**
- **`424242`가 되읽히면 ⇒ debug assertions ON. 중단, 지출 금지.**

⚠ **런마다 새 state home**을 주세요. 같은 home으로 두 번 돌리면
`orchestrator dispatch references raw worker output not a distillate`가 나는데, 이는 FR21의 60초
윈도우에 걸린 **기존 결함**이고 falsifier 실패로 오독됩니다.

### 1.3 `verify.py` 동작 확인 — Phase 7.4는 필수 중단 조건

```bash
python3 tools/verify-audit-bundle/verify.py \
  _bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-bundle.json \
  61f4f495dba703e74aff7d42b4286a1a914a89b592a98bf76ed3656c81107766
# 기대: OK — signature verified   (exit 0)
```

`FAIL`이면 2e 수정이 없는 트리입니다. **이 결함이 T6를 서명된 날부터 검증 불가로 만들었고**,
유료 런은 정확히 여기서 — 에이전트 과금 후에 — 죽었습니다.

### 1.4 ⛔ 앰비언트 자격증명 제거

```bash
ls -la ~/.claude/.credentials.json     # 존재하면 라이브 런이 거부됨
```

거부 메시지(실측):

```
Error: "maos run: ambient auth file …/.claude/.credentials.json present in the
sandbox home — refusing the live run. It lets the worker use a credential MAOS
never holds, so redaction is unattestable (a failed Tier-2)."
```

**구독 토큰이 없는 sandbox home**을 쓰세요. ⚠ `MAOS_HOME`은 audit 서명키를 리다이렉트하지
**않습니다** (`crates/maos-domain/src/audit_key.rs:88-118`) — `--audit-key` / `MAOS_AUDIT_KEY`만 유효.

### 1.5 과금형 키

```bash
export ANTHROPIC_API_KEY=<metered key>     # 구독 토큰 아님
```

지출 상한을 먼저 정해 기록하세요.

---

## 2장 — 준비물

### 2.1 키: **새로 만들지 마세요**

`PUBLISHED-FINGERPRINTS.md`는 **런 전에 커밋된 약정**입니다. 다른 키를 쓰면 약정이 무효가 되고,
올바른 대응은 파일 갱신이 아니라 **"이 런은 그 런이 아니다"라고 말하는 것**입니다.

```bash
export HOST_A_KEY=~/.maos/keys/j1-2d-host-a-audit.key   # FPR_A 4bbc1187…220be344
export HOST_B_KEY=~/.maos/keys/j1-2d-host-b-audit.key   # FPR_B 843dc5a8…08e7296f3
export OPERATOR_KEY=~/.config/maos/audit-signing.key    # receipt 서명자 433b27c1…33d48a3a
export FPR_A=4bbc1187ddf5908d9e96eecdbef6bb9fdfbc42a7977bc886c5d41046220be344
export FPR_B=843dc5a83dbbebcf3c5c5fbe79a45bdba405f61879f77eb932a741708e7296f3

test "$(cat $HOST_A_KEY)" != "$(cat $HOST_B_KEY)" \
  || { echo "ABORT: one root cannot attest two identities"; exit 1; }
```

⚠ `$OPERATOR_KEY`는 **T6를 서명한 키가 아닙니다**(T6는 `61f4f495…`, 이 키는 `433b27c1…`).
혼동하면 아무것도 검증하지 않는 receipt가 나오고, 실패가 "서명 깨짐"처럼 보입니다.

### 2.2 mTLS 리프 인증서 — ⚠ `openssl req -x509`만으로는 **안 됩니다**

`openssl req -x509`는 기본으로 `basicConstraints=CA:TRUE`를 넣고, rustls는 그것을
`BAD_CERTIFICATE: CaUsedAsEndEntity`로 **거부**합니다. 확장을 명시해야 합니다.

```bash
export LAB=$(mktemp -d)              # 두 호스트의 재료를 모아두는 작업 디렉터리
cat > $LAB/leaf.ext <<'EOF'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature
extendedKeyUsage=serverAuth,clientAuth
subjectAltName=IP:127.0.0.1
EOF

for h in host-a host-b; do
  mkdir -p $LAB/$h
  openssl req -new -newkey ed25519 -nodes \
    -keyout $LAB/$h/own.key.pem -out $LAB/$h/req.pem -subj "/CN=$h"
  openssl x509 -req -in $LAB/$h/req.pem -signkey $LAB/$h/own.key.pem \
    -out $LAB/$h/own.cert.pem -days 30 -extfile $LAB/leaf.ext
done
```

지문은 **DER의 sha256**입니다 (`PeerCertFingerprint::from_cert_der`):

```bash
fpr() { openssl x509 -in $LAB/$1/own.cert.pem -outform DER | sha256sum | cut -d' ' -f1; }
export FA=$(fpr host-a); export FB=$(fpr host-b)
echo "host-a=$FA"; echo "host-b=$FB"
test "$FA" != "$FB" || { echo "ABORT: identical leaves"; exit 1; }
```

⚠ 실제 두 대의 머신이라면 `subjectAltName`을 각 호스트의 실제 IP/DNS로 바꾸고, 아래 모든
`127.0.0.1`을 그에 맞추세요.

### 2.3 cohort manifest 작성 + 서명 (2e AC2 신규)

2e 이전에는 워크스페이스에 cohort manifest를 서명할 수단이 **아무것도 없었고**, 그래서 host B가
부팅하지 못했습니다 (`EInvalidSignature("expected 64 bytes (128 hex chars), got 0 bytes")`).

```bash
# 코호트 authority 시드 — 32 raw bytes. audit 키와 반드시 별개.
export COHORT_AUTH=$LAB/cohort-authority.key
head -c 32 /dev/urandom > $COHORT_AUTH && chmod 600 $COHORT_AUTH
export AUTH_PUB=$(python3 - <<'EOF'
import os
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization as s
k=Ed25519PrivateKey.from_private_bytes(open(os.environ["COHORT_AUTH"],"rb").read())
print(k.public_key().public_bytes(s.Encoding.Raw, s.PublicFormat.Raw).hex())
EOF
)

cat > $LAB/cohort-unsigned.toml <<EOF
schema_version = 4
cohort_id = "j1-crosshost-2d"
version = 1
t_stale_secs = 120
reserved_intents = ["cohort:manifest-reissue", "cohort:halt-receipt"]

[authority]
threshold = 1
keys = ["$AUTH_PUB"]

[[members]]
host_id = "host-a"
fingerprint = "sha256:$FA"
roles = ["worker"]
team = "team-a"

[[members]]
host_id = "host-b"
fingerprint = "sha256:$FB"
roles = ["worker"]
team = "team-b"

[consent]

[[teams]]
team_id = "team-a"
region = "region-a"
datname = "maos_team_a"
members = ["spirit-a"]

[[teams]]
team_id = "team-b"
region = "region-b"
datname = "maos_team_b"
members = ["spirit-b"]

[signature]
sig = ""
EOF

./target/release/maosctl cohort sign \
  --manifest $LAB/cohort-unsigned.toml \
  --authority-key $COHORT_AUTH \
  --output $LAB/cohort-signed.toml
```

기대 출력:

```
maosctl: cohort sign — signed …/cohort-unsigned.toml
  (cohort `j1-crosshost-2d` v1, 2 member(s)) under authority <64-hex> → …/cohort-signed.toml
```

⚠ `reserved_intents`는 **`cohort:manifest-reissue`**(`cohort:reissue`가 아님) + `cohort:halt-receipt`
입니다. 틀리면 서명자가 자기 출력을 재검증하다 잡아냅니다 — 이 도구는 서명 전에 validate하고
서명 후 재검증하며, `authority.keys`에 서명자가 없는 manifest를 **거부**합니다.

### 2.4 host grants — `[author].name`과 정확히 일치해야 함

```bash
cat > $LAB/host-grants.toml <<'EOF'
[[grant]]
attested_image = "claude"
signing_key_id = "Anthropic"
permitted_tier = "T3"
permitted_egress_destinations = ["api.anthropic.com"]
EOF
```

`signing_key_id`는 `spirits/worker/manifest-claude.toml`의 `[author].name = "Anthropic"`과
**정확히** 같아야 하며, 다르면 admission이 거부합니다.

### 2.5 데몬 설정 — ⚠ `local_host`와 `peer_id`는 **다른 이름공간**입니다

이게 가장 흔한 오류 지점입니다.

| 키 | 값의 출처 | host A | host B |
|---|---|---|---|
| `local_host` | **cohort manifest의 member `host_id`** | `host-a` | `host-b` |
| `[[peers]].peer_id` | **토폴로지의 역할 id** | `developer-remote-host` | `founder-loop-host` |
| `cert_fingerprint` / `peer_pins[].fingerprint` | **상대방**의 리프 지문 | `$FB` | `$FA` |
| `endpoint` | 상대방 주소, **`tls://` 스킴 필수** | `tls://<B>:19555` | `tls://127.0.0.1:1` (미사용 placeholder) |

토폴로지가 `host = "developer-remote-host"`를 선언하며, 그 이름이 목적지 `peer_id`입니다
(`spirits/topologies/j1-founder-loop-crosshost.toml`). J1 peer id는 cohort manifest에
**의도적으로 등장하지 않습니다** — ADR-012 양자간 경로이므로 cohort 게이트가 defer합니다.

```bash
export PORT_B=19555
export INTENT='development-task:write-workspace'

# ── host A (sender) ─────────────────────────────────────────────────────────
cat > $LAB/host-a.daemon.toml <<EOF
manifest_path = '$LAB/cohort-signed.toml'
authority_keys = ['$AUTH_PUB']
local_host = 'host-a'
control_spirit = 'orchestrator'

[[peers]]
peer_id = 'developer-remote-host'
endpoint = 'tls://127.0.0.1:$PORT_B'
cert_fingerprint = { algo = 'sha256', hex = '$FB' }
send_allowlist = ['$INTENT']
accept_allowlist = ['$INTENT']

[tcp]
listen_addr = '127.0.0.1:0'
own_cert_chain = '$LAB/host-a/own.cert.pem'
own_private_key = '$LAB/host-a/own.key.pem'
peer_pins = [{ peer_id = 'developer-remote-host', fingerprint = { algo = 'sha256', hex = '$FB' }, boot_nonce = 1 }]

[digest_summary]
frames = 0
halts = 0
conflicts = 0
EOF
```

⚠ **host A의 `peer_pins[].boot_nonce = 1`은 자리표시자이고, 그래도 됩니다.**
nonce는 **수신자만** 검증합니다 — 와이어로 실려온 발신자의 nonce를 수신자의 저장된 핀과 비교
(`crates/maos-a2a-core/src/router.rs:1325-1361`). 단방향 위임에서 host A는 host B의 nonce를
검증할 기회가 없습니다. 다만 스키마가 세 키를 모두 요구하고 `0`은 명시적으로 거부하므로
**비어 있거나 0이면 안 됩니다**. 이 사실이 페어링의 순환 문제를 해소합니다.

host B 설정은 3.2에서 host A가 nonce를 발행한 **뒤에** 작성합니다.

---

## 3장 — 페어링과 크로싱 (2e AC5 신규, 이전엔 실행 불가)

이전 절차는 문서 결함이 아니라 **토폴로지 결함**이었습니다: host A는 *sender*인데
`cohort:daemon-started`는 *receiver*의 행이고, "host A를 데몬으로 돌려라"도 해법이 아닙니다 —
데몬 모드는 cross-host 라우터를 `None`으로 만들어 시험 대상 arm 자체를 없앱니다.

### 3.1 host A: 발행 후 대기

```bash
export READY=$LAB/host-b-ready
rm -f $READY                                  # 미리 존재해선 안 됨
export HOME_A=$LAB/home-a && mkdir -p $HOME_A

HOME=$HOME_A MAOS_HOME=$HOME_A XDG_DATA_HOME=$HOME_A \
MAOS_AUDIT_KEY=$HOST_A_KEY \
MAOS_COHORT_DAEMON_CONFIG=$LAB/host-a.daemon.toml \
MAOS_HOST_GRANTS=$LAB/host-grants.toml \
MAOS_LIVE_AGENT=1 \
MAOS_DELEGATED_GOAL="<원격 워커가 수행할 구체적 과업>" \
MAOS_CROSSHOST_PAIRING_READY_FILE=$READY \
MAOS_CROSSHOST_PAIRING_TIMEOUT_SECS=600 \
  ./target/release/maos run spirits/topologies/j1-founder-loop-crosshost.toml --once
```

⚠ `MAOS_ONE_SHOT`을 **설정하지 마세요** — host A는 데몬이 아닙니다.

host A가 출력하고 멈춥니다:

```
maos: cross-host sender ready — boot_nonce 7196704562630440850 (decimal). …
maos: pairing rendezvous — holding up to 600s for …/host-b-ready …
```

기계 경로(두 번째 셸에서):

```bash
HOME=$HOME_A MAOS_HOME=$HOME_A ./target/release/maosctl audit query \
  --frame-kind TelemetryEvent --intent-contains cohort:crosshost-started --format ndjson
```

⛔ **`--format plain`으로 nonce를 읽지 마세요.** 그 렌더러는 `boot_nonce`라는 헤더 아래 값을
`{:016x}` **16진수**로 찍고 TOML은 **10진수**로 파싱합니다. 대부분은 `a`–`f`가 있어 시끄럽게
실패하지만, **전부 숫자인 hex는 다른 수로 조용히 파싱**되어 첫 프레임에서야 nonce 불일치로
터집니다. 플래그는 `--intent-contains`이며 `--intent`는 존재하지 않습니다.

### 3.2 host B: 발행된 nonce를 핀하고 기동

```bash
export N_A=7196704562630440850        # ← host A가 출력한 값으로 교체
export HOME_B=$LAB/home-b && mkdir -p $HOME_B
export WORK_B=$LAB/work-b && mkdir -p $WORK_B     # 워커가 파일을 쓰는 cwd

cat > $LAB/host-b.daemon.toml <<EOF
manifest_path = '$LAB/cohort-signed.toml'
authority_keys = ['$AUTH_PUB']
local_host = 'host-b'
control_spirit = 'orchestrator'
worker_manifest = '$PWD/spirits/worker/manifest-claude.toml'

[[peers]]
peer_id = 'founder-loop-host'
endpoint = 'tls://127.0.0.1:1'
cert_fingerprint = { algo = 'sha256', hex = '$FA' }
send_allowlist = ['$INTENT']
accept_allowlist = ['$INTENT']

[tcp]
listen_addr = '127.0.0.1:$PORT_B'
own_cert_chain = '$LAB/host-b/own.cert.pem'
own_private_key = '$LAB/host-b/own.key.pem'
peer_pins = [{ peer_id = 'founder-loop-host', fingerprint = { algo = 'sha256', hex = '$FA' }, boot_nonce = $N_A }]

[digest_summary]
frames = 0
halts = 0
conflicts = 0
EOF

cd $WORK_B && \
HOME=$HOME_B MAOS_HOME=$HOME_B XDG_DATA_HOME=$HOME_B \
MAOS_AUDIT_KEY=$HOST_B_KEY \
MAOS_ONE_SHOT=cohort-a2a-daemon \
MAOS_COHORT_DAEMON_CONFIG=$LAB/host-b.daemon.toml \
MAOS_HOST_GRANTS=$LAB/host-grants.toml \
MAOS_LIVE_AGENT=1 \
ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  /path/to/maos/target/release/maos
```

⚠ **데몬은 `run`도 토폴로지도 받지 않습니다** — 순수 `maos`입니다. `run <topology>`를 주면
host A의 일(founder loop)을 실행하고 위임을 받지 못합니다.

⚠ **`MAOS_LIVE_AGENT=1`은 host B에도 필요합니다.** 실제로 워커를 spawn하는 쪽이 host B이므로,
없으면 `refusing to spawn real agent CLI 'claude' on the hermetic path`로 프레임을 거부합니다.

⚠ **`worker_manifest`가 없으면 intake sink가 설치되지 않고**, 검증된 프레임이 ACK된 뒤
조용히 버려집니다 (`crates/maos-bin/src/main.rs:9327-9329`).

기대 출력:

```
cohort manifest initial pull from founder-loop-host failed: … connect 127.0.0.1:1: Connection refused
cohort-a2a-daemon listening on 127.0.0.1:19555
```

⚠ 첫 줄은 **무해합니다.** host B는 A를 다이얼하지 않으므로 `endpoint`가 자리표시자이고,
초기 manifest pull만 실패합니다. 그 뒤 정상적으로 listen합니다.

### 3.3 대기 해제

```bash
touch $READY
```

host A가 `pairing rendezvous — host B signalled ready, dialling`을 찍고 진행합니다.
**파일을 만들지 않으면 host A는 다이얼하지 않고 종료**합니다 — `--once`의 connect 시도는
재시도 없이 단 한 번이므로(거부된 connect는 즉시 `Io` 반환; `[100,300,1000]ms` 스케줄은
cert-class 재시도 예산이며 기동 유예가 아님), 눈 감고 쓰지 않는 것이 유일한 안전한 실패입니다.

### 3.4 크로싱 확인 (실측 출력)

host A: `maos run: topology --once complete — exiting cleanly`, **exit 0**.

host B:

```
{"event":"host_grant_disposition","attested_image":"claude","signing_key_id":"Anthropic",
 "granted_tier":"SandboxTier(3)","egress":"declared-not-enforced","egress_enforced":false,…}
{"event":"worker_completion","worker_cli":"claude","completion":"completed","completed":true,
 "last_stdout_tl_ref":"…"}
{"event":"host_b_delegation_served","frame_id":"010000000000000092370581abd4df63",
 "outcome_frame_id":"…","worker_outcome":"completed"}
```

**같은 16바이트가 양쪽 TL에 있어야 합니다:**

```bash
for H in $HOME_A $HOME_B; do
  echo "--- $H ---"
  sqlite3 $H/audit/transparency.sqlite \
    "select lower(hex(frame_id)), kind, intent from transparency_log;"
done
```

⚠ TL은 `$MAOS_HOME/audit/transparency.sqlite`입니다. **`MAOS_AUDIT_DB`는 이 경로에서 무시**되며,
지정하면 0바이트 파일만 만듭니다.

⚠ 전사 오류는 실패보다 나쁩니다: `invalidate_if_boot_nonce_differs`가 핀을 무효화하므로 두 번째
시도가 **다른** 에러로 실패하고, 복구는 host B **재시작**입니다. 또한 `-32004` nonce 거부는
host B에 **TL 행을 남기지 않습니다** — sender만 압니다.

---

## 4장 — 증거 수집

### 4.1 각 호스트가 자기 키로 봉인 — `--host` **필수**

```bash
HOME=$HOME_A MAOS_HOME=$HOME_A ./target/release/maosctl audit sealed-export \
  --range 1d --host host-a --audit-key $HOST_A_KEY --output $LAB/host-a-bundle.json

HOME=$HOME_B MAOS_HOME=$HOME_B ./target/release/maosctl audit sealed-export \
  --range 1d --host host-b --audit-key $HOST_B_KEY --output $LAB/host-b-bundle.json
```

⚠ **`--host`를 빼면 `reconcile-hosts`가 거부합니다:**
`refused: bundle carries no host claim, so it cannot be half of a two-host run`.
`keygen`은 **잘린** 지문만 찍습니다 — 진짜 64-hex는 `sealed-export`가 **STDERR**로 출력합니다.

### 4.2 낯선 사람의 경로 (선택 아님)

```bash
python3 tools/verify-audit-bundle/verify.py $LAB/host-a-bundle.json $FPR_A
python3 tools/verify-audit-bundle/verify.py $LAB/host-b-bundle.json $FPR_B
```

둘 다 `OK — signature verified`. **아티팩트에 담긴 `attester_pubkey`가 아니라, 커밋된
지문과 대조**하세요 — R-RG1이 그것을 신뢰하는 것을 금지하고
`j1_crosshost_2c_proven_red.rs:458-470`가 기계적으로 강제합니다.

### 4.3 reconcile

```bash
./target/release/maosctl audit reconcile-hosts \
  --bundle-a $LAB/host-a-bundle.json --pubkey-a $FPR_A \
  --bundle-b $LAB/host-b-bundle.json --pubkey-b $FPR_B \
  --receipt-key $OPERATOR_KEY
```

실측 출력:

```
maosctl: audit reconcile-hosts — OK (hosts host-a + host-b, 1 shared frame_ids, 13 A-only, 5 B-only)
claim scope: two keyed identities signed; not two machines, two processes, or two operators
```

두 반쪽이 한 루트로 서명되면 `SharedAttesterRoot`로 **하드 거부**됩니다.

---

## 5장 — capture 두 문서 (⚠ **규칙이 서로 반대**)

여기가 운영자가 과금 후 거부되는 가장 흔한 지점입니다. 정확한 필드 규칙은
`j1-two-host-evidence/README.md`가 규범입니다. 요지:

### 5.1 `CaptureDoc` — `maosctl audit record-capture`용

비어있지 않아야 하는 문자열 7개: `signer`, `live_agent_identity`, `command_metadata`,
`host_grant_disposition`, `egress_followup`, `fs_jail_followup`, `outcome`.
`audit_refs` 최소 1개. 정확히 일치해야 하는 3개:

| 필드 | 값 |
|---|---|
| `egress` | `declared-not-enforced` |
| `fs_jail` | `adapter-enforced-maos-declared` |
| `redaction_result` | `verified` |

2-호스트 주장 시 추가 4개:

| 필드 | 값 |
|---|---|
| `two_host_shape` | `two-processes-one-box` 또는 `two-machines` (닫힌 집합) |
| `two_host_trust_anchor` | `out-of-band-human-operator` |
| `two_host_host_b_audit_key` | `hand-provisioned-separately` |
| `two_host_stranger_verification` | 비어있지 않을 것 |

릴리스 바이너리의 `sha256`은 **여기에** 넣으세요 — 여기서만 번들 서명이 덮습니다.

### 5.2 `two-host-capture.json` — 게이트 leg 9용

`two-host-capture.example.json`을 **복사**하세요. 문자열 4개 + **불리언 2개**(`as_bool()`로
읽히므로 문자열 `"true"`도 `1`도 실패).

`claim_scope`는 **78바이트 그대로**, trim 없이 바이트 비교:

```
two keyed identities signed; not two machines, two processes, or two operators
```

⛔ **이 78바이트를 다른 top-level 문자열에 복사하면 RED입니다.** `claim_scope`만 스캔에서
면제되고 **그 텍스트는 면제가 아닙니다** — `operator_note`에 넣으면 `or two operators`의 앞이
`not `이 아니라 `or `라서 걸립니다. 주장을 친절하게 재진술하는 신중한 운영자가 과금 후
거부되는 가장 유력한 경로입니다.

⛔ **`shape`에 `two machines` 인접 bigram 금지.** 실제로 두 대의 물리 머신이었다면 검증된 문장:
`two distinct physical machines on separate hardware, separate OS kernels, separate NICs`.

⛔ **CLI 토큰을 재사용하지 마세요.** `record-capture`는 `two-machines`를 **강제**하지만 이 게이트는
하이픈을 정규화해 **거부**합니다. 두 문서, 같은 단어, 정반대 계약입니다.

---

## 6장 — 배치, 게이트, 스토리

```bash
cd _bmad-output/test-artifacts/j1-two-host-evidence/
cp $LAB/two-host-capture.json .
cp $LAB/host-a-bundle.json .
cp $LAB/host-b-bundle.json .
# two-host-evidence.txt 는 쓰지 마세요 — 2e AC3가 상수를 삭제했고,
# 읽는 것도 만드는 것도 없습니다.
```

```bash
cargo run -p xtask -- check-j1-two-host-signed-run --json | jq .
```

⛔ **exit code가 아니라 JSON 필드를 읽으세요.** `passed`와 `oracle_green`은 capture가 **없어도,
유효해도, 위조여도** green입니다 — `j1_crosshost_2c_proven_red.rs:386-414`가 단일근·무서명 위조
쌍이 통과함을 커밋으로 증명합니다. 판별자는 `paid_run_capture_present`와
`two_host_signed_run_claimed` **둘뿐**이며, `capture_signature_verified`는 2e AC3가 **삭제**했습니다.

**`two_host_signed_run_claimed: false`가 정상이자 정직한 결과입니다** — R1 재범위화에 따라
`PROVEN_LIVE_SIGNED`는 **이 게이트에서** 도달 불가이며(좁고 의도적: 운영자 레인에서는 27개 leg가
도달), 참인 사실로 공표됩니다.

마지막:

```bash
# 스토리 파일: AC8 + T8 체크박스, Dev Agent Record에 실측 증거, Change Log
# sprint-status.yaml: j1-crosshost-2d-paid-two-host-run: review
cargo run -p xtask -- check-dev-record-completeness --json | jq .violation_count   # 0
```

§A6 리뷰는 **`anthropic/claude-opus-5`가 아닌 모델**이 수행해야 합니다.

---

## 부록 A — 이 절차를 만들며 실제로 부딪힌 함정 7개

전부 실행해서 발견했고, 전부 기존 문서에 없었습니다.

| # | 증상 | 원인 | 해결 |
|---|---|---|---|
| 1 | `failed to read manifest ./topology/a` (정상 부팅 로그 ~30줄 뒤) | 매니페스트가 **마지막에** 읽힘. `maos run --help`도 없음 | `spirits/topologies/…toml`. 초기화 로그는 인자 수락의 증거가 아님 |
| 2 | `endpoint scheme must be 'tls://', got 127.0.0.1:19443` | 스킴 누락 | `tls://host:port` |
| 3 | `BAD_CERTIFICATE: CaUsedAsEndEntity` | `openssl req -x509`이 `CA:TRUE`를 넣음 | `basicConstraints=critical,CA:FALSE` + EKU 명시 (2.2) |
| 4 | `missing reserved intent … cohort:manifest-reissue` | 상수 이름 오류 | `cohort:manifest-reissue` (`cohort:reissue` 아님) |
| 5 | `no peer config for host_id developer-remote-host` | `peer_id`에 cohort 멤버 이름을 씀 | `peer_id` = 토폴로지 역할 id, `local_host` = cohort 멤버 id (2.5) |
| 6 | `refusing to spawn real agent CLI 'claude' on the hermetic path` | `MAOS_LIVE_AGENT`가 host A에만 있음 | **host B에도** 설정 (3.2) |
| 7 | `refused: bundle carries no host claim` | `sealed-export --host` 누락 | `--host host-a` / `--host host-b` (4.1) |

추가로: 데몬을 `maos run <topology>`로 띄우면 위임을 받지 못하고 founder loop를 실행합니다(3.2);
같은 state home 재사용은 FR21 60초 윈도우에 걸립니다(1.2); TL은 `$MAOS_HOME/audit/transparency.sqlite`
이며 `MAOS_AUDIT_DB`는 무시됩니다(3.4).

---

## 부록 B — 중단 조건 (하나라도 걸리면 서명 금지)

| # | 조건 | 이유 |
|---|---|---|
| 1 | `verify.py`가 FAIL | Phase 7.4 필수 중단 |
| 2 | double-boot falsifier가 override 값 반복 | debug assertions ON, 릴리스 아님 |
| 3 | `~/.claude/.credentials.json` 존재 | redaction 입증 불가 = Tier-2 실패 |
| 4 | `FPR_A == FPR_B` | `SharedAttesterRoot` |
| 5 | 서명자가 커밋된 지문과 불일치 | 약정한 런이 아님 |
| 6 | 워커의 CRUD가 지정 cwd 밖 | capability 범위 이탈 |
| 7 | TL이나 capture에 비밀값 잔존 | redaction 실패 |
| 8 | nonce 전사 오류 후 재시도 | 핀 무효화 — host B 재시작 필요 |
| **9** | **`completed=true`인데 워커가 실제로 파일을 쓰지 않음** | ⛔ **차단 조건.** `claude`의 판정은 **효과 oracle이 아닙니다** — 아래 참조 |

### ⛔ 부록 B-9 — `claude`의 `completed=true`는 "파일이 써졌다"는 증거가 아닙니다

두 어댑터는 **비대칭**이고, 이 런에서 그게 중요합니다.

| | `codex` | `claude` (host B가 실행하는 쪽) |
|---|---|---|
| 완료 판정 근거 | **네이티브 효과 증거** — `item.completed` 타입 `file_change`, `status: "completed"`, 비어있지 않은 `changes` (`worker_cli.rs:462-465`) | `subtype == "success"` + `is_error == false` + **빈** `permission_denials` (`:498-537`) |
| 효과 검사 | 있음 — 없으면 `NoEffectEvidence` | **없음** |

코드가 이 residual을 정확히 명시합니다(`:490-497`): *"도구 호출을 시도하지 않고 그냥 거절하는
모델은 `permission_denials`를 비워 두므로 성공과 구별할 수 없다 … 효과 oracle이 아니다."*
**T6(이미 서명됨)는 codex라서 효과 증거가 네이티브였습니다. 2-호스트 런은 host B에 claude를
올리며 그 속성을 조용히 잃습니다.**

2026-08-22 실측 (가짜 `claude` result object 3종을 실제 어댑터에 투입):

| 입력 | 판정 | exit |
|---|---|---|
| `permission_denials` 비어있지 않음 | `not_completed:permission_denied`, `completed=false` | **1** ✅ ship-blocker의 그 형태는 잡힙니다 |
| 깨끗한 객체 + *"I have written the file"* + 손대지 않은 트리 | **`completed=true`** | **0** ⚠ |

**따라서 서명 전에 손으로 확인하고, 그 결과를 capture에 기록하세요:**

```bash
ls -la $WORK_B                        # 워커가 실제로 만든 것
git -C $WORK_B status --porcelain     # 워크트리라면
```

아무것도 없으면 `completed=true`여도 **서명하지 마세요.** 서명은 판정을 덮고, 판정은 *거절이
없었음*을 덮을 뿐 *효과가 있었음*을 덮지 않습니다. `RELEASE-HOLDS.md` row 16이 이 경계입니다.

---

## 검증 기록

2026-08-22, 커밋 `dd4cf959` + `j1-crosshost-2e` 적용 트리. 1장~4장 전체를 **가짜 `claude`
픽스처**로 실행:

- cohort 서명: rc=0, authority `ed4d4ebe…`
- host A: nonce `7196704562630440850` 발행 후 대기 → 해제 → **exit 0**
- host B: `listening on 127.0.0.1:19555`, grant 매칭(`claude`/`Anthropic`/T3),
  `worker_completion completed=true`, `host_b_delegation_served`
- 효과 증거: 워커가 `h.txt` 생성, 내용이 `MAOS_DELEGATED_GOAL`과 **바이트 일치**(비ASCII `café` 보존)
- 공유 `frame_id`: `010000000000000092370581abd4df63` — **양쪽 TL에 존재**
- `sealed-export --host` ×2 (14 / 6 entries), `verify.py` ×2 `OK — signature verified`
- `reconcile-hosts`: `OK (hosts host-a + host-b, 1 shared frame_ids, 13 A-only, 5 B-only)`

**검증되지 않은 것: 실제 과금 claude spawn 하나.** 가짜 픽스처를 실제 `claude` +
`ANTHROPIC_API_KEY`로 바꾸면 5장으로 진행합니다.
