use getopts::Options;
use std::env;

use crate::REPO_INFO;

pub struct Settings {
    pub method: String,
    pub select_all: bool,
    pub query: String,
    pub debug: bool,
    pub show_parent: bool,
    only_open: bool,
    options: getopts::Options,
}

impl Settings {
    pub fn new() -> Self {
        let mut opts = Options::new();
        opts.optflag("h", "help", "Print this menu");
        opts.optflag("c", "closed", "Include closed commits");
        opts.optflag(
            "o",
            "open",
            "Don't include closed commits (default, will override -c if set)",
        );
        opts.optflag("a", "all", "pre-select all commits");
        opts.optflag("p",
            "show-parent",
            "Show info about parent commit in preview even in ssh (This will lead to additional queries)");
        opts.optflag("", "debug", "Print debug information while running");

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
            query: "limit:200 ".to_string(),
            select_all: false,
            debug: false,
            only_open: true,
            show_parent: false,
            options: opts,
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

        s
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
        if matches.opt_present("show-parent") {
            self.show_parent = true;
        }
        if matches.opt_present("debug") {
            self.debug = true;
        }
    }

    fn create_query(&mut self, query: &str) {
        if self.only_open {
            self.query += "status:open ";
        }
        if !REPO_INFO.project_name.is_empty() {
            self.query += format!("project:{} ", REPO_INFO.project_name).as_str();
        }
        self.query += query;
    }
}
