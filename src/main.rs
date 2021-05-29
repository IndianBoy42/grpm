use pico_args::Error::MissingArgument;

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
    install   [OWNER] [REPO] [RELEASE] [ASSET] Install from URL with 
    releases  [OWNER] [REPO] [RELEASE]         Search releases from URL
    assets    [OWNER] [REPO] [RELEASE] [ASSET] Search assets from URL

    RELEASE and ASSET can be one of the following:
    `[RELEASE] [ASSET]`     may be replaced by `[ASSETID]` for directly finding a certain asset
    [RELEASE] = latest,    get the latest download
    [RELEASE] = t:{TAG},  get a certain tag
    [RELEASE] = r:{REGEX},  get first matching a certain regex
    URL is a string like 'user/repo'
";

#[derive(Debug)]
pub struct Args {
    command: String,
    owner: Option<String>,
    repo: Option<String>,
    release: Option<String>,
    asset: Option<String>,
}
impl Default for Args {
    fn default() -> Self {
        Args {
            command: "tui".to_string(),
            owner: None,
            repo: None,
            release: None,
            asset: None,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    let mut pargs = pico_args::Arguments::from_env();
    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let mut arg = || pargs.opt_free_from_str().unwrap();
    let command = arg().expect("No command given");
    let owner: Option<String> = arg();
    let (owner, repo) = if let Some(owner) = owner {
        if let Some((owner, repo)) = owner.split_once('/') {
            (Some(owner.to_owned()), Some(repo.to_owned()))
        } else {
            (Some(owner), arg())
        }
    } else {
        (None, None)
    };
    let release = arg();
    let asset = arg();

    let dargs = Args::default();
    let args = Args {
        command,
        owner,
        repo,
        release,
        asset,
    };

    println!("{:?}", args);

    if args.command.as_str() == "tui" {
        return tuiapp::tui(args);
    }
    match args.command.as_str() {
        "install" => todo!("CLI install"),
        "search" => todo!("CLI search"),
        _ => panic!("Invalid Command"),
    }
}
