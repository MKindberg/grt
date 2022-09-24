use getopts::Options;
use std::env;
use std::process::{Command, Stdio};

pub struct Settings {
    pub method: String,
    pub file: String,
    base_url: String,
    project: String,
    query: String,
    debug: bool,
    only_open: bool,
    http_query_fields: String,
    ssh_query_flags: String,
    options: getopts::Options,
}

impl Settings {
    pub fn new() -> Self {
        let mut opts = Options::new();
        opts.optflag("h", "help", "Print this menu");
        opts.optopt("u", "url", "Override the auto-detected url", "URL");
        opts.optopt(
            "p",
            "project",
            "Override the auto-detected project name",
            "NAME",
        );
        opts.optflag("c", "closed", "Include closed commits");
        opts.optflag(
            "o",
            "open",
            "Don't include closed commits (default, will override -c if set)",
        );
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
            base_url: Self::guess_remote(),
            project: "".to_string(),
            query: "".to_string(),
            debug: false,
            only_open: true,
            http_query_fields: "o=CURRENT_REVISION&o=CURRENT_COMMIT&o=CURRENT_FILES".to_string(),
            ssh_query_flags: "--format=JSON --current-patch-set --files".to_string(),
            options: opts,
        };

        s.parse_args(&matches_env);
        s.project = Self::get_project(&s.base_url);
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
        if s.base_url.is_empty() && s.file.is_empty() {
            println!("Couldn't guess Gerrit url, must provide a url through either $GERRIT_URL or the -u option");
            std::process::exit(1);
        }

        s
    }

    fn get_git_config(config: &str, dir: &str) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("config")
            .arg("--get")
            .arg(config)
            .output()
            .expect("Failed to run");
        std::str::from_utf8(&out.stdout).unwrap().trim().to_string()
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
    fn get_repo_manifest_dir() -> String {
        let out = Command::new("repo")
            .arg("list")
            .arg("manifest.git")
            .arg("--relative-to=.")
            .output()
            .expect("Failed to run");
        std::str::from_utf8(&out.stdout)
            .unwrap()
            .trim()
            .to_string()
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    }

    fn guess_remote() -> String {
        let manifest_dir = Self::get_repo_manifest_dir();
        let git_dir = if Self::is_git() || manifest_dir.is_empty() {
            "."
        } else {
            &manifest_dir[..]
        };

        let remote = Self::get_git_config("remote.origin.url", git_dir);
        // Only support http for now
        if !(remote.starts_with("http") || remote.starts_with("ssh")) {
            return "".to_string();
        }
        let parts: Vec<&str> = remote.split('/').collect();
        // authenticated URLs end in /a/, but other letters seems to be possible as well.
        if parts.len() > 3 && parts[3].len() == 1 {
            return parts[..4].join("/");
        }
        parts[..3].join("/")
    }

    fn get_project(url: &str) -> String {
        let mut project = Self::get_git_config("remote.origin.projectname", ".")
            .trim_end_matches(".git")
            .to_string();
        if project.is_empty() {
            project = Self::get_git_config("remote.origin.url", ".")
                .trim_end_matches(".git")
                .trim_start_matches(url)
                .trim_start_matches('/')
                .to_string();
        }
        project
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

        if matches.opt_present("debug") {
            self.debug = true;
        }
        if matches.opt_present("file") {
            self.file = matches.opt_str("file").unwrap();
        }

        if let Some(p) = matches.opt_str("project") {
            self.project = p;
        }

        if matches.opt_present("url") {
            self.base_url = matches.opt_str("url").unwrap();
        }
    }

    fn create_query(&mut self, query: &str) {
        if self.only_open {
            self.query += "status:open ";
        }
        if !self.project.is_empty() {
            self.query += format!("project:{} ", self.project).as_str();
        }
        self.query += query;
    }

    pub fn get_url(&self) -> String {
        let mut url: String = self.base_url.to_string();

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
