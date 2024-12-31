use std::{collections::HashMap, iter::FusedIterator, num::Wrapping};

use super::{AnnotatedCell, Cell, OptimisedPattern, OutputCell, PatternId};

// This algorithm is described in the README.
pub(super) fn find_pattern_overlap(
    patterns: &HashMap<PatternId, OptimisedPattern>,
) -> (RowPoolBuilder<'_>, usize) {
    // A hashmap's keys are not guaranteed to be returned in a consistent order, so collect them to ensure that.
    let pattern_ids: Vec<_> = patterns.keys().cloned().collect();
    let nb_patterns = pattern_ids.len();

    // The first iteration is really simple: just shove every pattern, and there can be no overlap.
    // This also ensures that no ordering will ever be empty.
    // TODO: two separate allocations? Meh...
    let mut prev_row = vec![None; nb_patterns]; // We just need to init this somehow.
    let mut new_row = pattern_ids
        .iter()
        .map(|&i| Some(RowPoolBuilder::new(patterns, i)))
        .collect();

    // Now for all the other iterations!
    for _ in 1..nb_patterns {
        std::mem::swap(&mut prev_row, &mut new_row); // Putting this first helps with type deduction!

        for (&pattern_id, target) in pattern_ids.iter().zip(new_row.iter_mut()) {
            *target = prev_row
                .iter()
                .filter_map(|maybe| maybe.as_ref()) // Ignore empty cells (and unwrap the rest).
                .filter(|builder| !builder.contains(pattern_id)) // Reject builders that already contain the pattern.
                .map(|builder| {
                    let (score, start_row_idx) = builder.score_with(pattern_id);
                    (start_row_idx, score, builder)
                })
                .max_by_key(|(_, score, _)| *score)
                .map(|(start_row_idx, new_score, builder)| {
                    let mut new_builder = builder.clone();
                    new_builder.add(pattern_id, start_row_idx, new_score);
                    new_builder
                });
        }
    }

    let best_builder = new_row
        .into_iter()
        .flatten() // Skip over empty cells.
        .max_by_key(|builder| builder.score)
        .expect("How come no ordering survived!?");
    let score = best_builder.score;
    (best_builder, score)
}

impl AnnotatedCell {
    fn can_overlap_with(&self, other: &Self) -> bool {
        // Checking for reachability like this is fine, because we always try hard to find an overlapping reachable row.
        !self.reachable || !other.reachable || self.cell == other.cell
    }
}

#[derive(Debug, Clone)]
pub(super) struct RowPoolBuilder<'patterns> {
    patterns: &'patterns HashMap<PatternId, OptimisedPattern>,
    // Vector of (pattern id, how many rows into pool before its start)
    ordering: Vec<(PatternId, usize)>,
    score: usize,
}

impl<'patterns> RowPoolBuilder<'patterns> {
    fn new(
        patterns: &'patterns HashMap<PatternId, OptimisedPattern>,
        initial_pattern_id: PatternId,
    ) -> Self {
        let mut ordering = Vec::with_capacity(patterns.len());
        ordering.push((initial_pattern_id, 0));
        Self {
            patterns,
            ordering,
            score: 0,
        }
    }

    fn contains(&self, pattern_id: PatternId) -> bool {
        self.ordering.iter().any(|&(id, _)| id == pattern_id)
    }

    fn score_with(&self, pattern_id: PatternId) -> (usize, usize) {
        let pattern = &self.patterns[&pattern_id];
        let first_row = &pattern.0[0];

        let mut rows = self.rows();
        let mut start_row_idx = 0;
        'try_somewhere_else: while let Some(cell) = rows.next() {
            start_row_idx += 1;
            if !cell.can_overlap_with(first_row) {
                continue;
            }

            // We will want to resume our search later, so we'll keep the original iterator intact.
            let mut overlappable_rows = rows.clone();
            let mut row_idx = 1; // We already matched the first row.
            while let (Some(pattern_row), Some(row)) =
                (pattern.0.get(row_idx), overlappable_rows.next())
            {
                if !pattern_row.can_overlap_with(row) {
                    continue 'try_somewhere_else;
                }
                row_idx += 1;
            }
            // `row_idx` is how many rows we've managed to overlap.
            return (self.score + row_idx, start_row_idx);
        }

