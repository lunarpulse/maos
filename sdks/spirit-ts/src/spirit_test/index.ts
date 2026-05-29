// SpiritTest harness + expectFrame, expectHalt, assert test helpers.

import { Spirit, Ctx } from "../spirit.js";
import { MockCtx } from "../ctx.js";
import { HaltResolutionKind, HaltResolutionRecord } from "../halt.js";

export interface MockBusFrame {
  kind: string;
  bytes: Uint8Array;
}

export interface RunReport {
  hooksFired: Map<string, number>;
  capturedFrames: MockBusFrame[];
  haltResolutions: HaltResolutionRecord[];
  deprecationWarningsSurfaced: Array<{ surface: string; sinceVersion: string; plannedRemoval: string; migrationHint: string }>;
}

export class SpiritTest {
  private spirit: Spirit;

  constructor(spirit: Spirit) {
    this.spirit = spirit;
  }

  run(ctx?: Ctx): RunReport {
    const report: RunReport = {
      hooksFired: new Map(),
      capturedFrames: [],
      haltResolutions: [],
      deprecationWarningsSurfaced: [],
    };
    const context = ctx || new MockCtx();
    this.spirit.onIdle(context);
    report.hooksFired.set("on_idle", 1);
    return report;
  }
}

export function assert(condition: boolean, diagnostic: string): void {
  if (!condition) {
    const err = new Error(
      `assert FAILED\n  condition: ${condition}\n  diagnostic: ${diagnostic}\n  suggested fix: verify the expected hook fired AND emitted the expected frame BEFORE the condition was evaluated.`
    );
    throw err;
  }
}

export function expectFrame(
  report: RunReport,
  criteria: { kind?: string; bytesMatches?: Uint8Array; bytesExact?: Uint8Array }
): void {
  const matched = report.capturedFrames.some((f) => {
    let ok = true;
    if (criteria.kind !== undefined && f.kind !== criteria.kind) ok = false;
    if (criteria.bytesMatches !== undefined) {
      if (f.bytes.length < criteria.bytesMatches.length || !criteria.bytesMatches.every((b, i) => f.bytes[i] === b)) ok = false;
    }
    if (criteria.bytesExact !== undefined) {
      if (f.bytes.length !== criteria.bytesExact.length || !criteria.bytesExact.every((b, i) => f.bytes[i] === b)) ok = false;
    }
    return ok;
  });
  if (!matched) {
    throw new Error(
      `expectFrame FAILED\n  criteria: ${JSON.stringify(criteria)}\n  captured: ${report.capturedFrames.length} frames\n  suggested fix: verify the Spirit emits a matching frame via ctx.send(...) BEFORE the hook returns.`
    );
  }
}

export function expectHalt(
  report: RunReport,
  criteria: { haltId?: string; kindMatches?: HaltResolutionKind }
): void {
  const matched = report.haltResolutions.some((r) => {
    let ok = true;
    if (criteria.haltId !== undefined && r.haltId !== criteria.haltId) ok = false;
    if (criteria.kindMatches !== undefined && r.kind !== criteria.kindMatches) ok = false;
    return ok;
  });
  if (!matched) {
    throw new Error(
      `expectHalt FAILED\n  criteria: ${JSON.stringify(criteria)}\n  recorded resolutions: ${report.haltResolutions.length}\n  suggested fix: verify the test invokes harness.resolveHalt(haltId, HaltResolutionKind...) BEFORE harness.run().`
    );
  }
}
