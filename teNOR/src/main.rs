use std::{ffi::OsString, fmt::Display, io::Write, path::Path, process::exit};

use clap::{Parser, ValueEnum};
use termcolor::{Color, ColorSpec, StandardStream, StandardStreamLock, WriteColor};

mod export;
mod optimise;
mod song;
mod uge;

const LAST_NOTE: u8 = 72;

#[derive(Debug, Clone, Parser)]
#[command(version, about)]
struct CliArgs {
    /// Path to the `.uge` file to be exported.
    input_path: OsString,
    /// Path to the `.asm` file to write to.
    ///
    /// If omitted, the file will be written to standard output.
    output_path: Option<OsString>,

    /// Path to include file to emit.
    ///
    /// Keep in mind that this path will be evaluated by RGBASM, so relative to the directory that it will be invoked in!
    /// If empty, no INCLUDE directive will be emitted.
    #[arg(short, long, default_value = "fortISSimO.inc")]
    include_path: String,

    /// Type of the section that the data will be exported to; if omitted, no SECTION directive will be emitted.
    ///
    /// Can include constraints, for example: `ROMX,BANK[2]`.
    #[arg(short = 't', long)]
    section_type: Option<String>,
    /// Name of the section that the data will be exported to.
    ///
    /// Be wary of characters special to RGBASM, such as double quotes!
    /// This has no effect if the section type is omitted.
    #[arg(short = 'n', long, default_value = "Song Data")]
    section_name: String,

    /// Name of the label that will point to the song's header (hUGETracker calls this the "song descriptor").
    ///
    /// If omitted, this will be deduced from the input file name.
    #[arg(short = 'd', long)]
    song_descriptor: Option<String>,

    /// Do not emit stats at the end.
    #[arg(short = 'q', long)]
    quiet: bool,

    /// Use colours when writing to standard error (errors, stats, etc.)
    #[arg(long, default_value_t)]
    color: CliColorChoice,
}

fn main() {
    let args = CliArgs::parse();
    let color_choice = match args.color {
        CliColorChoice::Always => termcolor::ColorChoice::Always,
        CliColorChoice::Auto if atty::is(atty::Stream::Stderr) => termcolor::ColorChoice::Auto,
        CliColorChoice::Auto => termcolor::ColorChoice::Never,
        CliColorChoice::Never => termcolor::ColorChoice::Never,
    };
    let stderr = StandardStream::stderr(color_choice);
    let mut stderr = stderr.lock();
    let input_path: &Path = args.input_path.as_ref();

    macro_rules! write_error {
        ($descr:literal $(, $($descr_args:expr),+)? ; $(,)? $inner:literal $(, $($inner_args:expr),+)? $(,)?) => {
            stderr
                .set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Red)))
                .unwrap();
            write!(stderr, "Error: ").unwrap();
            stderr
                .set_color(ColorSpec::new().set_bold(true).set_fg(None))
                .unwrap();
            write!(stderr, $descr $(, $($descr_args),+)?).unwrap();
            stderr.set_color(ColorSpec::new().set_bold(false)).unwrap();
            writeln!(stderr, $inner $(, $($inner_args),+)?).unwrap();
        };
    }

    let data = match std::fs::read(input_path) {
        Ok(data) => data,
        Err(err) => {
            write_error!("Failed to read file \"{}\": ", input_path.display();
                "{err}");
            exit(1);
        }
    };
    let song = match uge::parse_song(&data) {
        Ok(song) => song,
        Err(err) => {
            write_error!("Unable to parse a UGE song from \"{}\": ", input_path.display();
                "{err}");
            exit(1);
        }
    };

    let (cell_pool, duty_instr_usage, wave_instr_usage, noise_instr_usage, wave_usage, optim_stats) =
        optimise::optimise(&song);

    export::export(
        &args,
        &song,
        input_path,
        &cell_pool,
        &duty_instr_usage,
        &wave_instr_usage,
        &noise_instr_usage,
        &wave_usage,
    );

    if !args.quiet {
        print_stats(&mut stderr, &optim_stats);
    }
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum CliColorChoice {
    /// Always use colours.
    Always,
    /// Use colours only if writing directly to a terminal.
    #[default]
    Auto,
    /// Never use colours.
    Never,
}

impl Display for CliColorChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Always => write!(f, "always"),
            Self::Auto => write!(f, "auto"),
            Self::Never => write!(f, "never"),
        }
    }
}

fn print_stats(stderr: &mut StandardStreamLock<'_>, optim_stats: &optimise::OptimStats) {
    stderr
        .set_color(ColorSpec::new().set_underline(true))
        .unwrap();
    writeln!(stderr, "teNOR optimisation stats:").unwrap();
    let mut report = |verb, how_many, what, bytes_saved| {
        let mut color_spec = ColorSpec::new();
        if bytes_saved == 0 {
            color_spec.set_dimmed(true).set_italic(true);
        }
        stderr.set_color(&color_spec).unwrap();
        write!(stderr, "\t{verb} {how_many} {what} saved ").unwrap();
        stderr
            .set_color(color_spec.set_bold(bytes_saved != 0))
            .unwrap();
        writeln!(stderr, "{bytes_saved} bytes").unwrap();
    };
    report(
        "Pruning",
        optim_stats.pruned_patterns,
        "unreachable patterns",
        optim_stats.saved_bytes_pruned_patterns(),
    );
    report(
        "Trimming",
        optim_stats.trimmed_rows,
        "unreachable rows",
        optim_stats.saved_bytes_trimmed_rows(),
    );
    report(
        "Overlapping",
        optim_stats.overlapped_rows,
        "rows",
        optim_stats.saved_bytes_overlapped_rows(),
    );
    report(
        "Omitting",
        optim_stats.pruned_instrs,
        "unused instruments",
        optim_stats.pruned_instrs_bytes,
    );
    report(
        "Skipping",
        optim_stats.trimmed_waves,
        "unused waves",
        optim_stats.saved_bytes_trimmed_waves(),
    );
    if optim_stats.duplicated_patterns != 0 {
        stderr.set_color(&ColorSpec::new()).unwrap();
        write!(
            stderr,
            "\t...though duplicating {} patterns wasted ",
            optim_stats.duplicated_patterns
        )
        .unwrap();
        stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
        writeln!(
            stderr,
            "{} bytes",
            optim_stats.wasted_bytes_duplicated_patterns()
        )
        .unwrap();
    }
    write!(
        stderr,
        "Total: {} bytes saved",
        optim_stats.total_saved_bytes()
    )
    .unwrap();
    stderr.set_color(&ColorSpec::new()).unwrap();
    writeln!(stderr, " (give or take a few.)").unwrap();
}
