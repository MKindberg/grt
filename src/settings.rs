    use getopts::Options;
    use std::env;

    pub struct Settings {
        pub method: String,
        base_url: String,
        query: String,
        pub file: String,
        debug: bool,
        query_fields: String,
        pub download_with: String,
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
        pub fn new(args: Vec<String>) -> Self {
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
                env::var("GERRIT_URL")
                    .expect("Must set environment var GERRIT_URL or pass --url flag")
            });

            Self {
                method,
                base_url,
                debug,
                file,
                query,
                query_fields:
                    "o=CURRENT_REVISION&o=CURRENT_COMMIT&o=CURRENT_FILES&o=DOWNLOAD_COMMANDS"
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
