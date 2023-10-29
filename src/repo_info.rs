use std::process::{Command, Stdio};

use crate::remote::RemoteUrl;

#[derive(PartialEq, Eq)]
pub enum RepoType {
    Git,
    Repo,
    GitInRepo,
}

pub struct RepoInfo {
    pub remote_url: RemoteUrl,
    pub repo_type: RepoType,
    pub project_name: String,
}

impl RepoInfo {
    pub fn new() -> Self {
        let repo_type = Self::get_repo_type();
        let remote_url = Self::guess_remote(&repo_type);
        let project_name = Self::get_project_name(&remote_url);

        return RepoInfo {
            remote_url: RemoteUrl::new(&remote_url),
            repo_type,
            project_name,
        };
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

    fn guess_remote(repo_type: &RepoType) -> String {
        let manifest_dir = Self::get_repo_manifest_dir();
        let git_dir = if *repo_type == RepoType::Git || manifest_dir.is_empty() {
            "."
        } else {
            &manifest_dir[..]
        };

        let remote = Self::read_git_config("remote.origin.url", git_dir);
        let parts: Vec<&str> = remote.split('/').collect();
        // authenticated URLs end in /a/, but other letters seems to be possible as well.
        if parts.len() > 3 && parts[3].len() == 1 {
            return parts[..4].join("/");
        }
        parts[..3].join("/")
    }

    fn get_project_name(url: &str) -> String {
        let mut project_name = Self::read_git_config("remote.origin.projectname", ".")
            .trim_end_matches(".git")
            .to_string();
        if project_name.is_empty() {
            project_name = Self::read_git_config("remote.origin.url", ".")
                .trim_end_matches(".git")
                .trim_start_matches(url)
                .trim_start_matches('/')
                .to_string();
        }
        return project_name;
    }

    fn read_git_config(config: &str, dir: &str) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("config")
            .arg("--get")
            .arg(config)
            .output()
            .expect("Failed to run 'git'");
        std::str::from_utf8(&out.stdout).unwrap().trim().to_string()
    }

    fn get_repo_type() -> RepoType {
        let is_repo = Command::new("repo")
            .arg("status")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("Failed to run 'repo'")
            .success();
        let is_git = Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("Failed to run 'git'")
            .success();
        return match (is_repo, is_git) {
            (true, true) => RepoType::GitInRepo,
            (true, false) => RepoType::Repo,
            (false, true) => RepoType::Git,
            (false, false) => panic!("Must be run in a repo"),
        };
    }
}
