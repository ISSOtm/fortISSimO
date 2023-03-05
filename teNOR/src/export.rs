use std::{
    fmt::Display,
    fs::File,
    io::{StdoutLock, Write},
    path::{Path, PathBuf},
    process::exit,
};

use clap::{crate_name, crate_version};

use crate::{
    optimise::{CellFirstHalf, OutputCell, PatternId, SubpatternKind},
    song::{
        DutyType, EnvelopeDirection, Instrument, InstrumentKind, LfsrWidth, Note, Song, Subpattern,
        SweepDirection, WaveOutputLevel,
    },
    CliArgs, LAST_NOTE,
};

pub(super) fn export(args: &CliArgs, song: &Song, input_path: &Path, cell_pool: &[OutputCell]) {
    let mut output = Output::new(args.output_path.as_ref());
    macro_rules! output {
        ($($arg:tt)*) => {
            writeln!(output, $($arg)*).unwrap()
        };
    }

    output!("; Generated from {} on TODO: date", input_path.display());
    output!("; Song: {}", song.name);
    output!("; Artist: {}", song.artist);
    output!("; Comment: {}", song.comment);
    output!();
    output!(
        "REDEF fortISSimO_VERSION equs /* Generated with {} version: */ \"{}\"",
        crate_name!(),
        crate_version!(),
    );
    output!("INCLUDE \"{}\"", args.include_path);
    output!();
    output!();
    if let Some(kind) = &args.section_type {
        output!("SECTION \"{}\", {kind}", args.section_name);
        output!();
    }

    match &args.song_descriptor {
        Some(label) => output!("{label}::"),
        None => {
            let stem: &Path = input_path
                .file_stem()
                .expect("Input file path has no stem?")
                .as_ref();
            output!("{}::", stem.display())
        }
    }
    output!("\tdb {} ; Tempo (ticks/row)", song.ticks_per_row);
    output!(
        "\tdb ({} - 1) * 2 ; Max index into order \"columns\"",
        song.order_matrix.len(),
    );
    output!("\tdw .dutyInstrs, .waveInstrs, .noiseInstrs");
    output!("\tdw .routine");
    output!("\tdw .waves");
    output!();
    output!("\tdw .ch1, .ch2, .ch3, .ch4");
    output!();

    for i in 0..4 {
        write!(output, ".ch{}  dw", i + 1).unwrap();
        for id in &song.order_matrix {
            write!(output, " .{:2},", PatternId::Pattern(id[i].into())).unwrap();
        }
        output!();
    }
    output!();

    let mut row_idx = 0;
    for entry in cell_pool {
        output!("{}", CellWithLine(entry, row_idx));
        row_idx += 1;
        if matches!(entry, OutputCell::Label(_)) {
            row_idx = 0
        }
    }
    output!();
    output!("assert LAST_NOTE == {LAST_NOTE}, \"LAST_NOTE == {{LAST_NOTE}}\"");
    output!();

    fn decode_len(instr: &Instrument) -> u8 {
        match instr.length {
            Some(len) => u8::from(len) - 1,
            None => 0,
        }
    }

    output!(".dutyInstrs");
    for (id, instr) in song.instruments.duty.iter().enumerate() {
        let &InstrumentKind::Square {
            initial_volume,
            envelope_dir,
            envelope_pace,
            sweep_time,
            sweep_dir,
            sweep_shift,
            duty,
        } = &instr.kind else {
            panic!("Non-duty instrument in duty instr bank!?");
        };

        output!("; Duty instrument {}: {}", id + 1, instr.name);
        output!(
            "\tdb {} << 4 | {} | {} ; Sweep (NR10)",
            sweep_time,
            sweep_dir,
            sweep_shift,
        );
        output!(
            "\tdb {} | {} ; Duty & length (NRx1)",
            duty,
            decode_len(instr),
        );
        output!(
            "\tdb {} ; Volume & envelope (NRx2)",
            NRx2 {
                initial_volume,
                envelope_dir,
                envelope_pace
            },
        );
        output!(
            "\tdw {} ; Subpattern pointer",
            (SubpatternPtr::new(
                &instr.subpattern,
                PatternId::Subpattern(SubpatternKind::Duty, id)
            )),
        );
        output!(
            "\tdb $80 | {} << 6 ; Retrigger bit, and length enable (NRx4)",
            instr.length.is_some() as u8,
        );
    }
    output!();

    output!(".waveInstrs");
    for (id, instr) in song.instruments.wave.iter().enumerate() {
        let &InstrumentKind::Wave { output_level, waveform } = &instr.kind else {
            panic!("Non-wave instrument in wave instr bank!?");
        };

        output!("; Wave instrument {}: {}", id + 1, instr.name);
        output!("\tdb {} ; Length (NR31)", decode_len(instr),);
        output!("\tdb {output_level} ; Output level (NR32)");
        output!("\tdb {waveform} ; Wave ID");
        output!(
            "\tdw {} ; Subpattern pointer",
            (SubpatternPtr::new(
                &instr.subpattern,
                PatternId::Subpattern(SubpatternKind::Wave, id)
            )),
        );
        output!(
            "\tdb $80 | {} << 6 ; Retrigger bit, and length enable (NRx4)",
            instr.length.is_some() as u8,
        );
    }
    output!();

    output!(".noiseInstrs");
    for (id, instr) in song.instruments.noise.iter().enumerate() {
        let &InstrumentKind::Noise {
            initial_volume,
            envelope_dir,
            envelope_pace,
            lfsr_width,
        } = &instr.kind else {
            panic!("Non-noise instrument in noise instr bank!?");
        };

        output!("; Noise instrument {}: {}", id + 1, instr.name);
        output!(
            "\tdb {} ; Volume & envelope (NR42)",
            NRx2 {
                initial_volume,
                envelope_dir,
                envelope_pace
            },
        );
        output!(
            "\tdw {} ; Subpattern pointer",
            (SubpatternPtr::new(
                &instr.subpattern,
                PatternId::Subpattern(SubpatternKind::Noise, id)
            )),
        );
        output!(
            "\tdb {} | {} << 6 | {} ; LFSR width (NR43), length enable (NR44), and length (NR41)",
            lfsr_width,
            instr.length.is_some() as u8,
            decode_len(instr),
        );
        output!("\tds 2 ; Padding");
    }
    output!();

    output!(".waves");
    for (id, wave) in song.waves.iter().enumerate() {
        write!(output, "\tdb ").unwrap();
        for byte in wave {
            write!(output, "${byte:02x},").unwrap();
        }
        output!(" ; {id}");
    }
    output!();

    output!(".routine");
}

