mod settings;

use settings::Settings;
use skim::prelude::*;
use std::process::Command;

#[derive(Debug)]
struct CommitInfo {
    title: String,
    body: String,
    reference: String,
}

impl CommitInfo {
    fn new(
        is_git: bool,
        project: String,
        title: String,
        author: String,
        body: String,
        reference: String,
        files: Vec<String>,
    ) -> Self {
        CommitInfo {
            title: if is_git {
                "".to_string()
            } else {
                project.to_string() + " - "
            } + &title
                + " - "
                + &author,
            body: body + "\n\n" + &files.join("\n"),
            reference: if is_git {
                reference
            } else {
                project + ".git " + &reference.split('/').collect::<Vec<&str>>()[3..].join("/")
            },
        }
    }
}

impl From<json::JsonValue> for CommitInfo {
    fn from(data: json::JsonValue) -> Self {
        let current_revision = data["current_revision"].as_str().unwrap_or("");
        let project = data["project"]
            .as_str()
            .expect("Failed to get project name");
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
        let mut files: Vec<String> = Vec::new();
        for file in data["revisions"][current_revision]["files"].entries() {
            files.push(format!(
                "{} {} +{} -{}",
                file.1["status"].as_str().unwrap_or(""),
                file.0,
                file.1["lines_inserted"].as_i32().unwrap_or(0),
                file.1["lines_deleted"].as_i32().unwrap_or(0)
            ));
        }

        for file in data["currentPatchSet"]["files"].members().skip(1) {
            files.push(format!(
                "{} {} +{} -{}",
                file["type"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .next()
                    .unwrap_or(' '),
                file["file"],
                file["insertions"],
                file["deletions"]
            ));
        }
        Self::new(
            Settings::is_git(),
            project.to_string(),
            title.to_string(),
            author.to_string(),
            body.to_string(),
            reference.to_string(),
            files,
        )
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
        Cow::Borrowed(&self.reference)
    }
}

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
    println!("{} the following commit(s) now? (y/N) ", s.method);
    for i in selected_items {
        println!("* {}", i.text());
    }

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

    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(true)
        .select1(true)
        .exit0(true)
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
