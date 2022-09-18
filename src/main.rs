mod settings;

use settings::Settings;
use skim::prelude::*;
use std::process::Command;

#[derive(Debug)]
struct CommitInfo {
    title: String,
    author: String,
    body: String,
    reference: String,
}

impl CommitInfo {
    fn new(title: String, author: String, body: String, reference: String) -> Self {
        CommitInfo {
            title,
            author,
            body,
            reference,
        }
    }
}

impl From<json::JsonValue> for CommitInfo {
    fn from(data: json::JsonValue) -> Self {
        let current_revision = data["current_revision"].as_str().unwrap_or("");
        let title = data["subject"]
            .as_str()
            .expect("Failed to find commit subject");
        let author = data["revisions"][current_revision]["commit"]["author"]["name"]
            .as_str()
            .unwrap_or_else(|| {
                data["currentPatchSet"]["author"]["name"]
                    .as_str()
                    .expect("Failed to find commit author")
            });
        let body = data["revisions"][current_revision]["commit"]["message"]
            .as_str()
            .unwrap_or_else(|| {
                data["commitMessage"]
                    .as_str()
                    .expect("Failed to find commit message")
            });
        let reference = data["revisions"][current_revision]["ref"]
            .as_str()
            .unwrap_or_else(|| {
                data["currentPatchSet"]["ref"]
                    .as_str()
                    .expect("Failed to find ref")
            });
        Self::new(
            title.to_string(),
            author.to_string(),
            body.to_string(),
            reference.to_string(),
        )
    }
}

impl SkimItem for CommitInfo {
    fn text(&self) -> Cow<str> {
        Cow::Owned(format!("{} - {}", self.title, self.author))
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(self.body.to_string())
    }
    fn output(&self) -> Cow<str> {
        Cow::Borrowed(&self.reference)
    }
}

fn get_data(s: &Settings) -> String {
    // Need to remove the first line as it contains the magic string )]}' to prevent
    // Cross Site Script Inclusion attacks (https://gerrit.onap.org/r/Documentation/rest-api.html#output)
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
            std::str::from_utf8(&out.stdout)
                .unwrap()
                .lines()
                .nth(1)
                .unwrap()
                .to_string()
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

fn execute_command(s: &Settings, selected_item: &Arc<dyn SkimItem>) {
    println!("{} '{}' now? (y/N) ", s.method, selected_item.text());

    let mut line = String::new();
    let command = format!(
        "git fetch origin {}; git {} FETCH_HEAD",
        selected_item.output(),
        s.method.to_lowercase()
    );
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

    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(false)
        .select1(true)
        .exit0(true)
        .preview(Some("")) // preview should be specified to enable preview window
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    let _ = json::parse(&get_data(&s))
        .unwrap()
        .members()
        .cloned()
        .map(CommitInfo::from)
        .map(Arc::new)
        .map(|x| tx_item.send(x));
    drop(tx_item); // so that skim could know when to stop waiting for more items.

    let res = &Skim::run_with(&options, Some(rx_item)).unwrap();
    if res.final_event == Event::EvActAbort {
        std::process::exit(1);
    }
    execute_command(&s, &res.selected_items[0])
}
