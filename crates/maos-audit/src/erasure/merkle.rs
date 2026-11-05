//! Story 9.2 — small binary Merkle tree over sorted Transparency-Log frame_id hashes.
//!
//! Leaves are `sha256(frame_id)`; the tree is built over a sorted leaf set so
//! exclusion proofs are simple adjacent-leaf proofs.  No external Merkle crate.

#![forbid(unsafe_code)]

use sha2::{Digest, Sha256};

/// 32-byte leaf / node hash.
pub type NodeHash = [u8; 32];

/// Inclusion/exclusion proof path through a binary Merkle tree.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerkleProof {
    /// The leaf hash the proof is for (excluded leaf for exclusion proofs).
    pub leaf: NodeHash,
    /// For exclusion proofs, the adjacent in-tree leaf used to recompute the root.
    #[serde(deserialize_with = "required_option")]
    pub adjacent_leaf: Option<NodeHash>,
    /// Index of the leaf in the sorted leaf vector.
    pub leaf_index: usize,
    /// Sibling hashes from leaf to root (root excluded).
    pub siblings: Vec<NodeHash>,
    /// `false` = this leaf is the left child at the level, `true` = right child.
    pub directions: Vec<bool>,
}

pub(super) fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
}

/// Small binary Merkle tree built over a sorted leaf set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleTree {
    /// Sorted leaf hashes.
    pub leaves: Vec<NodeHash>,
    /// Layer 0 = leaves, last layer = root.
    pub layers: Vec<Vec<NodeHash>>,
    /// Merkle root.
    pub root: NodeHash,
}

/// Hash a raw 16-byte TL frame_id into a leaf.
pub fn hash_leaf(frame_id: &[u8; 16]) -> NodeHash {
    let mut hasher = Sha256::new();
    hasher.update(frame_id);
    hasher.finalize().into()
}

