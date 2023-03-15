use std::collections::HashMap;

use crate::song::{EffectId, Instrument, InstrumentKind, Note, PatternCell, Song, SubpatternCell};

pub fn optimise(
    song: &Song,
) -> (
    Vec<OutputCell>,
    CompactedMapping<15>,
    CompactedMapping<15>,
    CompactedMapping<15>,
    CompactedMapping<16>,
    OptimStats,
) {
    let mut patterns = collect_patterns(song);

    let (used_duty_instrs, used_wave_instrs, used_noise_instrs, mut used_waves) =
        mark_reachable_pattern_rows(song, &mut patterns);

    let mut pruned_patterns = 0;
    let mut pruned_pattern_rows = 0;
    let mut trimmed_rows = 0;
    // Eliminating patterns now means `remove` will move less data since the subpatterns aren't in yet,
    // and iterating over fewer rows when remapping instruments.
    trim_trailing_unreachable_rows(
        &mut patterns,
        &mut pruned_patterns,
        &mut pruned_pattern_rows,
        &mut trimmed_rows,
    );

    collect_subpatterns(
        &mut patterns,
        &song.instruments.duty,
        used_duty_instrs,
        InstrKind::Duty,
    );
    collect_subpatterns(
        &mut patterns,
        &song.instruments.wave,
        used_wave_instrs,
        InstrKind::Wave,
    );
    collect_subpatterns(
        &mut patterns,
        &song.instruments.noise,
        used_noise_instrs,
        InstrKind::Noise,
    );

    for (id, subpattern) in &mut patterns {
        let PatternId::Subpattern(..) = id else { continue; };
        mark_reachable_subpattern_rows(*id, subpattern, &mut used_waves);
    }

    // FIXME: this is not ideal, since it will iterate on the regular patterns again.
    //        This might be fixable by doing the trimming in the collection phase instead.
    trim_trailing_unreachable_rows(
        &mut patterns,
        &mut pruned_patterns,
        &mut pruned_pattern_rows,
        &mut trimmed_rows,
    );

    // Eliminate "dead" instruments and reorder remaining ones.
    // Note: doing this modifies patterns, so they need CoW semantics!
    let duty_instr_usage = compacted_mapping_from_mask(used_duty_instrs);
    let wave_instr_usage = compacted_mapping_from_mask(used_wave_instrs);
    let noise_instr_usage = compacted_mapping_from_mask(used_noise_instrs);
    for (id, pattern) in &mut patterns {
        let PatternId::Pattern(kind, _) = id else { continue; };
        remap_instrs(
            pattern,
            &match kind {
                InstrKind::Duty => &duty_instr_usage,
                InstrKind::Wave => &wave_instr_usage,
                InstrKind::Noise => &noise_instr_usage,
            }
            .0,
        )
    }

    // Eliminate "dead" waves and reorder remaining ones.
    // The last usage contributor unaccounted for is wave instruments.
    for (i, instr) in song.instruments.wave.iter().enumerate() {
        if used_wave_instrs & 1 << i == 0 {
            continue; // Ignore unused instruments.
        }
        let InstrumentKind::Wave { output_level: _, wave_id } = instr.kind else { unreachable!(); };
        used_waves |= 1 << wave_id;
    }
    let wave_usage = compacted_mapping_from_mask(used_waves);
    remap_waves(&mut patterns, &wave_usage.0);

    // TODO: pattern deduplication (including finding patterns "in the middle of" of others) would
    //       cut down on the number of patterns, and potentially speed up following steps.
    let (pattern_ordering, overlapped_rows) = find_pattern_overlap(&patterns);
    let cell_pool = generate_cell_pool(&patterns, &pattern_ordering);

    // We're done! Time to compute some stats for reporting, and return our hard work!

    let mut pattern_usage = vec![0u8; (song.patterns.len() + 7) / 8];
    let mut duplicated_patterns = 0; // Innocent until proven guilty.
    for id in patterns.keys() {
        let PatternId::Pattern(_, index) = id else { continue; };
        let byte = &mut pattern_usage[index / 8];
        let mask = 1 << (index % 8);
        if *byte & mask == 0 {
            *byte |= mask;
        } else {
            duplicated_patterns += 1;
        }
    }

    let saved_bytes_instrs = |instrs: &[Instrument], ids: &[u8]| {
        ids.iter().cloned().fold(0, |sum, id| {
            let instr = &instrs[usize::from(id)];
            sum + instr.kind.data_size()
                + instr
                    .subpattern
                    .map_or(0, |subpattern| subpattern.len() * 3)
        })
    };
    let stats = OptimStats {
        duplicated_patterns,
        overlapped_rows,
        pruned_patterns,
        pruned_pattern_rows,
        trimmed_rows,
        pruned_instrs: duty_instr_usage.nb_saved()
            + wave_instr_usage.nb_saved()
            + noise_instr_usage.nb_saved(),
        pruned_instrs_bytes: saved_bytes_instrs(
            &song.instruments.duty,
            &duty_instr_usage.0[duty_instr_usage.1..],
        ) + saved_bytes_instrs(
            &song.instruments.wave,
            &wave_instr_usage.0[wave_instr_usage.1..],
        ) + saved_bytes_instrs(
            &song.instruments.noise,
            &noise_instr_usage.0[noise_instr_usage.1..],
        ),
        trimmed_waves: wave_usage.nb_saved(),
    };

    (
        cell_pool,
        duty_instr_usage,
        wave_instr_usage,
        noise_instr_usage,
        wave_usage,
        stats,
    )
}

