// Public exports for @maos/spirit-ts
// Type-only re-exports use `export type` (required by tsconfig isolatedModules);
// runtime values (enums, classes) use a plain `export`.
export type { Spirit, Ctx, MockBus } from "./spirit.js";
export type { SpiritId } from "./identity.js";
export { FrameKind } from "./identity.js";
export { HaltResolutionKind } from "./halt.js";
export type { HaltResolutionRecord } from "./halt.js";
