use std::{borrow::Cow, num::NonZeroU8};

#[derive(Debug, Clone, Copy, Default)]
pub struct Cell {
    note: u8,
    instr_and_fx: u8,
    fx_params: u8,
}

impl Cell {
    pub fn new(note: u8, instr: u8, fx_code: u8, fx_params: u8) -> Self {
        debug_assert_eq!(instr & 0xF0, 0);
        debug_assert_eq!(fx_code & 0xF0, 0);
        Self {
            note,
            instr_and_fx: (instr << 4) + fx_code,
            fx_params,
        }
    }

    pub fn note(&self) -> u8 {
        self.note
    }

    pub fn instr(&self) -> u8 {
        self.instr_and_fx >> 4
    }

    pub fn fx_code(&self) -> u8 {
        self.instr_and_fx & 0x0F
    }

    pub fn fx_params(&self) -> u8 {
        self.fx_params
    }
}

pub type Pattern = [Cell; 64];

#[derive(Debug, Clone, Copy)]
pub enum EnvelopeDirection {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy)]
pub enum SweepDirection {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy)]
pub enum DutyType {
    Percent12_5,
    Percent25,
    Percent50,
    Percent75,
}

#[derive(Debug, Clone, Copy)]
pub enum WaveOutputLevel {
    Mute,
    Full,
    Half,
    Quarter,
}

#[derive(Debug, Clone, Copy)]
pub enum LfsrWidth {
    Fifteen,
    Seven,
}

#[derive(Debug, Clone)]
pub enum InstrumentKind {
    Square {
        initial_volume: u8,
        envelope_dir: EnvelopeDirection,
        envelope_pace: u8,
        sweep_time: u8,
        sweep_dir: SweepDirection,
        sweep_shift: u8,
        duty: DutyType,
    },
    Wave {
        output_level: WaveOutputLevel,
        waveform: u8,
    },
    Noise {
        initial_volume: u8,
        envelope_dir: EnvelopeDirection,
        envelope_pace: u8,
        lfsr_width: LfsrWidth,
    },
}

impl Default for InstrumentKind {
    fn default() -> Self {
        Self::Wave {
            output_level: WaveOutputLevel::Mute,
            waveform: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Instrument<'input> {
    pub name: Cow<'input, str>,
    pub length: Option<NonZeroU8>,
    pub kind: InstrumentKind,
    pub subpattern: Option<Pattern>,
}

pub type InstrumentBank<'input> = [Instrument<'input>; 15];

#[derive(Debug, Clone)]
pub struct InstrCollection<'input> {
    pub duty: InstrumentBank<'input>,
    pub wave: InstrumentBank<'input>,
    pub noise: InstrumentBank<'input>,
}

pub type Wave = [u8; 16];

pub type WaveBank = [Wave; 16];

pub type Routine<'input> = Cow<'input, str>;

pub type RoutineBank<'input> = [Routine<'input>; 16];

#[derive(Debug, Clone)]
pub struct Song<'input> {
    pub name: Cow<'input, str>,
    pub artist: Cow<'input, str>,
    pub comment: Cow<'input, str>,

    pub instruments: InstrCollection<'input>,
    pub waves: WaveBank,

    pub ticks_per_row: u8,

    pub timer_divider: Option<NonZeroU8>,

    pub patterns: Vec<Pattern>,
    pub order_matrix: Vec<[u8; 4]>,
    pub routines: RoutineBank<'input>,
}
