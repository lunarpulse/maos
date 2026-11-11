import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";

vi.mock("maos:spirit/recall@2.4.0", () => ({
  recall: vi.fn(),
  fetch: vi.fn(),
}));
vi.mock("maos:spirit/skills@2.4.0", () => ({ author: vi.fn() }));

import { recall, fetch } from "maos:spirit/recall@2.4.0";
import { author } from "maos:spirit/skills@2.4.0";
import { handleFrame, lifecycle, onShutdown, onStart } from "../src/index.js";

// Imported WIT result errors are wrapped by componentize-js in this shape.
function componentError(tag: string): Error {
  return Object.defineProperty(new Error("component error"), "payload", {
    value: { tag }, enumerable: true,
  });
}

const frame = { frameId: new Uint8Array(16).fill(17) } as Parameters<typeof handleFrame>[0];


it("ships without an unavailable prerequisite and uses routed intent vocabulary", () => {
  const manifest = readFileSync(new URL("../manifest.toml", import.meta.url), "utf8");
  expect(manifest).not.toMatch(/^\[skills\]$/m);
  expect(manifest).toContain('allowed = ["standard"]');
});
describe("WASM component guest", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(recall).mockReturnValue({ entries: [] });
    onStart();
  });
  afterEach(onShutdown);

  it("faults before start without calling recall", () => {
    onShutdown();
    expect(() => handleFrame(frame)).toThrow(expect.objectContaining({ tag: "fault" }));
    expect(recall).not.toHaveBeenCalled();
    expect(fetch).not.toHaveBeenCalled();
  });

  it("echoes after successful recall and fetch", () => {
    vi.mocked(fetch).mockReturnValue({
      frameId: frame.frameId, timestampNs: 0n, kind: "TaskAssign",
      intent: "standard", payload: new Uint8Array(), origin: "human-authored",
    });
    expect(handleFrame(frame)).toEqual([frame]);
    expect(author).toHaveBeenCalledTimes(1);
    expect(recall).toHaveBeenCalledWith({ limit: 16 });
    expect(fetch).toHaveBeenCalledWith(frame.frameId);
    expect(vi.mocked(recall).mock.invocationCallOrder[0]).toBeLessThan(
      vi.mocked(fetch).mock.invocationCallOrder[0],
    );
  });

  it.each(["refused", "unavailable", "capability-denied"])(
    "continues after skill authoring reports %s", (tag) => {
      vi.mocked(author).mockImplementation(() => { throw componentError(tag); });
      expect(handleFrame(frame)).toEqual([frame]);
      expect(handleFrame(frame)).toEqual([frame]);
      expect(author).toHaveBeenCalledTimes(1);
    },
  );

  it("echoes when fetch reports scope-violation", () => {
    vi.mocked(fetch).mockImplementation(() => { throw componentError("scope-violation"); });
    expect(handleFrame(frame)).toEqual([frame]);
  });

  it("echoes when fetch reports consent-withheld", () => {
    vi.mocked(fetch).mockImplementation(() => { throw componentError("consent-withheld"); });
    expect(handleFrame(frame)).toEqual([frame]);
    expect(fetch).toHaveBeenCalledWith(frame.frameId);
  });

  it("rethrows fetch capability-denied unchanged", () => {
    const error = componentError("capability-denied");
    vi.mocked(fetch).mockImplementation(() => { throw error; });
    expect(() => handleFrame(frame)).toThrow(error);
  });

  it.each(["unavailable", "scope-violation"])("rethrows recall %s without fetching", (tag) => {
    const error = componentError(tag);
    vi.mocked(recall).mockImplementation(() => { throw error; });
    expect(() => handleFrame(frame)).toThrow(error);
    expect(fetch).not.toHaveBeenCalled();
  });

  it("does not swallow an unwrapped scope-violation", () => {
    const error = { tag: "scope-violation" };
    vi.mocked(fetch).mockImplementation(() => { throw error; });
    expect(() => handleFrame(frame)).toThrow(expect.objectContaining(error));
  });
});

describe("lifecycle export", () => {
  it("hands the successor the predecessor's snapshot", () => {
    lifecycle.swapIn(new TextEncoder().encode("0"));
    lifecycle.idle();
    lifecycle.idle();
    const state = lifecycle.snapshot();
    expect(new TextDecoder().decode(state)).toBe("2");
    lifecycle.swapIn(state);
    lifecycle.idle();
    expect(new TextDecoder().decode(lifecycle.snapshot())).toBe("3");
  });

  it("treats a corrupt snapshot as no state and migrates unchanged", () => {
    lifecycle.swapIn(new TextEncoder().encode("not a number"));
    expect(new TextDecoder().decode(lifecycle.snapshot())).toBe("0");
    const state = new Uint8Array([1, 2, 3]);
    expect(lifecycle.migrate(state)).toEqual(state);
  });
});
