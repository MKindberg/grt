mod commit_info;
mod remote;
mod repo_info;
mod settings;

use commit_info::CommitInfo;
use lazy_static::lazy_static;
use settings::Settings;
use skim::prelude::*;
use std::collections::HashSet;
use std::io::Write;
use std::process::Command;

lazy_static! {
    static ref SETTINGS: Settings = Settings::new();
}

fn execute_command(selected_items: &Vec<Arc<dyn SkimItem>>) {
    let mut line = String::new();
    let mut topics: Vec<&str> = Vec::new();
    let mut refs: HashSet<(String, String, String)> = HashSet::new();
    for item in selected_items {
        let commit = (**item)
            .as_any()
            .downcast_ref::<CommitInfo>()
            .expect("Could not cast to CommitInfo");
        if let Some(t) = &commit.topic {
            topics.push(t);
        }
        refs.insert((commit.get_title(), commit.get_git_reference(), commit.get_repo_reference()));
    }
    if !topics.is_empty() {
        println!("Your selected commits are part of the following topic(s):");
        for t in &topics {
            println!("* {}", t);
        }
        println!("Would you like to download those commits as well? (y/N)");
        std::io::stdin()
            .read_line(&mut line)
            .expect("Could not read user input");
        if ["y", "yes"].contains(&line.trim().to_lowercase().as_str()) {
            for t in &topics {
                let commits = SETTINGS
                    .repo_info
                    .remote_url
                    .perform_query(&format!("status:open topic:{}", t));
                for c in &commits {
                    refs.insert((c.get_title(), c.get_git_reference(), c.get_repo_reference()));
                }
            }
        }
        line.clear();
    }
    println!("{} the following commit(s) now?", SETTINGS.method);
    for (t, _, _) in &refs {
        println!("* {}", t);
    }
    print!("(y/N) ");
    std::io::stdout().flush().unwrap();

    let commands: Vec<String> = if topics.is_empty() && Settings::is_git() {
        refs.iter()
            .map(|(_, i, _)| {
                format!(
                    "git fetch origin {} && git {} FETCH_HEAD",
                    i,
                    SETTINGS.method.to_lowercase()
                )
            })
            .collect()
    } else {
        refs.iter()
            .map(|(_, _, i)| {
                format!(
                    "repo download {} {}",
                    i,
                    if SETTINGS.method.to_lowercase() == "cherry-pick" {
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
    let selector = if SETTINGS.select_all {
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
    SETTINGS
        .repo_info
        .remote_url
        .perform_query(&SETTINGS.query)
        .iter()
        .cloned()
        .map(Arc::new)
        .for_each(|x| {
            let _ = tx_item.send(x);
        });
    drop(tx_item); // so that skim could know when to stop waiting for more items.

    let res = &Skim::run_with(&options, Some(rx_item)).unwrap();
    if res.final_event == Event::EvActAbort {
        std::process::exit(1);
    }
    execute_command(&res.selected_items)
}
