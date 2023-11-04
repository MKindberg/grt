use crate::{remote::RemoteUrl, repo_info::RepoType, SETTINGS};
use skim::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CommitInfo {
    project: String,
    pub subject: String,
    message: String,
    author: String,
    branch: String,
    reference: String,
    files: Vec<String>,
    pub topic: Option<String>,
}

#[allow(clippy::too_many_arguments)]
impl CommitInfo {
    fn new(
        project: &str,
        subject: &str,
        message: &str,
        author: &str,
        branch: &str,
        reference: &str,
        files: Vec<String>,
        topic: Option<&str>,
    ) -> Self {
        CommitInfo {
            project: project.to_string(),
            subject: subject.to_string(),
            message: message.to_string(),
            author: author.to_string(),
            branch: branch.to_string(),
            reference: reference.to_string(),
            files,
            topic: topic.map(|s| s.to_string()),
        }
    }

    pub fn parse_json<'a>(
        commit_data: &'a json::JsonValue,
    ) -> impl Iterator<Item = CommitInfo> + 'a {
        commit_data
            .members()
            .cloned()
            .map(|data| CommitInfo::from_json(&data))
    }

    pub fn get_title(&self) -> String {
        return if SETTINGS.repo_info.repo_type == RepoType::Git {
            "".to_string()
        } else {
            self.project.clone() + " - "
        } + &self.subject
            + " - "
            + &self.author;
    }

    pub fn get_body(&self) -> String {
        return self.message.clone()
            + "\n---\n\nBranch: "
            + &self.branch
            + "\n\n"
            + &self.files.join("\n");
    }

    pub fn get_git_reference(&self) -> String {
        self.reference.clone()
    }
    pub fn get_repo_reference(&self) -> String {
        self.project.clone()
            + ".git "
            + &self.reference.split('/').collect::<Vec<&str>>()[3..].join("/")
    }
    pub fn get_reference(&self) -> String {
        return if SETTINGS.repo_info.repo_type == RepoType::Git {
            self.get_git_reference()
        } else {
            self.get_repo_reference()
        };
    }

    fn from_ssh_json(data: &json::JsonValue) -> Self {
        let project = data["project"]
            .as_str()
            .expect("Failed to get project name");
        let subject = data["subject"]
            .as_str()
            .expect("Failed to find commit subject");
        let author = data["currentPatchSet"]["author"]["name"]
            .as_str()
            .expect("Failed to find commit author");
        let message = data["commitMessage"]
            .as_str()
            .expect("Failed to find commit message");
        let reference = data["currentPatchSet"]["ref"]
            .as_str()
            .expect("Failed to find ref");
        let branch = data["branch"].as_str().expect("Failed to find branch");

        let mut files: Vec<String> = Vec::new();
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

        let topic = data["topic"].as_str();
        Self::new(
            project, subject, message, author, branch, reference, files, topic,
        )
    }

    fn from_http_json(data: &json::JsonValue) -> Self {
        let current_revision = data["current_revision"].as_str().unwrap_or("");
        let project = data["project"]
            .as_str()
            .expect("Failed to get project name");
        let subject = data["subject"]
            .as_str()
            .expect("Failed to find commit subject");
        let author = data["revisions"][current_revision]["commit"]["author"]["name"]
            .as_str()
            .expect("Failed to find commit author");
        let message = data["revisions"][current_revision]["commit"]["message"]
            .as_str()
            .expect("Failed to find commit message");
        let reference = data["revisions"][current_revision]["ref"]
            .as_str()
            .expect("Failed to find ref");
        let branch = data["branch"].as_str().expect("Failed to find branch");

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

        let topic = data["topic"].as_str();
        Self::new(
            project, subject, message, author, branch, reference, files, topic,
        )
    }
    pub fn from_json(data: &json::JsonValue) -> Self {
        match SETTINGS.repo_info.remote_url {
            RemoteUrl::SSH(_) => Self::from_ssh_json(data),
            RemoteUrl::HTTP(_) => Self::from_http_json(data),
        }
    }
}

impl SkimItem for CommitInfo {
    fn text(&self) -> Cow<str> {
        Cow::Owned(self.get_title())
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(self.get_body())
    }
    fn output(&self) -> Cow<str> {
        Cow::Owned(self.get_reference())
    }
}

impl Selector for CommitInfo {
    fn should_select(&self, _index: usize, _item: &dyn SkimItem) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn test_ssh() {
        let json_data = fs::read_to_string("ssh-commit.json").expect("Failed to open test file");
        let parsed_data = json::parse(&json_data)
            .unwrap()
            .members()
            .cloned()
            .map(|data| CommitInfo::from_json(&data))
            .collect::<Vec<CommitInfo>>();
        assert_eq!(parsed_data.len(), 2);
        assert_eq!(
            parsed_data[0],
            CommitInfo::new(
                "dummy",
                "follow-up commit",
                "follow-up commit\n\nChange-Id: I95eda6180426529e4c959c60a7a575751a00fc20\n",
                "Administrator",
                "main",
                "refs/changes/41/41/1",
                vec![],
                None,
            )
        );
        assert_eq!(
            parsed_data[1],
            CommitInfo::new(
                "dummy",
                "Second commit",
                "Second commit\n\nChange-Id: Ie61179aba5e7ef87541b6dc8ec26fe58403b336e\n",
                "Administrator",
                "main",
                "refs/changes/02/2/2",
                vec!["A README-md +1 -0".to_string()],
                None,
            )
        );
    }
}
