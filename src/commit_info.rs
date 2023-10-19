use crate::settings::Settings;
use skim::prelude::*;

#[derive(Debug)]
pub struct CommitInfo {
    title: String,
    body: String,
    reference: String,
}

#[allow(clippy::too_many_arguments)]
impl CommitInfo {
    fn new(
        is_git: bool,
        project: String,
        title: String,
        author: String,
        body: String,
        reference: String,
        files: Vec<String>,
        branch: String,
    ) -> Self {
        CommitInfo {
            title: if is_git {
                "".to_string()
            } else {
                project.to_string() + " - "
            } + &title
                + " - "
                + &author,
            body: body + "\n---\n\nBranch: " + &branch + "\n\n" + &files.join("\n"),
            reference: if is_git {
                reference
            } else {
                project + ".git " + &reference.split('/').collect::<Vec<&str>>()[3..].join("/")
            },
        }
    }
}

impl From<json::JsonValue> for CommitInfo {
    fn from(data: json::JsonValue) -> Self {
        let current_revision = data["current_revision"].as_str().unwrap_or("");
        let project = data["project"]
            .as_str()
            .expect("Failed to get project name");
        let title = data["subject"]
            .as_str()
            .expect("Failed to find commit subject");
        let author = data["revisions"][current_revision]["commit"]["author"]["name"]
            .as_str()
            .unwrap_or_else(|| {
                data["currentPatchSet"]["author"]["name"]
                    .as_str()
                    .expect("Failed to find commit author")
            });
        let body = data["revisions"][current_revision]["commit"]["message"]
            .as_str()
            .unwrap_or_else(|| {
                data["commitMessage"]
                    .as_str()
                    .expect("Failed to find commit message")
            });
        let reference = data["revisions"][current_revision]["ref"]
            .as_str()
            .unwrap_or_else(|| {
                data["currentPatchSet"]["ref"]
                    .as_str()
                    .expect("Failed to find ref")
            });
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
        let branch = data["branch"].as_str().expect("Failed to find branch");
        Self::new(
            Settings::is_git(),
            project.to_string(),
            title.to_string(),
            author.to_string(),
            body.to_string(),
            reference.to_string(),
            files,
            branch.to_string(),
        )
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


