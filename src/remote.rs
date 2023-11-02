use std::process::Command;

use crate::commit_info::{CommitInfo, JsonType};

pub enum RemoteUrl {
    SSH(String),
    HTTP(String),
}

impl RemoteUrl {
    pub fn new(url: &str) -> Self {
        if url.starts_with("ssh://") {
            Self::SSH(url.to_string())
        } else if url.starts_with("http://") || url.starts_with("https://") {
            if !url.ends_with('/') {
                Self::HTTP(url.to_string() + "/")
            } else {
                Self::HTTP(url.to_string())
            }
        } else {
            panic!("Invalid url");
        }
    }

    pub fn full_url(&self, query: &str) -> String {
        match self {
            Self::SSH(url) => {
                let flags = "--format=JSON --current-patch-set --files --commit-message ";
                format!("{} gerrit query {} {}", url, flags, query)
            }
            Self::HTTP(url) => {
                let fields = "o=CURRENT_REVISION&o=CURRENT_COMMIT&o=CURRENT_FILES";
                format!("{}changes/?q={}&{}", url, query.replace(" ", "+"), fields)
            }
        }
    }

    pub fn perform_query(&self, query: &str) -> Vec<CommitInfo> {
        let url = self.full_url(query);
        let (json_type, commit_data) = match self {
            Self::SSH(_) => {
                let out = Command::new("ssh")
                    .args(url.split_whitespace())
                    .output()
                    .expect("Faield to fetch ssh commit data");
                let mut items = std::str::from_utf8(&out.stdout)
                    .unwrap()
                    .lines()
                    .collect::<Vec<&str>>();
                items.pop(); // Last element contains stats
                (JsonType::SSH, format!("[{}]", items.join(",")))
            }
            Self::HTTP(_) => {
                let mut cmd = Command::new("curl");
                let full_cmd = cmd
                    .arg("--netrc")
                    .arg("--request")
                    .arg("GET")
                    .arg("--url")
                    .arg(url)
                    .arg("--header")
                    .arg("Content-Type: application/json");
                let out = full_cmd.output().expect("Failed to fetch http commit data");
                // Need to remove the first line as it contains the magic string )]}' to prevent
                // Cross Site Script Inclusion attacks (https://gerrit.onap.org/r/Documentation/rest-api.html#output)
                (
                    JsonType::HTTP,
                    std::str::from_utf8(&out.stdout)
                        .unwrap()
                        .split('\n')
                        .skip(1)
                        .collect(),
                )
                // reqwest::blocking::get(s.get_url())
                //     .unwrap()
                //     .text()
                //     .unwrap()
                //     .split('\n')
                //     .nth(1)
                //     .expect("Failed to get commit data")
                //     .to_string()
            }
        };
        json::parse(&commit_data)
            .unwrap()
            .members()
            .cloned()
            .map(|data| CommitInfo::from_json(&json_type, &data))
            .collect()
    }
}
