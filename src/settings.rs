use getopts::Options;
use std::env;

pub struct Settings {
    pub method: String,
    pub file: String,
    pub fetch_method: String,
    base_url: String,
    query: String,
    debug: bool,
    query_fields: String,
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
    pub fn parse_args() -> Self {
        let args: Vec<String> = env::args().collect();
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
        opts.optflag("c", "closed", "Include closed commits");
        opts.optflag("o", "open", "Don't include closed commits");
        opts.optflag("", "ssh", "Download over ssh");
        opts.optflag("", "https", "Download over https");
        opts.optflag("", "anon", "Download over anonymous https");

        let matches_cmd = opts.parse(&args[1..]).expect("Failed to parse args");
        let matches_env = opts
            .parse(
                env::var("GRT_ARGS")
                    .unwrap_or_else(|_| "".to_string())
                    .split(' '),
            )
            .expect("Failed to parse args");

        if matches_cmd.opt_present("help") {
            print_usage(&args[0], opts);
        }

        let method = match matches_cmd.free[0].as_str() {
            "checkout" | "co" => "Checkout".to_string(),
            "cherry-pick" | "cp" => "Cherry pick".to_string(),
            op => {
                println!("Unsupported operation '{}'", op);
                print_usage(&args[0], opts);
            }
        };

        let debug = matches_cmd.opt_present("debug");

        let file = matches_cmd
            .opt_str("file")
            .unwrap_or_else(|| "".to_string());

        let fetch_method = Self::get_fetch_method(&matches_cmd, &matches_env);

        let query =
            Self::create_query(&matches_cmd, &matches_env, &matches_cmd.free[1..].join(" "));
        if debug {
            println!("Query: '{}'", query);
        }
        let base_url = matches_cmd.opt_str("url").unwrap_or_else(|| {
            matches_env.opt_str("url").unwrap_or_else(|| {
                env::var("GERRIT_URL")
                    .expect("Must set environment var GERRIT_URL or pass --url flag")
            })
        });

        Self {
            method,
            base_url,
            debug,
            file,
            query,
            query_fields: "o=CURRENT_REVISION&o=CURRENT_COMMIT&o=CURRENT_FILES&o=DOWNLOAD_COMMANDS"
                .to_string(),
            fetch_method,
        }
    }

    fn get_fetch_method(
        matching_cmd: &getopts::Matches,
        matching_env: &getopts::Matches,
    ) -> String {
        if matching_cmd.opt_present("ssh") {
            "ssh".to_string()
        } else if matching_cmd.opt_present("https") {
            "https".to_string()
        } else if matching_cmd.opt_present("anon") {
            "anonymous http".to_string()
        } else if matching_env.opt_present("ssh") {
            "ssh".to_string()
        } else if matching_env.opt_present("https") {
            "https".to_string()
        } else if matching_env.opt_present("anon") {
            "anonymous http".to_string()
        } else {
            "ssh".to_string()
        }
    }

    fn create_query(
        matches_cmd: &getopts::Matches,
        matches_env: &getopts::Matches,
        query: &str,
    ) -> String {
        let mut q = "".to_string();
        if matches_cmd.opt_present("open")
            || (!matches_cmd.opt_present("closed")
                && (matches_cmd.opt_present("open") || !matches_cmd.opt_present("closed")))
        {
            q += "status:open ";
        }
        if let Some(p) = matches_cmd.opt_str("project") {
            q += format!("project:{} ", p).as_str();
        } else if let Some(p) = matches_env.opt_str("project") {
            q += format!("project:{} ", p).as_str();
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
            println!("Url: {}", url);
        }
        url
    }
}
