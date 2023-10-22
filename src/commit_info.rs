use crate::settings::Settings;
use skim::prelude::*;

#[derive(Debug, PartialEq, Eq)]
pub struct CommitInfo {
    title: String,
    body: String,
    reference: String,
}

#[derive(Debug)]
pub enum JsonType {
    SSH,
    HTTP,
}

#[allow(clippy::too_many_arguments)]
impl CommitInfo {
    fn new(
        is_git: bool,
        project: &str,
        title: &str,
        author: &str,
        body: &str,
        reference: &str,
        files: Vec<String>,
        branch: &str,
    ) -> Self {
        CommitInfo {
            title: if is_git {
                "".to_string()
            } else {
                project.to_string() + " - "
            } + title
                + " - "
                + author,
            body: body.to_string() + "\n---\n\nBranch: " + branch + "\n\n" + &files.join("\n"),
            reference: if is_git {
                reference.to_string()
            } else {
                project.to_string()
                    + ".git "
                    + &reference.split('/').collect::<Vec<&str>>()[3..].join("/")
            },
        }
    }

    fn from_ssh_json(data: &json::JsonValue) -> Self {
        let project = data["project"]
            .as_str()
            .expect("Failed to get project name");
        let title = data["subject"]
            .as_str()
            .expect("Failed to find commit subject");
        let author = data["currentPatchSet"]["author"]["name"]
            .as_str()
            .expect("Failed to find commit author");
        let body = data["commitMessage"]
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
        Self::new(
            Settings::is_git(),
            project,
            title,
            author,
            body,
            reference,
            files,
            branch,
        )
    }

    fn from_http_json(data: &json::JsonValue) -> Self {
        let current_revision = data["current_revision"].as_str().unwrap_or("");
        let project = data["project"]
            .as_str()
            .expect("Failed to get project name");
        let title = data["subject"]
            .as_str()
            .expect("Failed to find commit subject");
        let author = data["revisions"][current_revision]["commit"]["author"]["name"]
            .as_str()
            .expect("Failed to find commit author");
        let body = data["revisions"][current_revision]["commit"]["message"]
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
        Self::new(
            Settings::is_git(),
            project,
            title,
            author,
            body,
            reference,
            files,
            branch,
        )
    }
    pub fn from_json(json_type: &JsonType, data: &json::JsonValue) -> Self {
        match json_type {
            JsonType::SSH => Self::from_ssh_json(data),
            JsonType::HTTP => Self::from_http_json(data),
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
        Cow::Borrowed(&self.reference)
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
            .map(|data| CommitInfo::from_json(&JsonType::SSH, &data))
            .collect::<Vec<CommitInfo>>();
        assert_eq!(parsed_data.len(), 2);
        assert_eq!(
            parsed_data[0],
            CommitInfo::new(
                true,
                "dummy",
                "follow-up commit",
                "Administrator",
                "follow-up commit\n\nChange-Id: I95eda6180426529e4c959c60a7a575751a00fc20\n",
                "refs/changes/41/41/1",
                vec![],
                "main"
            )
        );
        assert_eq!(
            parsed_data[1],
            CommitInfo::new(
                true,
                "dummy",
                "Second commit",
                "Administrator",
                "Second commit\n\nChange-Id: Ie61179aba5e7ef87541b6dc8ec26fe58403b336e\n",
                "refs/changes/02/2/2",
                vec!["A README-md +1 -0".to_string()],
                "main"
            )
        );
    }
}
