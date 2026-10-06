export function halt(reason: string): never {
  throw { tag: "fault", val: reason };
}
