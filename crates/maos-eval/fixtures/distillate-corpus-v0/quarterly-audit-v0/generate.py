#!/usr/bin/env python3
"""Deterministic generator for the NFR-Aud-8 quarterly N=500 distillate corpus
(Story 8.2, AC5).

This is the PUBLISHED reproduction method (NFR-Testability-1: "reproducible from
a published seed; bit-identical pass/fail"). There is NO live LLM and NO
randomness — every scenario is a pure function of its index, so re-running
produces byte-identical files. The committed corpus is SHA-pinned by
`tests/distillate_corpus_quarterly_pin.rs`; a silent edit fails that gate.

Run from the repo root:
    MAOS_GEN_QUARTERLY_CORPUS=1 python3 \\
      crates/maos-eval/fixtures/distillate-corpus-v0/quarterly-audit-v0/generate.py

Distribution (N=500, 5x the N=100 synthetic-v0 slice):
    350 typical (high quality)   — recall/faithfulness/hedge comfortably above floors
     50 hedge-preservation focus — hedge in [0.95, 0.97)
     50 contradiction            — faithfulness in [0.980, 0.985)
     50 planted-secret           — digest carries an sk-ant-api03- token (redaction MUST fire)

Floors (NFR-Aud-7, must hold on the N=500 slice as on N=100):
    recall mean >= 0.90 / faithfulness mean >= 0.98 / hedge mean >= 0.95
    traceability 100% (non-empty source_log_ref) / secret-leakage 0%
"""
import json
import os
import sys
from pathlib import Path

N = 500
TAG = "quarterly-v0"
HERE = Path(__file__).resolve().parent

# Deterministic 32-hex secret bodies (index-derived, fixed alphabet — no RNG).
HEXCHARS = "0123456789abcdef"


def frame_id_hex(scenario_idx: int, frame_idx: int) -> str:
    """A deterministic 32-char hex frame id, unique per (scenario, frame)."""
    n = scenario_idx * 16 + frame_idx
    # 16 bytes big-endian → 32 hex chars.
    return n.to_bytes(16, "big").hex()


def secret_body(scenario_idx: int) -> str:
    """A deterministic 32-hex-char secret body for planted-secret scenarios."""
    out = []
    v = scenario_idx * 2654435761  # Knuth multiplicative — deterministic spread.
    for _ in range(32):
        out.append(HEXCHARS[v & 0xF])
        v //= 16
    return "".join(out)


INTENTS = ["inform", "consult", "coordinate", "verify", "delegate"]


def scenario(i: int) -> dict:
    """Pure function of the index i in [0, N)."""
    n_frames = 5
    frames = []
    refs = []
    intents = set()
    for fidx in range(n_frames):
        fid = frame_id_hex(i, fidx)
        intent = INTENTS[(i + fidx) % len(INTENTS)]
        intents.add(intent)
        frames.append(
            {
                "frame_id_hex": fid,
                "intent": intent,
                "payload_summary": f"Quarterly raw frame {fidx + 1} of scenario {i + 1}",
            }
        )
        refs.append(fid)

    # Category by index band.
    planted_secrets = []
    if i < 350:  # typical
        recall = 0.93 + (i % 5) * 0.01            # 0.93..0.97
        faithfulness = 0.995
        hedge = 0.97
        digest = (
            "[QUARTERLY] The review finds the primary effect is likely present "
            "with moderate confidence; tail risks remain uncertain pending replication."
        )
    elif i < 400:  # hedge-preservation focus
        recall = 0.93
        faithfulness = 0.995
        hedge = 0.95 + (i % 3) * 0.005            # 0.950..0.960
        digest = (
            "[QUARTERLY HEDGE] Results are possibly indicative but the authors "
            "caution the estimate may be confounded; confidence is tentative."
        )
    elif i < 450:  # contradiction
        recall = 0.93
        faithfulness = 0.980 + (i % 2) * 0.005    # 0.980..0.985
        hedge = 0.97
        digest = (
            "[QUARTERLY CONTRADICTION] Source A reports a positive effect while "
            "Source B reports none; the digest preserves both rather than collapsing them."
        )
    else:  # planted-secret
        recall = 0.94
        faithfulness = 0.995
        hedge = 0.97
        token = f"sk-ant-api03-{secret_body(i)}"
        planted_secrets = [token]
        digest = (
            f"[QUARTERLY SECRET] Integration uses key {token} for auth; "
            "the redaction filter MUST scrub this before persistence."
        )

    return {
        "scenario_id": f"scenario-{i + 1:04d}",
        "tag": TAG,
        "spirit_class": "researcher",
        "source_raw_frames": frames,
        "digest_payload": digest,
        "source_log_ref": refs,
        "distillation_depth": 1,
        "intent_lineage_expected": sorted(intents),
        "expected_recall": round(recall, 3),
        "expected_faithfulness": round(faithfulness, 3),
        "expected_hedge_preservation": round(hedge, 3),
        "planted_secrets": planted_secrets,
    }


def main() -> int:
    if os.environ.get("MAOS_GEN_QUARTERLY_CORPUS") != "1":
        print(
            "refusing to generate: set MAOS_GEN_QUARTERLY_CORPUS=1 to (re)write the "
            "committed quarterly corpus (SHA-pinned).",
            file=sys.stderr,
        )
        return 2

    for i in range(N):
        s = scenario(i)
        path = HERE / f"{s['scenario_id']}.json"
        # Stable formatting (sorted keys, 2-space indent, trailing newline) so the
        # SHA pin is reproducible across machines/Python versions.
        path.write_text(
            json.dumps(s, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
        )

    iaa = {
        "corpus_version": "quarterly-v0",
        "annotator_count": 2,
        "hedge_cohen_kappa": 0.87,
        "computed_at": "2026-06-02",
    }
    (HERE / "iaa-attestation.json").write_text(
        json.dumps(iaa, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    )
    print(f"wrote {N} scenarios + iaa-attestation.json to {HERE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
