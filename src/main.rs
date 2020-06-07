use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use gio::prelude::*;
use gtk::prelude::*;
use regex::Regex;
use std::env;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug)]
struct DesktopApp {
    name: String,
    exec: String,
}

fn get_desktop_files() -> Vec<String> {
    let xdg_dirs = env::var("XDG_DATA_DIRS").expect("XDG_DATA_DIRS isn't set");

    let dirs = xdg_dirs
        .split(":")
        .map(|p| Path::new(p).join("applications"))
        .map(|dir| std::fs::read_dir(dir))
        .filter(|dir| dir.is_ok())
        .flat_map(|d| d.unwrap());

    dirs.filter(|s| s.is_ok())
        .map(|s| s.unwrap())
        .map(|s| s.path().to_str().unwrap().to_string())
        .filter(|s| s.ends_with(".desktop"))
        .collect()
}

fn parse_desktop_file(file: String) -> Option<DesktopApp> {
    let groups: Vec<_> = file.split("\n\n").collect();

    let lines = groups
        .iter()
        .filter(|g| g.starts_with("[Desktop Entry]"))
        .take(1)
        .flat_map(|s| s.lines());

    if lines.clone().find(|l| l == &"NoDisplay=true").is_some() {
        return None;
    }

    let mut keys = lines.clone().map(|l| l.split("=").collect::<Vec<&str>>());

    let name = keys.find(|p| p[0] == "Name").map(|n| n[1].to_string())?;
    let exec = keys.find(|p| p[0] == "Exec").map(|n| n[1].to_string())?;
    let re = Regex::new(r"(?i) %[ufkci]").unwrap();
    let exec = re.replace(&exec, "").to_string();

    Some(DesktopApp { name, exec })
}

fn get_desktop_apps() -> Vec<DesktopApp> {
    let files = get_desktop_files();

    files
        .iter()
        .map(|f| std::fs::read_to_string(f).unwrap())
        .map(parse_desktop_file)
        .filter(|d| d.is_some())
        .map(|d| d.unwrap())
        .collect()
}

fn main() {
    let application =
        gtk::Application::new(Some("com.github.technical27.search"), Default::default())
            .expect("failed to create application");

    application.connect_activate(|app| {
        let win = gtk::ApplicationWindow::new(app);
        win.add_events(gdk::EventMask::KEY_PRESS_MASK);

        win.set_title("Search...");
        win.set_default_size(350, 70);
        win.set_decorated(false);
        win.set_position(gtk::WindowPosition::Center);

        let text = gtk::Entry::new();
        text.set_placeholder_text(Some("Search..."));
        win.add(&text);

        {
            let app = app.clone();
            win.connect_key_press_event(move |_, key| {
                if key.get_keyval() == gdk::enums::key::Return {
                    if let Some(text) = text.get_text() {
                        let matcher = SkimMatcherV2::default();
                        let apps = get_desktop_apps();
                        let mut matches = apps
                            .clone()
                            .iter()
                            .map(|app| matcher.fuzzy_match(&app.name, text.as_str()))
                            .enumerate()
                            .filter(|res| res.1.is_some())
                            .map(|res| (res.0, res.1.unwrap()))
                            .filter(|res| res.1 > 0)
                            .collect::<Vec<_>>();

                        matches.sort_by(|a, b| b.1.cmp(&a.1));

                        let matches = matches
                            .iter()
                            .map(|m| (apps[m.0].clone(), m.1))
                            .take(1)
                            .collect::<Vec<_>>();

                        if let Some(app) = matches.get(0) {
                            println!("app: {:?}", app);
                            let exec = app.0.exec.clone();
                            let args: Vec<_> = exec.split(" ").collect();
                            Command::new(args[0]).args(&args[1..]).spawn().unwrap();
                        } else {
                            println!("no app found");
                        }
                    }
                    app.quit();
                }

                glib::signal::Inhibit(false)
            });
        }

        win.show_all();
    });

    application.run(&[]);
}
