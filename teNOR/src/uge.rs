//! This module is entirely concerned with deserialising `.uge` files.
//! The type definitions are extracted from `hugedatatypes.pas` and `song.pas`.

use std::{borrow::Cow, fmt::Display, num::TryFromIntError};

use nom::{
    bytes::complete::take,
    combinator::all_consuming,
    error::context,
    multi::{fill, length_count},
    number::complete::le_u32,
    Finish, IResult,
};

use crate::song::{
    EffectId, InstrCollection, Instrument, InstrumentBank, InstrumentKind, Pattern, Routine,
    RoutineBank, Song, Subpattern, Wave, WaveBank,
};

type PResult<'input, O> = IResult<&'input [u8], O, InnerError<'input>>;

pub fn parse_song(input: &[u8]) -> Result<Song<'_>, ParseError<'_>> {
    let (song_input, version) = integer(input).map_err(|_| ParseErrorKind::NotUge)?;

    let parser = match version {
        n @ 0..=5 => return Err(ParseErrorKind::UnsupportedVersion(n).into()),
        6 => song_v6,
        n @ 7.. => return Err(ParseErrorKind::TooNew(n).into()),
    };

    match all_consuming(parser)(song_input).finish() {
        Ok((_input, song)) => Ok(song),
        Err(inner) => Err(ParseErrorKind::BadData { input, inner }.into()),
    }
}

#[derive(Debug, Clone)]
pub struct ParseError<'input>(ParseErrorKind<'input>);

#[derive(Debug, Clone)]
enum ParseErrorKind<'input> {
    NotUge,
    UnsupportedVersion(u32),
    TooNew(u32),
    BadData {
        input: &'input [u8],
        inner: InnerError<'input>,
    },
}

impl<'input> From<ParseErrorKind<'input>> for ParseError<'input> {
    fn from(value: ParseErrorKind<'input>) -> Self {
        Self(value)
    }
}

impl Display for ParseError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            ParseErrorKind::NotUge => write!(f, "This is too short to be a UGE file"),
            ParseErrorKind::UnsupportedVersion(n) => write!(f, "UGE version {n} is not supported; please open and save this in hUGETracker to upgrade it"),
            ParseErrorKind::TooNew(n) => write!(f, "UGE version {n} is not supported; please pester someone to update teNOR!"),
            ParseErrorKind::BadData { input, inner: InnerError(errors) } => (|| {
                writeln!(f, "There was an error parsing the UGE file!")?;
                let base_ptr = input.as_ptr();
                for (input, kind) in errors {
                    // SAFETY: both pointers originate from the same slice, which is still live (we keep holding `input`).
                    writeln!(f, "\t(0x{:<4x} bytes into the data) {kind}", unsafe { input.as_ptr().offset_from(base_ptr) })?;
                }
                write!(f, "(Either the file is corrupted, or our UGE parser has a bug. If the latter, please attach your UGE file and the above in your bug report!)")
            })(),
        }
    }
}

fn song_v6(input: &[u8]) -> PResult<Song<'_>> {
    fn inner(input: &[u8]) -> PResult<Song<'_>> {
        let (input, name) = short_string(input)?;
        let (input, artist) = short_string(input)?;
        let (input, comment) = short_string(input)?;
        let (input, instruments) = instr_collection_v3(input)?;
        let (input, waves) = wave_bank_v2(input)?;
        let (input, ticks_per_row) = try_convert(input, integer)?;
        let (input, timer_enabled) = boolean(input)?;
        let (new_input, timer_divider) = integer(input)?;
        let timer_divider = timer_divider
            .try_into()
            .map_err(|_| InnerError::err(input, InnerErrorKind::BadTimerDivider(timer_divider)))?;
        let (input, patterns) = pattern_map_v2(new_input)?;
        let (input, order_matrix) = order_matrix(input)?;
        let (input, routines) = routine_bank(input)?;

        Ok((
            input,
            Song {
                name,
                artist,
                comment,
                instruments,
                waves,
                ticks_per_row,
                timer_divider: timer_enabled.then_some(timer_divider),
                patterns,
                order_matrix,
                routines,
            },
        ))
    }
    context("parsing v6 song from here", inner)(input)
}

