# MAOS DSA 세션 인수인계

## 현재 상태

DSA(Domain-Specific Agent, 도메인 특화 에이전트) 작업 경로는 별도 Git worktree `/mnt/build/wt-maos-dsa`다. 현재 작업 브랜치는 `dsa-heldout-evidence`다. 이전 경로 `~/dev_ws-dsa`와 공용 체크아웃을 사용하지 않는다. 빌드 결과도 분리한다: `CARGO_TARGET_DIR=/mnt/build/cargo-maos-dsa`. 커널과 Spirit ABI는 수정하지 않았다.

## 무엇을 만들었나

1. **업무 흐름을 Rust 밖에서 정의한다.** 업무 담당자는 TOML에서 역할과 결과별 이동 경로를 정한다. `maos workflow plan`이 실행 계획과 해시를 만들고, `apply`는 승인한 해시와 현재 카탈로그 권한을 확인한 뒤 활성화한다. `run`은 활성 버전을 실행하며, `eval`은 최종 결과와 실제 단계 경로·단계별 결과를 확인할 수 있다. 최초 실험에서는 인보이스와 도서 반납을 같은 실행기로 검증했다. 새 검사기의 권한을 계획 후 철회하면 활성화가 거부되고 기존 계획이 유지됐다.
2. **한 에이전트가 새 증거를 만들고 다음 에이전트가 실제로 사용한다.** GPT-6.2 Astra의 독립 계획에 따라 `Governed Evidence Handoff`를 구현했다. `extract`는 인보이스 발췌문에서 `purchase_order_present`와 원문 인용 `quote`를 생성한다. `review`는 원래 문서가 아니라 그 결과를 받아 판단한다. 같은 입력과 응답으로 TOML의 경로만 바꿨을 때 `request_order`가 `hold_for_procurement`로 바뀌었다. 주문서가 있는 경우는 두 번째 계획에서도 `ready_for_approval`이며, 실제 승인이나 결제는 일어나지 않는다.
3. **권한과 전달을 검증한다.** 추론은 기존 MAOS 포트에서 Spirit의 PID에 발급한 권한으로 실행한다. 선언되지 않은 결과, 잘못된 JSON·필드 타입, 원문에 없는 인용, 프롬프트 해시 불일치와 와일드카드 해시는 거부한다. 운영자 정책이 추론을 금지하면 추론 호출과 검토 단계 모두 실행되지 않는다. 감사 로그에서 생성 단계의 출력 해시와 검토 단계의 입력 해시가 일치하는 것을 확인했다. 입력·출력 스키마가 같아도 새로 생성한 출력의 실제 내용으로 해시를 계산한다.

## 검증 기록과 한계

```sh
cd /mnt/build/wt-maos-dsa
CARGO_TARGET_DIR=/mnt/build/cargo-maos-dsa cargo test --locked --offline \
  -p maos-topology -p maos-bin --test infer --test workflow_evidence_cli --test workflow_cli
```

