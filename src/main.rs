mod daemon;
mod keys;
mod pack;
mod state;

use clap::{Parser, Subcommand};
use state::State;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "vibes", version, about = "Mechanical keyboard sounds on every key press, from Mechvibes sound packs")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Listen to the keyboard and play the pack (what the launchd agent runs)
    Run,
    /// List the installed packs (> marks the current one)
    List,
    /// Switch to a pack, and turn sounds on (`vibes <pack>` does the same)
    Use { pack: String },
    /// Turn sounds on
    On,
    /// Turn sounds off
    Off,
    /// Turn sounds on or off
    Toggle,
    /// Show the volume, or set it (0-100)
    Volume {
        #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
        level: Option<u8>,
    },
    /// One line for scripts: on or off, the pack and the volume, tab-separated
    Status,
    /// Load packs (all, or the ones named) and say how many keys each gives a sound
    Check { packs: Vec<String> },
    #[command(external_subcommand)]
    Pack(Vec<String>),
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("vibes: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Option<Command>) -> Result<(), String> {
    let mut state = State::load();
    match command {
        None => {
            let on = if state.enabled { "on" } else { "off" };
            println!("{on}: {} ({}), volume {}", state.pack, pack_title(&state.pack), state.volume);
            return Ok(());
        }
        Some(Command::Run) => return daemon::run(),
        Some(Command::List) => {
            let packs = state::packs();
            if packs.is_empty() {
                return Err(format!("no packs in {}", state::packs_dir().display()));
            }
            let width = packs.iter().map(String::len).max().unwrap_or(0);
            for pack in packs {
                let mark = if pack == state.pack { ">" } else { " " };
                println!("{mark} {pack:width$}  {}", pack_title(&pack));
            }
            return Ok(());
        }
        Some(Command::Status) => {
            println!("{}\t{}\t{}", if state.enabled { "on" } else { "off" }, state.pack, state.volume);
            return Ok(());
        }
        Some(Command::Check { packs }) => {
            let packs = if packs.is_empty() { state::packs() } else { packs };
            let width = packs.iter().map(String::len).max().unwrap_or(0);
            let mut failed = 0;
            for pack in &packs {
                match pack::Pack::load(&state::packs_dir().join(pack)) {
                    Ok(p) => {
                        let (press, release) = p.counts();
                        println!("ok    {pack:width$}  {press} keys, {release} with release sounds");
                        for warning in p.warnings {
                            println!("      {:width$}  skipped {warning}", "");
                        }
                    }
                    Err(e) => {
                        println!("FAIL  {pack:width$}  {e}");
                        failed += 1;
                    }
                }
            }
            return if failed == 0 { Ok(()) } else { Err(format!("{failed} of {} packs failed", packs.len())) };
        }
        Some(Command::Volume { level: None }) => {
            println!("{}", state.volume);
            return Ok(());
        }
        Some(Command::Volume { level: Some(level) }) => state.volume = level,
        Some(Command::On) => state.enabled = true,
        Some(Command::Off) => state.enabled = false,
        Some(Command::Toggle) => state.enabled = !state.enabled,
        Some(Command::Use { pack }) => use_pack(&mut state, pack)?,
        Some(Command::Pack(args)) => {
            if args.len() > 1 {
                return Err(format!("unexpected arguments after {}", args[0]));
            }
            use_pack(&mut state, args.into_iter().next().unwrap_or_default())?
        }
    }
    state.save().map_err(|e| format!("{}: {e}", state::state_path().display()))
}

fn use_pack(state: &mut State, pack: String) -> Result<(), String> {
    if !state::packs().contains(&pack) {
        return Err(format!("no pack named {pack} (vibes list shows them)"));
    }
    state.pack = pack;
    state.enabled = true;
    Ok(())
}

/// The name a pack's config.json gives it.
fn pack_title(pack: &str) -> String {
    std::fs::read_to_string(state::packs_dir().join(pack).join("config.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|c| c.get("name")?.as_str().map(str::to_string))
        .unwrap_or_default()
}
