mod app;
mod git;
mod review;
mod theme;
mod ui;
mod watcher;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use gtk::prelude::*;

#[derive(Debug, Parser)]
#[command(
    name = "zako",
    version,
    about = "A lightweight desktop diff viewer for reviewing agent-written code"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(value_name = "REPOSITORY", default_value = ".")]
    repository: PathBuf,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect and export local review comments.
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ReviewCommand {
    /// Print all review comments.
    List {
        #[arg(value_name = "REPOSITORY", default_value = ".")]
        repository: PathBuf,
    },
    /// Export unresolved review comments as Markdown.
    Export {
        #[arg(value_name = "REPOSITORY", default_value = ".")]
        repository: PathBuf,
    },
    /// Print unresolved and resolved comment counts.
    Status {
        #[arg(value_name = "REPOSITORY", default_value = ".")]
        repository: PathBuf,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("zako: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    if let Some(Command::Review { command }) = cli.command {
        let target = match &command {
            ReviewCommand::List { repository }
            | ReviewCommand::Export { repository }
            | ReviewCommand::Status { repository } => repository,
        };
        let repository = git::Repository::discover(target)?;
        let store = review::ReviewStore::open(repository.root())?;
        match command {
            ReviewCommand::List { .. } => {
                let mut comments: Vec<_> = store.document.comments.iter().collect();
                comments.sort_by_key(|comment| (&comment.file, comment.start_line));
                if comments.is_empty() {
                    println!("No review comments.");
                }
                for comment in comments {
                    let marker = if comment.resolved { "✓" } else { "●" };
                    let stale = if comment.stale { " [stale]" } else { "" };
                    let side = if comment.side == review::DiffSide::Old {
                        " old"
                    } else {
                        ""
                    };
                    let body = review::display_file_path(&comment.body);
                    println!(
                        "{marker} {} {}{side}{stale}  {}",
                        review::display_file_path(&comment.file),
                        review::line_range(comment.start_line, comment.end_line),
                        body
                    );
                }
            }
            ReviewCommand::Export { .. } => {
                print!("{}", review::export_markdown(&store.document));
            }
            ReviewCommand::Status { .. } => {
                println!("{} unresolved comments", store.document.unresolved_count());
                println!("{} resolved comments", store.document.resolved_count());
                println!("{} total comments", store.document.comments.len());
            }
        }
        return Ok(());
    }

    let target = if cli.repository.as_os_str() == "." {
        std::env::current_dir()?
    } else {
        cli.repository
    };
    let repository = git::Repository::discover(&target)?;
    let application = gtk::Application::builder()
        .application_id("dev.zako.Zako")
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    application.connect_activate(move |application| {
        app::build(application, repository.clone());
    });
    let program = std::env::args_os().next().unwrap_or_else(|| "zako".into());
    application.run_with_args_os(&[program]);
    Ok(())
}
