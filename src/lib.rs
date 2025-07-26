pub mod command;
pub mod manifest;
pub mod process;
pub mod state;
pub mod utils;

use anyhow::Result;
use colored::*;

use command::{flatpak_builder, run_command};
use dialoguer::{theme::ColorfulTheme, Select};
use manifest::Manifest;
use state::State;
use std::fs;
use utils::{get_a11y_bus_args, get_host_env};
use walkdir::WalkDir;

use crate::process::kill_process_group;

const BUILD_DIR: &str = ".flatplay";

pub struct FlatpakManager<'a> {
    state: &'a mut State,
    manifest: Option<Manifest>,
}

use std::path::PathBuf;

impl<'a> FlatpakManager<'a> {
    fn find_manifests(&self) -> Result<Vec<PathBuf>> {
        let mut manifests = vec![];
        for entry in WalkDir::new(".")
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_str().unwrap();
                // Allow the root directory (".") but skip other dotfiles/dirs
                name == "." || !name.starts_with('.')
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                matches!(
                    e.path().extension().and_then(|s| s.to_str()),
                    Some("json") | Some("yaml") | Some("yml")
                )
            })
            .filter(|e| Manifest::from_file(e.path()).is_ok())
        {
            manifests.push(entry.into_path());
        }

        manifests.sort_by(|a, b| {
            let a_is_devel = a.to_str().unwrap().contains(".Devel.");
            let b_is_devel = b.to_str().unwrap().contains(".Devel.");
            b_is_devel
                .cmp(&a_is_devel)
                .then_with(|| a.components().count().cmp(&b.components().count()))
        });

        Ok(manifests)
    }

    fn auto_select_manifest(&mut self) -> Result<()> {
        if let Some(manifest_path) = self.find_manifests()?.first() {
            self.state.active_manifest = Some(manifest_path.clone());
            self.state.save()?;
            println!("{} {:?}", "Auto-selected manifest:".green(), manifest_path);
            self.manifest = Some(Manifest::from_file(manifest_path)?);
        }
        Ok(())
    }

    pub fn new(state: &'a mut State) -> Result<Self> {
        let manifest = if let Some(path) = &state.active_manifest {
            Some(Manifest::from_file(path)?)
        } else {
            None
        };
        let mut manager = Self { state, manifest };
        if manager.manifest.is_none() {
            manager.auto_select_manifest()?;
        }
        Ok(manager)
    }

    fn is_build_initialized(&self) -> Result<bool> {
        let repo_dir = format!("{BUILD_DIR}/repo");
        let metadata_file = format!("{repo_dir}/metadata");
        let files_dir = format!("{repo_dir}/files");
        let var_dir = format!("{repo_dir}/var");

        // Check if all required directories and files exist
        // From gnome-builder: https://gitlab.gnome.org/GNOME/gnome-builder/-/blob/8579055f5047a0af5462e8a587b0742014d71d64/src/plugins/flatpak/gbp-flatpak-pipeline-addin.c#L220
        Ok(std::path::Path::new(&metadata_file).is_file()
            && std::path::Path::new(&files_dir).is_dir()
            && std::path::Path::new(&var_dir).is_dir())
    }

    fn init_build(&self) -> Result<()> {
        let manifest = self.manifest.as_ref().unwrap();
        let repo_dir = format!("{BUILD_DIR}/repo");

        println!("{}", "Initializing build environment...".bold());
        run_command(
            "flatpak",
            &[
                "build-init",
                &repo_dir,
                &manifest.id,
                &manifest.sdk,
                &manifest.runtime,
                &manifest.runtime_version,
            ],
        )
    }

    fn ensure_initialized_build(&mut self) -> Result<()> {
        if self.is_build_initialized()? {
            println!(
                "{}",
                "Skipped build initialization. Already initialized.".green()
            );
            return Ok(());
        }

        self.init_build()?;
        Ok(())
    }

    fn build_application(&self) -> Result<()> {
        let manifest = self.manifest.as_ref().unwrap();
        let repo_dir = format!("{BUILD_DIR}/repo");

        if let Some(module) = manifest.modules.last() {
            match module {
                crate::manifest::Module::Object {
                    buildsystem,
                    config_opts,
                    build_commands,
                    ..
                } => {
                    let buildsystem = buildsystem.as_deref();
                    match buildsystem {
                        Some("meson") => {
                            let build_dir = format!("{BUILD_DIR}/_build");
                            let mut meson_args = vec!["build", &repo_dir, "meson", "setup"];
                            if let Some(config_opts) = config_opts {
                                meson_args.extend(config_opts.iter().map(|s| s.as_str()));
                            }
                            meson_args.extend(&["--prefix=/app", &build_dir]);
                            run_command("flatpak", &meson_args)?;
                            run_command(
                                "flatpak",
                                &["build", &repo_dir, "ninja", "-C", &build_dir],
                            )?;
                            run_command(
                                "flatpak",
                                &["build", &repo_dir, "meson", "install", "-C", &build_dir],
                            )?;
                        }
                        Some("cmake") | Some("cmake-ninja") => {
                            let build_dir = format!("{BUILD_DIR}/_build");
                            let b_flag = format!("-B{build_dir}");
                            let mut cmake_args = vec![
                                "build",
                                &repo_dir,
                                "cmake",
                                "-G",
                                "Ninja",
                                &b_flag,
                                "-DCMAKE_EXPORT_COMPILE_COMMANDS=1",
                                "-DCMAKE_BUILD_TYPE=RelWithDebInfo",
                                "-DCMAKE_INSTALL_PREFIX=/app",
                            ];
                            if let Some(config_opts) = config_opts {
                                cmake_args.extend(config_opts.iter().map(|s| s.as_str()));
                            }
                            cmake_args.push(".");
                            run_command("flatpak", &cmake_args)?;
                            run_command(
                                "flatpak",
                                &["build", &repo_dir, "ninja", "-C", &build_dir],
                            )?;
                            run_command(
                                "flatpak",
                                &["build", &repo_dir, "ninja", "-C", &build_dir, "install"],
                            )?;
                        }
                        Some("simple") => {
                            if let Some(build_commands) = build_commands {
                                for command in build_commands {
                                    let mut args = vec!["build", &repo_dir];
                                    args.extend(command.split_whitespace());
                                    run_command("flatpak", &args)?;
                                }
                            }
                        }
                        _ => {
                            // Default to autotools
                            let mut autotools_args =
                                vec!["build", &repo_dir, "./configure", "--prefix=/app"];
                            if let Some(config_opts) = config_opts {
                                autotools_args.extend(config_opts.iter().map(|s| s.as_str()));
                            }
                            run_command("flatpak", &autotools_args)?;
                            run_command("flatpak", &["build", &repo_dir, "make"])?;
                            run_command("flatpak", &["build", &repo_dir, "make", "install"])?;
                        }
                    }
                }
                crate::manifest::Module::Reference(_) => {
                    // Skip string references for build_application
                }
            }
        }

        if let Some(crate::manifest::Module::Object {
            post_install: Some(post_install),
            ..
        }) = manifest.modules.last()
        {
            for command in post_install {
                let args: Vec<&str> = command.split_whitespace().collect();
                run_command(args[0], &args[1..])?;
            }
        }

        Ok(())
    }

    fn build_dependencies(&mut self) -> Result<()> {
        println!("{}", "Building dependencies...".bold());
        let manifest = self.manifest.as_ref().unwrap();
        let manifest_path = self.state.active_manifest.as_ref().unwrap();
        let repo_dir = format!("{BUILD_DIR}/repo");
        flatpak_builder(&[
            "--ccache",
            "--force-clean",
            "--disable-updates",
            "--disable-download",
            "--build-only",
            "--keep-build-dirs",
            &format!("--state-dir={BUILD_DIR}/flatpak-builder"),
            &format!(
                "--stop-at={}",
                match manifest.modules.last().unwrap() {
                    crate::manifest::Module::Object { name, .. } => name,
                    crate::manifest::Module::Reference(s) => s,
                }
            ),
            &repo_dir,
            manifest_path.to_str().unwrap(),
        ])?;
        self.state.dependencies_built = true;
        self.state.save()
    }

    pub fn update_dependencies(&mut self) -> Result<()> {
        println!("{}", "Updating dependencies...".bold());

        // Ensure build is initialized before updating dependencies
        self.ensure_initialized_build()?;

        let manifest = self.manifest.as_ref().unwrap();
        let manifest_path = self.state.active_manifest.as_ref().unwrap();
        let repo_dir = format!("{BUILD_DIR}/repo");
        flatpak_builder(&[
            "--ccache",
            "--force-clean",
            "--disable-updates",
            "--download-only",
            &format!("--state-dir={BUILD_DIR}/flatpak-builder"),
            &format!(
                "--stop-at={}",
                match manifest.modules.last().unwrap() {
                    crate::manifest::Module::Object { name, .. } => name,
                    crate::manifest::Module::Reference(s) => s,
                }
            ),
            &repo_dir,
            manifest_path.to_str().unwrap(),
        ])?;
        self.state.dependencies_updated = true;
        self.state.save()
    }

    pub fn build(&mut self) -> Result<()> {
        if self.manifest.is_none() {
            println!(
                "{}",
                "No manifest selected. Please run `select-manifest` first.".yellow()
            );
            return Ok(());
        }

        // Ensure build is initialized before proceeding
        self.ensure_initialized_build()?;

        if !self.state.dependencies_updated {
            self.update_dependencies()?;
        }
        if !self.state.dependencies_built {
            self.build_dependencies()?;
        }
        if self.state.application_built {
            // TODO: Implement rebuild
            self.build_application()?;
        } else {
            self.build_application()?;
        }
        self.state.application_built = true;
        self.state.save()
    }

    pub fn build_and_run(&mut self) -> Result<()> {
        self.build()?;
        self.run()
    }

    pub fn stop(&mut self) -> Result<()> {
        kill_process_group(self.state)
    }

    pub fn run(&self) -> Result<()> {
        if !self.state.application_built {
            println!(
                "{}",
                "Application not built. Please run `build` first.".yellow()
            );
            return Ok(());
        }

        // Check if build is initialized
        if !self.is_build_initialized()? {
            println!(
                "{}",
                "Build not initialized. Please run `build` first.".yellow()
            );
            return Ok(());
        }

        let manifest = self.manifest.as_ref().unwrap();
        let repo_dir = format!("{BUILD_DIR}/repo");

        let mut args = vec![
            "build".to_string(),
            "--with-appdir".to_string(),
            "--allow=devel".to_string(),
            "--talk-name=org.freedesktop.portal.*".to_string(),
            "--talk-name=org.a11y.Bus".to_string(),
        ];

        for (key, value) in get_host_env() {
            args.push(format!("--env={key}={value}"));
        }

        for arg in get_a11y_bus_args() {
            args.push(arg);
        }

        for arg in &manifest.finish_args {
            args.push(arg.clone());
        }
        args.push(repo_dir);
        args.push(manifest.command.clone());
        if let Some(x_run_args) = &manifest.x_run_args {
            for arg in x_run_args {
                args.push(arg.clone());
            }
        }

        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        run_command("flatpak", &args_str)
    }

    pub fn clean(&mut self) -> Result<()> {
        if fs::metadata(BUILD_DIR).is_ok() {
            fs::remove_dir_all(BUILD_DIR)?;
            println!("{}", "Cleaned .flatplay directory.".green());
        }
        Ok(())
    }

    pub fn runtime_terminal(&self) -> Result<()> {
        if self.manifest.is_none() {
            println!(
                "{}",
                "No manifest selected. Please run `select-manifest` first.".yellow()
            );
            return Ok(());
        }
        let manifest = self.manifest.as_ref().unwrap();
        let sdk_id = format!("{}//{}", manifest.sdk, manifest.runtime_version);
        run_command("flatpak", &["run", "--command=bash", &sdk_id])
    }

    pub fn build_terminal(&self) -> Result<()> {
        if self.manifest.is_none() {
            println!(
                "{}",
                "No manifest selected. Please run `select-manifest` first.".yellow()
            );
            return Ok(());
        }
        let manifest = self.manifest.as_ref().unwrap();
        let _app_id = &manifest.id;
        let repo_dir = format!("{BUILD_DIR}/repo");
        run_command("flatpak", &["build", &repo_dir, "bash"])
    }

    pub fn show_output_terminal(&self) -> Result<()> {
        println!("{}", "Showing output terminal...".bold());
        Ok(())
    }

    pub fn show_data_directory(&self) -> Result<()> {
        println!("{}", "Showing data directory...".bold());
        Ok(())
    }

    pub fn select_manifest(&mut self) -> Result<()> {
        println!("{}", "Searching for manifest files...".bold());
        let manifests = self.find_manifests()?;

        if manifests.is_empty() {
            println!("{}", "No manifest files found.".yellow());
            return Ok(());
        }

        let mut default_selection = 0;
        let manifest_strings: Vec<String> = manifests
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let path_str = p.to_str().unwrap().to_string();
                if let Some(active_manifest) = &self.state.active_manifest {
                    if active_manifest == p {
                        default_selection = i;
                        return format!("{} {}", "*".green().bold(), path_str);
                    }
                }
                format!("  {path_str}")
            })
            .collect();

        let theme = ColorfulTheme::default();
        let selection = Select::with_theme(&theme)
            .with_prompt("Select a manifest")
            .items(&manifest_strings)
            .default(default_selection)
            .interact()?;

        self.state.active_manifest = Some(manifests[selection].clone());
        self.state.save()?;

        println!(
            "{} {:?}",
            "Selected manifest:".green(),
            manifests[selection]
        );

        Ok(())
    }
}
