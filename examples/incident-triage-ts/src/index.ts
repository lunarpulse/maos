import type { IacFrame } from "maos:spirit/spirit-lifecycle@2.4.0";

import { halt } from "./halt.js";

/**
 * A deterministic domain Spirit: it triages an incident description with a fixed rule table.
 *
 * A WebAssembly component has no inference import, so the "domain" lives in this source, not in a
 * model. The first rule whose keyword occurs in the text (case-insensitive) wins, so the table is
 * ordered from most to least severe.
 */
export interface Rule {
  readonly id: string;
  readonly severity: "sev1" | "sev2" | "sev3";
  readonly keywords: readonly string[];
  readonly firstStep: string;
}

export const RULES: readonly Rule[] = [
  {
    id: "data-loss",
    severity: "sev1",
    keywords: ["data loss", "corrupt", "deleted production"],
    firstStep: "Freeze writes and page the data on-call.",
  },
  {
    id: "full-outage",
    severity: "sev1",
    keywords: ["outage", "site down", "all users", "unreachable"],
    firstStep: "Open the incident bridge and declare sev1 on the status page.",
  },
  {
    id: "security-breach",
    severity: "sev1",
    keywords: ["breach", "credential leak", "unauthorized access", "exposed secret"],
    firstStep: "Revoke the exposed credential and isolate the affected host.",
  },
  {
    id: "degraded-latency",
    severity: "sev2",
    keywords: ["latency", "slow", "timeout", "degraded"],
    firstStep: "Check the saturation dashboard of the affected service.",
  },
  {
    id: "partial-failure",
    severity: "sev2",
    keywords: ["error rate", "5xx", "intermittent", "some users"],
    firstStep: "Roll back the most recent deploy of the affected service.",
  },
  {
    id: "minor-defect",
    severity: "sev3",
    keywords: ["typo", "cosmetic", "minor", "ui glitch"],
    firstStep: "File a ticket in the owning team's backlog.",
  },
];

/** The answer for one incident description; unknown text is `sev4`, rule `none`. */
export function triage(text: string): string {
  const lowered = text.toLowerCase();
  const rule = RULES.find((r) => r.keywords.some((keyword) => lowered.includes(keyword)));
  return rule === undefined
    ? "sev4 | rule: none | first step: no rule matched"
    : `${rule.severity} | rule: ${rule.id} | first step: ${rule.firstStep}`;
}

let started = false;

// The local shell is a host-only pseudo-address, outside the `[class].name` grammar, so no
// installed Spirit can hold it. `maosctl ask` delivers its text as exactly this frame: these checks
// only select this Spirit's behavior; they grant no authority and do not choose the reply route.
const SHELL_SOURCE = "@maos-shell";
const SHELL_SUCCESS_CRITERIA = "Answer the operator's shell message";

function isShellRequest(frame: IacFrame): boolean {
  return frame.autoMarker === "human-authored" &&
    frame.kind === "task-assign" &&
    frame.frameFrom?.spiritId === SHELL_SOURCE &&
    frame.frameFrom.hostId == null &&
    frame.frameFrom.role == null &&
    frame.payload?.tag === "task-assign" &&
    frame.payload.val.successCriteria === SHELL_SUCCESS_CRITERIA;
}

/**
 * Answers the shell's request with one `task-complete` addressed to the sender; any other frame
 * is left unanswered. `jco guest-types` exposes the result as `IacFrame[]` and omits WIT's
 * `result<..., halt>` wrapper; throwing `{ tag: "fault", val: reason }` maps to `Halt::Fault`.
 */
export function handleFrame(frame: IacFrame): IacFrame[] {
  if (!started) {
    halt("handleFrame called before onStart");
  }
  if (!isShellRequest(frame) || frame.payload.tag !== "task-assign") {
    return [];
  }
  return [
    {
      ...frame,
      to: [frame.frameFrom],
      kind: "task-complete",
      payload: { tag: "task-complete", val: { resultText: triage(frame.payload.val.goal) } },
    },
  ];
}

export function onStart(): void {
  started = true;
}

export function onShutdown(): void {
  started = false;
}

/**
 * The optional `maos:spirit/lifecycle@2.4.0` export, required by the `spirit-lifecycle` world the
 * toolchain targets. This Spirit keeps no state, so every hook is a no-op and a hot swap carries
 * nothing across.
 */
export const lifecycle = {
  idle(): void {},
  consolidate(_batch: Uint8Array): void {},
  telemetry(_event: unknown): void {},
  schedule(_payload: Uint8Array): void {},
  pause(): void {},
  resume(): void {},
  snapshot(): Uint8Array {
    return new Uint8Array();
  },
  swapIn(_state: Uint8Array): void {},
  migrate(state: Uint8Array): Uint8Array {
    return state;
  },
};
