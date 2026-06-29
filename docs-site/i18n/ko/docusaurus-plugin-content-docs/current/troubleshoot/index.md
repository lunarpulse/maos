---
title: 트러블슈팅
sidebar_position: 0
description: 에러 코드 참조 — 모든 MAOS 에러의 원인과 해결 방법.
review_status: machine
---

# 트러블슈팅

이 페이지는 모든 MAOS 에러 코드를 상세 원인 및 해결 페이지 링크와 함께 나열합니다.

| 에러 코드 | 심각도 | 복구 | 담당 | 상세 |
|------------|----------|----------|-------|---------|
| `EAbiTooNew` | policy | reject | maos-kernel-core | [EAbiTooNew](/errors/EAbiTooNew) |
| `EAbiTooOld` | policy | reject | maos-kernel-core | [EAbiTooOld](/errors/EAbiTooOld) |
| `EClassRequired` | security | reject | maos-kernel-core | [EClassRequired](/errors/EClassRequired) |
| `ECliBinaryNotFound` | infra | fix_config | maos-domain | [ECliBinaryNotFound](/errors/ECliBinaryNotFound) |
| `ECliProbeFailed` | infra | retry | maos-domain | [ECliProbeFailed](/errors/ECliProbeFailed) |
| `ECliWrapperRequiresT3` | policy | reject | maos-domain | [ECliWrapperRequiresT3](/errors/ECliWrapperRequiresT3) |
| `ECliWrapperTierNotGranted` | security | reject | maos-domain | [ECliWrapperTierNotGranted](/errors/ECliWrapperTierNotGranted) |
| `EComplianceRejection::ContextDrift` | security | reject | maos-compliance | [EComplianceRejection-ContextDrift](/errors/EComplianceRejection::ContextDrift) |
| `EComplianceRejection::ExpiredClaim` | security | reject | maos-compliance | [EComplianceRejection-ExpiredClaim](/errors/EComplianceRejection::ExpiredClaim) |
| `EComplianceRejection::MalformedClaim` | security | reject | maos-compliance | [EComplianceRejection-MalformedClaim](/errors/EComplianceRejection::MalformedClaim) |
| `EComplianceRejection::SignatureInvalid` | security | reject | maos-compliance | [EComplianceRejection-SignatureInvalid](/errors/EComplianceRejection::SignatureInvalid) |
| `EHaltContinuityViolation` | policy | reject | maos-domain | [EHaltContinuityViolation](/errors/EHaltContinuityViolation) |
| `EIntentLineageBroken` | security | reject | maos-domain | [EIntentLineageBroken](/errors/EIntentLineageBroken) |
| `EManifestSchemaConflict` | policy | reject | maos-domain | [EManifestSchemaConflict](/errors/EManifestSchemaConflict) |
| `EMigratorMissing` | policy | reject | maos-domain | [EMigratorMissing](/errors/EMigratorMissing) |
| `EMigratorMissing_PrecheckOutcome` | policy | reject | maos-domain | [EMigratorMissing_PrecheckOutcome](/errors/EMigratorMissing_PrecheckOutcome) |
| `EOrchestratorDispatchRawOutput` | policy | reject | maos-domain | [EOrchestratorDispatchRawOutput](/errors/EOrchestratorDispatchRawOutput) |
| `EOutputShapeAdapterMismatch` | policy | reject | maos-domain | [EOutputShapeAdapterMismatch](/errors/EOutputShapeAdapterMismatch) |
| `EOutputShapeAdapterNotRegistered` | policy | reject | maos-domain | [EOutputShapeAdapterNotRegistered](/errors/EOutputShapeAdapterNotRegistered) |
| `EPinMismatch::Invalidated` | user | escalate | maos-a2a-core | [EPinMismatch-Invalidated](/errors/EPinMismatch::Invalidated) |
| `EPinMismatch::Mismatch` | security | escalate | maos-a2a-core | [EPinMismatch-Mismatch](/errors/EPinMismatch::Mismatch) |
| `EPinMismatch::NotPinned` | user | retry_with_correction | maos-a2a-core | [EPinMismatch-NotPinned](/errors/EPinMismatch::NotPinned) |
| `ERespawnWithContextUnsupported` | policy | reject | maos-domain | [ERespawnWithContextUnsupported](/errors/ERespawnWithContextUnsupported) |
| `ESkillProposal::EmptyDiff` | user | retry_with_correction | maos-skill | [ESkillProposal-EmptyDiff](/errors/ESkillProposal::EmptyDiff) |
| `ESkillProposal::EmptyTargetId` | user | retry_with_correction | maos-skill | [ESkillProposal-EmptyTargetId](/errors/ESkillProposal::EmptyTargetId) |
| `ESkillProposal::InvalidTargetIdCharset` | user | retry_with_correction | maos-skill | [ESkillProposal-InvalidTargetIdCharset](/errors/ESkillProposal::InvalidTargetIdCharset) |
| `ESkillProposal::InvalidTargetVersion` | user | retry_with_correction | maos-skill | [ESkillProposal-InvalidTargetVersion](/errors/ESkillProposal::InvalidTargetVersion) |
| `ESkillQueue::DuplicateSkillId` | user | retry_with_correction | maos-skill | [ESkillQueue-DuplicateSkillId](/errors/ESkillQueue::DuplicateSkillId) |
| `ESkillSchema::EmptyBody` | user | retry_with_correction | maos-skill | [ESkillSchema-EmptyBody](/errors/ESkillSchema::EmptyBody) |
| `ESkillSchema::EmptyId` | user | retry_with_correction | maos-skill | [ESkillSchema-EmptyId](/errors/ESkillSchema::EmptyId) |
| `ESkillSchema::EmptyName` | user | retry_with_correction | maos-skill | [ESkillSchema-EmptyName](/errors/ESkillSchema::EmptyName) |
| `ESkillSchema::InvalidIdCharset` | user | retry_with_correction | maos-skill | [ESkillSchema-InvalidIdCharset](/errors/ESkillSchema::InvalidIdCharset) |
| `ESkillSchema::InvalidSemver` | user | retry_with_correction | maos-skill | [ESkillSchema-InvalidSemver](/errors/ESkillSchema::InvalidSemver) |
| `ESkillSchema::MissingFence` | user | retry_with_correction | maos-skill | [ESkillSchema-MissingFence](/errors/ESkillSchema::MissingFence) |
| `ESkillSchema::TomlParse` | user | retry_with_correction | maos-skill | [ESkillSchema-TomlParse](/errors/ESkillSchema::TomlParse) |
| `ESkillSchema::UnknownField` | user | retry_with_correction | maos-skill | [ESkillSchema-UnknownField](/errors/ESkillSchema::UnknownField) |
| `ESubstrateTooOld` | policy | reject | maos-kernel-core | [ESubstrateTooOld](/errors/ESubstrateTooOld) |