#[derive(Debug)]
enum Output {
    File(File),
    Stdout(StdoutLock<'static>),
}

impl Output {
    fn new<P: Into<PathBuf>>(path: Option<P>) -> Self {
        match path {
            Some(path) => {
                let path = path.into();
                let out_file = match File::create(&path) {
                    Ok(file) => file,
                    Err(err) => {
                        eprintln!(
                            "Failed to open file \"{}\" for writing: {err}",
                            path.display()
                        );
                        exit(1);
                    }
                };
                Self::File(out_file)
            }
            None => Self::Stdout(std::io::stdout().lock()),
        }
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments) -> std::io::Result<()> {
        match self {
            Self::File(file) => file.write_fmt(fmt),
            Self::Stdout(lock) => lock.write_fmt(fmt),
        }
    }
}

#[derive(Debug, Clone)]
struct CellWithLine<'cell>(&'cell OutputCell, usize);

impl Display for CellWithLine<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            OutputCell::Label(whose) => write!(f, ".{whose}"),
            OutputCell::Cell(cell) => {
                write!(
                    f,
                    "\tdn {}, {:X},{:02X} ; {}",
                    cell.0, cell.1.id as u8, cell.1.param, self.1,
                )
            }
            OutputCell::OverlapMarker(1) => {
                write!(f, "\t; Continued on next row.")
            }
            OutputCell::OverlapMarker(how_many) => {
                write!(f, "\t; Continued on next {how_many} rows.")
            }
        }
    }
}

impl Display for PatternId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let width = f.width().unwrap_or(0);
        match self {
            Self::Pattern(id) => write!(f, "p{id:<width$}"),
            Self::Subpattern(kind, id) => write!(f, "{kind}{:<width$}Subpattern", id + 1),
        }
    }
}

impl Display for SubpatternKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duty => write!(f, "duty"),
            Self::Wave => write!(f, "wave"),
            Self::Noise => write!(f, "noise"),
        }
    }
}

