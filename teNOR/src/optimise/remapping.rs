use crate::song::EffectId;

use super::{
    AnnotatedCell, CellFirstHalf, Effect, InstrKind, OptimisedPattern, PatternId, PatternStore,
};

#[derive(Debug, Clone)]
pub struct CompactedMapping<const N: usize>(pub(super) [u8; N], pub(super) usize);

pub(super) fn compacted_mapping_from_mask<const N: usize>(mut mask: u16) -> CompactedMapping<N> {
    if N != 16 {
        debug_assert_eq!(mask & (u16::MAX << N), 0); // All unused bits must be considered as unoccupied, otherwise `leading_zeros` will be wrong.
    }

    // Fill in the default mapping, which is the identity.
    let mut mapping = std::array::from_fn(|i| i as u8); // This is 15 at most.

    // Now, compact the mapping.
    if mask != 0 {
        loop {
            // To minimise the number of changes, we'll move the largest ID into the smallest available slot.
            let available = mask.trailing_ones() as usize;
            let displaced = 15 - (mask.leading_zeros() as usize);
            debug_assert_ne!(available, displaced); // Unless I'm drunk, this shouldn't be possible..!
            if available > displaced {
                // If the smallest zero bit is later than the first used slot, we're done.
                break;
            }
            mapping.swap(available, displaced); // Swapping both ensures that the mapping is bidirectionsl (since we never swap the same slot twice).
            mask &= !(1 << displaced);
            mask |= 1 << available;
        }
    }

    CompactedMapping(
        mapping,
        16 - (mask.leading_zeros() as usize), // This is 15 at most, definitely fits in a usize
    )
}

impl<const N: usize> CompactedMapping<N> {
    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        self.0[..self.1].iter().cloned()
    }

    pub(super) fn nb_saved(&self) -> usize {
        N - self.1
    }

    pub fn get_remapped_id(&self, original: usize) -> u8 {
        debug_assert!(usize::from(self.0[original]) < self.1); // The new ID should be within the "compacted" range.
        self.0[original]
    }
}

pub(super) fn remap_instrs(pattern: &mut OptimisedPattern, mapping: &[u8; 15]) {
    for cell in &mut pattern.0 {
        let AnnotatedCell {
            reachable: true,
            cell,
        } = cell
        else {
            continue;
        }; // Don't bother with unreachable cells.
        let CellFirstHalf::Pattern {
            note: _,
            instrument,
        } = &mut cell.0
        else {
            unreachable!();
        };
        // ID 0 means "no instrument", and that doesn't change.
        if *instrument != 0 {
            // Actual instruments are 1-indexed.
            *instrument = mapping[usize::from(*instrument) - 1] + 1;
        }
    }
}

pub(super) fn remap_waves(patterns: &mut PatternStore, mapping: &[u8; 16]) {
    for (id, pattern) in patterns {
        if matches!(
            id,
            PatternId::Pattern(InstrKind::Wave, _) | PatternId::Subpattern(InstrKind::Wave, _)
        ) {
            for cell in &mut pattern.0 {
                let AnnotatedCell {
                    reachable: true,
                    cell,
                } = cell
                else {
                    continue;
                };
                if let Effect {
                    id: EffectId::ChangeTimbre,
                    param,
                } = &mut cell.1
                {
                    *param = mapping[usize::from(*param)];
                }
            }
        }
    }
}