// Instruments.

fn instr_collection_v3(input: &[u8]) -> PResult<InstrCollection<'_>> {
    fn inner(input: &[u8]) -> PResult<InstrCollection<'_>> {
        let (input, duty) = instr_bank_v3(input)?;
        if cfg!(debug_assertions) {
            if let Some((i, instr)) = duty
                .iter()
                .enumerate()
                .find(|(_i, instr)| !matches!(instr.kind, InstrumentKind::Square { .. }))
            {
                panic!("Wrong kind for duty instr #{i}! ({:?})", instr.kind);
            }
        }
        let (input, wave) = instr_bank_v3(input)?;
        if cfg!(debug_assertions) {
            if let Some((i, instr)) = wave
                .iter()
                .enumerate()
                .find(|(_i, instr)| !matches!(instr.kind, InstrumentKind::Wave { .. }))
            {
                panic!("Wrong kind for wave instr #{i}! ({:?})", instr.kind);
            }
        }
        let (input, noise) = instr_bank_v3(input)?;
        if cfg!(debug_assertions) {
            if let Some((i, instr)) = noise
                .iter()
                .enumerate()
                .find(|(_i, instr)| !matches!(instr.kind, InstrumentKind::Noise { .. }))
            {
                panic!("Wrong kind for noise instr #{i}! ({:?})", instr.kind);
            }
        }

        Ok((input, InstrCollection { duty, wave, noise }))
    }
    context("parsing v3 instr collection from here", inner)(input)
}

fn instr_bank_v3(input: &[u8]) -> PResult<InstrumentBank<'_>> {
    fn inner(input: &[u8]) -> PResult<InstrumentBank<'_>> {
        let mut bank = std::array::from_fn(|_| Default::default());
        let (input, ()) = fill(instrument_v3, &mut bank)(input)?;
        Ok((input, bank))
    }
    context("parsing v3 instrument bank from here", inner)(input)
}

fn instrument_v3(input: &[u8]) -> PResult<Instrument<'_>> {
    fn inner(input: &[u8]) -> PResult<Instrument<'_>> {
        let kind_input = input;
        let (input, kind) = nom::number::complete::le_u32(input)?;
        let (input, name) = short_string(input)?;
        let (new_input, length) = try_convert(input, integer)?;
        let (input, length_enabled) = boolean(new_input)?;
        let (input, initial_volume) = nom::number::complete::u8(input)?;
        let (input, envelope_dir) = try_convert(input, nom::number::complete::le_u32)?;
        let (input, envelope_pace) = nom::number::complete::u8(input)?;
        let (input, sweep_time) = try_convert(input, nom::number::complete::le_u32)?;
        let (input, sweep_dir) = try_convert(input, nom::number::complete::le_u32)?;
        let (input, sweep_shift) = try_convert(input, nom::number::complete::le_u32)?;
        let (input, duty) = try_convert(input, nom::number::complete::u8)?;
        let (input, output_level) = try_convert(input, nom::number::complete::le_u32)?;
        let (input, waveform) = try_convert(input, nom::number::complete::le_u32)?;
        let (input, lfsr_width) = try_convert(input, nom::number::complete::le_u32)?;
        let (input, subpattern_enabled) = boolean(input)?;
        let (input, subpattern) = subpattern_v2(input)?;

        Ok((
            input,
            Instrument {
                name,
                length: length_enabled.then_some(length),
                kind: match kind {
                    0 => InstrumentKind::Square {
                        initial_volume,
                        envelope_dir,
                        envelope_pace,
                        sweep_time,
                        sweep_dir,
                        sweep_shift,
                        duty,
                    },
                    1 => InstrumentKind::Wave {
                        output_level,
                        wave_id: waveform,
                    },
                    2 => InstrumentKind::Noise {
                        initial_volume,
                        envelope_dir,
                        envelope_pace,
                        lfsr_width,
                    },
                    n => return Err(InnerError::err(kind_input, InnerErrorKind::BadInstrType(n))),
                },
                subpattern: subpattern_enabled.then_some(subpattern),
            },
        ))
    }
    context("parsing v3 instrument from here", inner)(input)
}

