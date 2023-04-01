use std::{
    fmt::Display,
    fs::File,
    io::{StdoutLock, Write},
    path::{Path, PathBuf},
    process::exit,
};

use chrono::prelude::*;
use clap::{crate_name, crate_version};

use crate::{
    optimise::{InstrKind, OptimResults, OutputCell, PatternId},
    song::{
        DutyType, EnvelopeDirection, Instrument, InstrumentKind, LfsrWidth, Song, Subpattern,
        SweepDirection, WaveOutputLevel,
    },
    CliArgs, LAST_NOTE, PATTERN_LENGTH,
};

pub(super) fn export(
    args: &CliArgs,
    song: &Song,
    input_path: &Path,
    OptimResults {
        row_pool,
        cell_catalog,
        duty_instr_usage,
        wave_instr_usage,
        noise_instr_usage,
        wave_usage,
    }: &OptimResults,
) {
    let mut output = Output::new(args.output_path.as_ref());
    macro_rules! output {
        ($($arg:tt)*) => {
            writeln!(output, $($arg)*).unwrap()
        };
    }

    output!(
        "; Generated from {} on {}",
        input_path.display(),
        Utc::now().trunc_subsecs(0),
    );
    output!("; Song: {}", song.name);
    output!("; Artist: {}", song.artist);
    output!("; Comment: {}", song.comment);
    if let Some(divider) = song.timer_divider {
        output!("; Expected playback method: TMA = ${:02x}", divider);
    } else {
        output!("; Expected playback method: VBlank");
    }
    output!();
    output!(
        "REDEF fortISSimO_VERSION equs /* Generated with {} version: */ \"{}\"",
        crate_name!(),
        crate_version!(),
    );
    if !args.include_path.is_empty() {
        output!("INCLUDE \"{}\"", args.include_path);
        output!();
    }
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
    output!("\tdb HIGH(.cellCatalog)");
    output!();

    for i in 0..4 {
        let kind = InstrKind::from_channel_id(i);
        write!(output, ".ch{}  dw", i + 1).unwrap();
        for id in &song.order_matrix {
            write!(output, " .{:2},", PatternId::Pattern(kind, id[i])).unwrap();
        }
        output!();
    }
    output!();

    let mut reverse_lookup = [0; 256];
    for (i, id) in cell_catalog.values().enumerate() {
        reverse_lookup[usize::from(*id)] = i as u8;
    }
    let mut row_idx = 0;
    for entry in row_pool {
        match entry {
            OutputCell::Label(whose) => {
                write!(output, "\n.{whose}").unwrap();
                row_idx = 0;
            }
            OutputCell::Cell(id) => {
                write!(
                    output,
                    "{}{:3}",
                    if row_idx == 0 { "\n\tdb " } else { "," },
                    reverse_lookup[usize::from(*id)]
                )
                .unwrap();
                row_idx += 1;
            }
            OutputCell::OverlapMarker(1) => {
                write!(output, "\n\t; Continued on next row.").unwrap();
            }
            OutputCell::OverlapMarker(how_many) => {
                write!(output, "\n\t; Continued on next {how_many} rows.").unwrap();
            }
        }
    }
    output!();
    output!();
    output!(".cellCatalog  align 8");
    write!(output, "\tdb ").unwrap();
    for cell in cell_catalog.keys() {
        write!(output, "${:02x},", cell.first_byte()).unwrap();
    }
    output!();
    output!(
        "\tds {} ; Padding to maintain alignment",
        256 - cell_catalog.len()
    );
    write!(output, "\tdb ").unwrap();
    for cell in cell_catalog.keys() {
        write!(output, "${:02x},", cell.second_byte()).unwrap();
    }
    output!();
    output!(
        "\tds {} ; Padding to maintain alignment",
        256 - cell_catalog.len()
    );
    write!(output, "\tdb ").unwrap();
    for cell in cell_catalog.keys() {
        write!(output, "${:02x},", cell.third_byte()).unwrap();
    }
    output!();
    output!();
    output!("assert LAST_NOTE == {LAST_NOTE}, \"LAST_NOTE == {{LAST_NOTE}}\"");
    output!("assert PATTERN_LENGTH == {PATTERN_LENGTH}, \"PATTERN_LENGTH == {{PATTERN_LENGTH}}\"");
    output!();

    fn decode_len(instr: &Instrument) -> u8 {
        match instr.length {
            Some(len) => u8::from(len) - 1,
            None => 0,
        }
    }

    output!(".dutyInstrs");
    for id in duty_instr_usage.iter() {
        let instr = &song.instruments.duty[usize::from(id)];
        let id = id + 1;
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

        output!("; Duty instrument {}: {}", id, instr.name);
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
                PatternId::Subpattern(InstrKind::Duty, id.into())
            )),
        );
        output!(
            "\tdb $80 | {} << 6 ; Retrigger bit, and length enable (NRx4)",
            instr.length.is_some() as u8,
        );
    }
    output!();

    output!(".waveInstrs");
    for id in wave_instr_usage.iter() {
        let instr = &song.instruments.wave[usize::from(id)];
        let id = id + 1;
        let &InstrumentKind::Wave { output_level, wave_id: waveform } = &instr.kind else {
            panic!("Non-wave instrument in wave instr bank!?");
        };

        output!("; Wave instrument {}: {}", id, instr.name);
        output!("\tdb {} ; Length (NR31)", decode_len(instr),);
        output!("\tdb {output_level} ; Output level (NR32)");
        output!(
            "\tdw {} ; Subpattern pointer",
            (SubpatternPtr::new(
                &instr.subpattern,
                PatternId::Subpattern(InstrKind::Wave, id.into())
            )),
        );
        output!(
            "\tdb $80 | {} << 6 ; Retrigger bit, and length enable (NRx4)",
            instr.length.is_some() as u8,
        );
        output!("\tdb {waveform} << 4 ; Wave ID");
    }
    output!();

    output!(".noiseInstrs");
    for id in noise_instr_usage.iter() {
        let instr = &song.instruments.noise[usize::from(id)];
        let id = id + 1;
        let &InstrumentKind::Noise {
            initial_volume,
            envelope_dir,
            envelope_pace,
            lfsr_width,
        } = &instr.kind else {
            panic!("Non-noise instrument in noise instr bank!?");
        };

        output!("; Noise instrument {}: {}", id, instr.name);
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
                PatternId::Subpattern(InstrKind::Noise, id.into())
            )),
        );
        output!(
            "\tdb {} | {} << 6 | {} ; LFSR width (NR43), length enable (NR44), and length (NR41)",
            lfsr_width,
            instr.length.is_some() as u8,
            decode_len(instr),
        );
    }
    output!();

    output!(".waves");
    for id in wave_usage.iter() {
        write!(output, "\tdb ").unwrap();
        for byte in &song.waves[usize::from(id)] {
            write!(output, "${byte:02x},").unwrap();
        }
        output!(" ; Originally #{id}");
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

impl Display for PatternId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let width = f.width().unwrap_or(0);
        match self {
            Self::Pattern(kind, id) => write!(f, "{kind}Ptrn{id:<width$}"),
            Self::Subpattern(kind, id) => write!(f, "{kind}Inst{:<width$}Subpattern", id),
        }
    }
}

impl Display for InstrKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duty => write!(f, "duty"),
            Self::Wave => write!(f, "wave"),
            Self::Noise => write!(f, "noise"),
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
