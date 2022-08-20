use getopts::Options;
use skim::prelude::*;
use std::env;
use std::process::Command;

struct Settings {
    method: String,
    base_url: String,
    query: String,
    file: String,
    debug: bool,
    query_fields: String,
    download_with: String,
}

fn print_usage(program: &str, opts: Options) -> ! {
    let brief = format!(
        "Usage: {} [options] checkout|co|cherry-pick|cp [query]",
        program
    );
    print!("{}", opts.usage(&brief));
    std::process::exit(1);
}

impl Settings {
    fn new(args: Vec<String>) -> Self {
        let mut opts = Options::new();
        opts.optopt(
            "f",
            "file",
            "Read json data from file instead of Gerrit",
            "FILE",
        );
        opts.optopt("p", "project", "The project to search in", "NAME");
        opts.optopt("u", "url", "The url to Gerrit", "URL");
        opts.optflag("", "debug", "Print debug information while running");
        opts.optflag("h", "help", "Print this menu");
        opts.optflag("c", "closed", "include closed commits");
        opts.optflag("", "ssh", "Download over ssh");
        opts.optflag("", "https", "Download over https");
        opts.optflag("", "anon", "Download over anonymous https");

        let matches = opts.parse(&args[1..]).expect("Failed to parse args");

        if matches.opt_present("help") {
            print_usage(&args[0], opts);
        }

        let method = match matches.free[0].as_str() {
            "checkout" | "co" => "Checkout".to_string(),
            "cherry-pick" | "cp" => "Cherry pick".to_string(),
            op => {
                println!("Unsupported operation '{}'", op);
                print_usage(&args[0], opts);
            }
        };

        let debug = matches.opt_present("debug");

        let file = matches.opt_str("file").unwrap_or_else(|| "".to_string());

        let download_with = if matches.opt_present("ssh") {
            "ssh".to_string()
        } else if matches.opt_present("https") {
            "https".to_string()
        } else if matches.opt_present("anon") {
            "anonymous http".to_string()
        } else {
            "ssh".to_string()
        };

        let query = Self::create_query(
            matches.opt_present("closed"),
            matches.opt_str("project"),
            &matches.free[1..].join(" "),
        );
        if debug {
            println!("Query: '{}'", query);
        }
        let base_url = matches.opt_str("url").unwrap_or_else(|| {
            std::env::var("GERRIT_URL")
                .expect("Must set environment var GERRIT_URL or pass --url flag")
        });

        Self {
            method,
            base_url,
            debug,
            file,
            query,
            query_fields: "o=CURRENT_REVISION&o=CURRENT_COMMIT&o=CURRENT_FILES&o=DOWNLOAD_COMMANDS"
                .to_string(),
            download_with,
        }
    }

    fn create_query(include_closed: bool, project: Option<String>, query: &str) -> String {
        let mut q = "".to_string();
        if !include_closed {
            q += "status:open ";
        }
        if let Some(p) = project {
            q += format!("project:{} ", p).as_str();
        }
        q += query;
        q
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
        if self.debug {
            println!("Url: {}", url);
        }
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

fn get_data(s: &Settings) -> String {
    // Need to remove the first line as it contains the magic string )]}' to prevent
    // Cross Site Script Inclusion attacks (https://gerrit.onap.org/r/Documentation/rest-api.html#output)
    if s.file.is_empty() {
        reqwest::blocking::get(s.get_url())
            .unwrap()
            .text()
            .unwrap()
            .split('\n')
            .nth(1)
            .expect("Failed to get commit data")
            .to_string()
    } else {
        std::fs::read_to_string(&s.file).expect("Should have been able to read the file")
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let s = Settings::new(args);
    let data = get_data(&s);
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
        let download = item["revisions"][current_revision]["fetch"][&s.download_with]["commands"]
            [&s.method]
            .as_str()
            .expect(&("Failed to find download link for ".to_string() + &s.download_with));
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
