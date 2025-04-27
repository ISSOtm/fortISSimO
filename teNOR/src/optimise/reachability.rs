use crate::song::{EffectId, Song};

use super::{CellFirstHalf, Effect, InstrKind, OptimisedPattern, PatternId, PatternStore};

pub(super) fn mark_reachable_pattern_rows(
    song: &Song,
    patterns: &mut PatternStore,
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
                } => next_order = Some((param - 1).into()),
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
            let CellFirstHalf::Pattern {
                note: _,
                instrument,
            } = cell.cell.0
            else {
                unreachable!();
            };
            *match i {
                0 | 1 => &mut used_duty_instrs,
                2 => &mut used_wave_instrs,
                3 => &mut used_noise_instrs,
                _ => unreachable!(),
            } |= 1 << instrument;
        }

        // Go to the next row, or follow the overrides if any are set.
        if let Some(order) = next_order {
            row_index = next_row.map_or(0, |row: usize| row - 1);
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

pub(super) fn mark_reachable_subpattern_rows(
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

pub(super) fn trim_trailing_unreachable_rows(
    patterns: &mut PatternStore,
    pruned_patterns: &mut usize,
    pruned_pattern_rows: &mut usize,
    trimmed_rows: &mut usize,
) {
    // FIXME: this is not very ergonomic, but required since `Hashmap::drain_filter()` is not stable yet.
    // https://github.com/rust-lang/rust/issues/59618
    let keys: Vec<_> = patterns.keys().cloned().collect();

    for key in keys.iter().cloned() {
        let std::collections::hash_map::Entry::Occupied(mut entry) = patterns.entry(key) else {
            unreachable!();
        };

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