        // Couldn't overlap anything. Too bad!
        (self.score, start_row_idx)
    }

    fn add(&mut self, pattern_id: PatternId, start_row_idx: usize, new_score: usize) {
        // Keep the array sorted by `start_row_idx`.
        let insert_idx = self
            .ordering
            .iter()
            .enumerate()
            .find(|(_, &(_, start_idx))| start_idx >= start_row_idx)
            .map_or(self.ordering.len(), |(i, _)| i);
        self.ordering
            .insert(insert_idx, (pattern_id, start_row_idx));

        self.score = new_score;
    }

    fn rows(&self) -> RowsIter<'patterns, '_> {
        RowsIter {
            builder: self,
            ordering_idx: 0,
            row_idx: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct RowsIter<'builder, 'patterns> {
    builder: &'builder RowPoolBuilder<'patterns>,
    ordering_idx: usize,
    row_idx: usize,
}

impl<'builder, 'patterns: 'builder> Iterator for RowsIter<'builder, 'patterns> {
    type Item = &'patterns AnnotatedCell;

    fn next(&mut self) -> Option<Self::Item> {
        let &(pattern_id, start_ofs) = self.builder.ordering.get(self.ordering_idx)?;
        let pattern = &self.builder.patterns[&pattern_id];
        debug_assert!(self.row_idx >= start_ofs);
        let pattern_ofs = self.row_idx - start_ofs;

        debug_assert!(pattern_ofs < pattern.0.len()); // Guaranteed by init / last iteration.
        let mut row = &pattern.0[pattern_ofs];
        if !row.reachable {
            // If this row is not reachable, try providing a row overlapping with it that is reachable.
            // For brevity, "overlapping" in the below variables will be shortened to "ovlpg".
            // TODO: rewrite this using iterators, and compare. Discuss with nyanpasu.
            for &(ovlpg_pattern_id, ovlpg_pattern_row_idx) in
                &self.builder.ordering[self.ordering_idx..]
            {
                // Patterns are sorted by their "start row index"; if we overshoot, so will all subsequent iterations.
                let Some(ovlpg_pattern_ofs) = self.row_idx.checked_sub(ovlpg_pattern_row_idx)
                else {
                    break;
                };

                let ovlpg_row = &self.builder.patterns[&ovlpg_pattern_id].0[ovlpg_pattern_ofs];
                // I found you, faker!
                if ovlpg_row.reachable {
                    // Faker? You're not even good enough to be my fake.
                    row = ovlpg_row;
                    break;
                }
            }
        }

        // Advance the indices.
        self.row_idx += 1;
        let check_past_end = |&(pattern_id, start_ofs)| {
            let pattern_ofs = self.row_idx - start_ofs;
            pattern_ofs >= self.builder.patterns[&pattern_id].0.len()
        };
        while self
            .builder
            .ordering
            .get(self.ordering_idx)
            .map_or(false, check_past_end)
        {
            // Gone over the end of `pattern`, switch to the next one.
            self.ordering_idx += 1;
        }

        Some(row)
    }
}

impl FusedIterator for RowsIter<'_, '_> {}

pub(super) fn generate_row_pool(
    RowPoolBuilder {
        patterns,
        ordering,
        score: _,
    }: RowPoolBuilder,
) -> (Vec<OutputCell>, HashMap<Cell, u8>, isize) {
    let mut output = Vec::new();
    let mut cell_catalog = HashMap::new();
    let mut next_id = Wrapping(0); // An ID overflow will trigger an error in a later stage; for now, just ensure that we get there without panicking.
    let mut nb_saved_bytes = 0;

    debug_assert_eq!(ordering[0].1, 0); // The first pattern should be starting at the first row.
    let mut nb_rows_emitted = 0; // For sanity checking.
    let mut idx = 0;
    let mut next_idx = 0; // Index of the next pattern to consider.
    'outer: loop {
        let (pattern_id, start_ofs) = ordering[idx];
        let rows = &patterns[&pattern_id].0;

        for row in &rows[nb_rows_emitted - start_ofs..] {
            // Check if we need to emit any labels.
            while let Some(&(next_pattern_id, next_start_ofs)) = ordering.get(next_idx) {
                debug_assert!(next_start_ofs >= nb_rows_emitted); // The ordering is supposed to be sorted this way.
                if next_start_ofs != nb_rows_emitted {
                    break;
                }

                // TODO: this is wrong, we'd need to examine the previous pattern.
                // let overlap_amount = patterns[&next_pattern_id].0.len() - i;
                let overlap_amount = 0;
                if overlap_amount != 0 {
                    output.push(OutputCell::OverlapMarker(overlap_amount));
                }
                output.push(OutputCell::Label(next_pattern_id));

                next_idx += 1;
            }

            if let Some(row) = row.reachable.then_some(row).or_else(|| {
                // Try finding an overlapping cell that is reachable.
                ordering[next_idx..]
                    .iter()
                    .take_while(|&&(_, start_ofs)| start_ofs <= nb_rows_emitted) // The list is sotred by increasing `start_ofs`, so if this is true once, it will remain so.
                    .filter_map(|&(pattern_id, start_ofs)| {
                        // We checked that we aren't past the pattern's beginning, but also check that we aren't past its end.
                        // (This is possible when a pattern is "nested" inside of another.)
                        patterns[&pattern_id].0.get(nb_rows_emitted - start_ofs)
                    })
                    .find(|candidate| candidate.reachable)
            }) {
                nb_saved_bytes += 2; // Each 3-byte cell is replaced with a 1-byte ID.

                let id = *cell_catalog.entry(row.cell).or_insert_with(|| {
                    // Inserting a new cell costs 3 bytes.
                    nb_saved_bytes -= 3;

                    let new_id = next_id.0;
                    next_id += 1;
                    new_id
                });
                output.push(OutputCell::Cell(id));
            } else {
                output.push(OutputCell::Cell(255)); // Arbitrary placeholder, hopefully rare, value.
            }
            nb_rows_emitted += 1;
        }

        loop {
            idx += 1;
            match ordering.get(idx) {
                // If we exhausted all rows, stop!
                None => break 'outer,
                // If this pattern contains the next row we would like to emit, pick it.
                Some((pattern_id, start_ofs))
                    if start_ofs + patterns[pattern_id].0.len() > nb_rows_emitted =>
                {
                    break;
                }
                // Otherwise, keep looking.
                Some(_) => {}
            }
        }
    }

    // We should've been through every pattern.
    debug_assert_eq!(next_idx, ordering.len());

    let nb_unique_cells = cell_catalog.len() as isize;
    (
        output,
        cell_catalog,
        nb_saved_bytes - (256 - nb_unique_cells) * 2,
    )
}
