use getopts::Options;
use std::env;

pub struct Settings {
    pub method: String,
    pub file: String,
    pub fetch_method: String,
    base_url: String,
    project: String,
    query: String,
    debug: bool,
    only_open: bool,
    query_fields: String,
    options: getopts::Options,
}

impl Settings {
    pub fn new() -> Self {
        let mut opts = Options::new();
        opts.optflag("h", "help", "Print this menu");
        opts.optopt("u", "url", "The url to Gerrit", "URL");
        opts.optopt("p", "project", "The project to search in", "NAME");
        opts.optflag("c", "closed", "Include closed commits");
        opts.optflag("o", "open", "Don't include closed commits");
        opts.optflag("", "ssh", "Download over ssh");
        opts.optflag("", "https", "Download over https");
        opts.optflag("", "anon", "Download over anonymous https");
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
            fetch_method: "ssh".to_string(),
            base_url: env::var("GERRIT_URL").unwrap_or_default(),
            project: "".to_string(),
            query: "".to_string(),
            debug: false,
            only_open: true,
            query_fields: "o=CURRENT_REVISION&o=CURRENT_COMMIT&o=CURRENT_FILES&o=DOWNLOAD_COMMANDS"
                .to_string(),
            options: opts,
        };
        s.parse_args(&matches_env);
        s.parse_args(&matches_cmd);
        s.method = match matches_cmd.free[0].as_str() {
            "checkout" | "co" => "Checkout".to_string(),
            "cherry-pick" | "cp" => "Cherry pick".to_string(),
            op => {
                println!("Unsupported operation '{}'", op);
                s.print_usage();
            }
        };
        let query = s.create_query(&matches_cmd.free[1..].join(" "));
        if s.debug {
            println!(
                "Env args: '{}'",
                env::var("GRT_ARGS").unwrap_or_else(|_| "".to_string())
            );
            println!(
                "Cmd args: '{}'",
                env::args().collect::<Vec<String>>().join(" ")
            );
            println!("Query: '{}'", query);
        }
        if s.base_url.is_empty() && s.file.is_empty() {
            println!("Must provide a url through either $GERRIT_URL or the -u option");
            std::process::exit(1);
        }

        s
    }

    fn print_usage(&self) -> ! {
        let brief = format!(
            "Usage: {} [options] checkout|co|cherry-pick|cp [query]",
            env::args().next().unwrap()
        );
        print!("{}", self.options.usage(&brief));
        std::process::exit(1);
    }

    pub fn parse_args(&mut self, matches: &getopts::Matches) {
        if matches.opt_present("help") {
            self.print_usage();
        }

        if matches.opts_present(&["ssh".to_string(), "hppts".to_string(), "anon".to_string()]) {
            self.fetch_method = if matches.opt_present("ssh") {
                "ssh".to_string()
            } else if matches.opt_present("https") {
                "https".to_string()
            } else {
                "anonymous http".to_string()
            };
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

    fn create_query(&self, query: &str) -> String {
        let mut q = "".to_string();
        if self.only_open {
            q += "status:open ";
        }
        if !self.project.is_empty() {
            q += format!("project:{} ", self.project).as_str();
        }
        q += query;
        q
    }

    pub fn get_url(&self) -> String {
        let mut url: String = self.base_url.to_string();

        if !url.ends_with('/') {
            url += "/";
        }
        url += "changes/?q=";
        url += &self.query.replace(' ', "+");
        url += "&";
        url += &self.query_fields;
        if self.debug {
            println!("Url: '{}'", url);
        }
        url
    }
}
