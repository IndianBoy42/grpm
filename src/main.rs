mod tuiapp;
mod common;

const HELP: &str = "\
Github Releases Package Manager

Usage: grpm [OPTIONS] command

OPTIONS:
    -h --help Print this messsage and exit

COMMAND:
    tui               Open the TUI for interactively finding and installing
cli -- If any arguments are not provided then the TUI opens
    install [URL] [VERSION] [FILE] Install from URL with 
    search  [URL] [VERSION] [FILE] Search releases from URL
    VERSION and FILE are regexes
    `[VERSION] [FILE]` may be replaced by `[ASSETID]` for directly finding a certain asset
    URL is a string like 'user/repo'
";

#[derive(Debug)]
pub struct Args {}
impl Default for Args {
    fn default() -> Self {
        Args {}
    }
}
impl Args {
    fn parse_env() -> Args {
        let mut pargs = pico_args::Arguments::from_env();
        if pargs.contains(["-h", "--help"]) {
            print!("{}", HELP);
            std::process::exit(0);
        }

        let dargs = Args::default();

        Args {}
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    let args = Args::parse_env();

    Ok(())
}


