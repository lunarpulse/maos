import { describe, expect, it } from "vitest";

import { handleFrame, onShutdown, onStart } from "../src/index.js";

describe("WASM component guest", () => {
  it("faults before start and echoes the inbound frame after start", () => {
    const frame = {} as Parameters<typeof handleFrame>[0];

    try {
      handleFrame(frame);
      throw new Error("handleFrame should halt before onStart");
    } catch (thrown) {
      expect(thrown).toMatchObject({ tag: "fault" });
    }

    onStart();
    expect(handleFrame(frame)).toEqual([frame]);
    onShutdown();
  });
});
