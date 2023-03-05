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
    Cell, InstrCollection, Instrument, InstrumentBank, InstrumentKind, Pattern, Routine,
    RoutineBank, Song, Wave, WaveBank,
};

type PResult<'input, O> = IResult<&'input [u8], O, InnerError<'input>>;

pub fn parse_song<'input>(input: &'input [u8]) -> Result<Song<'_>, ParseError<'input>> {
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
            ParseErrorKind::TooNew(n) => write!(f, "UGE version {n} is not supported; please pester someone to update this tool!"),
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
        let (input, timer_divider) = integer(input)?;
        let (input, patterns) = pattern_map_v2(input)?;
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
                timer_divider: if timer_enabled {
                    Some(may_fail(
                        input,
                        (timer_divider + 1)
                            .try_into()
                            .and_then(|n: u8| n.try_into()),
                    )?)
                } else {
                    None
                },
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
        let (input, wave) = instr_bank_v3(input)?;
        let (input, noise) = instr_bank_v3(input)?;

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
        let (input, length) = integer(input)?;
        let (input, length_enabled) = boolean(input)?;
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
        let (input, subpattern) = pattern_v2(input)?;

        Ok((
            input,
            Instrument {
                name,
                length: if length_enabled {
                    Some(may_fail(
                        input,
                        (length + 1).try_into().and_then(|n: u8| n.try_into()),
                    )?)
                } else {
                    None
                },
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
                        waveform,
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
        let (input, ()) = fill(cell_v2, &mut pattern)(input)?;
        Ok((input, pattern))
    }
    context("parsing v2 pattern from here", inner)(input)
}

fn cell_v2(input: &[u8]) -> PResult<Cell> {
    fn inner(input: &[u8]) -> PResult<Cell> {
        let (input, note) = try_convert(input, integer)?;
        let (input, instrument) = try_convert(input, integer)?;
        let (input, _volume) = integer(input)?;
        let (input, effect_code) = try_convert(input, integer)?;
        let (input, effect_params) = nom::number::complete::u8(input)?;

        assert_eq!(_volume, 0);
        Ok((
            input,
            Cell::new(note, instrument, effect_code, effect_params),
        ))
    }
    context("parsing v2 cell from here", inner)(input)
}

// Order.

fn order_matrix(input: &[u8]) -> PResult<Vec<[u8; 4]>> {
    fn inner(input: &[u8]) -> PResult<Vec<[u8; 4]>> {
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

fn order_column(input: &[u8]) -> PResult<Vec<u8>> {
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
    BadInstrType(u32),
    BadEnvDir(u32),
    BadSweepDir(u32),
    BadDutyType(u8),
    BadWaveOutLevel(u32),
    BadLfsrWidth(u32),
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
            Self::BadInstrType(n) => write!(f, "Instrument type out of range (0x{n:08x})"),
            Self::BadEnvDir(n) => write!(f, "Envelope direction out of range (0x{n:08x})"),
            Self::BadSweepDir(n) => write!(f, "Sweep direction out of range (0x{n:08x})"),
            Self::BadDutyType(n) => write!(f, "Duty type out of range (0x{n:08x})"),
            Self::BadWaveOutLevel(n) => write!(f, "Wave output level out of range (0x{n:08x})"),
            Self::BadLfsrWidth(n) => write!(f, "LFSR width out of range (0x{n:08x})"),
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
        Err(err) => Err(InnerError::err(input, err.into())),
    }
}
// TODO: get rid of this function, it's passed the wrong `input` and reporting the wrong location
fn may_fail<T, E: Into<InnerErrorKind>>(
    input: &[u8],
    res: Result<T, E>,
) -> Result<T, nom::Err<InnerError>> {
    res.map_err(|err| InnerError::err(input, err.into()))
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
