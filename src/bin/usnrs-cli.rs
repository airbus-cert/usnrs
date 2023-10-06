extern crate usnrs;

use clap::{arg, Parser, ValueEnum};
use usnrs::Usn;

#[derive(Parser)]
struct Args {
    /// UsnJrnl:$J file to parse
    file: String,

    /// Path to a $MFT file. If present, resolves full paths to files
    #[arg(long)]
    mft: Option<String>,

    /// Start offset
    #[arg(long)]
    start: Option<u64>,

    /// Output format
    #[arg(short, long)]
    format: Option<OutputFormat>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    Default,
    Bodyfile,
    Debug,
}

fn main() {
    let args = Args::parse();

    let usn = if let Some(mft_path) = args.mft {
        Usn::from_usn_with_mft(&args.file, args.start, &mft_path).unwrap()
    }
    else {
        Usn::from_usn(&args.file, args.start).unwrap()
    };

    for (filename, entry) in usn {
        let line = match args.format.unwrap_or(OutputFormat::Default) {
            OutputFormat::Default => {
                format!(
                    "{} | {} | {} | {}",
                    entry.time(),
                    filename,
                    entry.attributes(),
                    entry.reasons()
                )
            }
            OutputFormat::Bodyfile => {
                format!(
                    "0|{} (USN: {})|{}-{}|0|0|0|0|{}|{}|{}|{}",
                    filename,
                    entry.reasons(),
                    entry.mft_entry_num(),
                    entry.sequence_num(),
                    entry.unix_timestamp(),
                    entry.unix_timestamp(),
                    entry.unix_timestamp(),
                    entry.unix_timestamp()
                )
            }
            OutputFormat::Debug => {
                format!("{:?}", entry)
            }
        };
        println!("{}", line);
    }
}
