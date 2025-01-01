use super::*;

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
