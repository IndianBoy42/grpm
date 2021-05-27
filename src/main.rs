mod common;
mod tuiapp;

const HELP: &str = "\
Github Releases Package Manager

Usage: grpm [OPTIONS] command

OPTIONS:
    -h --help Print this messsage and exit

COMMAND:
    tui               Open the TUI for interactively finding and installing
cli -- If any arguments are not provided then the TUI opens
    install [OWNER] [REPO] [VERSION] [FILE] Install from URL with 
    search  [OWNER] [REPO] [VERSION] [FILE] Search releases from URL

    VERSION and FILE can be one of the following:
    `[VERSION] [FILE]`     may be replaced by `[ASSETID]` for directly finding a certain asset
    [VERSION] = latest,    get the latest download
    [VERSION] = -t {TAG},  get a certain tag
    [VERSION] = -r {TAG},  get first matching a certain regex
    URL is a string like 'user/repo'
";

#[derive(Debug)]
pub struct Args {
    command: String,
}
impl Default for Args {
    fn default() -> Self {
        Args {
            command: "tui".to_string(),
        }
    }
}
impl Args {
    fn parse_env() -> Args {
        let mut pargs = pico_args::Arguments::from_env();
        if pargs.contains(["-h", "--help"]) {
            print!("{}", HELP);
            std::process::exit(0);
        }

        let command: String = pargs.free_from_str().unwrap();

        let dargs = Args::default();

        Args { command }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    let args = Args::parse_env();

    if args.command.eq_ignore_ascii_case("tui") {
       tuiapp::tui(args); 
    }

    Ok(())
}
