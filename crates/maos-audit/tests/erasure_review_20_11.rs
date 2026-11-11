#![forbid(unsafe_code)]

use ed25519_dalek::Signer as _;
use maos_audit::erasure::{merkle, proof::*};
use sha2::{Digest as _, Sha256};

const SEED: [u8; 32] = [29; 32];

fn fixture(seed: &[u8; 32]) -> ErasureProof {
    let pre = [[1; 16], [2; 16]];
    let mut post = pre.to_vec();
    post.extend((100..116).map(|id| [id; 16]));
    post.extend([[42; 16], [199; 16]]);
    let observations = REQUIRED_CATEGORIES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let removed = matches!(*name, "intent_lineage" | "principal_frames");
            CategoryObservation::new(
                ErasureCategory {
                    name: (*name).into(),
                    status: if removed {
                        CategoryStatus::Removed { count: 1 }
                    } else {
                        CategoryStatus::VerifiedEmpty
                    },
                },
                [100 + index as u8; 16],
                if removed {
                    vec![RemovalEvidence::Frame {
                        frame_id: hex::encode(pre[usize::from(*name == "principal_frames")]),
                    }]
                } else {
                    vec![]
                },
            )
        })
        .collect();
    build_erasure_proof(
        "review-spirit".into(),
        7,
        77,
        &pre,
        &post,
        &pre[..1],
        &["subject@example.org".into()],
        &pre[1..],
        observations,
        Some(CompletionObservation {
            frame_id: [199; 16],
            through_rowid: 19,
        }),
        seed,
    )
    .unwrap_or_else(|error| panic!("valid fixture: {error}"))
}

fn sorted(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let ordered: std::collections::BTreeMap<_, _> = map
                .into_iter()
                .map(|(key, value)| (key, sorted(value)))
                .collect();
            serde_json::Value::Object(ordered.into_iter().collect())
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sorted).collect())
        }
        value => value,
    }
}

fn resign(proof: &mut ErasureProof) {
    let mut value =
        serde_json::to_value(&*proof).unwrap_or_else(|error| panic!("proof value: {error}"));
    value
        .as_object_mut()
        .expect("proof object")
        .remove("signature_block");
    let bytes = serde_json::to_vec(&sorted(value))
        .unwrap_or_else(|error| panic!("canonical proof: {error}"));
    proof.signature_block.signature = hex::encode(
        ed25519_dalek::SigningKey::from_bytes(&SEED)
            .sign(&Sha256::digest(bytes))
            .to_bytes(),
    );
}

fn public_key() -> [u8; 32] {
    ed25519_dalek::SigningKey::from_bytes(&SEED)
        .verifying_key()
        .to_bytes()
}

#[test]
fn f5_paired_category_omission_is_rejected() {
    let mut proof = fixture(&SEED);
    proof.categories.remove(0);
    proof.category_evidence.remove(0);
    resign(&mut proof);
    assert!(
        verify_erasure_proof(&proof, &public_key()).is_err(),
        "omitted mandatory category verified"
    );
}

#[test]
fn f6_empty_tree_exclusion_cannot_be_used_as_pre_inclusion() {
    let mut proof = fixture(&SEED);
    let empty = merkle::build_tree_from_frame_ids(&[]);
    proof.pre_leaves.clear();
    proof.pre_root = empty.root;
    let sentinel =
        merkle::prove_exclusion(&empty, merkle::hash_leaf(&[1; 16])).expect("empty exclusion");
    for frame in proof
        .erased_frame_proofs
        .iter_mut()
        .chain(&mut proof.redacted_principal_frame_proofs)
    {
        frame.pre_inclusion = sentinel.clone();
    }
    resign(&mut proof);
    assert!(
        verify_erasure_proof(&proof, &public_key()).is_err(),
        "empty-tree inclusion forgery verified"
    );
}

#[test]
fn f11_missing_subject_exclusion_is_rejected() {
    let mut proof = fixture(&SEED);
    proof.subject_exclusion_proofs.clear();
    resign(&mut proof);
    assert!(
        verify_erasure_proof(&proof, &public_key()).is_err(),
        "missing subject exclusion verified"
    );
}

#[test]
fn f12_malformed_leaf_is_rejected_not_silently_dropped() {
    let mut proof = fixture(&SEED);
    proof.pre_leaves.push("malformed".into());
    resign(&mut proof);
    assert!(
        verify_erasure_proof(&proof, &public_key()).is_err(),
        "malformed leaf was discarded"
    );
}

#[test]
fn f4_removed_count_is_bound_to_real_removal_evidence() {
    let mut proof = fixture(&SEED);
    let index = proof
        .categories
        .iter()
        .position(|category| category.name == "intent_lineage")
        .expect("lineage");
    proof.categories[index].status = CategoryStatus::Removed { count: 2 };
    resign(&mut proof);
    assert!(
        verify_erasure_proof(&proof, &public_key()).is_err(),
        "unsupported removed count verified"
    );
}