// Waves.

fn wave_bank_v2(input: &[u8]) -> PResult<WaveBank> {
    fn inner(input: &[u8]) -> PResult<WaveBank> {
        let mut bank = [Default::default(); 16];
        let (input, ()) = fill(wave_v2, &mut bank)(input)?;
        Ok((input, bank))
    }
    context("parsing v2 wave bank from here", inner)(input)
}

fn wave_v2(input: &[u8]) -> PResult<Wave> {
    fn inner(wave_input: &[u8]) -> PResult<Wave> {
        let (input, raw_wave) = take(32u8)(wave_input)?;

        let sanitize = |index| {
            let raw = raw_wave[index];
            if raw & 0xF0 != 0 {
                Err(InnerError::err(
                    &wave_input[index..],
                    InnerErrorKind::BadWave(raw),
                ))
            } else {
                Ok(raw)
            }
        };
        let mut wave = [Default::default(); 16];
        for (i, byte) in wave.iter_mut().enumerate() {
            *byte = (sanitize(i * 2)? << 4) + sanitize(i * 2 + 1)?;
        }
        Ok((input, wave))
    }
    context("parsing v2 wave from here", inner)(input)
}

// Patterns.

fn pattern_map_v2(input: &[u8]) -> PResult<Vec<Pattern>> {
    fn inner(input: &[u8]) -> PResult<Vec<Pattern>> {
        let (mut input, nb_entries) = try_convert(input, integer)?;
        let mut patterns = Vec::with_capacity(nb_entries);
        for _ in 0..nb_entries {
            let (new_input, (id, cells)) = pattern_map_entry_v2(input)?;
            input = new_input;
            if patterns.len() <= id {
                patterns.resize_with(id + 1, || std::array::from_fn(|_| Default::default()));
            }
            patterns[id] = cells;
        }
        Ok((input, patterns))
    }
    context("parsing v2 pattern map from here", inner)(input)
}

fn pattern_map_entry_v2(input: &[u8]) -> PResult<(usize, Pattern)> {
    fn inner(input: &[u8]) -> PResult<(usize, Pattern)> {
        let (input, id) = try_convert(input, integer)?;
        let (input, cells) = pattern_v2(input)?;
        Ok((input, (id, cells)))
    }
    context("parsing v2 pattern map entry from here", inner)(input)
}

fn pattern_v2(input: &[u8]) -> PResult<Pattern> {
    fn inner(input: &[u8]) -> PResult<Pattern> {
        let mut pattern = [Default::default(); 64];
        let (input, ()) = fill(|input| try_convert(input, cell_v2), &mut pattern)(input)?;
        Ok((input, pattern))
    }
    context("parsing v2 pattern from here", inner)(input)
}

fn subpattern_v2(input: &[u8]) -> PResult<Subpattern> {
    fn inner(input: &[u8]) -> PResult<Subpattern> {
        let mut pattern: Subpattern = [Default::default(); 32];
        let (mut input, ()) = fill(|input| try_convert(input, cell_v2), &mut pattern)(input)?;
        // The remainder of the pattern is encoded, but not used.
        for _ in 32..64 {
            (input, _) = cell_v2(input)?;
        }
        // Adjust the jump targets. There is actually a reason for doing this!
        //
        // hUGETracker encodes them as "0 for no jump, otherwise the target column, 1-indexed".
        // However, there are only 5 bits to encode this information, which introduces a subtle bug:
        // 32 needs 6 bits to be encoded!
        // hUGETracker's export emits "32" verbatim, and hUGEDriver's `dn` macro silently truncates
        // that to 0, meaning "no jump".
        // We fix this by making each row *unconditionally* jump! The 32 row IDs fit in 5 bits.
        for (i, cell) in pattern.iter_mut().enumerate() {
            cell.next_row_idx = match cell.next_row_idx {
                0 => (i as u8 + 1) % 32, // i is in 0..32
                n => n - 1,
            };
        }
        Ok((input, pattern))
    }
    context("parsing v2 (sub)pattern from here", inner)(input)
}

