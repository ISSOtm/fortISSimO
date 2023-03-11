use crate::song::{EffectId, Instrument, Note, Pattern, Song, Subpattern};

pub fn optimise<'song>(song: &'song Song) -> Vec<OutputCell> {
    let mut patterns = collect_patterns(song);
    let (song_patterns, subpatterns) = patterns.split_at_mut(song.patterns.len());

    mark_reachable_pattern_rows(song, song_patterns);
    for subpattern in subpatterns {
        mark_reachable_subpattern_rows(subpattern);
    }

    // TODO: eliminating trailing unreachable rows from patterns; this can also be combined with
    //       the step below.

    // TODO: eliminating "dead" patterns, i.e. those with no reachable rows (which is not possible
    //       for subpatterns), would reduce pressure on every following step.

    // TODO: eliminate "dead" instruments

    // TODO: eliminate "dead" waves and reorder remaining ones (account for `9` effect on CH3!)

    // TODO: instrument reorg
    // Note: doing this would modify patterns, so they need CoW semantics!

    // TODO: pattern deduplication (including finding patterns "in the middle of" of others) would
    //       cut down on the number of patterns, and potentially speed up following steps.
    let (pattern_ordering, score) = find_pattern_overlap(&patterns);
    eprintln!("Managed to overlap {score} rows!");
    let cell_pool = generate_cell_pool(&patterns, &pattern_ordering);

    // TODO: instrument list truncation

    cell_pool
}

fn collect_patterns(song: &Song) -> Vec<OptimisedPattern> {
    let mut patterns = Vec::with_capacity(song.patterns.len() + song.instruments.len());

    // First, let's collect all patterns.
    patterns.extend(song.patterns.iter().enumerate().map(Into::into));
    let mut collect_subpatterns = |instruments: &[Instrument], kind| {
        patterns.extend(instruments.iter().enumerate().filter_map(|(i, instr)| {
            instr
                .subpattern
                .as_ref()
                .map(|subpattern| (i, kind, subpattern).into())
        }))
    };
    collect_subpatterns(&song.instruments.duty, SubpatternKind::Duty);
    collect_subpatterns(&song.instruments.wave, SubpatternKind::Wave);
    collect_subpatterns(&song.instruments.noise, SubpatternKind::Noise);

    patterns
}