#[derive(Debug, Clone)]
pub struct OptimStats {
    pub duplicated_patterns: usize,
    pub overlapped_rows: usize,
    pub pruned_patterns: usize,
    pub pruned_pattern_rows: usize,
    pub trimmed_rows: usize,
    pub pruned_instrs: usize,
    pub pruned_instrs_bytes: usize,
    pub trimmed_waves: usize,
}

impl OptimStats {
    pub fn wasted_bytes_duplicated_patterns(&self) -> usize {
        self.duplicated_patterns * 64 * 3
    }

    pub fn saved_bytes_overlapped_rows(&self) -> usize {
        self.overlapped_rows * 3
    }

    pub fn saved_bytes_pruned_patterns(&self) -> usize {
        self.pruned_pattern_rows * 3
    }

    pub fn saved_bytes_trimmed_rows(&self) -> usize {
        self.trimmed_rows * 3
    }

    pub fn saved_bytes_trimmed_waves(&self) -> usize {
        self.trimmed_waves * 16
    }

    pub fn total_saved_bytes(&self) -> isize {
        (self.saved_bytes_overlapped_rows()
            + self.saved_bytes_pruned_patterns()
            + self.saved_bytes_trimmed_rows()
            + self.pruned_instrs_bytes
            + self.saved_bytes_trimmed_waves())
        .wrapping_sub(self.wasted_bytes_duplicated_patterns()) as isize // I doubt the savings will ever grow that large...
    }
}

fn collect_patterns(song: &Song) -> HashMap<PatternId, OptimisedPattern> {
    // We duplicate patterns across instrument kinds to allow reasoning on the kinds individually;
    // for example, instrument IDs are implicitly per-kind, so modifying them across kinds require
    // copy-on-write semantics, and that kind of sucks.
    // While doing this increases the number of patterns, the later deduplication steps SHOULD bring
    // the numbers down again.
    let mut patterns = HashMap::new();
    for order_row in &song.order_matrix {
        for (channel_id, pattern_id) in order_row.iter().cloned().enumerate() {
            patterns
                .entry(PatternId::Pattern(
                    InstrKind::from_channel_id(channel_id),
                    pattern_id,
                ))
                .or_insert_with(|| song.patterns[pattern_id].iter().collect());
        }
    }
    patterns
}