#[derive(Debug, Clone, Copy, Default)]
struct RawCell {
    /// Actually a note offset in subpatterns, so this is not a `Note`.
    note: u8,
    /// Unused in subpatterns.
    instrument: u8,
    /// For subpatterns only.
    jump_index: u8,
    effect_code: EffectId,
    effect_params: u8,
}

impl RawCell {
    fn try_new(
        note: u8,
        instrument: u8,
        jump_index: u8,
        effect_code: EffectId,
        effect_params: u8,
    ) -> Result<Self, InnerErrorKind> {
        if instrument >= 16 {
            Err(InnerErrorKind::BadInstrument(instrument))
        } else {
            Ok(Self {
                note,
                instrument,
                jump_index,
                effect_code,
                effect_params,
            })
        }
    }
}

fn cell_v2(input: &[u8]) -> PResult<RawCell> {
    fn inner(input: &[u8]) -> PResult<RawCell> {
        let (input, note) = try_convert(input, integer)?;
        let (input, instrument) = try_convert(input, integer)?;
        let (input, jump_index) = try_convert(input, integer)?;
        let (input, effect_code) = try_convert(input, integer)?;
        let (input, effect_params) = nom::number::complete::u8(input)?;

        Ok((
            input,
            RawCell::try_new(note, instrument, jump_index, effect_code, effect_params)
                .map_err(|err_kind| InnerError::err(input, err_kind))?,
        ))
    }
    context("parsing v2 cell from here", inner)(input)
}

// Order.

fn order_matrix(input: &[u8]) -> PResult<Vec<[usize; 4]>> {
    fn inner(input: &[u8]) -> PResult<Vec<[usize; 4]>> {
        let mut orders = std::array::from_fn(|_| Default::default());
        let (input, ()) = fill(order_column, &mut orders)(input)?;

        let [ch1, ch2, ch3, ch4] = orders;
        match (ch1.len(), ch2.len(), ch3.len(), ch4.len()) {
            (len1, len2, len3, len4) if len1 == len2 && len2 == len3 && len3 == len4 => Ok((
                input,
                (0..len1 - 1) // For some reason, hUGE stores one extra zero per "column".
                    .map(|i| [ch1[i], ch2[i], ch3[i], ch4[i]])
                    .collect(),
            )),
            (len1, len2, len3, len4) => Err(InnerError::err(
                input,
                InnerErrorKind::OrderNotMatrix(len1, len2, len3, len4),
            )),
        }
    }
    context("parsing order matrix from here", inner)(input)
}

fn order_column(input: &[u8]) -> PResult<Vec<usize>> {
    context(
        "parsing order \"column\" from here",
        length_count(integer, |input| try_convert(input, integer)),
    )(input)
}

// Routines.

fn routine_bank(input: &[u8]) -> PResult<RoutineBank> {
    fn inner(input: &[u8]) -> PResult<RoutineBank> {
        let mut routines = std::array::from_fn(|_| Default::default());
        let (input, ()) = fill(routine, &mut routines)(input)?;
        Ok((input, routines))
    }
    context("parsing routine bank from here", inner)(input)
}

fn routine(input: &[u8]) -> PResult<Routine> {
    context("parsing routine from here", ansi_string)(input)
}

// Elementary types.

type Integer = u32;
fn integer(input: &[u8]) -> PResult<Integer> {
    context("parsing Integer from here", le_u32)(input)
}

fn boolean(input: &[u8]) -> PResult<bool> {
    context("parsing Boolean from here", nom::number::complete::u8)(input).and_then(|(input, n)| {
        match n {
            0 => Ok((input, false)),
            1 => Ok((input, true)),
            n => Err(InnerError::err(input, InnerErrorKind::BadBool(n))),
        }
    })
}

