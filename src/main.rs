mod settings;

use settings::Settings;
use skim::prelude::*;
use std::process::Command;

#[derive(Debug)]
struct CommitInfo {
    title: String,
    body: String,
    download: String,
}

impl CommitInfo {
    fn new(title: String, body: String, download: String) -> Self {
        CommitInfo {
            title,
            body,
            download,
        }
    }
}

impl SkimItem for CommitInfo {
    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.title)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(self.body.to_string())
    }
    fn output(&self) -> Cow<str> {
        Cow::Borrowed(&self.download)
    }
}

fn get_data(s: &Settings) -> String {
    // Need to remove the first line as it contains the magic string )]}' to prevent
    // Cross Site Script Inclusion attacks (https://gerrit.onap.org/r/Documentation/rest-api.html#output)
    if s.file.is_empty() {
        let out = Command::new("curl")
            .arg("--netrc")
            .arg("--request")
            .arg("GET")
            .arg("--url")
            .arg(s.get_url())
            .arg("--header")
            .arg("Content-Type: application/json")
            .output()
            .expect("Failed to fetch commit data");
        std::str::from_utf8(&out.stdout).unwrap().split('\n').nth(1).unwrap().to_string()
        // reqwest::blocking::get(s.get_url())
        //     .unwrap()
        //     .text()
        //     .unwrap()
        //     .split('\n')
        //     .nth(1)
        //     .expect("Failed to get commit data")
        //     .to_string()
    } else {
        std::fs::read_to_string(&s.file).expect("Should have been able to read the file")
    }
}

fn execute_command(s: &Settings, selected_item: &Arc<dyn SkimItem>) {
    println!(
        "Would you like to {} the commit '{}' now? (y/N) ",
        s.method.to_lowercase(),
        selected_item.text()
    );

    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .expect("Could not read user input");
    if ["y", "yes"].contains(&line.trim().to_lowercase().as_str()) {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("'{}'", selected_item.output()))
            .output()
            .expect("Failed to run");
        dbg!(std::str::from_utf8(&out.stdout).unwrap());
    } else {
        println!("Run '{}' to do it later", selected_item.output());
    }
}

fn parse_data(s: &Settings, json_data: json::JsonValue) -> Vec<CommitInfo> {
    let mut commits: Vec<CommitInfo> = Vec::new();

    for item in json_data.members() {
        let current_revision = item["current_revision"].as_str().unwrap();
        let title = item["subject"]
            .as_str()
            .expect("Failed to find commit subject");
        let body = item["revisions"][current_revision]["commit"]["message"]
            .as_str()
            .expect("Failed to find commit message");
        let download = item["revisions"][current_revision]["fetch"][&s.fetch_method]["commands"]
            [&s.method]
            .as_str()
            .expect(&("Failed to find download link for ".to_string() + &s.fetch_method));
        commits.push(CommitInfo::new(
            title.to_string(),
            body.to_string(),
            download.to_string(),
        ));
    }
    commits
}

fn main() {
    let s = Settings::new();
    let data = get_data(&s);

    let json_data = json::parse(&data).unwrap();

    let commits = parse_data(&s, json_data);

    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(false)
        .select1(true)
        .exit0(true)
        .preview(Some("")) // preview should be specified to enable preview window
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    for commit in commits {
        let _ = tx_item.send(Arc::new(commit));
    }
    drop(tx_item); // so that skim could know when to stop waiting for more items.

    let res = &Skim::run_with(&options, Some(rx_item)).unwrap();
    if res.final_event == Event::EvActAbort {
        std::process::exit(1);
    }
    execute_command(&s, &res.selected_items[0])
}
