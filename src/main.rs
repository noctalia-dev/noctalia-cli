use clap::{Parser, Subcommand};

mod install;
mod update;
mod run;
mod ipc;
mod config;
mod ui;

pub use config::SourceKind;

#[derive(Parser, Debug)]
#[command(
    name = "noctalia",
    version,
    about = "Noctalia CLI",
    long_about = "A simple CLI for installing and updating Noctalia components.",
    arg_required_else_help = true,
    help_template = "{about-with-newline}Usage:\n  {usage}\n\nCommands:\n{subcommands}\nOptions:\n{options}\n\nExamples:\n  noctalia install shell --release\n  noctalia update shell\n  noctalia run\n  noctalia ipc call <target> <function>\n  noctalia ipc show\n"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(
        arg_required_else_help = true,
        about = "Install noctalia-shell",
        help_template = "Install\n\nUsage:\n  {usage}\n\nComponents:\n{subcommands}\nOptions:\n{options}\n\nExamples:\n  noctalia install shell --release\n"
    )]
    Install(InstallTargets),
    #[command(
        arg_required_else_help = true,
        about = "Update noctalia-shell",
        help_template = "Update\n\nUsage:\n  {usage}\n\nComponents:\n{subcommands}\nOptions:\n{options}\n\nExamples:\n  noctalia update shell\n"
    )]
    Update(UpdateTargets),
    #[command(
        about = "Run noctalia-shell",
        long_about = "Start the noctalia-shell using quickshell (qs -c noctalia-shell).",
        help_template = "Run Shell\n\nUsage:\n  {usage}\n\nOptions:\n{options}\n\nExamples:\n  noctalia run\n  noctalia run --debug\n"
    )]
    Run {
        /// Run noctalia-shell with debug mode enabled (NOCTALIA_DEBUG=1)
        #[arg(long)]
        debug: bool,
    },
    #[command(
        about = "IPC commands for noctalia-shell",
        long_about = "Send IPC commands to the running noctalia-shell instance.",
        help_template = "IPC\n\nUsage:\n  {usage}\n\nSubcommands:\n{subcommands}\n\nExamples:\n  noctalia ipc call <target> <function>\n  noctalia ipc show\n"
    )]
    Ipc(IpcTargets),
}

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
struct InstallTargets {
    #[command(subcommand)]
    target: InstallSub,
}

#[derive(Subcommand, Debug)]
enum InstallSub {
    #[command(
        about = "Install the Noctalia shell",
        long_about = "Install the Noctalia shell from either the latest release or git main.",
        help_template = "Install Shell\n\nUsage:\n  {usage}\n\nOptions:\n{options}\n\nExamples:\n  noctalia install shell --release\n  noctalia install shell --git\n"
    )]
    Shell { #[arg(long)] git: bool, #[arg(long)] release: bool },
}

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
struct UpdateTargets {
    #[command(subcommand)]
    target: UpdateSub,
}

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
struct IpcTargets {
    #[command(subcommand)]
    target: IpcSub,
}

#[derive(Subcommand, Debug)]
enum IpcSub {
    #[command(
        about = "Send an IPC call to noctalia-shell",
        help_template = "IPC Call\n\nUsage:\n  {usage}\n\nArguments:\n{args}\n\nExamples:\n  noctalia ipc call <target> <function>\n"
    )]
    Call {
        /// Target name for the IPC call
        target: String,
        /// Function name for the IPC call
        function: String,
    },
    #[command(
        about = "Show available IPC targets and functions",
        help_template = "IPC Show\n\nUsage:\n  {usage}\n\nExamples:\n  noctalia ipc show\n"
    )]
    Show,
}

#[derive(Subcommand, Debug)]
enum UpdateSub {
    #[command(
        about = "Update the Noctalia shell",
        help_template = "Update Shell\n\nUsage:\n  {usage}\n\nOptions:\n{options}\n\nExamples:\n  noctalia update shell --release\n  noctalia update shell --git\n"
    )]
    Shell { #[arg(long)] git: bool, #[arg(long)] release: bool },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install(InstallTargets { target }) => {
            let (cfg, _path) = config::CliConfig::load().expect("load config");
            match target {
                InstallSub::Shell { git, release } => {
                    let resolved = resolve_source("shell", git, release, &cfg);
                    install::shell::run(resolved);
                }
            }
        }
        Commands::Update(UpdateTargets { target }) => {
            let (cfg, _path) = config::CliConfig::load().expect("load config");
            match target {
                UpdateSub::Shell { git, release } => {
                    let resolved = resolve_source("shell", git, release, &cfg);
                    update::shell::run(resolved);
                }
            }
        }
        Commands::Run { debug } => {
            run::shell::run(debug);
        }
        Commands::Ipc(IpcTargets { target }) => {
            match target {
                IpcSub::Call { target, function } => {
                    ipc::shell::run_call(target, function);
                }
                IpcSub::Show => {
                    ipc::shell::run_show();
                }
            }
        }
    }
}

fn resolve_source(component: &str, git: bool, release: bool, cfg: &config::CliConfig) -> SourceKind {
    if git && release {
        eprintln!("Both --git and --release provided; please specify only one.");
        std::process::exit(2);
    }
    if git { return SourceKind::Git; }
    if release { return SourceKind::Release; }

    if let Some(saved) = cfg.get_component_source(component) {
        return saved;
    }

    prompt_and_persist_choice(component)
}

fn prompt_and_persist_choice(component: &str) -> SourceKind {
    use dialoguer::{theme::ColorfulTheme, Select};
    let (mut cfg, path) = config::CliConfig::load().expect("load config");
    let items = ["release", "git"];
    let theme = ColorfulTheme::default();
    let selection = Select::with_theme(&theme)
        .with_prompt(format!("Choose source for {}", component))
        .default(0)
        .items(&items)
        .interact_opt();

    let chosen = match selection {
        Ok(Some(idx)) => if idx == 1 { SourceKind::Git } else { SourceKind::Release },
        _ => {
            // Non-interactive or error: default to release
            SourceKind::Release
        }
    };

    cfg.set_component_source(component, chosen);
    let _ = cfg.save(&path);
    chosen
}