#[derive(Debug, Clone)]
pub struct OptimisedPattern {
    id: PatternId,
    cells: Vec<AnnotatedCell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternId {
    Pattern(usize),
    Subpattern(SubpatternKind, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubpatternKind {
    Duty,
    Wave,
    Noise,
}

#[derive(Debug, Clone, Copy)]
pub struct AnnotatedCell(bool, Cell);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell(pub CellFirstHalf, pub Effect);

#[derive(Debug, Clone, Copy)]
pub enum CellFirstHalf {
    Pattern { note: Note, instrument: u8 },
    Subpattern { offset: u8, next_row_idx: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    pub id: EffectId,
    pub param: u8,
}

fn mark_reachable_pattern_rows(song: &Song, patterns: &mut [OptimisedPattern]) {
    debug_assert_eq!(song.patterns.len(), patterns.len()); // This ensures that we only access regular patterns, and panic if we attempted to access subpatterns, for example.
    let nb_orders = song.order_matrix.len();
    let next_order_idx = |idx| (idx + 1) % nb_orders;

    let mut order_idx = 0;
    let mut row_index = 0;
    // Since patterns may be aliased/reused in several places, we can't trust their "reachable" flag
    // to know if we already visited rows. So instead, we'll maintain a separate vec of bitfields.
    let mut reached = vec![0u64; song.order_matrix.len()];

    loop {
        let reached_this = &mut reached[order_idx];
        // Since there are no conditional jumps, if we reach an already-reached row, we know we're done.
        if *reached_this & (1 << row_index) != 0 {
            break;
        }
        *reached_this |= 1 << row_index;

        let mut next_order = None;
        let mut next_row = None;
        for id in song.order_matrix[order_idx] {
            assert_eq!(patterns[id].id, PatternId::Pattern(id));
            let cell = &mut patterns[id].cells[row_index];

            // Mark the row as reachable.
            cell.0 = true;
            // Check for control flow effects.
            match cell.1 .1.id {
                EffectId::PatternBreak => {
                    next_row = Some(cell.1 .1.param.into());
                    if next_order.is_none() {
                        next_order = Some(next_order_idx(order_idx));
                    }
                }
                EffectId::PosJump => next_order = Some(cell.1 .1.param.into()),
                // These do not affect control flow.
                EffectId::Arpeggio
                | EffectId::PortaUp
                | EffectId::PortaDown
                | EffectId::TonePorta
                | EffectId::Vibrato
                | EffectId::SetMasterVol
                | EffectId::CallRoutine
                | EffectId::NoteDelay
                | EffectId::SetPanning
                | EffectId::ChangeTimbre
                | EffectId::VolSlide
                | EffectId::SetVol
                | EffectId::NoteCut
                | EffectId::SetTempo => {}
            }
        }

        // Go to the next row, or follow the overrides if any are set.
        if let Some(order) = next_order {
            row_index = next_row.unwrap_or(0);
            order_idx = order;
        } else {
            row_index += 1;
            if row_index == 64 {
                row_index = 0;
                order_idx = next_order_idx(order_idx);
            }
        }
    }
}

fn mark_reachable_subpattern_rows(subpattern: &mut OptimisedPattern) {
    let mut row_index = 0;

    // Subpatterns being entirely self-contained, "reachable" == "already visited".
    while !std::mem::replace(&mut subpattern.cells[row_index].0, true) {
        row_index = match subpattern.cells[row_index].1 .0 {
            CellFirstHalf::Pattern { .. } => unreachable!(),
            CellFirstHalf::Subpattern { next_row_idx, .. } => next_row_idx.into(),
        };
    }
}

// This algorithm is described in the README.
fn find_pattern_overlap(patterns: &[OptimisedPattern]) -> (Vec<(usize, usize)>, usize) {
    let nb_patterns = patterns.len();
    let overlap_amount = |ordering: &[(usize, usize)], idx_to_append: usize| {
        let to_append = &patterns[idx_to_append].cells;
        let first_row = to_append
            .first()
            .expect("Entirely unreachable patterns should have been pruned!?");
        let last_pattern = &patterns[ordering.last().expect("Orderings can't be empty!?").0].cells;

        // TODO: we can't use the `aho-corasick` crate because unreachable rows can match anything,
        //       but we could implement a similar strategy.
        for (start_idx, _) in last_pattern
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.can_overlap_with(first_row))
        {
            // We already checked the first row, so we can offset the rest of the iterators a bit.
            if last_pattern[start_idx + 1..]
                .iter()
                .zip(&to_append[1..])
                .all(|(last_pat_cell, to_append_cell)| {
                    last_pat_cell.can_overlap_with(to_append_cell)
                })
            {
                return last_pattern.len() - start_idx;
            }
        }
        // Welp, can't make anything overlap. Sorry!
        0
    };

    // The first iteration is really simple: just shove every pattern, and there can be no overlap.
    // This also ensures that no ordering will ever be empty.
    let mut prev_row = vec![None; nb_patterns]; // We just need to init this somehow.
    let mut new_row = (0..nb_patterns)
        .map(|i| {
            let mut vec = Vec::with_capacity(nb_patterns);
            vec.push((i, 0));
            Some((vec, 0))
        })
        .collect();

    // Now for all the other iterations!
    for _ in 1..nb_patterns {
        std::mem::swap(&mut prev_row, &mut new_row); // Putting this first helps type deduction :3

        for (i, slot) in new_row.iter_mut().enumerate() {
            *slot = prev_row
                .iter()
                .filter_map(|maybe| maybe.as_ref()) // Ignore empty cells (and unwrap the rest).
                .filter(|(ordering, _)| ordering.iter().cloned().all(|(j, _)| j != i)) // Ignore orderings that already contain the candidate.
                .map(|(ordering, score)| {
                    let overlap = overlap_amount(ordering, i);
                    (ordering, score + overlap, overlap)
                }) // Accumulate the candidate's score.
                .max_by_key(|(_, score, _)| *score) // Only retain the best score out of those.
                .map(|(ordering, score, overlap)| {
                    let mut new_ordering = ordering.clone();
                    // Update the overlap amount on the last entry.
                    new_ordering
                        .last_mut()
                        .expect("Orderings cannot be empty!?")
                        .1 = overlap;
                    // Append the candidate to the retained ordering.
                    new_ordering.push((i, 0));
                    (new_ordering, score)
                });
        }
    }

    // And, in the end, we only retain the ordering with the best score.
    new_row
        .into_iter()
        .filter_map(|opt| opt)
        .max_by_key(|(_, score)| *score)
        .expect("How come no ordering survived!?")
}

fn generate_cell_pool(
    patterns: &[OptimisedPattern],
    pattern_ordering: &[(usize, usize)],
) -> Vec<OutputCell> {
    let mut rows = Vec::new();

    for (pattern_id, nb_overlapped_rows) in pattern_ordering.iter().cloned() {
        let pattern = &patterns[pattern_id];
        rows.push(OutputCell::Label(pattern.id));
        rows.extend(
            pattern.cells[..pattern.cells.len() - nb_overlapped_rows]
                .iter()
                .map(|cell| OutputCell::Cell(cell.1)),
        );
        if nb_overlapped_rows != 0 {
            rows.push(OutputCell::OverlapMarker(nb_overlapped_rows));
        }
    }

    rows
}

#[derive(Debug, Clone, Copy)]
pub enum OutputCell {
    Label(PatternId),
    Cell(Cell), // TODO: actually, if a cell overlaps between a pattern and a subpattern, it'd be nice to know that
    OverlapMarker(usize),
}

impl AnnotatedCell {
    fn is_reachable(&self) -> bool {
        self.0
    }

    fn can_overlap_with(&self, other: &Self) -> bool {
        // Here is why one of the conditions below is commented:
        //   In theory, it is fine to overlap anything with an unused row.
        //   However, this "can_overlap" relation is NOT transitive!
        //   Imagine cells A, B, and C, with A and C being non-overlappable but B being unreachable.
        //   A and B can overlap, B and C can overlap, but A and C cannot overlap!
        //   Disabling the condition below is a band-aid that fixes this issue; the proper fix would
        //   be to check if the other overlapping rows are compatible, but that's a TODO.
        // Note that we only disable the RHS, because it is always the LHS' rows that get truncated;
        // and we wouldn't want unreachable rows to be emitted in stead of reachable rows.
        !self.is_reachable() /* || !other.is_reachable() */
            || (other.is_reachable() && self.1 == other.1)
    }
}

impl PartialEq for CellFirstHalf {
    fn eq(&self, other: &Self) -> bool {
        // TOOD: this may be useful when exporting, actually
        fn as_raw(half: &CellFirstHalf) -> (u8, u8) {
            match half {
                CellFirstHalf::Pattern { note, instrument } => (*note as u8, *instrument),
                CellFirstHalf::Subpattern {
                    offset,
                    next_row_idx,
                } => (
                    offset << 1 | (next_row_idx & 0x10) >> 4,
                    next_row_idx & 0x0F,
                ),
            }
        }

        as_raw(self) == as_raw(other)
    }
}

impl From<(usize, &Pattern)> for OptimisedPattern {
    fn from((id, pattern): (usize, &Pattern)) -> Self {
        Self {
            id: PatternId::Pattern(id),
            cells: pattern
                .iter()
                .map(|pattern_cell| {
                    AnnotatedCell(
                        false,
                        Cell(
                            CellFirstHalf::Pattern {
                                note: pattern_cell.note,
                                instrument: pattern_cell.instrument,
                            },
                            Effect {
                                id: pattern_cell.effect_code,
                                param: pattern_cell.effect_param,
                            },
                        ),
                    )
                })
                .collect(),
        }
    }
}

impl From<(usize, SubpatternKind, &Subpattern)> for OptimisedPattern {
    fn from((id, kind, pattern): (usize, SubpatternKind, &Subpattern)) -> Self {
        Self {
            id: PatternId::Subpattern(kind, id),
            cells: pattern
                .iter()
                .map(|subpattern_cell| {
                    AnnotatedCell(
                        false,
                        Cell(
                            CellFirstHalf::Subpattern {
                                offset: subpattern_cell.offset,
                                next_row_idx: subpattern_cell.next_row_idx,
                            },
                            Effect {
                                id: subpattern_cell.effect_code,
                                param: subpattern_cell.effect_param,
                            },
                        ),
                    )
                })
                .collect(),
        }
    }
}
