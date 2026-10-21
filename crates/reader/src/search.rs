use crate::parse::parse_meeting;
use crate::types::{ActionItem, ParsedMeeting};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// List meetings from a directory, sorted by date descending.
pub fn list_meetings(dir: &Path, limit: usize) -> Vec<ParsedMeeting> {
    let mut meetings: Vec<ParsedMeeting> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .filter_map(|e| parse_meeting(e.path()).ok())
        .collect();

    meetings.sort_by_key(|meeting| std::cmp::Reverse(meeting.frontmatter.date));
    meetings.truncate(limit);
    meetings
}

/// Search meetings by a text query in title and body.
pub fn search_meetings(dir: &Path, query: &str, limit: usize) -> Vec<ParsedMeeting> {
    let query_lower = query.to_lowercase();

    let mut results: Vec<ParsedMeeting> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .filter_map(|e| parse_meeting(e.path()).ok())
        .filter(|m| {
            m.frontmatter.title.to_lowercase().contains(&query_lower)
                || m.body.to_lowercase().contains(&query_lower)
        })
        .collect();

    results.sort_by_key(|meeting| std::cmp::Reverse(meeting.frontmatter.date));
    results.truncate(limit);
    results
}

/// Find open action items across all meetings in a directory.
pub fn find_open_actions(dir: &Path, assignee: Option<&str>) -> Vec<(PathBuf, ActionItem)> {
    let mut results = Vec::new();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
    {
        if let Ok(meeting) = parse_meeting(entry.path()) {
            for item in &meeting.frontmatter.action_items {
                if item.status != "open" {
                    continue;
                }
                if let Some(filter) = assignee {
                    if !item.assignee.eq_ignore_ascii_case(filter) {
                        continue;
                    }
                }
                results.push((meeting.path.clone(), item.clone()));
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_meeting(dir: &Path, name: &str, title: &str, body: &str) {
        let path = dir.join(name);
        let content = format!(
            "---\ntitle: {}\ntype: meeting\ndate: 2026-03-17T12:00:00-07:00\nduration: 5m\nstatus: complete\ntags: []\nattendees: []\npeople: []\naction_items: []\ndecisions: []\nintents: []\n---\n\n{}\n",
            title, body
        );
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn list_meetings_returns_sorted() {
        let dir = TempDir::new().unwrap();
        write_meeting(dir.path(), "a.md", "First", "Body A");
        write_meeting(dir.path(), "b.md", "Second", "Body B");

        let meetings = list_meetings(dir.path(), 10);
        assert_eq!(meetings.len(), 2);
    }

    #[test]
    fn search_meetings_filters_by_query() {
        let dir = TempDir::new().unwrap();
        write_meeting(
            dir.path(),
            "a.md",
            "Pricing Discussion",
            "Talked about pricing",
        );
        write_meeting(dir.path(), "b.md", "Onboarding", "New hire onboarding");

        let results = search_meetings(dir.path(), "pricing", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].frontmatter.title, "Pricing Discussion");
    }

    #[test]
    fn find_open_actions_works() {
        let dir = TempDir::new().unwrap();
        let content = "---\ntitle: Test\ntype: meeting\ndate: 2026-03-17T12:00:00-07:00\nduration: 5m\nstatus: complete\ntags: []\nattendees: []\npeople: []\naction_items:\n  - assignee: mat\n    task: Send doc\n    status: open\ndecisions: []\nintents: []\n---\n\nTranscript\n";
        std::fs::write(dir.path().join("test.md"), content).unwrap();

        let actions = find_open_actions(dir.path(), None);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].1.assignee, "mat");

        let filtered = find_open_actions(dir.path(), Some("nobody"));
        assert!(filtered.is_empty());
    }

    fn write_dated(dir: &Path, name: &str, title: &str, date: &str, actions: &str, body: &str) {
        let content = format!(
            "---\ntitle: {title}\ntype: meeting\ndate: {date}\nduration: 5m\n{actions}---\n\n{body}\n"
        );
        std::fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn list_meetings_orders_newest_first_and_truncates() {
        let dir = TempDir::new().unwrap();
        write_dated(
            dir.path(),
            "a.md",
            "Old",
            "2026-01-01T09:00:00-07:00",
            "",
            "x",
        );
        write_dated(
            dir.path(),
            "b.md",
            "Newest",
            "2026-03-01T09:00:00-07:00",
            "",
            "x",
        );
        write_dated(
            dir.path(),
            "c.md",
            "Middle",
            "2026-02-01T09:00:00-07:00",
            "",
            "x",
        );

        let all: Vec<_> = list_meetings(dir.path(), 10)
            .into_iter()
            .map(|m| m.frontmatter.title)
            .collect();
        assert_eq!(all, ["Newest", "Middle", "Old"]);

        let top: Vec<_> = list_meetings(dir.path(), 2)
            .into_iter()
            .map(|m| m.frontmatter.title)
            .collect();
        assert_eq!(top, ["Newest", "Middle"]);
        assert!(list_meetings(dir.path(), 0).is_empty());
    }

    #[test]
    fn list_meetings_skips_non_markdown_unparseable_and_recurses() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("memos/nested")).unwrap();
        write_dated(
            dir.path(),
            "top.md",
            "Top",
            "2026-03-01T09:00:00-07:00",
            "",
            "x",
        );
        write_dated(
            &dir.path().join("memos/nested"),
            "deep.md",
            "Deep",
            "2026-03-02T09:00:00-07:00",
            "",
            "x",
        );
        std::fs::write(dir.path().join("notes.txt"), "---\ntitle: no\n---\n").unwrap();
        std::fs::write(dir.path().join("broken.md"), "no frontmatter").unwrap();

        let titles: Vec<_> = list_meetings(dir.path(), 10)
            .into_iter()
            .map(|m| m.frontmatter.title)
            .collect();
        assert_eq!(titles, ["Deep", "Top"]);
    }

    #[test]
    fn list_meetings_on_missing_dir_is_empty() {
        let dir = TempDir::new().unwrap();
        assert!(list_meetings(&dir.path().join("nope"), 10).is_empty());
    }

    #[test]
    fn search_meetings_is_case_insensitive_over_title_and_body() {
        let dir = TempDir::new().unwrap();
        write_dated(
            dir.path(),
            "a.md",
            "BUDGET Review",
            "2026-03-01T09:00:00-07:00",
            "",
            "nothing",
        );
        write_dated(
            dir.path(),
            "b.md",
            "Standup",
            "2026-03-02T09:00:00-07:00",
            "",
            "we discussed the Budget",
        );
        write_dated(
            dir.path(),
            "c.md",
            "Other",
            "2026-03-03T09:00:00-07:00",
            "",
            "unrelated",
        );

        let titles: Vec<_> = search_meetings(dir.path(), "bUdGeT", 10)
            .into_iter()
            .map(|m| m.frontmatter.title)
            .collect();
        assert_eq!(titles, ["Standup", "BUDGET Review"]);
        assert_eq!(search_meetings(dir.path(), "budget", 1).len(), 1);
        assert!(search_meetings(dir.path(), "absent", 10).is_empty());
    }

    #[test]
    fn search_meetings_does_not_match_frontmatter_only_text() {
        let dir = TempDir::new().unwrap();
        write_dated(
            dir.path(),
            "a.md",
            "Title",
            "2026-03-01T09:00:00-07:00",
            "tags:\n  - secretword\n",
            "body",
        );
        assert!(search_meetings(dir.path(), "secretword", 10).is_empty());
    }

    #[test]
    fn find_open_actions_filters_status_and_assignee_case_insensitively() {
        let dir = TempDir::new().unwrap();
        write_dated(
            dir.path(),
            "a.md",
            "A",
            "2026-03-01T09:00:00-07:00",
            "action_items:\n  - assignee: Mat\n    task: one\n    status: open\n  - assignee: mat\n    task: two\n    status: done\n  - assignee: sam\n    task: three\n    status: open\n  - assignee: mat\n    task: four\n    status: Open\n",
            "x",
        );

        let all = find_open_actions(dir.path(), None);
        let tasks: Vec<_> = all.iter().map(|(_, i)| i.task.as_str()).collect();
        assert_eq!(tasks, ["one", "three"]);

        let mat = find_open_actions(dir.path(), Some("MAT"));
        assert_eq!(mat.len(), 1);
        assert_eq!(mat[0].1.task, "one");
        assert!(mat[0].0.ends_with("a.md"));
    }
}
