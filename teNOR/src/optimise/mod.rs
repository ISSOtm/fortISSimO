use std::{collections::HashMap, hash::Hash};

use crate::{
    song::{EffectId, Instrument, InstrumentKind, Note, PatternCell, Song, SubpatternCell},
    PATTERN_LENGTH,
};

mod overlapping;
use overlapping::*;
mod reachability;
use reachability::*;
mod remapping;
use remapping::*;

pub fn optimise(song: &Song) -> (OptimResults, OptimStats) {
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
    // Instruments' waves are remapped during export.

    // TODO: pattern deduplication (including finding patterns "in the middle of" of others) would
    //       cut down on the number of patterns, and potentially speed up following steps.
    let (row_pool_builder, overlapped_rows) = find_pattern_overlap(&patterns);
    let (row_pool, cell_map, saved_bytes_catalog) = generate_row_pool(row_pool_builder);

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
        saved_bytes_catalog,
    };

    (
        OptimResults {
            row_pool,
            cell_catalog: cell_map,
            duty_instr_usage,
            wave_instr_usage,
            noise_instr_usage,
            wave_usage,
        },
        stats,
    )
}

#[derive(Debug)]
pub struct OptimResults {
    pub row_pool: Vec<OutputCell>,
    pub cell_catalog: HashMap<Cell, u8>,
    pub duty_instr_usage: CompactedMapping<15>,
    pub wave_instr_usage: CompactedMapping<15>,
    pub noise_instr_usage: CompactedMapping<15>,
    pub wave_usage: CompactedMapping<16>,
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
    pub saved_bytes_catalog: isize,
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
        + self.saved_bytes_catalog
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OutputCell {
    Label(PatternId),
    Cell(u8),
    OverlapMarker(usize),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell(pub CellFirstHalf, pub Effect);

impl Cell {
    // TODO: add a "validation" function, and report invalid cells during export.
    //       toneporta requires a note
    //       pos_jump's arg must be non-zero and in-bounds
    //       pattern break: same

    pub fn first_byte(&self) -> u8 {
        match self.1 {
            Effect {
                id: EffectId::PosJump,
                param,
            } => {
                // This is 1-based in the tracker. Converting to `wOrderIdx` format requires subtracting 1.
                (param - 1)
                    // In addition, "pos jump" sets `wForceRow`, which causes the order index to be advanced; compensate for that as well.
                    .wrapping_sub(1)
                    // And, convert to "byte offset".
                    .wrapping_mul(2)
            }

            Effect {
                id: EffectId::SetVol,
                param,
            } => {
                match (param >> 4, param & 0x0F) {
                    // This would kill the channel; only mute it, but keep the DAC active.
                    (envelope @ 1.., 0) if envelope & 8 == 0 => 0x08,
                    // Swap the nibbles to match NRx2 order.
                    (envelope, volume) => volume << 4 | envelope,
                }
            }

            Effect {
                id: EffectId::PatternBreak,
                param,
            } => {
                debug_assert!(PATTERN_LENGTH.is_power_of_two()); // So that the bit inversion below is correct.
                debug_assert!(param - 1 < PATTERN_LENGTH);

                // This is 1-based in the tracker. Converting to `wForceRow` format requires subtracting 1.
                (param - 1)
                // Convert to `wForceRow` format.
                | 0u8.wrapping_sub(PATTERN_LENGTH)
            }

            // Catch-all.
            Effect { id: _, param } => param,
        }
    }

    pub fn second_byte(&self) -> u8 {
        (match self.0 {
            CellFirstHalf::Pattern { instrument, .. } => instrument,
            CellFirstHalf::Subpattern { next_row_idx, .. } => next_row_idx & 0x0F,
        }) << 4
            | self.1.id as u8
    }

    pub fn third_byte(&self) -> u8 {
        match self.0 {
            CellFirstHalf::Pattern { note, .. } => note as u8,
            CellFirstHalf::Subpattern {
                offset,
                next_row_idx,
            } => offset << 1 | (next_row_idx & 0x10) >> 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq)]
pub enum CellFirstHalf {
    Pattern { note: Note, instrument: u8 },
    Subpattern { offset: u8, next_row_idx: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl CellFirstHalf {
    fn as_raw(&self) -> (u8, u8) {
        match self {
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
}
impl PartialEq for CellFirstHalf {
    fn eq(&self, other: &Self) -> bool {
        self.as_raw() == other.as_raw()
    }
}
impl Hash for CellFirstHalf {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_raw().hash(state)
    }
}
