mod l1;
mod l2;

use core::array::{Array, ArrayTrait, SpanTrait};
use core::panic_with_felt252;
use core::traits::{Into, TryInto};

use l1::verify_proof::verify_mmr_proof as verify_l1;
use l2::verify_proof::verify_proof as verify_l2;
use l1::verify_proof::Hash256;

/// Entry point for verifying MMR proofs on L1 or L2.
///
/// # Input Format (Array<felt252>)
///
/// ## Common
/// - `input[0]`: mode (1 = L1, 2 = L2)
///
/// ## L1 Format (`mode == 1`)
/// - `input[1..=2]`: root as `Hash256` (high, low)
/// - `input[3..=4]`: leaf as `Hash256`
/// - `input[5]`: leaf index (`u32`)
/// - `input[6]`: MMR size (`u32`)
/// - `input[7]`: number of sibling hashes (`N`)
/// - `input[8..(8 + 2 * N - 1)]`: sibling hashes (`Array<Hash256>`)
/// - `input[8 + 2 * N]`: number of peak hashes (`M`)
/// - `input[...end]`: peak hashes (`Array<Hash256>`) – length `2 * M`
///
/// ## L2 Format (`mode == 2`)
/// - `input[1]`: root as `felt252`
/// - `input[2]`: leaf as `felt252`
/// - `input[3]`: leaf index (`u32`)
/// - `input[4]`: MMR size (`u32`)
/// - `input[5]`: number of sibling hashes (`N`)
/// - `input[6..(6 + N - 1)]`: sibling hashes (`Array<felt252>`)
/// - `input[6 + N]`: number of peaks (`M`)
/// - `input[...end]`: peak hashes (`Array<felt252>`) – length `M`
///
/// # Returns
/// - `0` if proof is valid
/// - `1` if proof is invalid
///
/// # Panics
/// - If the input is malformed or the mode is unsupported
fn main(input: Array<felt252>) -> felt252 {
    assert(input.len() >= 2, 'Input not complete');

    let mode = *input.at(0);
    let mode_index: usize = mode.try_into().unwrap() - 1;

    match mode_index {
        // === L1 MODE ===
        0 => {
            assert(input.len() >= 8, 'L1 input too short');

            let root = Hash256 { high: *input.at(1), low: *input.at(2) };
            let leaf = Hash256 { high: *input.at(3), low: *input.at(4) };

            let leaf_index: u32 = (*input.at(5)).try_into().unwrap();
            let mmr_size: u32 = (*input.at(6)).try_into().unwrap();
            let num_siblings: usize = (*input.at(7)).try_into().unwrap();

            let siblings_start = 8;
            let siblings_end = siblings_start + 2 * num_siblings;

            assert(input.len() > siblings_end, 'L1 input missing sibling hashes');

            let num_peaks: usize = (*input.at(siblings_end)).try_into().unwrap();
            let peaks_start = siblings_end + 1;
            let peaks_end = peaks_start + 2 * num_peaks;

            assert(input.len() == peaks_end, 'L1 length mismatch for peaks');

            let span = input.span();
            let siblings_span = span.slice(siblings_start, siblings_end);
            let peaks_span = span.slice(peaks_start, peaks_end);

            let mut siblings_array = ArrayTrait::<Hash256>::new();
            let mut i = 0;
            while i < siblings_span.len() {
                siblings_array.append(Hash256 {
                    high: *siblings_span.at(i),
                    low: *siblings_span.at(i + 1),
                });
                i += 2;
            }

            let mut peaks_array = ArrayTrait::<Hash256>::new();
            let mut j = 0;
            while j < peaks_span.len() {
                peaks_array.append(Hash256 {
                    high: *peaks_span.at(j),
                    low: *peaks_span.at(j + 1),
                });
                j += 2;
            }

            let is_valid = verify_l1(leaf, leaf_index, siblings_array, peaks_array, mmr_size, root);
            if is_valid { 0 } else { 1 }
        },

        // === L2 MODE ===
        1 => {
            assert(input.len() >= 7, 'L2 input too short');

            let root = *input.at(1);
            let leaf = *input.at(2);
            let leaf_index: u32 = (*input.at(3)).try_into().unwrap();
            let mmr_size: u32 = (*input.at(4)).try_into().unwrap();
            let num_siblings: usize = (*input.at(5)).try_into().unwrap();

            let siblings_start = 6;
            let siblings_end = siblings_start + num_siblings;

            assert(input.len() > siblings_end, 'L2 input missing siblings');

            let num_peaks: usize = (*input.at(siblings_end)).try_into().unwrap();
            let peaks_start = siblings_end + 1;
            let peaks_end = peaks_start + num_peaks;

            assert(input.len() == peaks_end, 'L2 length mismatch for peaks');

            let span = input.span();
            let siblings_span = span.slice(siblings_start, siblings_end);
            let peaks_span = span.slice(peaks_start, peaks_end);

            let mut peaks_array = ArrayTrait::<felt252>::new();
            for p in peaks_span {
                peaks_array.append(*p);
            }

            let is_valid = verify_l2(
                leaf,
                leaf_index,
                siblings_span,
                root,
                mmr_size.into(),
                peaks_array,
            );

            if is_valid { 0 } else { 1 }
        },

        _ => panic_with_felt252('Invalid Mode'),
    }
}
