import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("maos:spirit/recall@2.1.0", () => ({
  recall: vi.fn(),
  fetch: vi.fn(),
}));

import { recall, fetch } from "maos:spirit/recall@2.1.0";
import { handleFrame, onShutdown, onStart } from "../src/index.js";

// Imported WIT result errors are wrapped by componentize-js in this shape.
function componentError(tag: string): Error {
  return Object.defineProperty(new Error("component error"), "payload", {
    value: { tag }, enumerable: true,
  });
}

const frame = { frameId: new Uint8Array(16).fill(17) } as Parameters<typeof handleFrame>[0];

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
    expect(recall).toHaveBeenCalledWith({ limit: 16 });
    expect(fetch).toHaveBeenCalledWith(frame.frameId);
    expect(vi.mocked(recall).mock.invocationCallOrder[0]).toBeLessThan(
      vi.mocked(fetch).mock.invocationCallOrder[0],
    );
  });

  it("echoes when fetch reports scope-violation", () => {
    vi.mocked(fetch).mockImplementation(() => { throw componentError("scope-violation"); });
    expect(handleFrame(frame)).toEqual([frame]);
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