**6건 통과:** 기존 CLI 1건, 증거 전달 CLI 3건, 계약 검증 2건. 별도 CLI 실행에서도 `extract → review`, `request_order`, `purchase_order_present: false`, 인용 `No purchase order was supplied.`를 확인했다. [실행 가이드](README.md#run-the-governed-evidence-handoff).

이번 추론은 **정확한 프롬프트 해시에 묶인 합성 응답 재생(strict replay)** 이다. 실제 모델이나 네트워크 제공자를 호출하지 않았다. 인용이 원문에 존재함을 확인할 뿐, 인용이 결론을 뒷받침하는지까지 판정하지 않는다. 따라서 실제 모델의 정확도·비용·운영 품질을 검증한 결과가 아니다. 기존 추론 감사 경로가 프롬프트에 포함된 발췌문을 기록할 수 있으므로 합성 데이터만 사용한다. 상시 실행 데몬의 배포 API, 원격 에이전트, 반복·병렬 합류, 외부 업무 실행은 이번 범위가 아니다.

## 파일럿 코드 크기 승인

초기 파일럿의 `kloc-check`는 `maos-topology` 예산이 없어 실패했다. 운영자의 파일럿 KLOC 승인 후 `main` 기준으로 다시 측정하고 기존 예산 원장에 반영했다. 현재 정책은 크레이트별 한도만 차단에 사용한다. 전체 코드 크기 경보와 커널 한도는 변경하지 않았다.

| 항목 | 이전 한도 | 파일럿의 측정 줄 수 및 적용 한도 |
|---|---:|---:|
| `maos-bin` | 39,699 | 39,719 |
| `maos-topology` | 없음 | 923 |
| `xtask` | 46,753 | 46,754 |

[파일럿 PR #24](https://github.com/lunarpulse/maos/pull/24)는 `main`의 `ea8c51b6`에 병합됐다. [PR #25](https://github.com/lunarpulse/maos/pull/25)가 의존성 검사 차단을 해소했고, `workflow.rs`를 기존 권한 보호 테스트의 검사 대상에 추가했다. 파일럿의 [GitHub CI](https://github.com/lunarpulse/maos/actions/runs/44618174893)에서 `cargo-deny`, `workspace-test-suite`, `aggregate`, `release-ready`가 모두 통과했다.

## 활성화 전 후보 평가

`maos workflow compare <후보-plan.json> <후보-digest> <cases.json>`은 현재 활성 계획과 후보를 동일하게 라벨링한 사례로 비교한다. 후보의 부모 해시는 현재 활성 버전과 일치해야 한다. 두 계획 모두 현재 운영자 카탈로그에 등록돼 있어야 한다. 비교는 감사 로그를 남기지만 배포 DB를 바꾸거나 후보를 활성화하지 않는다. 활성화는 별도의 `apply`가 맡는다.

인보이스 실험에서 기준 계획은 `request_order`, 후보는 `hold_for_procurement`였다. 후보의 인용만 바꿔도 최종 결과는 그대로였지만 `extract.quote` 불일치로 비교가 실패했다. 보고서에는 계획·사례·재생 데이터의 해시와 불일치 필드 이름이 남고 증거 원문이나 정답 값은 포함되지 않는다. [후보 평가 실행 가이드](README.md#compare-a-candidate-before-applying-it).

운영자가 별도로 승인한 코드 예산을 `xtask/kloc.toml`에 반영했다. `maos-bin` 39,909줄(+190), `maos-topology` 953줄(+30)로 KLOC 검사를 통과했다. 파일럿과 후보 평가를 함께 검증한 테스트 23건이 통과했고, 별도 PostgreSQL 환경이 필요한 1건은 건너뛰었다.

`wasmtime-wasi` 보안 수정 [PR #27](https://github.com/lunarpulse/maos/pull/27)이 포함된 `main`에서 [PR #26의 CI](https://github.com/lunarpulse/maos/actions/runs/44904813136)가 통과했다. [PR #26](https://github.com/lunarpulse/maos/pull/26)은 `25536dec`로 병합됐다.

## 인보이스 홀드아웃 평가

[합성 사례 12건](invoice-evidence-holdout-v1.json)에 주문서가 있는 경우와 없는 경우를 각각 6건씩 넣었다. 다른 인보이스의 주문서, 취소·초안 주문서, 여러 줄로 된 필드 등에서 주문서 존재 여부·원문 인용·에이전트 경로·최종 결과를 구분해 채점한다. [실행 가이드](README.md#run-the-holdout-scoring-control)의 합성 재생에서 기준 계획 6/12, 후보 계획 12/12를 확인했다. 정답을 바탕으로 만든 재생 응답이므로 이 차이는 라우팅 변경을 검증하며, 모델 품질 차이를 뜻하지 않는다.

후보의 인용문만 원문의 무관한 문장으로 바꾼 대조 실험에서는 최종 결과 `ready_for_approval`가 맞아도 `extract.quote` 불일치로 11/12 및 비교 실패가 보고됐다. 비교 전후 배포 DB 해시는 같았다.

다음 실측은 별도의 미공개 평가 세트와 새로 수집한 모델 응답을 사용한다. 이 공개 세트는 프롬프트 튜닝과 분리된 개발용 평가 기준이다. 실시간 추론은 `MAOS_INFERENCE_MODE=live`를 명시한 `workflow compare`에서만 사용할 수 있다. `run`, `eval`, `apply`는 이 모드를 허용하지 않는다.

## 다시 시작할 때

- 진입점: [호스트 실행·권한·감사](../../crates/maos-bin/src/workflow.rs), [정확한 재생](../../crates/maos-bin/src/cassette_replay.rs), [계약·라우팅 엔진](../../crates/maos-topology/src/), [증거 전달 CLI 검증](../../crates/maos-bin/tests/workflow_evidence_cli.rs), [같은 스키마에서 실제 출력 해시를 검증하는 테스트](../../crates/maos-topology/tests/infer.rs).

### 실시간 모델로 후보 비교

앞 절의 합성 재생은 채점과 라우팅만 검증했다. 이제
[`workflow compare`의 실시간 측정 모드](README.md#measure-a-candidate-with-a-live-model-explicit-bounded)에서
`MAOS_INFERENCE_MODE=live`와 `MAOS_WORKFLOW_MAX_INFERENCE_CALLS`를 명시하고,
등록된 제공자와 모델을 카탈로그에 고정한 뒤 `--repeat`로 활성 계획과 후보를
같은 사례에서 반복 측정할 수 있다. 비교는 후보를 활성화하지 않는다. 보고서는
라벨 불일치와 응답 오류 등으로 실행을 마치지 못한 경우를 구분하고, 토큰과
추론 호출 지연 시간을 집계한다. 비용은 감사 로그에서 별도로 대사한다.

실제 모델의 정확도·응답 오류율·지연 시간·토큰 사용량·비용은 이 인수인계에
실측 결과가 없다. 공개 합성 사례 12건은 개발용 벤치마크이며, 적은 사례와
반복 횟수로는 운영 합격 기준을 정할 수 없다. 출시 품질을 추정하려면 별도로
보관한 미공개 사례에서 새 응답을 수집해야 한다.
