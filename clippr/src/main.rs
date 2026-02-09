mod cli;

use clap::Parser;
use cli::Cli;

fn main() {
    let args = Cli::parse();

    if args.gui || args.input.is_none() {
        if let Err(error) = clippr::gui::run() {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    } else {
        let input = args.input.unwrap();

        let options = clippr::ConvertOptions {
            input,
            output: args.output,
            max_size_mb: args.max_size_mb,
            width: args.width,
            fps: args.fps,
            colors: args.colors,
            chunk_secs: args.chunk_secs,
        };

        if let Err(error) = clippr::convert(&options, |message| eprintln!("{message}")) {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
