mod commit_info;
mod remote;
mod repo_info;
mod settings;

use commit_info::{CommitInfo, JsonType};
use remote::RemoteUrl;
use settings::Settings;
use skim::prelude::*;
use std::io::Write;
use std::process::Command;

fn execute_command(s: &Settings, selected_items: &Vec<Arc<dyn SkimItem>>) {
    println!("{} the following commit(s) now?", s.method);
    for i in selected_items {
        println!("* {}", i.text());
    }
    print!("(y/N) ");
    std::io::stdout().flush().unwrap();

    let mut line = String::new();
    let commands: Vec<String> = if Settings::is_git() {
        selected_items
            .iter()
            .map(|i| {
                format!(
                    "git fetch origin {} && git {} FETCH_HEAD",
                    i.output(),
                    s.method.to_lowercase()
                )
            })
            .collect()
    } else {
        selected_items
            .iter()
            .map(|i| {
                format!(
                    "repo download {} {}",
                    i.output(),
                    if s.method.to_lowercase() == "cherry-pick" {
                        "--cherry-pick"
                    } else {
                        ""
                    }
                )
            })
            .collect()
    };
    let command = commands.join(" && ");
    std::io::stdin()
        .read_line(&mut line)
        .expect("Could not read user input");
    if ["y", "yes"].contains(&line.trim().to_lowercase().as_str()) {
        let out = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .expect("Failed to run");
        println!("{}", std::str::from_utf8(&out.stderr).unwrap());
        println!("{}", std::str::from_utf8(&out.stdout).unwrap());
    } else {
        println!("Run '{}' to do it later", command);
    }
}

fn main() {
    let s = Settings::new();

    let selector = if s.select_all {
        DefaultSkimSelector::default().regex(".*")
    } else {
        DefaultSkimSelector::default().regex("")
    };
    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(true)
        .select1(true)
        .exit0(true)
        .selector(Some(Rc::new(selector)))
        .preview(Some("")) // preview should be specified to enable preview window
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    let json_type = match s.repo_info.remote_url {
        RemoteUrl::SSH(_) => JsonType::SSH,
        RemoteUrl::HTTP(_) => JsonType::HTTP,
    };
    json::parse(&s.repo_info.remote_url.perform_query(&s.query))
        .unwrap()
        .members()
        .cloned()
        .map(|data| CommitInfo::from_json(&json_type, &data))
        .map(Arc::new)
        .for_each(|x| {
            let _ = tx_item.send(x);
        });
    drop(tx_item); // so that skim could know when to stop waiting for more items.

    let res = &Skim::run_with(&options, Some(rx_item)).unwrap();
    if res.final_event == Event::EvActAbort {
        std::process::exit(1);
    }
    execute_command(&s, &res.selected_items)
}
