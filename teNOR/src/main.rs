use std::{ffi::OsString, path::Path, process::exit};

use clap::Parser;

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
    /// If omitted, the file will be written to standard output.
    output_path: Option<OsString>,

    /// Path to include file to emit.
    /// Keep in mind that this path will be evaluated by RGBASM, so relative to the directory that it will be invoked in!
    #[arg(short, long, default_value = "fortISSimO.inc")]
    include_path: String,

    /// Type of the section that the data will be exported to; if omitted, no SECTION directive will be emitted.
    /// Can include constraints, for example: `ROMX,BANK[2]`.
    #[arg(short = 't', long)]
    section_type: Option<String>,
    /// Name of the section that the data will be exported to; be wary of characters special to RGBASM, such as double quotes!
    /// This has no effect if the section type is omitted.
    #[arg(short = 'n', long, default_value = "Song Data")]
    section_name: String,

    /// Name of the label that will point to the song's header.
    /// (hUGETracker calls this the "song descriptor".)
    /// If omitted, this will be deduced from the input file name.
    #[arg(short = 'd', long)]
    song_descriptor: Option<String>,
}

fn main() {
    let args = CliArgs::parse();
    let input_path: &Path = args.input_path.as_ref();

    let data = std::fs::read(&input_path).expect("Failed to read UGE file"); // TODO
    let song = match uge::parse_song(&data) {
        Ok(song) => song,
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    };

    let cell_pool = optimise::optimise(&song);

    export::export(&args, &song, input_path, &cell_pool);
}
