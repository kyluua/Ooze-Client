use clap::{Parser, Subcommand, ValueEnum};
use ooze_launcher_core::library as lib;
use ooze_launcher_core::*;
use std::{path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(
    version,
    about = "Ooze L0 backend developer CLI. Uses real Microsoft authentication; no offline launch mode."
)]
struct Args {
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(long)]
    release: Option<PathBuf>,
    #[command(subcommand)]
    command: Action,
}
#[derive(Subcommand)]
enum Action {
    Library {
        #[command(subcommand)]
        command: LibraryAction,
    },
    Instance {
        id: String,
        #[command(subcommand)]
        command: InstanceAction,
    },
    Status,
    SignIn,
    SignOut,
    Install,
    Repair,
    Play,
    ExportDiagnostics {
        destination: PathBuf,
    },
    Settings {
        #[arg(long)]
        instance: Option<PathBuf>,
        #[arg(long, conflicts_with = "managed_java")]
        java: Option<PathBuf>,
        #[arg(long)]
        managed_java: bool,
        #[arg(long)]
        memory_mib: Option<u32>,
    },
}
#[derive(Clone, Copy, ValueEnum)]
enum PresetArg {
    Fps,
    Pvp,
    Minimal,
}
impl From<PresetArg> for lib::Preset {
    fn from(v: PresetArg) -> Self {
        match v {
            PresetArg::Fps => Self::Fps,
            PresetArg::Pvp => Self::Pvp,
            PresetArg::Minimal => Self::Minimal,
        }
    }
}
#[derive(Subcommand)]
enum InstanceAction {
    Install,
    Repair,
    Play,
}
#[derive(Subcommand)]
enum LibraryAction {
    Appearance {
        #[arg(long)]
        import: Option<PathBuf>,
    },
    Worlds {
        id: String,
    },
    Repair {
        id: String,
        project: String,
    },
    List,
    Create {
        name: String,
        #[arg(long)]
        vanilla: bool,
        #[arg(long)]
        managed: bool,
        #[arg(long, value_enum, default_value = "minimal")]
        preset: PresetArg,
    },
    Select {
        id: String,
    },
    Rename {
        id: String,
        name: String,
    },
    Duplicate {
        id: String,
        name: String,
    },
    Archive {
        id: String,
    },
    Custom {
        id: String,
    },
    Verify {
        id: String,
    },
    ImportProfile {
        id: String,
        file: PathBuf,
    },
    ExportProfile {
        id: String,
    },
    ImportTheme {
        id: String,
        file: PathBuf,
    },
    ExportTheme {
        id: String,
    },
    Search {
        #[arg(value_enum)]
        kind: KindArg,
        #[arg(default_value = "")]
        query: String,
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    Project {
        project: String,
    },
    Versions {
        project: String,
    },
    Install {
        id: String,
        project: String,
        version: String,
        #[arg(long)]
        world: Option<String>,
    },
    Enable {
        id: String,
        project: String,
    },
    Disable {
        id: String,
        project: String,
    },
    Remove {
        id: String,
        project: String,
    },
}
#[derive(Clone, Copy, ValueEnum)]
enum KindArg {
    Mod,
    Resourcepack,
    Datapack,
    Shader,
}
impl From<KindArg> for lib::ContentKind {
    fn from(v: KindArg) -> Self {
        match v {
            KindArg::Mod => Self::Mod,
            KindArg::Resourcepack => Self::Resourcepack,
            KindArg::Datapack => Self::Datapack,
            KindArg::Shader => Self::Shader,
        }
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let data = args
        .data_dir
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("OozeLauncher")))
        .ok_or("LOCALAPPDATA is missing; pass --data-dir with an absolute local directory")?;
    if let Action::Library { command } = args.command {
        return run_library(data, command);
    }
    let release = args.release.unwrap_or(
        std::env::current_exe()?
            .parent()
            .ok_or("Executable directory missing")?
            .join("release.json"),
    );
    let client_id = std::env::var("OOZE_MICROSOFT_CLIENT_ID")
        .ok()
        .or_else(|| option_env!("OOZE_MICROSOFT_CLIENT_ID").map(str::to_owned));
    let launcher = Launcher::open(LauncherConfig {
        data_dir: data.clone(),
        release_manifest: release,
        microsoft_client_id: client_id,
    })?;
    let command = match args.command {
        Action::Library { .. } => unreachable!(),
        Action::Instance { id, command } => {
            let library = lib::Library::open(&data)?;
            let instance = library
                .snapshot()
                .instances
                .into_iter()
                .find(|i| i.instance.id.0 == id && !i.instance.archived)
                .ok_or("Unknown or archived instance")?;
            let mut settings = launcher.snapshot().settings;
            settings.instance_dir = instance.game_dir;
            run_command(&launcher, LauncherCommand::SaveSettings(settings))?;
            match command {
                InstanceAction::Install => LauncherCommand::Install,
                InstanceAction::Repair => LauncherCommand::Repair,
                InstanceAction::Play => LauncherCommand::Launch,
            }
        }
        Action::Status => {
            println!("{:#?}", launcher.snapshot());
            return Ok(());
        }
        Action::SignIn => LauncherCommand::SignIn,
        Action::SignOut => LauncherCommand::SignOut,
        Action::Install => LauncherCommand::Install,
        Action::Repair => LauncherCommand::Repair,
        Action::Play => LauncherCommand::Launch,
        Action::ExportDiagnostics { destination } => {
            LauncherCommand::ExportDiagnostics(destination)
        }
        Action::Settings {
            instance,
            java,
            managed_java,
            memory_mib,
        } => {
            let mut settings = launcher.snapshot().settings;
            if let Some(path) = instance {
                settings.instance_dir = path;
            }
            if let Some(path) = java {
                settings.java_path = Some(path);
            }
            if managed_java {
                settings.java_path = None;
            }
            if let Some(memory) = memory_mib {
                settings.memory_mib = memory;
            }
            LauncherCommand::SaveSettings(settings)
        }
    };
    run_command(&launcher, command)
}
fn run_command(
    launcher: &Launcher,
    command: LauncherCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let id = launcher.submit(command)?;
    loop {
        if let Some(event) = launcher.try_event() {
            match event {
                LauncherEvent::Completed { id: event } if event == id => {
                    println!("Operation {} completed", id.0);
                    return Ok(());
                }
                LauncherEvent::Cancelled { id: event } if event == id => {
                    return Err("Operation cancelled".into());
                }
                LauncherEvent::Failed { id: event, error } if event == id => {
                    return Err(error.into());
                }
                event => println!("{event:?}"),
            }
        } else {
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}

fn read_import(path: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(65537)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err("Configuration import exceeds 64 KiB".into());
    }
    Ok(String::from_utf8(bytes)?)
}
fn library_wait(
    library: &lib::Library,
    command: lib::Command,
) -> Result<lib::Reply, Box<dyn std::error::Error>> {
    let id = library.submit(command)?;
    loop {
        if let Some(lib::Event::Finished { id: event, result }) = library.try_event()
            && event == id
        {
            while library.snapshot().active_operation.is_some() {
                std::thread::sleep(Duration::from_millis(1));
            }
            return result.map_err(Into::into);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn run_library(data: PathBuf, command: LibraryAction) -> Result<(), Box<dyn std::error::Error>> {
    let library = lib::Library::open(data)?;
    let command = match command {
        LibraryAction::Appearance { import } => {
            if let Some(file) = import {
                lib::Command::ImportAppearance(read_import(&file)?)
            } else {
                lib::Command::ExportAppearance
            }
        }
        LibraryAction::Worlds { id } => lib::Command::ListWorlds(lib::Id(id)),
        LibraryAction::Repair { id, project } => lib::Command::RepairContent {
            instance: lib::Id(id),
            project,
        },
        LibraryAction::List => {
            println!("{}", serde_json::to_string_pretty(&library.snapshot())?);
            return Ok(());
        }
        LibraryAction::Create {
            name,
            vanilla,
            managed,
            preset,
        } => lib::Command::Create {
            name,
            target: if vanilla {
                lib::Target::vanilla()
            } else {
                lib::Target::fabric()
            },
            preset: preset.into(),
            mode: if managed {
                lib::Mode::Managed
            } else {
                lib::Mode::Custom
            },
        },
        LibraryAction::Select { id } => lib::Command::Select(lib::Id(id)),
        LibraryAction::Rename { id, name } => lib::Command::Rename {
            instance: lib::Id(id),
            name,
        },
        LibraryAction::Duplicate { id, name } => lib::Command::Duplicate {
            instance: lib::Id(id),
            name,
        },
        LibraryAction::Archive { id } => lib::Command::Archive(lib::Id(id)),
        LibraryAction::Custom { id } => lib::Command::ConvertToCustom(lib::Id(id)),
        LibraryAction::Verify { id } => lib::Command::Verify(lib::Id(id)),
        LibraryAction::ImportProfile { id, file } => lib::Command::ImportProfile {
            instance: lib::Id(id),
            json: read_import(&file)?,
        },
        LibraryAction::ImportTheme { id, file } => lib::Command::ImportTheme {
            instance: lib::Id(id),
            json: read_import(&file)?,
        },
        LibraryAction::ExportProfile { id } => lib::Command::ExportProfile(lib::Id(id)),
        LibraryAction::ExportTheme { id } => lib::Command::ExportTheme(lib::Id(id)),
        LibraryAction::Search {
            kind,
            query,
            offset,
        } => lib::Command::Search(lib::Search {
            query,
            kind: kind.into(),
            target: lib::Target::fabric(),
            offset,
            limit: 10,
        }),
        LibraryAction::Project { project } => lib::Command::Project(project),
        LibraryAction::Versions { project } => lib::Command::Versions {
            project,
            target: lib::Target::fabric(),
        },
        LibraryAction::Install {
            id,
            project,
            version,
            world,
        } => {
            let reply = library_wait(
                &library,
                lib::Command::Plan {
                    instance: lib::Id(id),
                    project,
                    version,
                    world,
                },
            )?;
            let lib::Reply::Plan(plan) = reply else {
                return Err("Backend did not return a content plan".into());
            };
            lib::Command::Apply(plan.id)
        }
        LibraryAction::Enable { id, project } => lib::Command::SetEnabled {
            instance: lib::Id(id),
            project,
            enabled: true,
        },
        LibraryAction::Disable { id, project } => lib::Command::SetEnabled {
            instance: lib::Id(id),
            project,
            enabled: false,
        },
        LibraryAction::Remove { id, project } => lib::Command::RemoveContent {
            instance: lib::Id(id),
            project,
        },
    };
    let reply = library_wait(&library, command)?;
    if let lib::Reply::Export(json) = reply {
        println!("{json}");
    } else {
        println!("{}", serde_json::to_string_pretty(&reply)?);
    }
    Ok(())
}
