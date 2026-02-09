mod cli;

use clap::Parser;
use cli::Cli;

fn main() {
    let args = Cli::parse();

    #[cfg(feature = "gui")]
    let launch_gui = args.gui || args.input.is_none();

    #[cfg(not(feature = "gui"))]
    let launch_gui = false;

    if launch_gui {
        #[cfg(feature = "gui")]
        if let Err(error) = clippr::gui::run() {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
        #[cfg(not(feature = "gui"))]
        unreachable!();
    } else {
        let input = match args.input {
            Some(path) => path,
            None => {
                eprintln!(
                    "error: no input file provided (compile with --features gui for the graphical interface)"
                );
                std::process::exit(1);
            }
        };

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
