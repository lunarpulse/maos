/// <reference path="./interfaces/maos-spirit-frames.d.ts" />
declare module 'maos:spirit/spirit@1.0.0' {
  export type IacFrame = import('maos:spirit/frames@1.0.0').IacFrame;
  export type Halt = import('maos:spirit/frames@1.0.0').Halt;
  export type * as MaosSpiritFrames100 from 'maos:spirit/frames@1.0.0'; // import maos:spirit/frames@1.0.0
  /**
  * host → guest: deliver one inbound frame, get zero-or-more emitted
  * frames. A returned `halt` ends the session (maps to the ADR-032
  * EOF semantics).
  */
  export function handleFrame(frame: IacFrame): Array<IacFrame>;
  /**
  * host → guest: lifecycle hooks (mirror the native form's boot/shutdown).
  */
  export function onStart(): void;
  export function onShutdown(): void;
  export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };
}