fn short_string(input: &[u8]) -> PResult<Cow<'_, str>> {
    fn inner(input: &[u8]) -> PResult<Cow<'_, str>> {
        let (input, len) = nom::number::complete::u8(input)?;
        take(255u8)(input).map(|(input, raw)| (input, String::from_utf8_lossy(&raw[..len.into()])))
    }
    context("parsing ShortString from here", inner)(input)
}

fn ansi_string(input: &[u8]) -> PResult<Cow<'_, str>> {
    fn inner(input: &[u8]) -> PResult<Cow<'_, str>> {
        let (input, len) = try_convert(input, nom::number::complete::le_u32)?;
        take(len)(input).map(|(input, raw)| (input, String::from_utf8_lossy(&raw[..len])))
    }
    context("parsing AnsiString from here", inner)(input)
}

// Error handling.

#[derive(Debug, Clone)]
struct InnerError<'input>(Vec<(&'input [u8], InnerErrorKind)>);

impl<'input> InnerError<'input> {
    fn err(input: &'input [u8], err_kind: InnerErrorKind) -> nom::Err<Self> {
        nom::Err::Error(Self(vec![(input, err_kind)]))
    }
}

impl<'input> nom::error::ParseError<&'input [u8]> for InnerError<'input> {
    fn from_error_kind(input: &'input [u8], kind: nom::error::ErrorKind) -> Self {
        Self(vec![(input, InnerErrorKind::Nom(kind))])
    }

    fn append(input: &'input [u8], kind: nom::error::ErrorKind, other: Self) -> Self {
        let Self(mut errors) = other;
        errors.push((input, InnerErrorKind::Nom(kind)));
        Self(errors)
    }
}

impl<'input> nom::error::ContextError<&'input [u8]> for InnerError<'input> {
    fn add_context(input: &'input [u8], ctx: &'static str, mut other: Self) -> Self {
        other.0.push((input, InnerErrorKind::Context(ctx)));
        other
    }
}

#[derive(Debug, Clone)]
enum InnerErrorKind {
    BadBool(u8),
    BadTimerDivider(u32),
    BadInstrType(u32),
    BadEnvDir(u32),
    BadSweepDir(u32),
    BadDutyType(u8),
    BadWaveOutLevel(u32),
    BadLfsrWidth(u32),
    BadNote(u32),
    BadInstrument(u8),
    BadEffectId(u32),
    BadWave(u8),
    OrderNotMatrix(usize, usize, usize, usize),
    NumOutOfRange(TryFromIntError),
    Context(&'static str),
    Nom(nom::error::ErrorKind),
}

impl Display for InnerErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadBool(n) => write!(f, "Boolean out of range (0x{n:08x})"),
            Self::BadTimerDivider(n) => write!(f, "Timer divider out of range (0x{n:08x})"),
            Self::BadInstrType(n) => write!(f, "Instrument type out of range (0x{n:08x})"),
            Self::BadEnvDir(n) => write!(f, "Envelope direction out of range (0x{n:08x})"),
            Self::BadSweepDir(n) => write!(f, "Sweep direction out of range (0x{n:08x})"),
            Self::BadDutyType(n) => write!(f, "Duty type out of range (0x{n:08x})"),
            Self::BadWaveOutLevel(n) => write!(f, "Wave output level out of range (0x{n:08x})"),
            Self::BadLfsrWidth(n) => write!(f, "LFSR width out of range (0x{n:08x})"),
            Self::BadNote(n) => write!(f, "Note out of range (0x{n:08x})"),
            Self::BadInstrument(n) => write!(f, "Instrument out of range (0x{n:02x}))"),
            Self::BadEffectId(n) => write!(f, "Effect ID out of range (0x{n:08x})"),
            Self::BadWave(raw) => write!(f, "Wave sample out of range (0x{raw:02x})"),
            Self::OrderNotMatrix(ch1, ch2, ch3, ch4) => write!(
                f,
                "Length of order \"columns\" don't match! ({ch1}, {ch2}, {ch3}, {ch4})"
            ),
            Self::NumOutOfRange(err) => write!(f, "Number out of range: {err}"),
            Self::Context(ctx) => f.write_str(ctx),
            Self::Nom(err) => write!(f, "Error in parser \"{}\"", err.description()),
        }
    }
}

