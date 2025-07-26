use clap::{CommandFactory, Parser, Subcommand};
use colored::*;
use nix::unistd::{getpid, setpgid};
use std::panic;

use flatplay::process::{is_process_running, kill_process_group};
use flatplay::state::State;
use flatplay::FlatpakManager;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a Flatpak build, update the dependencies & build them
    Build,
    /// Build or rebuild the application then run it
    BuildAndRun,
    /// Stop the currently running task
    Stop,
    /// Run the application
    Run,
    /// Download/Update the dependencies and builds them
    UpdateDependencies,
    /// Clean the Flatpak repo directory
    Clean,
    /// Spawn a new terminal inside the specified SDK
    RuntimeTerminal,
    /// Spawn a new terminal inside the current build repository
    BuildTerminal,
    /// Show the output terminal of the build and run commands
    ShowOutputTerminal,
    /// Show the data directory for the active manifest
    ShowDataDirectory,
    /// Select or change the active manifest
    SelectManifest,
    /// Generate shell completion scripts for your shell
    Completions {
        /// The shell to generate completions for (e.g., bash, zsh, fish)
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

macro_rules! handle_command {
    ($command:expr) => {
        if let Err(err) = $command {
            eprintln!("{}: {}", "Error".red(), err);
        }
    };
}

fn main() {
    let cli = Cli::parse();
    let mut state = State::load().unwrap();

    if let Some(Commands::Stop) = cli.command {
        handle_command!(kill_process_group(&mut state));
        return;
    }

    // Check if another instance is already running.
    if let Some(pgid) = state.process_group_id {
        if is_process_running(pgid) {
            eprintln!(
                "{}: Another instance of flatplay is already running (PID: {}).",
                "Error".red(),
                pgid
            );
            eprintln!("Run '{}' to terminate it.", "flatplay stop".bold().italic());
            return;
        }
    }

    // Become a process group leader.
    // This also makes the pid the process group ID.
    let pid = getpid();
    if let Err(e) = setpgid(pid, pid) {
        eprintln!("Failed to set process group ID: {e}");
        return;
    }

    // Save the process group ID to the state.
    state.process_group_id = Some(pid.as_raw() as u32);
    if let Err(e) = state.save() {
        eprintln!("Failed to save state: {e}");
        return;
    }

    // Handle unclean ends where possible.
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        if let Ok(mut state) = State::load() {
            state.process_group_id = None;
            if let Err(e) = state.save() {
                eprintln!("Failed to save state in panic hook: {e}");
            }
        }
        original_hook(panic_info);
    }));

    let mut flatpak_manager = FlatpakManager::new(&mut state).unwrap();
    match &cli.command {
        Some(Commands::Completions { shell }) => {
            use clap_complete::generate;
            use std::io;
            let mut cmd = Cli::command();
            generate(*shell, &mut cmd, "flatplay", &mut io::stdout());
        }
        Some(Commands::Build) => handle_command!(flatpak_manager.build()),
        Some(Commands::BuildAndRun) => handle_command!(flatpak_manager.build_and_run()),
        Some(Commands::Stop) => handle_command!(flatpak_manager.stop()),
        Some(Commands::Run) => handle_command!(flatpak_manager.run()),
        Some(Commands::UpdateDependencies) => {
            handle_command!(flatpak_manager.update_dependencies())
        }
        Some(Commands::Clean) => handle_command!(flatpak_manager.clean()),
        Some(Commands::RuntimeTerminal) => handle_command!(flatpak_manager.runtime_terminal()),
        Some(Commands::BuildTerminal) => handle_command!(flatpak_manager.build_terminal()),
        Some(Commands::ShowOutputTerminal) => {
            handle_command!(flatpak_manager.show_output_terminal())
        }
        Some(Commands::ShowDataDirectory) => handle_command!(flatpak_manager.show_data_directory()),
        Some(Commands::SelectManifest) => handle_command!(flatpak_manager.select_manifest()),
        None => handle_command!(flatpak_manager.build_and_run()),
    }

    // Clean up pgid in the state file on normal exit.
    state.process_group_id = None;
    state.save().unwrap();
}
