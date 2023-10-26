use getopts::Options;
use std::env;
use std::process::{Command, Stdio};

use crate::repo_info::RepoInfo;

pub struct Settings {
    pub method: String,
    pub file: String,
    pub select_all: bool,
    query: String,
    debug: bool,
    only_open: bool,
    http_query_fields: String,
    ssh_query_flags: String,
    options: getopts::Options,
    repo_info: RepoInfo,
}

impl Settings {
    pub fn new() -> Self {
        let mut opts = Options::new();
        opts.optflag("h", "help", "Print this menu");
        opts.optopt("u", "url", "Override the auto-detected url", "URL");
        opts.optflag("c", "closed", "Include closed commits");
        opts.optflag(
            "o",
            "open",
            "Don't include closed commits (default, will override -c if set)",
        );
        opts.optflag("a", "all", "pre-select all commits");
        opts.optflag("", "debug", "Print debug information while running");
        opts.optopt(
            "f",
            "file",
            "Read json data from file instead of Gerrit",
            "FILE",
        );

        let matches_env = opts
            .parse(
                env::var("GRT_ARGS")
                    .unwrap_or_else(|_| "".to_string())
                    .split(' '),
            )
            .expect("Failed to parse env args");
        let matches_cmd = opts
            .parse(&env::args().collect::<Vec<String>>()[1..])
            .expect("Failed to parse cmd args");

        let mut s = Self {
            method: "".to_string(),
            file: "".to_string(),
            query: "limit:200 ".to_string(),
            select_all: false,
            debug: false,
            only_open: true,
            http_query_fields: "o=CURRENT_REVISION&o=CURRENT_COMMIT&o=CURRENT_FILES".to_string(),
            ssh_query_flags: "--format=JSON --current-patch-set --files --commit-message
".to_string(),
            options: opts,
            repo_info: RepoInfo::new(),
        };

        s.parse_args(&matches_env);
        s.parse_args(&matches_cmd);

        if matches_cmd.free.is_empty() {
            println!("Must add a command, valid options are 'checkout', 'co', 'cherry-pick', 'cp'");
            println!();
            s.print_usage();
        }
        s.method = match matches_cmd.free[0].as_str() {
            "checkout" | "co" => "Checkout".to_string(),
            "cherry-pick" | "cp" => "Cherry-Pick".to_string(),
            op => {
                println!("Unsupported operation '{}'", op);
                println!();
                s.print_usage();
            }
        };
        s.create_query(&matches_cmd.free[1..].join(" "));
        if s.debug {
            println!(
                "Env args: '{}'",
                env::var("GRT_ARGS").unwrap_or_else(|_| "".to_string())
            );
            println!(
                "Cmd args: '{}'",
                env::args().collect::<Vec<String>>().join(" ")
            );
            println!("Query: '{}'", s.query);
        }
        if s.repo_info.remote_url.is_empty() && s.file.is_empty() {
            println!("Couldn't guess Gerrit url, must provide a url through either $GERRIT_URL or the -u option");
            std::process::exit(1);
        }

        s
    }

    pub fn is_git() -> bool {
        Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("Failed to run")
            .success()
    }

    fn print_usage(&self) -> ! {
        let brief = format!(
            "Usage: {} [options] checkout|co|cherry-pick|cp [query]",
            env::args().next().unwrap()
        );
        print!("{}", self.options.usage(&brief));
        println!("\nThe options can be set either on command line or through");
        println!("the env var GRT_ARGS, anything set on command line will");
        println!("override what's set in the environment.");
        std::process::exit(1);
    }

    pub fn parse_args(&mut self, matches: &getopts::Matches) {
        if matches.opt_present("help") {
            self.print_usage();
        }

        if !self.only_open && matches.opt_present("open") {
            self.only_open = true;
        } else if matches.opt_present("closed") {
            self.only_open = false;
        }

        if matches.opt_present("all") {
            self.select_all = true;
        }
        if matches.opt_present("debug") {
            self.debug = true;
        }
        if matches.opt_present("file") {
            self.file = matches.opt_str("file").unwrap();
        }
    }

    fn create_query(&mut self, query: &str) {
        if self.only_open {
            self.query += "status:open ";
        }
        if !self.repo_info.project_name.is_empty() {
            self.query += format!("project:{} ", self.repo_info.project_name).as_str();
        }
        self.query += query;
    }

    pub fn get_url(&self) -> String {
        let mut url: String = self.repo_info.remote_url.to_string();

        if url.starts_with("http") {
            if !url.ends_with('/') {
                url += "/";
            }
            format!(
                "{}changes/?q={}&{}",
                url,
                &self.query.replace(' ', "+"),
                &self.http_query_fields
            )
        } else if url.starts_with("ssh") {
            format!(
                "{} gerrit query {} {}",
                url, &self.ssh_query_flags, &self.query
            )
        } else {
            "".to_string()
        }
    }
}