fn collect_subpatterns(
    patterns: &mut HashMap<PatternId, OptimisedPattern>,
    instruments: &[Instrument; 15],
    used_instrs_mask: u16,
    kind: InstrKind,
) {
    patterns.extend(
        instruments
            .iter()
            .enumerate()
            .filter(|(id, _)| used_instrs_mask & (1 << id) != 0) // Only keep "reachable" instruments.
            .filter_map(|(id, instr)| {
                instr.subpattern.as_ref().map(|subpattern| {
                    (
                        PatternId::Subpattern(kind, id + 1),
                        subpattern.iter().collect(),
                    )
                })
            }),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternId {
    Pattern(InstrKind, usize),
    Subpattern(InstrKind, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstrKind {
    Duty,
    Wave,
    Noise,
}

impl InstrKind {
    pub fn from_channel_id(channel_id: usize) -> Self {
        match channel_id {
            0 | 1 => Self::Duty,
            2 => Self::Wave,
            3 => Self::Noise,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimisedPattern(Vec<AnnotatedCell>);

#[derive(Debug, Clone, Copy)]
pub struct AnnotatedCell {
    reachable: bool,
    cell: Cell,
}

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

impl From<&PatternCell> for Cell {
    fn from(value: &PatternCell) -> Self {
        Self(
            CellFirstHalf::Pattern {
                note: value.note,
                instrument: value.instrument,
            },
            Effect {
                id: value.effect_code,
                param: value.effect_param,
            },
        )
    }
}

impl From<&SubpatternCell> for Cell {
    fn from(value: &SubpatternCell) -> Self {
        Self(
            CellFirstHalf::Subpattern {
                offset: value.offset,
                next_row_idx: value.next_row_idx,
            },
            Effect {
                id: value.effect_code,
                param: value.effect_param,
            },
        )
    }
}

impl<T: Into<Cell>> FromIterator<T> for OptimisedPattern {
    fn from_iter<C: IntoIterator<Item = T>>(container: C) -> Self {
        Self(
            container
                .into_iter()
                .map(|value| AnnotatedCell {
                    reachable: false,
                    cell: value.into(),
                })
                .collect(),
        )
    }
}

fn mark_reachable_pattern_rows(
    song: &Song,
    patterns: &mut HashMap<PatternId, OptimisedPattern>,
) -> (u16, u16, u16, u16) {
    let nb_orders = song.order_matrix.len();
    let next_order_idx = |idx| (idx + 1) % nb_orders;
    let mut used_duty_instrs = 0;
    let mut used_wave_instrs = 0;
    let mut used_noise_instrs = 0;
    let mut used_waves = 0;

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
        for (i, id) in song.order_matrix[order_idx].iter().cloned().enumerate() {
            let kind = InstrKind::from_channel_id(i);
            let cell = &mut patterns
                .get_mut(&PatternId::Pattern(kind, id))
                .expect("Order matrix references unknown pattern")
                .0[row_index];

            // Mark the row as reachable.
            cell.reachable = true;
            // Check for control flow effects.
            match cell.cell.1 {
                Effect {
                    id: EffectId::PatternBreak,
                    param,
                } => {
                    next_row = Some(param.into());
                    if next_order.is_none() {
                        next_order = Some(next_order_idx(order_idx));
                    }
                }
                Effect {
                    id: EffectId::PosJump,
                    param,
                } => next_order = Some(param.into()),
                // CH3's `9` effect references waves; use the time to mark one if relevant.
                Effect {
                    id: EffectId::ChangeTimbre,
                    param,
                } if kind == InstrKind::Wave => {
                    // TODO: report the error a little more nicely.
                    assert!(
                        param < 16,
                        "Param of FX 9 in pattern {id} row {row_index} is out of bounds! ({param} >= 16)",
                    );
                    used_waves |= 1 << param;
                }
                // These do not affect control flow.
                Effect {
                    id:
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
                        | EffectId::SetTempo,
                    param: _,
                } => {}
            }

            // Mark the corresponding instrument as reachable, too.
            // (Bit 0 is unused, since it marks "no instrument".)
            let CellFirstHalf::Pattern { note: _, instrument } = cell.cell.0 else { unreachable!(); };
            *match i {
                0 | 1 => &mut used_duty_instrs,
                2 => &mut used_wave_instrs,
                3 => &mut used_noise_instrs,
                _ => unreachable!(),
            } |= 1 << instrument;
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

    (
        used_duty_instrs >> 1,
        used_wave_instrs >> 1,
        used_noise_instrs >> 1,
        used_waves,
    )
}

fn mark_reachable_subpattern_rows(
    id: PatternId,
    subpattern: &mut OptimisedPattern,
    used_waves: &mut u16,
) {
    let mut row_index = 0;

    // Subpatterns being entirely self-contained, "reachable" == "already visited".
    while !std::mem::replace(&mut subpattern.0[row_index].reachable, true) {
        let cell = &subpattern.0[row_index].cell;

        if let (
            PatternId::Subpattern(InstrKind::Wave, instr_id),
            Effect {
                id: EffectId::ChangeTimbre,
                param,
            },
        ) = (id, cell.1)
        {
            // TODO: report the error a little more nicely.
            assert!(
                param < 16,
                "Param to FX 9 in wave instr {instr_id} out of bounds! ({param} >= 16)",
            );
            *used_waves |= 1 << param;
        }

        row_index = match cell.0 {
            CellFirstHalf::Pattern { .. } => unreachable!(),
            CellFirstHalf::Subpattern { next_row_idx, .. } => next_row_idx.into(),
        };
    }
}

fn trim_trailing_unreachable_rows(
    patterns: &mut HashMap<PatternId, OptimisedPattern>,
    pruned_patterns: &mut usize,
    pruned_pattern_rows: &mut usize,
    trimmed_rows: &mut usize,
) {
    // FIXME: this is not very ergonomic, but required since `Hashmap::drain_filter()` is not stable yet.
    // https://github.com/rust-lang/rust/issues/59618
    let keys: Vec<_> = patterns.keys().cloned().collect();

    for key in keys.iter().cloned() {
        let std::collections::hash_map::Entry::Occupied(mut entry) = patterns.entry(key) else { unreachable!(); };

        let pattern = &mut entry.get_mut().0;
        match pattern
            .iter()
            .enumerate()
            .rev()
            .find(|(_, cell)| cell.reachable)
        {
            Some((last_used_idx, _)) => {
                *trimmed_rows += pattern.len() - (last_used_idx + 1);
                pattern.truncate(last_used_idx + 1);
            }
            None => {
                *pruned_patterns += 1;
                *pruned_pattern_rows += pattern.len();
                entry.remove();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompactedMapping<const N: usize>([u8; N], usize);

fn compacted_mapping_from_mask<const N: usize>(mut mask: u16) -> CompactedMapping<N> {
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

    fn nb_saved(&self) -> usize {
        N - self.1
    }
}

fn remap_instrs(pattern: &mut OptimisedPattern, mapping: &[u8; 15]) {
    for cell in &mut pattern.0 {
        let AnnotatedCell { reachable: true, cell } = cell else { continue; }; // Don't bother with unreachable cells.
        let CellFirstHalf::Pattern { note: _ , instrument } = &mut cell.0 else { unreachable!(); };
        // ID 0 means "no instrument", and that doesn't change.
        if *instrument != 0 {
            // Actual instruments are 1-indexed.
            *instrument = mapping[usize::from(*instrument) - 1] + 1;
        }
    }
}

fn remap_waves(patterns: &mut HashMap<PatternId, OptimisedPattern>, mapping: &[u8; 16]) {
    for (id, pattern) in patterns {
        if matches!(
            id,
            PatternId::Pattern(InstrKind::Wave, _) | PatternId::Subpattern(InstrKind::Wave, _)
        ) {
            for cell in &mut pattern.0 {
                let AnnotatedCell { reachable: true, cell } = cell else { continue; };
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

// This algorithm is described in the README.
fn find_pattern_overlap(
    patterns: &HashMap<PatternId, OptimisedPattern>,
) -> (Vec<(PatternId, usize)>, usize) {
    // A hashmap's keys are not guaranteed to be returned in a consistent order, so collect them to ensure that.
    let pattern_ids: Vec<_> = patterns.keys().cloned().collect();
    let nb_patterns = patterns.len();

    let overlap_amount = |ordering: &[(PatternId, usize)], idx_to_append: PatternId| {
        let to_append = &patterns[&idx_to_append].0;
        let first_row = to_append
            .first()
            .expect("Entirely unreachable patterns should have been pruned!?");
        let last_pattern = &patterns[&ordering.last().expect("Orderings can't be empty!?").0].0;

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
    // TODO: two separate allocations? Meh...
    let mut prev_row = vec![None; nb_patterns]; // We just need to init this somehow.
    let mut new_row = pattern_ids
        .iter()
        .cloned()
        .map(|i| {
            let mut vec = Vec::with_capacity(nb_patterns);
            vec.push((i, 0));
            Some((vec, 0))
        })
        .collect();

    // Now for all the other iterations!
    for _ in 1..nb_patterns {
        std::mem::swap(&mut prev_row, &mut new_row); // Putting this first helps type deduction :3

        for (i, slot) in pattern_ids.iter().cloned().zip(new_row.iter_mut()) {
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
        .flatten()
        .max_by_key(|(_, score)| *score)
        .expect("How come no ordering survived!?")
}

fn generate_cell_pool(
    patterns: &HashMap<PatternId, OptimisedPattern>,
    pattern_ordering: &[(PatternId, usize)],
) -> Vec<OutputCell> {
    let mut rows = Vec::new();

    for (pattern_id, overlapped_rows) in pattern_ordering.iter().cloned() {
        let pattern = &patterns[&pattern_id];
        rows.push(OutputCell::Label(pattern_id));
        rows.extend(
            pattern.0[..pattern.0.len() - overlapped_rows]
                .iter()
                .map(|cell| OutputCell::Cell(cell.cell)),
        );
        if overlapped_rows != 0 {
            rows.push(OutputCell::OverlapMarker(overlapped_rows));
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
        !self.reachable /* || !other.reachable */
            || (other.reachable && self.cell == other.cell)
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
