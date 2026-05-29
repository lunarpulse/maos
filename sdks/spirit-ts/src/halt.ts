// HaltResolutionKind and HaltResolutionRecord mirrors.

export enum HaltResolutionKind {
  ProvidedContext = "ProvidedContext",
  AcceptedHalt = "AcceptedHalt",
  AuthorizedOverride = "AuthorizedOverride",
}

export interface HaltResolutionRecord {
  haltId: string;
  kind: HaltResolutionKind;
}
