use skim::prelude::*;
use std::env;
use std::process::Command;

struct Settings {
    method: String,
    base_url: String,
    query: String,
    query_fields: String,
}

impl Settings {
    fn new(args: Vec<String>) -> Self {
        let method = match args[1].as_str() {
            "checkout" | "co" => "Checkout".to_string(),
            "cherry-pick" | "cp" => "Cherry pick".to_string(),
            _ => {
                println!("Unsupported operation");
                std::process::exit(1);
            }
        };
        let mut query = if args.len() == 2 {
            "status:open".to_string()
        } else {
            args.join(" ")
        };
        let repo = "".to_string();
        if !repo.is_empty() {
            query += &(" project:".to_string() + &repo);
        }
        Self {
            method,
            base_url: std::env::var("GERRIT_URL").expect("Must set environment var GERRIT_URL"),
            query,
            query_fields: "o=CURRENT_REVISION&o=CURRENT_COMMIT&o=CURRENT_FILES&o=DOWNLOAD_COMMANDS"
                .to_string(),
        }
    }
    fn get_url(&self) -> String {
        let mut url: String = self.base_url.to_string();

        if !url.ends_with('/') {
            url += "/";
        }
        url += "changes/?q=";
        url += &self.query.replace(' ', "+");
        url += "&";
        url += &self.query_fields;
        url
    }
}

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

fn help() {
    println!("grt <checkout|co|cherry-pick|cp> [search]");
    std::process::exit(1);
}

fn get_data(url: &str) -> String {
    // Need to remove the first line as it contains the magic string )]}' to prevent
    // Cross Site Script Inclusion attacks (https://gerrit.onap.org/r/Documentation/rest-api.html#output)
    reqwest::blocking::get(url)
        .unwrap()
        .text()
        .unwrap()
        .split('\n')
        .nth(1)
        .expect("Failed to get commit data")
        .to_string()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        help();
    }
    let s = Settings::new(args);
    let data = get_data(&s.get_url());
    let json_data = json::parse(&data).unwrap();

    let mut commits: Vec<CommitInfo> = Vec::new();
    for item in json_data.members() {
        let current_revision = item["current_revision"].as_str().unwrap();
        let title = item["subject"]
            .as_str()
            .expect("Failed to find commit subject");
        let body = item["revisions"][current_revision]["commit"]["message"]
            .as_str()
            .expect("Failed to find commit message");
        let download = item["revisions"][current_revision]["fetch"]["anonymous http"]["commands"]
            [&s.method]
            .as_str()
            .expect("Failed to find download link");
        commits.push(CommitInfo::new(
            title.to_string(),
            body.to_string(),
            download.to_string(),
        ));
    }

    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(false)
        .preview(Some("")) // preview should be specified to enable preview window
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    for commit in commits {
        let _ = tx_item.send(Arc::new(commit));
    }
    drop(tx_item); // so that skim could know when to stop waiting for more items.

    let selected_item = &Skim::run_with(&options, Some(rx_item))
        .unwrap()
        .selected_items[0];

    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("echo '{}'", selected_item.output()))
        .output()
        .expect("Failed to run");
    dbg!(std::str::from_utf8(&out.stdout).unwrap());
}
