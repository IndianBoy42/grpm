#![feature(maybe_uninit_array_assume_init)]
#![feature(maybe_uninit_uninit_array)]
mod common;
mod tuiapp;

const HELP: &str = "\
Github Releases Package Manager

Usage: grpm [OPTIONS] command

OPTIONS:
    -h --help Print this messsage and exit

COMMANDS:
    tui               Open the TUI for interactively finding and installing
cli -- If any arguments are not provided then the TUI opens
    install   [OWNER] [REPO] [RELEASE] [ASSET] [INSTALL-CMD] Install from URL with 
    download  [OWNER] [REPO] [RELEASE] [ASSET] [LOCATION] Download from URL with 
    save      [OWNER] [REPO] [RELEASE] [ASSET] [LOCATION] Save a config file 
    search    [OWNER] [REPO] [RELEASE]         Search releases from URL
    search    [OWNER] [REPO] [RELEASE] [ASSET] Search assets from URL

    RELEASE and ASSET can be one of the following:
        `[RELEASE] [ASSET]`   may be replaced by `[ASSETID]` for directly finding a certain asset
        [RELEASE] = latest,   get the latest download
        [RELEASE] = {REGEX},  get first matching a certain regex
        [RELEASE] = t:{TAG},  get a certain tag
        [ASSET]   = all,      download all Assets
        [ASSET]   = {REGEX},  download all Assets that match a certain regex

    OWNER and REPO are the github username and repository name respectively
        you may also provide the suffix of the github url (eg. indianboy42/grpm)
";

#[derive(Debug, Clone, Copy)]
pub struct ArgFlags {}

#[derive(Debug)]
pub struct Args {
    command: String,
    owner: Option<String>,
    repo: Option<String>,
    release: Option<String>,
    asset: Option<String>,
    install: Option<String>,
    flags: ArgFlags,
}
impl Default for Args {
    fn default() -> Self {
        Args {
            command: "tui".to_string(),
            owner: None,
            repo: None,
            release: None,
            asset: None,
            install: None,
            flags: ArgFlags {},
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
    let install = arg();

    let dargs = Args::default();
    let args = Args {
        command,
        owner,
        repo,
        release,
        asset,
        install,
        flags: ArgFlags {},
    };

    if args.command.as_str() == "tui" {
        return tuiapp::tui(args);
    }
    match args.command.as_str() {
        "install" => todo!("CLI install"),
        "search" => todo!("CLI search"),
        _ => panic!("Invalid Command"),
    }
}

fn foo(x: &str, y: usize, z: bool){

}
