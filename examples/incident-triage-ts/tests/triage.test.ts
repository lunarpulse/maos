import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

import { handleFrame, lifecycle, onShutdown, onStart, triage } from "../src/index.js";

const frame = { frameId: new Uint8Array(16).fill(17) } as Parameters<typeof handleFrame>[0];
const shell = { spiritId: "@maos-shell" };

function shellRequest(goal: string): Parameters<typeof handleFrame>[0] {
  return {
    ...frame, frameFrom: shell, to: [{ spiritId: "incident-triage-ts" }], kind: "task-assign",
    autoMarker: "human-authored",
    payload: { tag: "task-assign", val: {
      goal, scope: [], successCriteria: "Answer the operator's shell message",
    } },
  } as unknown as Parameters<typeof handleFrame>[0];
}

function answer(goal: string): string {
  const [reply, ...rest] = handleFrame(shellRequest(goal));
  expect(rest).toEqual([]);
  expect(reply.kind).toBe("task-complete");
  expect(reply.to).toEqual([shell]);
  if (reply.payload.tag !== "task-complete") throw new Error("not a task-complete");
  return reply.payload.val.resultText;
}

it("declares no capability the guest does not use", () => {
  const manifest = readFileSync(new URL("../manifest.toml", import.meta.url), "utf8");
  expect(manifest).toContain("[capabilities.required.iac]");
  expect(manifest).not.toMatch(/^\[capabilities\.required\.(log|skill)\]$/m);
  expect(manifest).not.toMatch(/^\[skills\]$/m);
});

describe("incident triage guest", () => {
  beforeEach(onStart);
  afterEach(onShutdown);

  it.each([
    ["Customers report data loss after the migration", "sev1 | rule: data-loss | first step: Freeze writes and page the data on-call."],
    ["Checkout is slow, p99 latency doubled", "sev2 | rule: degraded-latency | first step: Check the saturation dashboard of the affected service."],
    ["There is a typo on the pricing page", "sev3 | rule: minor-defect | first step: File a ticket in the owning team's backlog."],
  ])("answers %j", (goal, expected) => {
    expect(answer(goal)).toBe(expected);
  });

  it("answers an unknown text with sev4 and says no rule matched", () => {
    expect(answer("The cafeteria ran out of oat milk")).toBe(
      "sev4 | rule: none | first step: no rule matched",
    );
  });

  it("matches case-insensitively and takes the first rule in table order", () => {
    expect(triage("SITE DOWN and also a typo")).toContain("rule: full-outage");
  });

  it("never echoes the goal", () => {
    expect(answer("hello")).not.toContain("echo");
  });

  it("leaves a task-assign that is not the shell's request unanswered", () => {
    const other = {
      ...shellRequest("outage"), frameFrom: { spiritId: "peer" },
    } as unknown as Parameters<typeof handleFrame>[0];
    expect(handleFrame(other)).toEqual([]);
  });

  it("leaves a non-task frame unanswered", () => {
    const other = { ...frame, kind: "log-event", frameFrom: shell } as unknown as Parameters<typeof handleFrame>[0];
    expect(handleFrame(other)).toEqual([]);
  });

  it("faults before start", () => {
    onShutdown();
    expect(() => handleFrame(shellRequest("outage"))).toThrow();
  });
});

describe("lifecycle export", () => {
  it("carries no state across a swap and migrates unchanged", () => {
    expect(lifecycle.snapshot()).toEqual(new Uint8Array());
    const state = new Uint8Array([1, 2, 3]);
    expect(lifecycle.migrate(state)).toEqual(state);
  });
});
