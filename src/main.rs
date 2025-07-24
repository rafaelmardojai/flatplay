use clap::{CommandFactory, Parser, Subcommand};
use colored::*;
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
    let mut flatpak_manager = FlatpakManager::new().unwrap();

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
}
