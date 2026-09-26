import type { IacFrame } from "maos:spirit/spirit@1.0.0";

let started = false;

export function handleFrame(frame: IacFrame): IacFrame[] {
  if (!started) {
    throw new Error("q1 handle-frame ran before on-start");
  }

  const frameId: Uint8Array = frame.frameId;
  const timestampNs: bigint = frame.timestampNs;
  console.log(
    `q1 handle-frame id-bytes=${frameId.length} timestamp-ns=${timestampNs}`,
  );
  return [frame];
}

export function onStart(): void {
  started = true;
  console.log("q1 on-start");
}

export function onShutdown(): void {
  console.log("q1 on-shutdown");
}
