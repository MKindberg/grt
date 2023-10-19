mod settings;
mod commit_info;

use settings::Settings;
use commit_info::CommitInfo;
use skim::prelude::*;
use std::io::Write;
use std::process::Command;

fn get_data(s: &Settings) -> String {
    if s.file.is_empty() {
        let url = s.get_url();
        if url.starts_with("http") {
            let out = Command::new("curl")
                .arg("--netrc")
                .arg("--request")
                .arg("GET")
                .arg("--url")
                .arg(url)
                .arg("--header")
                .arg("Content-Type: application/json")
                .output()
                .expect("Failed to fetch http commit data");
            // Need to remove the first line as it contains the magic string )]}' to prevent
            // Cross Site Script Inclusion attacks (https://gerrit.onap.org/r/Documentation/rest-api.html#output)
            std::str::from_utf8(&out.stdout)
                .unwrap()
                .split('\n')
                .skip(1)
                .collect()
        // reqwest::blocking::get(s.get_url())
        //     .unwrap()
        //     .text()
        //     .unwrap()
        //     .split('\n')
        //     .nth(1)
        //     .expect("Failed to get commit data")
        //     .to_string()
        } else if url.starts_with("ssh") {
            let out = Command::new("ssh")
                .args(url.split_whitespace())
                .output()
                .expect("Faield to fetch ssh commit data");
            let mut items = std::str::from_utf8(&out.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<&str>>();
            items.pop(); // Last element contains stats
            format!("[{}]", items.join(","))
        } else {
            "".to_string()
        }
    } else {
        std::fs::read_to_string(&s.file).expect("Should have been able to read the file")
    }
}

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
    json::parse(&get_data(&s))
        .unwrap()
        .members()
        .cloned()
        .map(CommitInfo::from)
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
