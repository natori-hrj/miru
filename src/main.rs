mod app;
mod git;
mod theme;
mod ui;
mod watcher;

use std::path::PathBuf;

use gtk::prelude::*;

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() > 1 {
        eprintln!("usage: zako [repository]");
        std::process::exit(2);
    }

    let target = args.first().map(PathBuf::from).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|error| {
            eprintln!("zako: cannot determine current directory: {error}");
            std::process::exit(2);
        })
    });

    let repository = match git::Repository::discover(&target) {
        Ok(repository) => repository,
        Err(error) => {
            eprintln!("zako: {error}");
            std::process::exit(1);
        }
    };

    let application = gtk::Application::builder()
        .application_id("dev.zako.Zako")
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    application.connect_activate(move |application| {
        app::build(application, repository.clone());
    });
    let program = std::env::args_os().next().unwrap_or_else(|| "zako".into());
    application.run_with_args_os(&[program]);
}