impl From<TryFromIntError> for InnerErrorKind {
    fn from(value: TryFromIntError) -> Self {
        Self::NumOutOfRange(value)
    }
}

// Conversion to `song` enumerations.

/// Conversion from a generic error to something Nom can process.
fn try_convert<'input, T, U: TryConstrain<T>, F: Fn(&'input [u8]) -> PResult<'input, U>>(
    input: &'input [u8],
    parser: F,
) -> PResult<'input, T> {
    let (remaining, raw) = parser(input)?;
    match raw.try_constrain() {
        Ok(t) => Ok((remaining, t)),
        Err(err) => Err(InnerError::err(input, err)),
    }
}

trait TryConstrain<T> {
    fn try_constrain(self) -> Result<T, InnerErrorKind>;
}

impl<E: Into<InnerErrorKind>, T, U: TryInto<T, Error = E>> TryConstrain<T> for U {
    fn try_constrain(self) -> Result<T, InnerErrorKind> {
        self.try_into().map_err(Into::into)
    }
}

impl TryConstrain<crate::song::EnvelopeDirection> for u32 {
    fn try_constrain(self) -> Result<crate::song::EnvelopeDirection, InnerErrorKind> {
        use crate::song::EnvelopeDirection::*;

        match self {
            0 => Ok(Up),
            1 => Ok(Down),
            n => Err(InnerErrorKind::BadEnvDir(n)),
        }
    }
}

impl TryConstrain<crate::song::SweepDirection> for u32 {
    fn try_constrain(self) -> Result<crate::song::SweepDirection, InnerErrorKind> {
        use crate::song::SweepDirection::*;

        match self {
            0 => Ok(Up),
            1 => Ok(Down),
            n => Err(InnerErrorKind::BadSweepDir(n)),
        }
    }
}

impl TryConstrain<crate::song::DutyType> for u8 {
    fn try_constrain(self) -> Result<crate::song::DutyType, InnerErrorKind> {
        use crate::song::DutyType::*;

        match self {
            0 => Ok(Percent12_5),
            1 => Ok(Percent25),
            2 => Ok(Percent50),
            3 => Ok(Percent75),
            n => Err(InnerErrorKind::BadDutyType(n)),
        }
    }
}

impl TryConstrain<crate::song::WaveOutputLevel> for u32 {
    fn try_constrain(self) -> Result<crate::song::WaveOutputLevel, InnerErrorKind> {
        use crate::song::WaveOutputLevel::*;

        match self {
            0 => Ok(Mute),
            1 => Ok(Full),
            2 => Ok(Half),
            3 => Ok(Quarter),
            n => Err(InnerErrorKind::BadWaveOutLevel(n)),
        }
    }
}

impl TryConstrain<crate::song::LfsrWidth> for u32 {
    fn try_constrain(self) -> Result<crate::song::LfsrWidth, InnerErrorKind> {
        use crate::song::LfsrWidth::*;

        match self {
            0 => Ok(Fifteen),
            1 => Ok(Seven),
            n => Err(InnerErrorKind::BadLfsrWidth(n)),
        }
    }
}

impl TryConstrain<crate::song::PatternCell> for RawCell {
    fn try_constrain(self) -> Result<crate::song::PatternCell, InnerErrorKind> {
        let note = u32::from(self.note).try_constrain()?;
        let instrument = self.instrument;
        let effect_code = self.effect_code;
        let effect_param = self.effect_params;
        Ok(crate::song::PatternCell {
            note,
            instrument,
            effect_code,
            effect_param,
        })
    }
}

impl TryConstrain<crate::song::SubpatternCell> for RawCell {
    fn try_constrain(self) -> Result<crate::song::SubpatternCell, InnerErrorKind> {
        let offset = self.note;
        // hUGETracker clamps an index greater than 32 as that.
        let next_row_idx = self.jump_index.min(32);
        let effect_code = self.effect_code;
        let effect_param = self.effect_params;
        Ok(crate::song::SubpatternCell {
            offset,
            next_row_idx,
            effect_code,
            effect_param,
        })
    }
}

