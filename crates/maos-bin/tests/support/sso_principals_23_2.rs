// Story 23-2 — offline OIDC fixtures for three principals, signed by ONE throwaway RSA-2048 key
// (`key-p`) generated for this story with `openssl genrsa`; the private key was not kept. Every
// token is a real RS256 JWS (PKCS#1 v1.5 over `<header>.<claims>`), iss = `ISS`, aud = `AUD`,
// `exp` = 9999999999, `nbf` = 0. No network, no IdP. The three differ only in `sub`.

#![allow(dead_code)]

pub const ISS: &str = "https://idp.maos.example";
pub const AUD: &str = "maos-deploy-alpha";

pub const JWKS: &str = "{\"keys\":[{\"kty\":\"RSA\",\"use\":\"sig\",\"alg\":\"RS256\",\"kid\":\"key-p\",\"n\":\"5sELQZZEchNMHggmixDHX5v7u8faxKTWpT-NoD2N2_YTqGGp6ru4pwlA59HvUv3WMeKy1pEoOuCzQoNjpdGeivIMXcZvLHapkFkquM-OwJrc0NWpd9dd93HXbFKYKDentGJ7UhxWwR2zEptoScqsSz2Obh0QNM7wAkzsa4qU_1ek14DoeNIJaNVOy_BGdjRhdbm2Cl8XRGeVcfQ2ahy6pTdYtJJbhY4H-1buZV8R6NfIhkpFdHeEwNkSEPpVQXR0LeEFKXzyATEPkynjhIzdX3D48Fq-IzFEexDJ847mVD-VfIWqpI4uISJjKKJfyeJEtEm2ZF65oqh58yt8nZXAxw\",\"e\":\"AQAB\"}]}";

/// `sub` = `env-principal@maos.example`: what `MAOS_SSO_ASSERTION` holds.
pub const SUB_ENV: &str = "env-principal@maos.example";
pub const TOKEN_ENV: &str = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImtleS1wIn0.eyJzdWIiOiJlbnYtcHJpbmNpcGFsQG1hb3MuZXhhbXBsZSIsImlzcyI6Imh0dHBzOi8vaWRwLm1hb3MuZXhhbXBsZSIsImF1ZCI6Im1hb3MtZGVwbG95LWFscGhhIiwiZXhwIjo5OTk5OTk5OTk5LCJuYmYiOjAsImlhdCI6MTcwMDAwMDAwMH0.cxU1I_tJDYlXA7Zc_cphthlOihCcIIojx1k0zWhfpsRVfhVCYf2sd8oH2AlhgvDzBu7JQVctluFf3TODbmnQFjBgh_pbqJgmxVdAlqD7ePhmIcYGuYE-04sbuB2QqisnDlL-KT2T04uB4zprrSInRmOrITVntFtTs6JXVI_qRnGhhJ_2Thm_0YX6QhFpzzNjpCZvg82YZUS7T5mAZVgd214qeAL4P444ZD8dm1N2jhWxEGY78PpVi7glvATt0wEqBRYI5ccYZ19igDZvaJFBmG8cpHOPcV0oCru5qsMMbP0I4NbLDsRtwFR0ID0f02orgDNq_GCAvf41nbatgP_vCA";

/// `sub` = `alice@maos.example`: the asking principal of a task.
pub const SUB_ALICE: &str = "alice@maos.example";
pub const TOKEN_ALICE: &str = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImtleS1wIn0.eyJzdWIiOiJhbGljZUBtYW9zLmV4YW1wbGUiLCJpc3MiOiJodHRwczovL2lkcC5tYW9zLmV4YW1wbGUiLCJhdWQiOiJtYW9zLWRlcGxveS1hbHBoYSIsImV4cCI6OTk5OTk5OTk5OSwibmJmIjowLCJpYXQiOjE3MDAwMDAwMDB9.FhLU7PRYoXAy5zHj7L62BpOmo_4acNiPfTLKuhOUc8unY4NsPQwzvwkVIpMxzDE-_JHtgEaNc-RurDvRAu48S8KRNtProwdv5s6n5m9CN1TyMmxTcrUmT5UEprjQ_NDhEsmGHS9ZNqOsohN1IatO75H5ZvacTzonXLX0xHM4qD1I7nEMLE2IEdFWfFgCmn6qWGQyrE7co3I3W5Yq_5IATJEQfTwRvF2FKuO2tHtV2ZhjAgoavE09yB2ZVHUyYLZRyVwCMpHtymKWMTUzpSgDapejBsSJ9y9VsZE2xPtLy1nVmNh1V5C2t16mpc-UALQIMXxncKTJv7-p9V_JgOghzQ";

/// `sub` = `bob@maos.example`: a second asking principal (replay with a different assertion).
pub const SUB_BOB: &str = "bob@maos.example";
pub const TOKEN_BOB: &str = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImtleS1wIn0.eyJzdWIiOiJib2JAbWFvcy5leGFtcGxlIiwiaXNzIjoiaHR0cHM6Ly9pZHAubWFvcy5leGFtcGxlIiwiYXVkIjoibWFvcy1kZXBsb3ktYWxwaGEiLCJleHAiOjk5OTk5OTk5OTksIm5iZiI6MCwiaWF0IjoxNzAwMDAwMDAwfQ.M9JnRopLmcCspPd6l8yRXQukx7Zz9mUnldh2YpFV5qLnbIStgBfL-ZXTDgGTEUmP8TTARcbU3DsFbKuLcApo5RNo5gKh2M45ZUJTCnNz-3bwlIqq7ForghW4fsmDGaNLAHBCPkxcDmMgAPspd0rlBC0nnXJaN8P0HOS-MjWMlyOKVbpwGQcwKWo7ns1jiXTYdrOFvr0lPCV9ztujWfgA0FagGO-NpSoRhGe_ClqDQ0sAvPfDqXcsfbw6WZnU8QVMFypAy6rXzxMU8HbHJ12r5_T8l_7xiSW9plDl-ogi9PHhcoDYNCnR1dji-ZMlCWeu2HCjUF985Db60ixSgdeaHQ";

/// A well-formed compact JWT that no key signed: refused at accept.
pub const TOKEN_FORGED: &str = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImtleS1wIn0.eyJzdWIiOiJtYWxsb3J5QG1hb3MuZXhhbXBsZSIsImlzcyI6Imh0dHBzOi8vaWRwLm1hb3MuZXhhbXBsZSIsImF1ZCI6Im1hb3MtZGVwbG95LWFscGhhIiwiZXhwIjo5OTk5OTk5OTk5LCJuYmYiOjB9.AAAA";
