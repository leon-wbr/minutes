use crate::types::{Frontmatter, ParsedMeeting};
use std::path::Path;

/// Split markdown content into frontmatter string and body string.
/// Returns `("", content)` if no frontmatter is found.
pub fn split_frontmatter(content: &str) -> (&str, &str) {
    if !content.starts_with("---") {
        return ("", content);
    }

    if let Some(end) = content[3..].find("\n---") {
        let fm_end = end + 3;
        let body_start = fm_end + 4;
        let body_start = content[body_start..]
            .find('\n')
            .map(|i| body_start + i + 1)
            .unwrap_or(body_start);
        (&content[3..fm_end], &content[body_start..])
    } else {
        ("", content)
    }
}

/// Parse a meeting markdown file into its frontmatter and body.
pub fn parse_meeting(path: &Path) -> Result<ParsedMeeting, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;

    let (fm_str, body) = split_frontmatter(&content);
    if fm_str.is_empty() {
        return Err(format!("no frontmatter found in {}", path.display()));
    }

    let frontmatter: Frontmatter = serde_yaml::from_str(fm_str)
        .map_err(|e| format!("failed to parse frontmatter in {}: {}", path.display(), e))?;

    Ok(ParsedMeeting {
        frontmatter,
        body: body.to_string(),
        path: path.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn split_frontmatter_works() {
        let content = "---\ntitle: Test\ndate: 2026-03-17\n---\n\nBody text here.";
        let (fm, body) = split_frontmatter(content);
        assert!(fm.contains("title: Test"));
        assert!(body.contains("Body text here"));
    }

    #[test]
    fn split_frontmatter_no_frontmatter() {
        let content = "Just plain text.";
        let (fm, body) = split_frontmatter(content);
        assert!(fm.is_empty());
        assert_eq!(body, content);
    }

    #[test]
    fn parse_meeting_reads_valid_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.md");
        std::fs::write(
            &path,
            "---\ntitle: Test Meeting\ntype: meeting\ndate: 2026-03-17T12:00:00-07:00\nduration: 5m\nstatus: complete\ntags: []\nattendees: []\npeople: []\naction_items: []\ndecisions: []\nintents: []\n---\n\n## Transcript\n\nHello world.\n",
        )
        .unwrap();

        let meeting = parse_meeting(&path).unwrap();
        assert_eq!(meeting.frontmatter.title, "Test Meeting");
        assert!(meeting.body.contains("Hello world"));
    }

    #[test]
    fn parse_meeting_preserves_recording_health() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("health.md");
        std::fs::write(
            &path,
            "---\ntitle: Test Meeting\ntype: meeting\ndate: 2026-03-17T12:00:00-07:00\nduration: 5m\nrecording_health:\n  voice_stem_active_ratio: 0.42\n  system_stem_active_ratio: 0.01\n  system_dominant_ratio: 0.2\n  capture_warnings:\n    - kind: source-starved\n      source: system\n      message: System source did not deliver frames.\n      diagnostic_confidence: inferred\n  diarization_path: stem-energy\n---\n\n## Transcript\n\nHello world.\n",
        )
        .unwrap();

        let meeting = parse_meeting(&path).unwrap();
        let health = meeting.frontmatter.recording_health.unwrap();

        assert_eq!(health.voice_stem_active_ratio, Some(0.42));
        assert_eq!(health.system_stem_active_ratio, Some(0.01));
        assert_eq!(health.system_dominant_ratio, Some(0.2));
        assert_eq!(
            health.diarization_path,
            Some(crate::types::DiarizationPath::StemEnergy)
        );
        assert_eq!(
            health.capture_warnings[0].kind,
            crate::types::FailureKind::SourceStarved
        );
        assert_eq!(
            health.capture_warnings[0].source,
            crate::types::CaptureSource::System
        );
        assert_eq!(
            health.capture_warnings[0].diagnostic_confidence,
            crate::types::DiagnosticConfidence::Inferred
        );
    }

    #[test]
    fn parse_meeting_preserves_sensitive_fields() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sensitive.md");
        std::fs::write(
            &path,
            "---\ntitle: Board Sync\ntype: meeting\ndate: 2026-06-10T12:00:00-07:00\nduration: 12m\ncapture: none\nsensitivity: restricted\ndebrief: pending\n---\n\n## Notes\n\n- [0:01] Marker.\n",
        )
        .unwrap();

        let meeting = parse_meeting(&path).unwrap();
        assert_eq!(
            meeting.frontmatter.capture,
            Some(crate::types::CapturePolicy::None)
        );
        assert_eq!(
            meeting.frontmatter.sensitivity,
            Some(crate::types::Sensitivity::Restricted)
        );
        assert_eq!(
            meeting.frontmatter.debrief,
            Some(crate::types::DebriefStatus::Pending)
        );
    }

    #[test]
    fn attribution_source_parses_new_variants() {
        assert_eq!(
            serde_yaml::from_str::<crate::types::AttributionSource>("ml-bleed-degraded").unwrap(),
            crate::types::AttributionSource::MlBleedDegraded
        );
        assert_eq!(
            serde_yaml::from_str::<crate::types::AttributionSource>("stem-recovery").unwrap(),
            crate::types::AttributionSource::StemRecovery
        );
    }

    const MINIMAL_FM: &str =
        "title: T\ntype: memo\ndate: 2026-03-17T12:00:00-07:00\nduration: 1m\n";

    #[test]
    fn split_frontmatter_returns_exact_slices() {
        let (fm, body) = split_frontmatter("---\na: 1\n---\nbody\nmore");
        assert_eq!(fm, "\na: 1");
        assert_eq!(body, "body\nmore");
    }

    #[test]
    fn split_frontmatter_unterminated_is_treated_as_plain_text() {
        let content = "---\ntitle: Never closed\nbody";
        assert_eq!(split_frontmatter(content), ("", content));
        assert_eq!(split_frontmatter("---"), ("", "---"));
        assert_eq!(split_frontmatter(""), ("", ""));
    }

    #[test]
    fn split_frontmatter_requires_marker_at_start() {
        let content = "\n---\ntitle: x\n---\nbody";
        assert_eq!(split_frontmatter(content), ("", content));
    }

    #[test]
    fn split_frontmatter_closing_marker_at_eof_leaves_empty_body() {
        let (fm, body) = split_frontmatter("---\na: 1\n---");
        assert_eq!(fm, "\na: 1");
        assert_eq!(body, "");
    }

    #[test]
    fn split_frontmatter_handles_crlf_line_endings() {
        let (fm, body) = split_frontmatter("---\r\na: 1\r\n---\r\nbody");
        assert!(fm.contains("a: 1"));
        assert_eq!(body, "body");
    }

    #[test]
    fn split_frontmatter_keeps_later_rules_in_body() {
        let (fm, body) = split_frontmatter("---\na: 1\n---\nabove\n---\nbelow");
        assert_eq!(fm, "\na: 1");
        assert_eq!(body, "above\n---\nbelow");
    }

    #[test]
    fn parse_meeting_errors_name_the_path_and_cause() {
        let dir = TempDir::new().unwrap();

        let missing = dir.path().join("missing.md");
        let err = parse_meeting(&missing).unwrap_err();
        assert!(err.starts_with("failed to read "), "{err}");
        assert!(err.contains("missing.md"), "{err}");

        let plain = dir.path().join("plain.md");
        std::fs::write(&plain, "no frontmatter here").unwrap();
        let err = parse_meeting(&plain).unwrap_err();
        assert!(err.starts_with("no frontmatter found in "), "{err}");

        let bad = dir.path().join("bad.md");
        std::fs::write(&bad, "---\ntitle: only a title\n---\nbody").unwrap();
        let err = parse_meeting(&bad).unwrap_err();
        assert!(err.starts_with("failed to parse frontmatter in "), "{err}");
    }

    #[test]
    fn parse_meeting_applies_defaults_for_optional_fields() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("min.md");
        std::fs::write(&path, format!("---\n{MINIMAL_FM}---\n\nbody\n")).unwrap();

        let fm = parse_meeting(&path).unwrap().frontmatter;
        assert_eq!(fm.r#type, crate::types::ContentType::Memo);
        assert!(fm.status.is_none() && fm.source.is_none() && fm.visibility.is_none());
        assert!(fm.tags.is_empty() && fm.attendees.is_empty() && fm.people.is_empty());
        assert!(fm.action_items.is_empty() && fm.decisions.is_empty() && fm.intents.is_empty());
        assert!(fm.speaker_map.is_empty() && fm.entities.is_empty());
        assert!(fm.capture.is_none() && fm.sensitivity.is_none() && fm.debrief.is_none());
    }

    #[test]
    fn parse_meeting_rejects_unknown_enum_values() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("badtype.md");
        std::fs::write(
            &path,
            "---\ntitle: T\ntype: webinar\ndate: 2026-03-17T12:00:00-07:00\nduration: 1m\n---\n",
        )
        .unwrap();
        assert!(parse_meeting(&path)
            .unwrap_err()
            .starts_with("failed to parse frontmatter in "));
    }

    #[test]
    fn parse_meeting_reads_speaker_map_with_unknown_provenance() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("speakers.md");
        std::fs::write(
            &path,
            format!(
                "---\n{MINIMAL_FM}speaker_map:\n  - speaker_label: SPEAKER_0\n    name: Ada\n    confidence: high\n    source: enrollment\n  - speaker_label: SPEAKER_1\n    name: Bo\n    confidence: low\n    source: hand-edited\n---\n"
            ),
        )
        .unwrap();

        let map = parse_meeting(&path).unwrap().frontmatter.speaker_map;
        assert_eq!(map.len(), 2);
        assert_eq!(map[0].source, crate::types::AttributionSource::Enrollment);
        assert_eq!(map[0].confidence, crate::types::Confidence::High);
        assert_eq!(
            map[1].source,
            crate::types::AttributionSource::Unknown("hand-edited".into())
        );
    }
}