/// Hash two child nodes into a parent.
fn hash_pair(left: &NodeHash, right: &NodeHash) -> NodeHash {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

fn empty_root() -> NodeHash {
    let mut hasher = Sha256::new();
    hasher.update(b"maos.erasure.empty-tree");
    hasher.finalize().into()
}

/// Build a Merkle tree from an unsorted leaf set.  Leaves are sorted before
/// tree construction so adjacent-leaf exclusion proofs are well-defined.
pub fn build_tree(leaves: &[NodeHash]) -> MerkleTree {
    let mut leaves = leaves.to_vec();
    leaves.sort_unstable();
    leaves.dedup();

    if leaves.is_empty() {
        return MerkleTree {
            leaves: Vec::new(),
            layers: vec![vec![empty_root()]],
            root: empty_root(),
        };
    }

    let mut layers: Vec<Vec<NodeHash>> = vec![leaves.clone()];
    while layers.last().unwrap().len() > 1 {
        let current = layers.last().unwrap();
        let mut next = Vec::with_capacity((current.len() + 1) / 2);
        for pair in current.chunks(2) {
            if pair.len() == 2 {
                next.push(hash_pair(&pair[0], &pair[1]));
            } else {
                // Odd number of leaves: duplicate the last one.
                next.push(hash_pair(&pair[0], &pair[0]));
            }
        }
        layers.push(next);
    }

    let root = layers.last().unwrap()[0];
    MerkleTree {
        leaves,
        layers,
        root,
    }
}

/// Build a tree directly from raw frame_ids.
pub fn build_tree_from_frame_ids(frame_ids: &[[u8; 16]]) -> MerkleTree {
    let leaves: Vec<NodeHash> = frame_ids.iter().map(hash_leaf).collect();
    build_tree_from_leaves(&leaves)
}

/// Build a tree from pre-hashed leaves (e.g. KV-row canonical hashes).
///
/// Story 11.2a (AC3): generalized from `build_tree_from_frame_ids` which
/// becomes a 2-line wrapper. The input leaves are ALREADY SHA-256 hashes
/// (e.g. from `CollectiveKvLeaf::canonical_hash`), so no re-hashing is
/// needed — they are passed directly as sorted leaf nodes.
///
/// Preserves 9.x `AuditBundle` byte-identity: `build_tree_from_frame_ids`
/// remains a wrapper that hashes raw 16-byte frame_ids into leaves first.
pub fn build_tree_from_leaves(leaf_hashes: &[NodeHash]) -> MerkleTree {
    build_tree(leaf_hashes)
}

/// Prove that `leaf` is included in the tree.  Returns `None` if the leaf is
/// not present.
pub fn prove_inclusion(tree: &MerkleTree, leaf: NodeHash) -> Option<MerkleProof> {
    let leaf_index = tree.leaves.binary_search(&leaf).ok()?;
    let mut siblings = Vec::new();
    let mut directions = Vec::new();
    let mut idx = leaf_index;

    for layer in &tree.layers {
        if layer.len() <= 1 {
            break;
        }
        let is_right = idx % 2 == 1;
        let sibling_idx = if is_right { idx - 1 } else { idx + 1 };
        directions.push(is_right);
        if sibling_idx < layer.len() {
            siblings.push(layer[sibling_idx]);
        } else {
            // Odd-length layer: sibling is the duplicated last leaf.
            siblings.push(layer[idx]);
        }
        idx /= 2;
    }

    Some(MerkleProof {
        leaf,
        adjacent_leaf: None,
        leaf_index,
        siblings,
        directions,
    })
}

/// Prove that `leaf` is NOT in the tree.  The proof is the inclusion proof
/// for the leaf immediately before the insertion point of `leaf` in the
/// sorted set, with the claimed excluded leaf recorded separately.
/// Verifiers recompute the root from the adjacent leaf and confirm the
/// claimed leaf is absent from the leaf set.
pub fn prove_exclusion(tree: &MerkleTree, leaf: NodeHash) -> Option<MerkleProof> {
    if tree.leaves.is_empty() {
        // Empty tree: any non-empty leaf is excluded.  Return a sentinel proof.
        return Some(MerkleProof {
            leaf,
            adjacent_leaf: None,
            leaf_index: 0,
            siblings: vec![],
            directions: vec![],
        });
    }

    let pos = match tree.leaves.binary_search(&leaf) {
        Ok(_) => return None, // leaf is present, cannot prove exclusion
        Err(p) => p,
    };

    let adjacent_index = if pos == 0 { 0 } else { pos - 1 };
    let adjacent_leaf = tree.leaves[adjacent_index];
    prove_inclusion(tree, adjacent_leaf).map(|mut proof| {
        proof.leaf = leaf;
        proof.adjacent_leaf = Some(adjacent_leaf);
        proof
    })
}

/// Verify an inclusion only in a nonempty tree, with its exact sorted index.
/// The empty-tree exclusion sentinel is never an inclusion proof.
pub fn verify_inclusion_proof(tree: &MerkleTree, leaf: NodeHash, proof: &MerkleProof) -> bool {
    proof.leaf == leaf
        && proof.adjacent_leaf.is_none()
        && tree.leaves.get(proof.leaf_index) == Some(&leaf)
        && verify_path(tree, leaf, proof)
}

/// Verify exclusion against the declared complete leaf set and its canonical
/// adjacent leaf. Empty-tree exclusion has a separate, unambiguous context.
pub fn verify_exclusion_proof(tree: &MerkleTree, proof: &MerkleProof) -> bool {
    if tree.leaves.is_empty() {
        return tree.root == empty_root()
            && proof.adjacent_leaf.is_none()
            && proof.leaf_index == 0
            && proof.siblings.is_empty()
            && proof.directions.is_empty();
    }
    let Err(position) = tree.leaves.binary_search(&proof.leaf) else {
        return false;
    };
    let adjacent_index = position.saturating_sub(1);
    let adjacent = tree.leaves[adjacent_index];
    proof.leaf_index == adjacent_index
        && proof.adjacent_leaf == Some(adjacent)
        && verify_path(tree, adjacent, proof)
}

fn verify_path(tree: &MerkleTree, leaf: NodeHash, proof: &MerkleProof) -> bool {
    if tree.leaves.is_empty()
        || proof.leaf_index >= tree.leaves.len()
        || proof.siblings.len() != proof.directions.len()
        || proof.siblings.len() != (usize::BITS - (tree.leaves.len() - 1).leading_zeros()) as usize
    {
        return false;
    }
    fold_path(tree.root, leaf, proof)
}

/// Membership against a trusted root when the application has no complete leaf
/// set (collective attestations). This is NEVER an exclusion verifier.
pub fn verify_inclusion_path(root: NodeHash, leaf: NodeHash, proof: &MerkleProof) -> bool {
    root != empty_root()
        && proof.leaf == leaf
        && proof.adjacent_leaf.is_none()
        && fold_path(root, leaf, proof)
}

fn fold_path(root: NodeHash, leaf: NodeHash, proof: &MerkleProof) -> bool {
    if proof.siblings.len() != proof.directions.len() || proof.siblings.len() > usize::BITS as usize
    {
        return false;
    }
    let mut index = proof.leaf_index;
    let mut current = leaf;
    for (sibling, is_right) in proof.siblings.iter().zip(&proof.directions) {
        if *is_right != (index & 1 == 1) {
            return false;
        }
        current = if *is_right {
            hash_pair(sibling, &current)
        } else {
            hash_pair(&current, sibling)
        };
        index /= 2;
    }
    index == 0 && current == root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inclusion_and_exclusion_round_trip() {
        let ids: Vec<[u8; 16]> = (0u8..8).map(|i| [i; 16]).collect();
        let tree = build_tree_from_frame_ids(&ids);
        let leaf = hash_leaf(&ids[3]);
        let proof = prove_inclusion(&tree, leaf).unwrap();
        assert!(verify_inclusion_proof(&tree, leaf, &proof));

        let missing: [u8; 16] = [255; 16];
        let excl = prove_exclusion(&tree, hash_leaf(&missing)).unwrap();
        assert!(verify_exclusion_proof(&tree, &excl));
        assert_ne!(excl.adjacent_leaf, Some(excl.leaf));
    }

    #[test]
    fn empty_tree_exclusion() {
        let tree = build_tree_from_frame_ids(&[]);
        let leaf = hash_leaf(&[1u8; 16]);
        let proof = prove_exclusion(&tree, leaf).unwrap();
        assert!(verify_exclusion_proof(&tree, &proof));
    }
    #[test]
    fn single_leaf_tree_inclusion_verifies() {
        // Regression: a single-leaf tree's inclusion proof has empty siblings;
        // the leaf IS the root and must verify.
        let ids: Vec<[u8; 16]> = vec![[7u8; 16]];
        let tree = build_tree_from_frame_ids(&ids);
        assert_eq!(tree.leaves.len(), 1);
        let leaf = hash_leaf(&ids[0]);
        assert_eq!(tree.root, leaf);
        let proof = prove_inclusion(&tree, leaf).expect("single-leaf inclusion proof");
        assert!(proof.siblings.is_empty());
        assert!(
            verify_inclusion_proof(&tree, leaf, &proof),
            "single-leaf inclusion MUST verify (leaf == root)"
        );
    }

    #[test]
    fn single_leaf_tree_exclusion_verifies() {
        // A leaf absent from a single-leaf tree must be provably excluded.
        let ids: Vec<[u8; 16]> = vec![[7u8; 16]];
        let tree = build_tree_from_frame_ids(&ids);
        let absent = hash_leaf(&[99u8; 16]);
        let excl = prove_exclusion(&tree, absent).expect("single-leaf exclusion proof");
        assert!(verify_exclusion_proof(&tree, &excl));
    }

    #[test]
    fn two_and_three_leaf_round_trips() {
        for n in [2usize, 3, 5, 7, 9, 16] {
            let ids: Vec<[u8; 16]> = (0u8..n as u8).map(|i| [i; 16]).collect();
            let tree = build_tree_from_frame_ids(&ids);
            // Inclusion of every member.
            for id in &ids {
                let leaf = hash_leaf(id);
                let proof = prove_inclusion(&tree, leaf).unwrap();
                assert!(
                    verify_inclusion_proof(&tree, leaf, &proof),
                    "{n}-leaf inclusion {id:?}"
                );
                assert!(verify_inclusion_path(tree.root, leaf, &proof));
            }
            // Exclusion of a non-member.
            let absent = hash_leaf(&[200u8; 16]);
            let excl = prove_exclusion(&tree, absent).unwrap();
            assert!(verify_exclusion_proof(&tree, &excl), "{n}-leaf exclusion");
        }
    }

    #[test]
    fn duplicate_frame_ids_dedup() {
        // Duplicate leaves must be deduplicated so adjacent-leaf exclusion
        // proofs remain well-defined.
        let ids: Vec<[u8; 16]> = vec![[3u8; 16], [3u8; 16], [3u8; 16], [9u8; 16]];
        let tree = build_tree_from_frame_ids(&ids);
        // Only two unique leaves survive dedup.
        assert_eq!(tree.leaves.len(), 2);
        let leaf = hash_leaf(&[3u8; 16]);
        let proof = prove_inclusion(&tree, leaf).unwrap();
        assert!(verify_inclusion_proof(&tree, leaf, &proof));
    }

    #[test]
    fn tampered_proof_is_rejected() {
        let ids: Vec<[u8; 16]> = (0u8..8).map(|i| [i; 16]).collect();
        let tree = build_tree_from_frame_ids(&ids);
        let leaf = hash_leaf(&ids[3]);
        let mut proof = prove_inclusion(&tree, leaf).unwrap();
        // Flip a sibling byte → the recomputed root must not match.
        if let Some(sib) = proof.siblings.first_mut() {
            sib[0] ^= 0xff;
            assert!(
                !verify_inclusion_proof(&tree, leaf, &proof),
                "tampered proof must fail"
            );
        }
        // Tamper the leaf claim against a valid proof for a different leaf.
        let good = prove_inclusion(&tree, hash_leaf(&ids[3])).unwrap();
        let wrong_leaf = hash_leaf(&ids[4]);
        assert!(
            !verify_inclusion_proof(&tree, wrong_leaf, &good),
            "valid proof for one leaf must not verify a different leaf"
        );
    }

    #[test]
    fn empty_tree_sentinel_is_never_inclusion_and_path_lengths_are_exact() {
        let empty = build_tree_from_frame_ids(&[]);
        let leaf = hash_leaf(&[1; 16]);
        let sentinel = prove_exclusion(&empty, leaf).expect("empty exclusion");
        assert!(verify_exclusion_proof(&empty, &sentinel));
        assert!(!verify_inclusion_proof(&empty, leaf, &sentinel));
        assert!(!verify_inclusion_path(empty.root, leaf, &sentinel));
        let tree = build_tree_from_frame_ids(&[[1; 16], [2; 16]]);
        let mut proof = prove_inclusion(&tree, leaf).expect("inclusion");
        proof.directions.clear();
        assert!(!verify_inclusion_proof(&tree, leaf, &proof));
        assert!(!verify_inclusion_path(tree.root, leaf, &proof));
    }
}
