declare module 'maos:spirit/frames@1.0.0' {
  /**
   * ── FrameKind (identity.rs:30-79) ──────────────────────────────────
   * WIT enum order encodes the discriminant; the comment is the on-wire
   * CBOR discriminant value.
   * # Variants
   * 
   * ## `"task-assign"`
   * 
   * ## `"task-complete"`
   * 
   * 0
   * ## `"decision-dispatch"`
   * 
   * 1
   * ## `"epistemic-halt"`
   * 
   * 2
   * ## `"telemetry-event"`
   * 
   * 3
   * ## `"consent-request"`
   * 
   * 4
   * ## `"retract"`
   * 
   * 5
   * ## `"capability-invocation"`
   * 
   * 6
   * ## `"sandbox-block"`
   * 
   * 7
   * ## `"inference-call"`
   * 
   * 8
   * ## `"budget-warning"`
   * 
   * 9
   * ## `"budget-exceeded"`
   * 
   * 12
   * ## `"cli-subprocess-output"`
   * 
   * 13
   * ## `"consent-rupture"`
   * 
   * 21
   * ## `"rate-limited"`
   * 
   * 22
   * ## `"gateway-inbound"`
   * 
   * 23
   * ## `"gateway-outbound"`
   * 
   * 24
   */
  export type FrameKind = 'task-assign' | 'task-complete' | 'decision-dispatch' | 'epistemic-halt' | 'telemetry-event' | 'consent-request' | 'retract' | 'capability-invocation' | 'sandbox-block' | 'inference-call' | 'budget-warning' | 'budget-exceeded' | 'cli-subprocess-output' | 'consent-rupture' | 'rate-limited' | 'gateway-inbound' | 'gateway-outbound';
  /**
   * ── FrameOrigin (i3) ───────────────────────────────────────────────
   * # Variants
   * 
   * ## `"human-authored"`
   * 
   * ## `"spirit-auto"`
   * 
   * ## `"spirit-drafted-human-approved"`
   * 
   * ## `"kernel"`
   */
  export type FrameOrigin = 'human-authored' | 'spirit-auto' | 'spirit-drafted-human-approved' | 'kernel';
  /**
   * ── FrameAddress (frame.rs:54-59) ──────────────────────────────────
   */
  export interface FrameAddress {
    spiritId: string,
    hostId?: string,
    role?: string,
  }
  /**
   * ── PostureHint (frame.rs:166-171) ─────────────────────────────────
   * # Variants
   * 
   * ## `"autonomous-with-halt"`
   * 
   * ## `"assistive"`
   * 
   * ## `"cautious"`
   */
  export type PostureHint = 'autonomous-with-halt' | 'assistive' | 'cautious';
  /**
   * ── HaltPolicyOverride (frame.rs:142-147) ──────────────────────────
   */
  export interface HaltPolicyOverride {
    tag: string,
    recallVsPrecision: number,
  }
  /**
   * ── PosturePreferences (frame.rs:118-130) ──────────────────────────
   */
  export interface PosturePreferences {
    preferredPosture?: PostureHint,
    haltPolicyOverrides: Array<HaltPolicyOverride>,
  }
  /**
   * ── PriorDistillateRef (frame.rs:100-110) ──────────────────────────
   */
  export interface PriorDistillateRef {
    digestFrameId: Uint8Array,
    /**
     * [u8; 16]
     */
    distillationDepth: number,
  }
  /**
   * ── TaskAssignPayload (frame.rs:79-92) ─────────────────────────────
   */
  export interface TaskAssignBody {
    goal: string,
    scope: Array<string>,
    /**
     * Vec<Scope> serialized as strings
     */
    successCriteria: string,
    posturePreferences: PosturePreferences,
    priorDistillateRef?: PriorDistillateRef,
  }
  /**
   * ── TaskCompletePayload (frame.rs:175-177) ─────────────────────────
   */
  export interface TaskCompleteBody {
    resultText: string,
  }
  /**
   * ── DecisionDispatchPayload (frame.rs:181-190) ─────────────────────
   */
  export interface DecisionDispatchBody {
    decisionId: bigint,
    approved: boolean,
  }
  /**
   * ── EpistemicHaltPayload (frame.rs:198-230) ────────────────────────
   */
  export interface EpistemicHaltBody {
    haltId: string,
    tag: string,
    value: number,
    threshold?: number,
    policyId: string,
    derivedFrom: string,
  }
  /**
   * ── TelemetryEventPayload (frame.rs:278-281) ───────────────────────
   */
  export interface TelemetryEventBody {
    eventType: string,
    data: string,
  }
  /**
   * ── ConsentRequestPayload (frame.rs:285-287) ───────────────────────
   */
  export interface ConsentRequestBody {
    capability: string,
  }
  /**
   * ── RetractPayload (frame.rs:296-308) ──────────────────────────────
   */
  export interface RetractBody {
    originalFrameId: Uint8Array,
    /**
     * [u8; 16]
     */
    reason: string,
    originalKind?: FrameKind,
  }
  /**
   * ── RuptureReason (frame.rs:370-382) ───────────────────────────────
   * # Variants
   * 
   * ## `"intent-allowlist-mismatch"`
   * 
   * ## `"posture-shifted-during-transmission"`
   * 
   * ## `"token-revoked"`
   * 
   * ## `"principal-revoked"`
   * 
   * ## `"recipient-unloaded"`
   */
  export type RuptureReason = 'intent-allowlist-mismatch' | 'posture-shifted-during-transmission' | 'token-revoked' | 'principal-revoked' | 'recipient-unloaded';
  /**
   * ── RuptureRejection (frame.rs:360-363) ────────────────────────────
   */
  export interface RuptureRejection {
    address: FrameAddress,
    reason: RuptureReason,
  }
  /**
   * ── ConsentRupturePayload (frame.rs:345-356) ───────────────────────
   */
  export interface ConsentRuptureBody {
    ruptureId: Uint8Array,
    /**
     * [u8; 16]
     */
    originalFrameId: Uint8Array,
    /**
     * [u8; 16]
     */
    originalKind: FrameKind,
    accepted: Array<FrameAddress>,
    rejected: Array<RuptureRejection>,
    rupturedAtNs: bigint,
  }
  /**
   * ── RateLimitedPayload (frame.rs:386-400) ──────────────────────────
   */
  export interface RateLimitedBody {
    providerId: string,
    credentialFingerprintPrefixHex: string,
    retryAfterMs: bigint,
    bucketRemaining: number,
    bucketCapacity: number,
    refillPerSec: number,
    scheduleId?: string,
  }
  /**
   * ── BudgetEnvelope (frame.rs) ─────────────────────────────────────
   */
  export interface BudgetEnvelope {
    spiritPid: number,
    hookName: string,
    wallNs: bigint,
    capSeconds: bigint,
  }
  /**
   * ── FramePayload (frame.rs:63-75) ──────────────────────────────────
   * Discriminated union — one variant per routed FrameKind.
   */
  export type FramePayload = FramePayloadTaskAssign | FramePayloadTaskComplete | FramePayloadDecisionDispatch | FramePayloadEpistemicHalt | FramePayloadTelemetryEvent | FramePayloadConsentRequest | FramePayloadRetract | FramePayloadBudgetWarning | FramePayloadBudgetExceeded | FramePayloadConsentRupture | FramePayloadRateLimited;
  export interface FramePayloadTaskAssign {
    tag: 'task-assign',
    val: TaskAssignBody,
  }
  export interface FramePayloadTaskComplete {
    tag: 'task-complete',
    val: TaskCompleteBody,
  }
  export interface FramePayloadDecisionDispatch {
    tag: 'decision-dispatch',
    val: DecisionDispatchBody,
  }
  export interface FramePayloadEpistemicHalt {
    tag: 'epistemic-halt',
    val: EpistemicHaltBody,
  }
  export interface FramePayloadTelemetryEvent {
    tag: 'telemetry-event',
    val: TelemetryEventBody,
  }
  export interface FramePayloadConsentRequest {
    tag: 'consent-request',
    val: ConsentRequestBody,
  }
  export interface FramePayloadRetract {
    tag: 'retract',
    val: RetractBody,
  }
  export interface FramePayloadBudgetWarning {
    tag: 'budget-warning',
    val: BudgetEnvelope,
  }
  export interface FramePayloadBudgetExceeded {
    tag: 'budget-exceeded',
    val: BudgetEnvelope,
  }
  export interface FramePayloadConsentRupture {
    tag: 'consent-rupture',
    val: ConsentRuptureBody,
  }
  export interface FramePayloadRateLimited {
    tag: 'rate-limited',
    val: RateLimitedBody,
  }
  /**
   * ── IacFrame envelope (frame.rs:26-51) ─────────────────────────────
   */
  export interface IacFrame {
    frameId: Uint8Array,
    /**
     * [u8; 16]
     */
    timestampNs: bigint,
    logicalClock: bigint,
    frameFrom: FrameAddress,
    to: Array<FrameAddress>,
    kind: FrameKind,
    payload: FramePayload,
    autoMarker: FrameOrigin,
  }
  /**
   * ── Halt signal ────────────────────────────────────────────────────
   * Returned by the guest (ADR-032: EOF after clean frame =
   * Halt::Voluntary; mid-frame EOF = Halt::Fault(Truncated)).
   */
  export type Halt = HaltVoluntary | HaltFault;
  export interface HaltVoluntary {
    tag: 'voluntary',
  }
  export interface HaltFault {
    tag: 'fault',
    val: string,
  }
}
