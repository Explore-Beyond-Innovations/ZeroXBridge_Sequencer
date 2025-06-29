use core::poseidon::PoseidonTrait;
use core::hash::HashStateTrait;

pub type MmrProof = Span<felt252>;

pub type MmrPeaks = Span<felt252>;

#[generate_trait]
pub impl PoseidonHasherImpl of PoseidonHasher {
    fn hash_double(left: felt252, right: felt252) -> felt252 {
        PoseidonTrait::new().update(left).update(right).finalize()
    }

    fn hash_span(elements: Span<felt252>) -> felt252 {
        let mut hasher = PoseidonTrait::new();
        let mut i = 0;
        loop {
            if i >= elements.len() {
                break hasher.finalize();
            }
            hasher = hasher.update(*elements[i]);
            i += 1;
        }
    }
}

#[generate_trait]
pub impl MmrProofImpl of MmrProofTrait {
    fn compute_peak(self: MmrProof, leaf_index: usize, leaf_hash: felt252) -> felt252 {
        let mut current_hash = leaf_hash;
        let mut current_index = leaf_index;
        let mut i = 0;
        
        loop {
            if i >= self.len() {
                break current_hash;
            }
            
            let sibling_hash = *self[i];
            
            if current_index % 2 == 0 {
                current_hash = PoseidonHasherImpl::hash_double(current_hash, sibling_hash);
            } else {
                current_hash = PoseidonHasherImpl::hash_double(sibling_hash, current_hash);
            }
            
            current_index /= 2;
            i += 1;
        }
    }
}

#[generate_trait]
pub impl MmrPeaksImpl of MmrPeaksTrait {
    fn bag(self: MmrPeaks) -> felt252 {
        if self.is_empty() {
            return 0;
        }
        
        if self.len() == 1 {
            return *self[0];
        }
        
        let mut result = *self[self.len() - 1];
        let mut i = self.len() - 1;
        
        loop {
            if i == 0 {
                break result;
            }
            i -= 1;
            result = PoseidonHasherImpl::hash_double(*self[i], result);
        }
    }

    fn valid(self: MmrPeaks, root: felt252) -> bool {
        let computed_root = self.bag();
        computed_root == root
    }
}

pub fn verify_proof(
    leaf: felt252,
    leaf_index: usize, 
    proof: MmrProof,
    peaks: MmrPeaks,
    root: felt252
) -> bool {
    if !peaks.valid(root) {
        return false;
    }
    
    if peaks.len() == 1 {
        let computed_peak = proof.compute_peak(leaf_index, leaf);
        return computed_peak == *peaks[0];
    }
    
    if !proof.is_empty() {
        let computed_peak = proof.compute_peak(leaf_index, leaf);
        let mut i = 0;
        loop {
            if i >= peaks.len() {
                break false;
            }
            if computed_peak == *peaks[i] {
                break true;
            }
            i += 1;
        }
    } else {
        let mut i = 0;
        loop {
            if i >= peaks.len() {
                break false;
            }
            if leaf == *peaks[i] {
                break true;
            }
            i += 1;
        }
    }
}

pub fn verify_proof_simple(
    leaf: felt252,
    leaf_index: usize,
    proof: MmrProof, 
    root: felt252
) -> bool {
    if proof.is_empty() {
        return leaf == root;
    }
    
    let computed_root = proof.compute_peak(leaf_index, leaf);
    computed_root == root
}

#[cfg(test)]
mod tests {
    use super::{verify_proof_simple, PoseidonHasherImpl, MmrPeaksImpl};

    #[test]
    fn test_poseidon_hash_double() {
        let left = 1;
        let right = 2;
        let hash = PoseidonHasherImpl::hash_double(left, right);
        
        assert(hash != 0, 'Hash should not be zero');
        
        let hash2 = PoseidonHasherImpl::hash_double(left, right);
        assert(hash == hash2, 'Hash should be deterministic');
    }

    #[test]
    fn test_simple_proof_verification() {
        let leaf = 42;
        let proof = array![].span();
        let root = leaf; 
        
        let result = verify_proof_simple(leaf, 0, proof, root);
        assert(result, 'Single node verification failed');
    }

    #[test]
    fn test_two_node_mmr() {
        let leaf1 = 10;
        let leaf2 = 20;
        
        let root = PoseidonHasherImpl::hash_double(leaf1, leaf2);
        
        let proof1 = array![leaf2].span();
        let result1 = verify_proof_simple(leaf1, 0, proof1, root);
        assert(result1, 'Two-node proof1 failed');
        
        let proof2 = array![leaf1].span();
        let result2 = verify_proof_simple(leaf2, 1, proof2, root);
        assert(result2, 'Two-node proof2 failed');
    }

    #[test]
    fn test_invalid_proof() {
        let leaf = 42;
        let wrong_sibling = 99;
        let correct_sibling = 24;
        
        let correct_root = PoseidonHasherImpl::hash_double(leaf, correct_sibling);
        let wrong_proof = array![wrong_sibling].span();
        
        let result = verify_proof_simple(leaf, 0, wrong_proof, correct_root);
        assert(!result, 'Invalid proof should fail');
    }

    #[test]
    fn test_peaks_bag_empty() {
        let empty_peaks = array![].span();
        let root = empty_peaks.bag();
        assert(root == 0, 'Empty peaks should bag to 0');
    }

    #[test]
    fn test_peaks_bag_single() {
        let peak = 100;
        let single_peak = array![peak].span();
        let root = single_peak.bag();
        assert(root == peak, 'Single peak bag failed');
    }

    #[test]
    fn test_peaks_bag_multiple() {
        let peak1 = 100;
        let peak2 = 200;
        let peak3 = 300;
        
        let peaks = array![peak1, peak2, peak3].span();
        let root = peaks.bag();
        
        let expected = PoseidonHasherImpl::hash_double(peak1, PoseidonHasherImpl::hash_double(peak2, peak3));
        assert(root == expected, 'Multiple peaks bagging failed');
    }

    #[test]
    fn test_peaks_valid() {
        let peak1 = 100;
        let peak2 = 200;
        let peaks = array![peak1, peak2].span();
        let root = peaks.bag();
        
        let is_valid = peaks.valid(root);
        assert(is_valid, 'Peaks validation failed');
        
        let wrong_root = 999;
        let is_invalid = peaks.valid(wrong_root);
        assert(!is_invalid, 'Wrong root validation failed');
    }
} 