#[test]
fn f2_subject_commitment_depends_on_secret_signer_key() {
    let first = fixture(&SEED);
    let second = fixture(&[31; 32]);
    assert_ne!(
        first.subject_exclusion_proofs[0].leaf, second.subject_exclusion_proofs[0].leaf,
        "publicly guessable subject commitment is unchanged across signing keys"
    );
}

#[cfg(unix)]
#[test]
fn f2_retained_bundle_is_owner_only() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().expect("retention");
    let path = write_proof_bundle(&fixture(&SEED), dir.path()).expect("persist proof");
    assert_eq!(
        std::fs::metadata(path)
            .expect("proof metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn f9_existing_bundle_is_never_overwritten() {
    let dir = tempfile::tempdir().expect("retention");
    let proof = fixture(&SEED);
    let path = write_proof_bundle(&proof, dir.path()).expect("first proof");
    let before = std::fs::read(&path).expect("first bytes");
    assert!(
        write_proof_bundle(&proof, dir.path()).is_err(),
        "same-name proof silently overwritten"
    );
    assert_eq!(std::fs::read(path).expect("retained bytes"), before);
}

#[cfg(unix)]
#[test]
fn f9_predictable_temp_symlink_cannot_redirect_the_write() {
    let dir = tempfile::tempdir().expect("retention");
    let proof = fixture(&SEED);
    let filename = format!(
        "review-spirit-77-{}.bundle",
        hex::encode(&proof.post_root[..8])
    );
    let victim = dir.path().join("unrelated");
    std::fs::write(&victim, b"must remain").expect("victim");
    std::os::unix::fs::symlink(&victim, dir.path().join(format!(".{filename}.tmp")))
        .expect("planted symlink");
    let _ = write_proof_bundle(&proof, dir.path());
    assert_eq!(std::fs::read(victim).expect("victim bytes"), b"must remain");
}

#[test]
fn p3_p4_f4_f5_f6_f11_f12_both_verifiers_enforce_the_same_signed_contract() {
    use std::io::Write as _;
    use std::process::{Command, Stdio};
    const MUTATE_AND_SIGN: &str = r#"
import copy, hashlib, json, sys
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
bundle = json.load(sys.stdin)
case = sys.argv[1]
index = next(i for i,c in enumerate(bundle["categories"]) if c["name"] == "intent_lineage")
category = bundle["categories"][index]
if case in ("negative", "overflow", "fractional", "boolean", "removed_count"):
    category["status"]["count"] = {"negative":-1,"overflow":2**64,"fractional":1.5,"boolean":True,"removed_count":2}[case]
elif case == "unknown_tag":
    category["status"]["status"] = "Unsupported"
elif case == "missing_tag":
    del category["status"]["status"]
elif case == "missing_status":
    del category["status"]
elif case == "wrong_name":
    category["name"] = True
elif case == "wrong_reason":
    category["status"] = {"status":"CoverageGap","reason":123}
elif case == "extra_category":
    category["unexpected"] = True
if case in ("weighted_memory_valid", "weighted_memory_mismatch"):
    memory = next(i for i,c in enumerate(bundle["categories"]) if c["name"] == "memory_namespace")
    bundle["categories"][memory]["status"] = {"status":"Removed","count":3 if case == "weighted_memory_valid" else 4}
    bundle["category_evidence"][memory]["removals"] = [{"kind":"memory_namespace_batch","receipt_frame_id":("2a"*16),"deleted_entries":3}]
elif case in ("weighted_capability_valid", "duplicate_batch", "batch_overflow"):
    capability = next(i for i,c in enumerate(bundle["categories"]) if c["name"] == "capability_tokens")
    count = 2 if case == "weighted_capability_valid" else 4 if case == "duplicate_batch" else 2**64-1
    bundle["categories"][capability]["status"] = {"status":"Removed","count":count}
    batch = {"kind":"capability_batch","boot_nonce":2011,"spirit_pid":7,"revoked_tokens":count if case == "batch_overflow" else 2}
    bundle["category_evidence"][capability]["removals"] = [batch]
    if case == "duplicate_batch":
        bundle["category_evidence"][capability]["removals"].append(copy.deepcopy(batch))
    elif case == "batch_overflow":
        bundle["category_evidence"][capability]["removals"].append({"kind":"capability_batch","boot_nonce":2011,"spirit_pid":8,"revoked_tokens":1})
elif case in ("halt_valid", "halt_missing_receipt", "halt_duplicate", "halt_wrong_kind"):
    halts = next(i for i,c in enumerate(bundle["categories"]) if c["name"] == "pending_halts")
    item = {"kind":"halt","halt_id":"halt-20-11","receipt_frame_id":("ee" if case == "halt_missing_receipt" else "2a")*16}
    count = 2 if case == "halt_duplicate" else 1
    bundle["categories"][halts]["status"] = {"status":"Removed","count":count}
    bundle["category_evidence"][halts]["removals"] = [{"kind":"schedule","schedule_id":"daily"}] if case == "halt_wrong_kind" else [copy.deepcopy(item) for _ in range(count)]
elif case == "missing_removal":
    bundle["category_evidence"][index]["removals"] = []
elif case == "duplicate_removal":
    bundle["category_evidence"][index]["removals"] *= 2
elif case == "wrong_removal_kind":
    bundle["category_evidence"][index]["removals"] = [{"kind":"schedule","schedule_id":"daily"}]
elif case == "cross_kind_duplicate":
    principal=next(i for i,c in enumerate(bundle["categories"]) if c["name"]=="principal_frames")
    bundle["redacted_principal_frame_proofs"]=copy.deepcopy(bundle["erased_frame_proofs"])
    bundle["category_evidence"][principal]["removals"]=copy.deepcopy(bundle["category_evidence"][index]["removals"])
elif case == "subject_count_zero":
    bundle["subject_count"] = 0
    bundle["subject_exclusion_proofs"] = []
elif case == "missing_marker":
    bundle["frame_sweep_completion"] = None
elif case == "borrowed_category_marker":
    observed = bundle["category_evidence"][index]
    bundle["frame_sweep_completion"]["frame_id"] = observed["frame_id"]
    bundle["frame_sweep_completion"]["post_inclusion"] = copy.deepcopy(observed["post_inclusion"])
elif case == "missing_nullable":
    del bundle["erased_frame_proofs"][0]["pre_inclusion"]["adjacent_leaf"]
elif case == "extra_merkle":
    bundle["erased_frame_proofs"][0]["pre_inclusion"]["unexpected"] = True
elif case == "duplicate_leaf":
    bundle["post_leaves"].append(bundle["post_leaves"][0])
elif case == "frame_junk":
    bundle["category_evidence"][index]["frame_id"] = "junk-" + bundle["category_evidence"][index]["frame_id"]
elif case == "extra_evidence":
    bundle["category_evidence"][index]["unexpected"] = True
elif case == "paired_omission":
    del bundle["categories"][0]
    del bundle["category_evidence"][0]
elif case == "empty_categories":
    bundle["categories"] = []
    bundle["category_evidence"] = []
elif case == "missing_subject":
    bundle["subject_exclusion_proofs"] = []
elif case == "malformed_leaf":
    bundle["pre_leaves"].append("malformed")
elif case == "empty_pre_inclusion":
    bundle["pre_leaves"] = []
    bundle["pre_root"] = list(hashlib.sha256(b"maos.erasure.empty-tree").digest())
    for entry in bundle["erased_frame_proofs"] + bundle["redacted_principal_frame_proofs"]:
        fid = bytes.fromhex(entry["frame_id"].replace(":",""))
        entry["pre_inclusion"] = {"leaf":list(hashlib.sha256(fid).digest()),"adjacent_leaf":None,"leaf_index":0,"siblings":[],"directions":[]}
elif case == "hex_upper":
    bundle["category_evidence"][index]["frame_id"] = bundle["category_evidence"][index]["frame_id"].upper()
elif case == "hex_plain":
    bundle["category_evidence"][index]["frame_id"] = bundle["category_evidence"][index]["frame_id"].replace(":","")
elif case in ("unicode", "wrong_identity"):
    identity = "review-étoile" if case == "unicode" else False
    bundle["spirit_id"] = identity
    for entry in bundle["category_evidence"]:
        entry["spirit_id"] = identity
elif case == "valid_gap":
    category["status"] = {"status":"CoverageGap","reason":"retained by the declared fixture"}
    bundle["frame_sweep_completion"] = None
unsigned = {k:v for k,v in bundle.items() if k != "signature_block"}
canonical = json.dumps(unsigned, sort_keys=True, separators=(",",":"), ensure_ascii=False).encode("utf-8")
key = Ed25519PrivateKey.from_private_bytes(bytes([29])*32)
bundle["signature_block"]["signature"] = key.sign(hashlib.sha256(canonical).digest()).hex()
encoded = json.dumps(bundle, sort_keys=True, separators=(",",":"), ensure_ascii=False)
if case == "duplicate_keys":
    encoded = encoded.replace('"spirit_id":','"spirit_id":"wrong","spirit_id":',1)
sys.stdout.buffer.write(encoded.encode("utf-8"))
"#;
    let base =
        serde_json::to_vec(&fixture(&SEED)).unwrap_or_else(|error| panic!("fixture JSON: {error}"));
    let verifier = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/verify-erasure/verify.py");
    let dir = tempfile::tempdir().expect("signed corpus");
    let cases = [
        ("valid_removed", true),
        ("valid_gap", true),
        ("negative", false),
        ("overflow", false),
        ("fractional", false),
        ("boolean", false),
        ("unknown_tag", false),
        ("missing_tag", false),
        ("missing_status", false),
        ("wrong_name", false),
        ("wrong_reason", false),
        ("wrong_identity", false),
        ("extra_category", false),
        ("extra_evidence", false),
        ("paired_omission", false),
        ("empty_categories", false),
        ("missing_subject", false),
        ("removed_count", false),
        ("malformed_leaf", false),
        ("empty_pre_inclusion", false),
        ("duplicate_keys", false),
        ("hex_upper", true),
        ("hex_plain", true),
        ("unicode", true),
        ("weighted_memory_valid", true),
        ("weighted_memory_mismatch", false),
        ("weighted_capability_valid", true),
        ("duplicate_batch", false),
        ("halt_valid", true),
        ("halt_missing_receipt", false),
        ("halt_duplicate", false),
        ("halt_wrong_kind", false),
        ("batch_overflow", false),
        ("missing_removal", false),
        ("duplicate_removal", false),
        ("wrong_removal_kind", false),
        ("cross_kind_duplicate", false),
        ("subject_count_zero", false),
        ("missing_marker", false),
        ("borrowed_category_marker", false),
        ("missing_nullable", false),
        ("extra_merkle", false),
        ("duplicate_leaf", false),
        ("frame_junk", false),
    ];
    let mut failures = Vec::new();
    for (case, expected) in cases {
        let mut signer = Command::new("python3")
            .args(["-c", MUTATE_AND_SIGN, case])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("declared Python verifier runner");
        signer
            .stdin
            .take()
            .expect("signer stdin")
            .write_all(&base)
            .expect("fixture input");
        let signed = signer.wait_with_output().expect("signer result");
        assert!(signed.status.success(),
            "declared verifier runner needs tools/verify-erasure/requirements.txt; missing dependency is not a skip: {}",
            String::from_utf8_lossy(&signed.stderr));
        let path = dir.path().join(format!("{case}.json"));
        std::fs::write(&path, &signed.stdout).expect("signed vector");
        let rust = decode_proof_bundle(&signed.stdout)
            .ok()
            .is_some_and(|proof| verify_erasure_proof(&proof, &public_key()).is_ok());
        let python = Command::new("python3")
            .arg(&verifier)
            .arg(&path)
            .arg(hex::encode(public_key()))
            .output()
            .expect("real Python verifier");
        if rust != expected || python.status.success() != expected {
            failures.push(format!(
                "{case}: expected={expected}, Rust={rust}, Python={}, stderr={}",
                python.status.success(),
                String::from_utf8_lossy(&python.stderr)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "signed contract mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn review_p8_whole_uninstall_inventory_proof_and_finalization_run_off_worker() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../maos-bin/src/main.rs"),
    )
    .expect("composition root source");
    let syntax = syn::parse_file(&source).expect("composition root syntax");
    let function = syntax
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == "run_uninstall_cascade" => {
                Some(function)
            }
            _ => None,
        })
        .expect("uninstall entry point");
    assert_eq!(function.block.stmts.len(), 1,
        "inventory, proof construction/publication, and final terminal I/O must all be inside the blocking boundary");
    let syn::Stmt::Expr(syn::Expr::Call(offload), None) = &function.block.stmts[0] else {
        panic!("the complete uninstall entry must be one block_in_place expression");
    };
    let syn::Expr::Path(path) = offload.func.as_ref() else {
        panic!("explicit off-worker boundary");
    };
    assert!(
        path.path.segments.len() == 3
            && path
                .path
                .segments
                .iter()
                .zip(["tokio", "task", "block_in_place"])
                .all(|(segment, expected)| segment.ident == expected),
        "the full cascade must transfer its Tokio worker before synchronous store work"
    );
    assert_eq!(offload.args.len(), 1);
    let syn::Expr::Closure(closure) = &offload.args[0] else {
        panic!("blocking closure");
    };
    let body = match closure.body.as_ref() {
        syn::Expr::Block(block) if block.block.stmts.len() == 1 => match &block.block.stmts[0] {
            syn::Stmt::Expr(expression, None) => expression,
            _ => panic!("only the async driver belongs in the blocking closure"),
        },
        expression => expression,
    };
    let syn::Expr::MethodCall(drive) = body else {
        panic!("drive the cascade on the blocking thread");
    };
    assert_eq!(drive.method, "block_on");
    assert!(
        drive.args.len() == 1 && matches!(drive.args[0], syn::Expr::Async(_)),
        "do not spawn the synchronous inventory/proof future back onto a Tokio worker"
    );
}