impl Display for CellFirstHalf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern { note, instrument } => write!(f, "{note}, {instrument:>2}"),
            Self::Subpattern {
                offset: 90,
                next_row_idx,
            } => write!(f, "___, {next_row_idx:>2}"),
            Self::Subpattern {
                offset,
                next_row_idx,
            } => write!(
                f,
                "{:>+3}, {next_row_idx:>2}",
                offset.wrapping_sub(LAST_NOTE / 2) as i8,
            ),
        }
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Note::C_3 => write!(f, "C_3"),
            Note::CSharp3 => write!(f, "C#3"),
            Note::D_3 => write!(f, "D_3"),
            Note::DSharp3 => write!(f, "D#3"),
            Note::E_3 => write!(f, "E_3"),
            Note::F_3 => write!(f, "F_3"),
            Note::FSharp3 => write!(f, "F#3"),
            Note::G_3 => write!(f, "G_3"),
            Note::GSharp3 => write!(f, "G#3"),
            Note::A_3 => write!(f, "A_3"),
            Note::ASharp3 => write!(f, "A#3"),
            Note::B_3 => write!(f, "B_3"),
            Note::C_4 => write!(f, "C_4"),
            Note::CSharp4 => write!(f, "C#4"),
            Note::D_4 => write!(f, "D_4"),
            Note::DSharp4 => write!(f, "D#4"),
            Note::E_4 => write!(f, "E_4"),
            Note::F_4 => write!(f, "F_4"),
            Note::FSharp4 => write!(f, "F#4"),
            Note::G_4 => write!(f, "G_4"),
            Note::GSharp4 => write!(f, "G#4"),
            Note::A_4 => write!(f, "A_4"),
            Note::ASharp4 => write!(f, "A#4"),
            Note::B_4 => write!(f, "B_4"),
            Note::C_5 => write!(f, "C_5"),
            Note::CSharp5 => write!(f, "C#5"),
            Note::D_5 => write!(f, "D_5"),
            Note::DSharp5 => write!(f, "D#5"),
            Note::E_5 => write!(f, "E_5"),
            Note::F_5 => write!(f, "F_5"),
            Note::FSharp5 => write!(f, "F#5"),
            Note::G_5 => write!(f, "G_5"),
            Note::GSharp5 => write!(f, "G#5"),
            Note::A_5 => write!(f, "A_5"),
            Note::ASharp5 => write!(f, "A#5"),
            Note::B_5 => write!(f, "B_5"),
            Note::C_6 => write!(f, "C_6"),
            Note::CSharp6 => write!(f, "C#6"),
            Note::D_6 => write!(f, "D_6"),
            Note::DSharp6 => write!(f, "D#6"),
            Note::E_6 => write!(f, "E_6"),
            Note::F_6 => write!(f, "F_6"),
            Note::FSharp6 => write!(f, "F#6"),
            Note::G_6 => write!(f, "G_6"),
            Note::GSharp6 => write!(f, "G#6"),
            Note::A_6 => write!(f, "A_6"),
            Note::ASharp6 => write!(f, "A#6"),
            Note::B_6 => write!(f, "B_6"),
            Note::C_7 => write!(f, "C_7"),
            Note::CSharp7 => write!(f, "C#7"),
            Note::D_7 => write!(f, "D_7"),
            Note::DSharp7 => write!(f, "D#7"),
            Note::E_7 => write!(f, "E_7"),
            Note::F_7 => write!(f, "F_7"),
            Note::FSharp7 => write!(f, "F#7"),
            Note::G_7 => write!(f, "G_7"),
            Note::GSharp7 => write!(f, "G#7"),
            Note::A_7 => write!(f, "A_7"),
            Note::ASharp7 => write!(f, "A#7"),
            Note::B_7 => write!(f, "B_7"),
            Note::C_8 => write!(f, "C_8"),
            Note::CSharp8 => write!(f, "C#8"),
            Note::D_8 => write!(f, "D_8"),
            Note::DSharp8 => write!(f, "D#8"),
            Note::E_8 => write!(f, "E_8"),
            Note::F_8 => write!(f, "F_8"),
            Note::FSharp8 => write!(f, "F#8"),
            Note::G_8 => write!(f, "G_8"),
            Note::GSharp8 => write!(f, "G#8"),
            Note::A_8 => write!(f, "A_8"),
            Note::ASharp8 => write!(f, "A#8"),
            Note::B_8 => write!(f, "B_8"),
            Note::None => write!(f, "___"),
        }
    }
}

impl Display for DutyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DutyType::Percent12_5 => write!(f, "%00 << 6"),
            DutyType::Percent25 => write!(f, "%01 << 6"),
            DutyType::Percent50 => write!(f, "%10 << 6"),
            DutyType::Percent75 => write!(f, "%11 << 6"),
        }
    }
}

impl Display for SweepDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} << 3",
            match self {
                SweepDirection::Down => '1',
                SweepDirection::Up => '0',
            }
        )
    }
}

impl Display for WaveOutputLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "%{} << 5",
            match self {
                WaveOutputLevel::Mute => "00",
                WaveOutputLevel::Full => "01",
                WaveOutputLevel::Half => "10",
                WaveOutputLevel::Quarter => "11",
            }
        )
    }
}

impl Display for LfsrWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} << 7",
            match self {
                LfsrWidth::Fifteen => '0',
                LfsrWidth::Seven => '1',
            }
        )
    }
}

#[derive(Debug, Clone)]
struct NRx2 {
    initial_volume: u8,
    envelope_dir: EnvelopeDirection,
    envelope_pace: u8,
}

impl Display for NRx2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} << 4 | {} << 3 | {}",
            self.initial_volume,
            match self.envelope_dir {
                EnvelopeDirection::Down => '0',
                EnvelopeDirection::Up => '1',
            },
            self.envelope_pace
        )
    }
}

#[derive(Debug, Clone)]
struct SubpatternPtr(Option<PatternId>);

impl SubpatternPtr {
    fn new(subpattern: &Option<Subpattern>, id: PatternId) -> Self {
        Self(subpattern.as_ref().map(|_| id))
    }
}

impl Display for SubpatternPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(id) => write!(f, ".{id}"),
            None => write!(f, "0"),
        }
    }
}
