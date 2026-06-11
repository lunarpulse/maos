// Types for the spirit_test module.
// `MockBusFrame`/`RunReport` are interfaces → `export type` (isolatedModules);
// `SpiritTest` is a runtime class → plain `export`.
export type { MockBusFrame, RunReport } from "./index.js";
export { SpiritTest } from "./index.js";
