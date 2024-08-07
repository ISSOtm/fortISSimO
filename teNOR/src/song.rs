use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct Song<'input> {
    pub name: Cow<'input, str>,
    pub artist: Cow<'input, str>,
    pub comment: Cow<'input, str>,

    pub instruments: InstrCollection<'input>,
    pub waves: WaveBank,

    pub ticks_per_row: u8,

    pub timer_divider: Option<u8>,

    pub patterns: Vec<Pattern>,
    pub order_matrix: Vec<[usize; 4]>,
    #[allow(dead_code)]
    pub routines: RoutineBank<'input>,
}

#[derive(Debug, Clone)]
pub struct InstrCollection<'input> {
    pub duty: InstrumentBank<'input>,
    pub wave: InstrumentBank<'input>,
    pub noise: InstrumentBank<'input>,
}

pub type InstrumentBank<'input> = [Instrument<'input>; 15];

#[derive(Debug, Clone, Default)]
pub struct Instrument<'input> {
    pub name: Cow<'input, str>,
    pub length: Option<u8>,
    pub kind: InstrumentKind,
    pub subpattern: Option<Subpattern>,
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
        wave_id: u8,
    },
    Noise {
        initial_volume: u8,
        envelope_dir: EnvelopeDirection,
        envelope_pace: u8,
        lfsr_width: LfsrWidth,
    },
}

impl InstrumentKind {
    pub fn data_size(&self) -> usize {
        match self {
            Self::Square { .. } => 6,
            Self::Wave { .. } => 6,
            Self::Noise { .. } => 4,
        }
    }
}

impl Default for InstrumentKind {
    fn default() -> Self {
        Self::Wave {
            output_level: WaveOutputLevel::Mute,
            wave_id: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnvelopeDirection {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SweepDirection {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DutyType {
    Percent12_5,
    Percent25,
    Percent50,
    Percent75,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaveOutputLevel {
    Mute,
    Full,
    Half,
    Quarter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LfsrWidth {
    Fifteen,
    Seven,
}

pub type Wave = [u8; 16];

pub type WaveBank = [Wave; 16];

pub type Routine<'input> = Cow<'input, str>;

pub type RoutineBank<'input> = [Routine<'input>; 16];

pub type Pattern = [PatternCell; 64];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PatternCell {
    pub note: Note,
    pub instrument: u8,
    pub effect_code: EffectId,
    pub effect_param: u8,
}

pub type Subpattern = [SubpatternCell; 32];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SubpatternCell {
    pub offset: u8,
    pub next_row_idx: u8,
    pub effect_code: EffectId,
    pub effect_param: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(non_camel_case_types)] // We're trying to mirror the ASM constants' names.
pub enum Note {
    C_3,
    CSharp3,
    D_3,
    DSharp3,
    E_3,
    F_3,
    FSharp3,
    G_3,
    GSharp3,
    A_3,
    ASharp3,
    B_3,
    C_4,
    CSharp4,
    D_4,
    DSharp4,
    E_4,
    F_4,
    FSharp4,
    G_4,
    GSharp4,
    A_4,
    ASharp4,
    B_4,
    C_5,
    CSharp5,
    D_5,
    DSharp5,
    E_5,
    F_5,
    FSharp5,
    G_5,
    GSharp5,
    A_5,
    ASharp5,
    B_5,
    C_6,
    CSharp6,
    D_6,
    DSharp6,
    E_6,
    F_6,
    FSharp6,
    G_6,
    GSharp6,
    A_6,
    ASharp6,
    B_6,
    C_7,
    CSharp7,
    D_7,
    DSharp7,
    E_7,
    F_7,
    FSharp7,
    G_7,
    GSharp7,
    A_7,
    ASharp7,
    B_7,
    C_8,
    CSharp8,
    D_8,
    DSharp8,
    E_8,
    F_8,
    FSharp8,
    G_8,
    GSharp8,
    A_8,
    ASharp8,
    B_8,
    #[default]
    None = 90,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EffectId {
    #[default]
    Arpeggio = 0x0,
    PortaUp = 0x1,
    PortaDown = 0x2,
    TonePorta = 0x3,
    Vibrato = 0x4,
    SetMasterVol = 0x5,
    CallRoutine = 0x6,
    NoteDelay = 0x7,
    SetPanning = 0x8,
    ChangeTimbre = 0x9,
    VolSlide = 0xA,
    PosJump = 0xB,
    SetVol = 0xC,
    PatternBreak = 0xD,
    NoteCut = 0xE,
    SetTempo = 0xF,
}
