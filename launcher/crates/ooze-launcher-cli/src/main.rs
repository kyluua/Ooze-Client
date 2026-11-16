use clap::{Parser, Subcommand};
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
    Status,
    SignIn,
    SignOut,
    Install,
    Repair,
    Play,
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
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let default_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or("LOCALAPPDATA is missing; pass a Windows local data directory")?
        .join("OozeLauncher");
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
        data_dir: args.data_dir.unwrap_or(default_data),
        release_manifest: release,
        microsoft_client_id: client_id,
    })?;
    let command = match args.command {
        Action::Status => {
            println!("{:#?}", launcher.snapshot());
            return Ok(());
        }
        Action::SignIn => LauncherCommand::SignIn,
        Action::SignOut => LauncherCommand::SignOut,
        Action::Install => LauncherCommand::Install,
        Action::Repair => LauncherCommand::Repair,
        Action::Play => LauncherCommand::Launch,
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