impl TryConstrain<crate::song::EffectId> for u32 {
    fn try_constrain(self) -> Result<crate::song::EffectId, InnerErrorKind> {
        match self {
            0x0 => Ok(EffectId::Arpeggio),
            0x1 => Ok(EffectId::PortaUp),
            0x2 => Ok(EffectId::PortaDown),
            0x3 => Ok(EffectId::TonePorta),
            0x4 => Ok(EffectId::Vibrato),
            0x5 => Ok(EffectId::SetMasterVol),
            0x6 => Ok(EffectId::CallRoutine),
            0x7 => Ok(EffectId::NoteDelay),
            0x8 => Ok(EffectId::SetPanning),
            0x9 => Ok(EffectId::ChangeTimbre),
            0xa => Ok(EffectId::VolSlide),
            0xb => Ok(EffectId::PosJump),
            0xc => Ok(EffectId::SetVol),
            0xd => Ok(EffectId::PatternBreak),
            0xe => Ok(EffectId::NoteCut),
            0xf => Ok(EffectId::SetTempo),
            n => Err(InnerErrorKind::BadEffectId(n)),
        }
    }
}

impl TryConstrain<crate::song::Note> for u32 {
    fn try_constrain(self) -> Result<crate::song::Note, InnerErrorKind> {
        use crate::song::Note::*;

        match self {
            0 => Ok(C_3),
            1 => Ok(CSharp3),
            2 => Ok(D_3),
            3 => Ok(DSharp3),
            4 => Ok(E_3),
            5 => Ok(F_3),
            6 => Ok(FSharp3),
            7 => Ok(G_3),
            8 => Ok(GSharp3),
            9 => Ok(A_3),
            10 => Ok(ASharp3),
            11 => Ok(B_3),
            12 => Ok(C_4),
            13 => Ok(CSharp4),
            14 => Ok(D_4),
            15 => Ok(DSharp4),
            16 => Ok(E_4),
            17 => Ok(F_4),
            18 => Ok(FSharp4),
            19 => Ok(G_4),
            20 => Ok(GSharp4),
            21 => Ok(A_4),
            22 => Ok(ASharp4),
            23 => Ok(B_4),
            24 => Ok(C_5),
            25 => Ok(CSharp5),
            26 => Ok(D_5),
            27 => Ok(DSharp5),
            28 => Ok(E_5),
            29 => Ok(F_5),
            30 => Ok(FSharp5),
            31 => Ok(G_5),
            32 => Ok(GSharp5),
            33 => Ok(A_5),
            34 => Ok(ASharp5),
            35 => Ok(B_5),
            36 => Ok(C_6),
            37 => Ok(CSharp6),
            38 => Ok(D_6),
            39 => Ok(DSharp6),
            40 => Ok(E_6),
            41 => Ok(F_6),
            42 => Ok(FSharp6),
            43 => Ok(G_6),
            44 => Ok(GSharp6),
            45 => Ok(A_6),
            46 => Ok(ASharp6),
            47 => Ok(B_6),
            48 => Ok(C_7),
            49 => Ok(CSharp7),
            50 => Ok(D_7),
            51 => Ok(DSharp7),
            52 => Ok(E_7),
            53 => Ok(F_7),
            54 => Ok(FSharp7),
            55 => Ok(G_7),
            56 => Ok(GSharp7),
            57 => Ok(A_7),
            58 => Ok(ASharp7),
            59 => Ok(B_7),
            60 => Ok(C_8),
            61 => Ok(CSharp8),
            62 => Ok(D_8),
            63 => Ok(DSharp8),
            64 => Ok(E_8),
            65 => Ok(F_8),
            66 => Ok(FSharp8),
            67 => Ok(G_8),
            68 => Ok(GSharp8),
            69 => Ok(A_8),
            70 => Ok(ASharp8),
            71 => Ok(B_8),
            90 => Ok(None),

            n => Err(InnerErrorKind::BadNote(n)),
        }
    }
}
