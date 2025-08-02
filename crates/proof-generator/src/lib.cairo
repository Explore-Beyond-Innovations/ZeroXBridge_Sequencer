mod l1;
mod l2;
use core::array::{Array, ArrayTrait, SpanTrait};
use core::panic_with_felt252;
use core::traits::{Into, TryInto};
use l1::verify_proof::verify_mmr_proof as verify_l1;
use l2::verify_proof::verify_proof as verify_l2;


/// Entry point for verifying Merkle proofs on L1 or L2
///
/// This function takes a flat input array and decides whether to verify the proof
/// using the L1 or L2 method, based on the `mode` argument.
///
/// # Input Layout (Array<felt252>)
///
/// - `input[0]`: mode (1 = L1, 2 = L2)
/// - `input[1]`: root
/// - `input[2]`: leaf
/// - `input[3]`: leaf index
/// - `input[4]`: MMR size
/// - `input[5]`: number of siblings
/// - `input[6..]`: sibling hashes
/// - After siblings: number of peaks
/// - Followed by: peak hashes
///
/// # Returns
///
/// - `0` if the proof is valid
/// - `1` if the proof is invalid
///
/// # Panics
///
/// - If the input is malformed or the mode is invalid
///
/// # Notes
///
/// Cairo requires `match` arms to start from 0, so we remap:
/// - mode 1 (L1) → 0
/// - mode 2 (L2) → 1
fn main(input: Array<felt252>) -> felt252 {
    assert(input.len() >= 7, 'Invalid input length');

    let mode = *input.at(0);
    let root = *input.at(1);
    let leaf = *input.at(2);

    let leaf_index: u32 = (*input.at(3)).try_into().unwrap();
    let mmr_size: u32 = (*input.at(4)).try_into().unwrap();
    let num_siblings: usize = (*input.at(5)).try_into().unwrap();

    let siblings_start = 6;
    let siblings_end = siblings_start + num_siblings;
    let num_peaks: usize = (*input.at(siblings_end)).try_into().unwrap();
    let peaks_start = siblings_end + 1;
    let peaks_end = peaks_start + num_peaks;

    let span = input.span();

    let siblings_span = span.slice(siblings_start, siblings_end);
    let peaks_span = span.slice(peaks_start, peaks_end);

    // this span conversion is neccesary for L1 proof verifier
    let mut siblings_array = ArrayTrait::<felt252>::new();
    for s in siblings_span {
        siblings_array.append(*s);
    }

    let mut peaks_array = ArrayTrait::<felt252>::new();
    for p in peaks_span {
        peaks_array.append(*p);
    }

    // pattern matching in cairo requires values to be sequential starting from 0.
    // So we map:
    //   mode = 1 (L1) => 0
    //   mode = 2 (L2) => 1
    // same way we count indexes from 0 upwards in arrays
    let mode_index: usize = mode.try_into().unwrap() - 1;

    let is_valid = match mode_index {
        0 => verify_l1(leaf, leaf_index, siblings_array, peaks_array, mmr_size, root),
        1 => verify_l2(
            leaf, leaf_index.try_into().unwrap(), siblings_span, root, mmr_size.into(), peaks_array,
        ),
        _ => panic_with_felt252('Invalid Mode'),
    };

    // Return 0 for true, 1 for false
    if is_valid {
        0
    } else {
        1
    }
}
