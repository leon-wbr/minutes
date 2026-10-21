use chrono::{DateTime, Local};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ──────────────────────────────────────────────────────────────
// Standalone types for parsing Minutes meeting files.
// Duplicated from minutes-core to avoid pulling in audio deps.
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Meeting,
    Memo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OutputStatus {
    Complete,
    NoSpeech,
    TranscriptOnly,
    /// Transcription ran but one or more post-transcript steps fell back
    /// to empty output. Details in `processing_warnings`. See issue #243.
    Degraded,
}

/// How a meeting artifact was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CapturePolicy {
    /// No audio was captured for this meeting artifact.
    None,
}

/// Sensitivity designation for read-only consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Sensitivity {
    /// Standard meeting artifact.
    Normal,
    /// Restricted artifact; agent enforcement is layered by higher-level callers.
    Restricted,
}

/// Human debrief state for no-capture meeting artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DebriefStatus {
    /// The meeting was stopped without an interactive debrief.
    Pending,
    /// A human supplied the debrief; the artifact is the finished record.
    Complete,
    /// The human marked the debrief unnecessary (test run, accidental
    /// trigger).
    NotApplicable,
}

/// A non-fatal failure of a post-transcript pipeline step. Mirrors the
/// core crate's `markdown::ProcessingWarning`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProcessingWarning {
    pub step: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Frontmatter {
    pub title: String,
    pub r#type: ContentType,
    pub date: DateTime<Local>,
    pub duration: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<OutputStatus>,
    /// Per-step failure context when `status: degraded` applies (#243).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub processing_warnings: Vec<ProcessingWarning>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attendees: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attendees_raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_event: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub people: Vec<String>,
    #[serde(default, skip_serializing_if = "EntityLinks::is_empty")]
    pub entities: EntityLinks,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_items: Vec<ActionItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<Decision>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<Intent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_by: Option<String>,
    /// Capture mode for the artifact. Absent means a normal captured meeting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<CapturePolicy>,
    /// Sensitivity designation. Absent means the normal sensitivity policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitivity: Option<Sensitivity>,
    /// Debrief completion state for no-capture meetings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debrief: Option<DebriefStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speaker_map: Vec<SpeakerAttribution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording_health: Option<RecordingHealth>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RecordingHealth {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_stem_active_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_stem_active_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_dominant_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capture_warnings: Vec<CaptureWarning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diarization_path: Option<DiarizationPath>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DiarizationPath {
    StemEnergy,
    Ml,
    MlBleedDegraded,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CaptureWarning {
    pub kind: FailureKind,
    pub source: CaptureSource,
    pub message: String,
    pub diagnostic_confidence: DiagnosticConfidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FailureKind {
    Silent,
    Sparse,
    Missing,
    BackendUnavailable,
    StreamError,
    SourceStarved,
    UnsupportedFormat,
    MisconfiguredRoute,
    PermissionDenied,
    RouteUnavailable,
    Other { code: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CaptureSource {
    Voice,
    System,
    Both,
    Backend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticConfidence {
    High,
    Inferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AttributionSource {
    Deterministic,
    Llm,
    Enrollment,
    Manual,
    #[serde(rename = "ml-bleed-degraded")]
    MlBleedDegraded,
    #[serde(rename = "stem-recovery")]
    StemRecovery,
    /// A provenance label this build does not know, preserved verbatim.
    ///
    /// Hand-edited files and `minutes import text` archives are supported
    /// inputs, so an unrecognized label must not make a meeting unreadable
    /// (#595). The raw string round-trips untouched rather than being
    /// normalized away, so provenance survives a rewrite by a command that
    /// never consumed it.
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpeakerAttribution {
    pub speaker_label: String,
    pub name: String,
    pub confidence: Confidence,
    pub source: AttributionSource,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct EntityLinks {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub people: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<EntityRef>,
}

impl EntityLinks {
    pub fn is_empty(&self) -> bool {
        self.people.is_empty() && self.projects.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntityRef {
    pub slug: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActionItem {
    pub assignee: String,
    pub task: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Decision {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum IntentKind {
    ActionItem,
    Decision,
    OpenQuestion,
    Commitment,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Intent {
    pub kind: IntentKind,
    pub what: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub who: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_date: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Private,
    Team,
}

#[derive(Debug, Clone)]
pub struct ParsedMeeting {
    pub frontmatter: Frontmatter,
    pub body: String,
    pub path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml<T: serde::de::DeserializeOwned>(s: &str) -> T {
        serde_yaml::from_str(s).unwrap()
    }

    #[test]
    fn attribution_source_known_labels_round_trip() {
        for (label, expected) in [
            ("deterministic", AttributionSource::Deterministic),
            ("llm", AttributionSource::Llm),
            ("enrollment", AttributionSource::Enrollment),
            ("manual", AttributionSource::Manual),
            ("ml-bleed-degraded", AttributionSource::MlBleedDegraded),
            ("stem-recovery", AttributionSource::StemRecovery),
        ] {
            assert_eq!(yaml::<AttributionSource>(label), expected, "{label}");
            assert_eq!(
                serde_yaml::to_string(&expected).unwrap().trim(),
                label,
                "{label}"
            );
        }
    }

    #[test]
    fn attribution_source_preserves_unknown_label_verbatim() {
        let parsed: AttributionSource = yaml("Some-New_Label");
        assert_eq!(parsed, AttributionSource::Unknown("Some-New_Label".into()));
        assert_eq!(
            serde_yaml::to_string(&parsed).unwrap().trim(),
            "Some-New_Label"
        );
    }

    #[test]
    fn attribution_source_label_matching_is_case_sensitive() {
        assert_eq!(
            yaml::<AttributionSource>("LLM"),
            AttributionSource::Unknown("LLM".into())
        );
    }

    #[test]
    fn enums_use_documented_spellings() {
        assert_eq!(
            yaml::<OutputStatus>("transcript-only"),
            OutputStatus::TranscriptOnly
        );
        assert_eq!(yaml::<OutputStatus>("no-speech"), OutputStatus::NoSpeech);
        assert_eq!(
            yaml::<DebriefStatus>("not-applicable"),
            DebriefStatus::NotApplicable
        );
        assert_eq!(
            yaml::<DiarizationPath>("ml-bleed-degraded"),
            DiarizationPath::MlBleedDegraded
        );
        assert_eq!(
            yaml::<IntentKind>("open-question"),
            IntentKind::OpenQuestion
        );
        assert_eq!(yaml::<IntentKind>("action-item"), IntentKind::ActionItem);
        assert_eq!(yaml::<Visibility>("team"), Visibility::Team);
        assert_eq!(yaml::<CapturePolicy>("none"), CapturePolicy::None);
    }

    #[test]
    fn enums_reject_unknown_values_unlike_attribution_source() {
        assert!(serde_yaml::from_str::<OutputStatus>("half-done").is_err());
        assert!(serde_yaml::from_str::<Confidence>("certain").is_err());
        assert!(serde_yaml::from_str::<Sensitivity>("secret").is_err());
        assert!(serde_yaml::from_str::<ContentType>("Meeting").is_err());
    }

    #[test]
    fn failure_kind_other_carries_its_code() {
        let kind: FailureKind =
            serde_json::from_str(r#"{"other":{"code":"weird-driver"}}"#).unwrap();
        assert_eq!(
            kind,
            FailureKind::Other {
                code: "weird-driver".into()
            }
        );
        assert_eq!(
            yaml::<FailureKind>("permission-denied"),
            FailureKind::PermissionDenied
        );
        assert_eq!(
            yaml::<FailureKind>("backend-unavailable"),
            FailureKind::BackendUnavailable
        );
    }

    #[test]
    fn entity_links_is_empty_requires_both_lists_empty() {
        let mut links = EntityLinks::default();
        assert!(links.is_empty());
        links.projects.push(EntityRef {
            slug: "apollo".into(),
            label: "Apollo".into(),
            aliases: vec![],
        });
        assert!(!links.is_empty());
        links.projects.clear();
        links.people.push(EntityRef {
            slug: "ada".into(),
            label: "Ada".into(),
            aliases: vec!["A.".into()],
        });
        assert!(!links.is_empty());
    }

    #[test]
    fn empty_optional_fields_are_omitted_when_serialized() {
        let fm: Frontmatter =
            yaml("title: T\ntype: meeting\ndate: 2026-03-17T12:00:00-07:00\nduration: 5m\n");
        let out = serde_yaml::to_string(&fm).unwrap();
        for key in [
            "tags",
            "attendees",
            "people",
            "entities",
            "action_items",
            "decisions",
            "intents",
            "speaker_map",
            "status",
            "source",
            "visibility",
            "recording_health",
            "capture",
            "processing_warnings",
        ] {
            assert!(
                !out.contains(&format!("{key}:")),
                "{key} leaked into:\n{out}"
            );
        }
        assert!(out.contains("title: T"));
        assert!(out.contains("type: meeting"));
    }

    #[test]
    fn frontmatter_round_trips_through_yaml() {
        let src = "title: Sync\ntype: meeting\ndate: 2026-03-17T12:00:00-07:00\nduration: 5m\nstatus: degraded\nprocessing_warnings:\n- step: summarize\n  reason: timeout\n  timeout_secs: 30\ntags:\n- a\naction_items:\n- assignee: mat\n  task: Send doc\n  due: friday\n  status: open\nintents:\n- kind: commitment\n  what: ship\n  who: ada\n  status: open\n";
        let fm: Frontmatter = yaml(src);
        assert_eq!(fm.status, Some(OutputStatus::Degraded));
        assert_eq!(fm.processing_warnings[0].timeout_secs, Some(30));
        assert!(fm.processing_warnings[0].message.is_none());

        let again: Frontmatter = yaml(&serde_yaml::to_string(&fm).unwrap());
        assert_eq!(again.title, fm.title);
        assert_eq!(again.date, fm.date);
        assert_eq!(again.processing_warnings, fm.processing_warnings);
        assert_eq!(again.action_items[0].due.as_deref(), Some("friday"));
        assert_eq!(again.intents[0].kind, IntentKind::Commitment);
        assert_eq!(again.intents[0].by_date, None);
    }
}
