//! Versioned evidence schema, limitation codes, and knowledge-graph record types.
//!
//! Layer 0: pure data types, zero I/O, no unsafe code, and fully WASM-compatible.

#![forbid(unsafe_code)]

/// The current canonical schema version for evidence records.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Confidence level for clean-room observations and findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Confidence {
    /// Directly observed from outside behavior in the current session.
    High,
    /// Expected behavior of the product class, not independently re-verified.
    Medium,
    /// Pending verification / scheduled test.
    Unverified,
}

impl Confidence {
    /// Returns the canonical label for the confidence level.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Unverified => "UNVERIFIED",
        }
    }
}

/// Structured limitation code explaining why an inspection or comparison was incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitationCode {
    /// Edit budget fallback was triggered due to huge input size.
    EditBudgetFallback,
    /// Directory tree traversal was capped at a maximum depth.
    DepthCapped,
    /// Binary difference hunks reached the maximum hunk limit.
    HunkCapped,
    /// File was classified as binary and diffed as opaque bytes.
    BinaryTreatedAsOpaque,
    /// Operating system or sandbox permission was denied.
    PermissionDenied,
    /// Symlink cycle or target was skipped to prevent loops.
    SymlinkSkipped,
    /// Output buffer was truncated to meet size/budget bounds.
    TruncatedOutput,
    /// Media or file format codec is unsupported.
    UnsupportedCodec,
}

impl LimitationCode {
    /// Returns the canonical kebab-case identifier for this limitation.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EditBudgetFallback => "edit-budget-fallback",
            Self::DepthCapped => "depth-capped",
            Self::HunkCapped => "hunk-capped",
            Self::BinaryTreatedAsOpaque => "binary-treated-as-opaque",
            Self::PermissionDenied => "permission-denied",
            Self::SymlinkSkipped => "symlink-skipped",
            Self::TruncatedOutput => "truncated-output",
            Self::UnsupportedCodec => "unsupported-codec",
        }
    }
}

/// Structured coverage report representing honest inspection limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReport {
    /// Version of the coverage report schema.
    pub schema_version: u32,
    /// True if the comparison or inspection was exhaustive and exact.
    pub exact: bool,
    /// True if results were truncated or capped.
    pub truncated: bool,
    /// List of limitation codes encountered during inspection.
    pub limitations: Vec<LimitationCode>,
}

impl CoverageReport {
    /// Creates a new exact coverage report with no limitations.
    #[must_use]
    pub const fn exact() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            exact: true,
            truncated: false,
            limitations: Vec::new(),
        }
    }

    /// Creates a truncated or limited coverage report.
    #[must_use]
    pub const fn limited(limitations: Vec<LimitationCode>, truncated: bool) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            exact: limitations.is_empty() && !truncated,
            truncated,
            limitations,
        }
    }
}

impl Default for CoverageReport {
    fn default() -> Self {
        Self::exact()
    }
}

/// Structured record of an external observation or parity evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    /// Schema version for forward/backward compatibility.
    pub schema_version: u32,
    /// Unique identifier for this evidence item.
    pub id: String,
    /// Feature, session type, or topic being evidenced.
    pub subject: String,
    /// Confidence tier.
    pub confidence: Confidence,
    /// Coverage report with honest limitations.
    pub coverage: CoverageReport,
    /// Primary source of the observation (e.g. "Info.plist", "Accessibility Menu").
    pub source: String,
    /// Descriptive notes or reproducible details.
    pub notes: String,
}

impl EvidenceRecord {
    /// Creates a new evidence record with the current schema version.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        subject: impl Into<String>,
        confidence: Confidence,
        coverage: CoverageReport,
        source: impl Into<String>,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: id.into(),
            subject: subject.into(),
            confidence,
            coverage,
            source: source.into(),
            notes: notes.into(),
        }
    }
}

/// Status of a hypothesis in the clean-room study loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HypothesisStatus {
    /// Newly proposed hypothesis.
    Draft,
    /// Under test or observation.
    Tested,
    /// Confirmed by repeatable evidence.
    Confirmed,
    /// Refuted by external evidence.
    Refuted,
}

impl HypothesisStatus {
    /// Returns the string representation of the hypothesis status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Tested => "tested",
            Self::Confirmed => "confirmed",
            Self::Refuted => "refuted",
        }
    }
}

/// Structured hypothesis record tracking design hypotheses and verifications.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypothesisRecord {
    /// Schema version for migration.
    pub schema_version: u32,
    /// Unique hypothesis ID.
    pub id: String,
    /// Statement of the hypothesis.
    pub statement: String,
    /// Current lifecycle status.
    pub status: HypothesisStatus,
    /// Current confidence level.
    pub confidence: Confidence,
    /// IDs of supporting evidence records.
    pub supporting_evidence_ids: Vec<String>,
    /// Rationale and experiment observations.
    pub notes: String,
}

impl HypothesisRecord {
    /// Creates a new hypothesis record.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        statement: impl Into<String>,
        status: HypothesisStatus,
        confidence: Confidence,
        supporting_evidence_ids: Vec<String>,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: id.into(),
            statement: statement.into(),
            status,
            confidence,
            supporting_evidence_ids,
            notes: notes.into(),
        }
    }
}

/// Directed relation in the knowledge graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRelation {
    /// Type of relation (e.g. "belongs_to", "depends_on", "evidenced_by").
    pub relation: String,
    /// Target entity identifier.
    pub target_id: String,
}

impl GraphRelation {
    /// Creates a new relation edge.
    #[must_use]
    pub fn new(relation: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            relation: relation.into(),
            target_id: target_id.into(),
        }
    }
}

/// Knowledge graph node recording entities, features, or design boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeGraphRecord {
    /// Schema version.
    pub schema_version: u32,
    /// Unique entity identifier.
    pub id: String,
    /// Kind of entity (e.g. "session_type", "menu_action", "file_format").
    pub entity_type: String,
    /// Human-readable entity name.
    pub name: String,
    /// Key-value attributes associated with the entity.
    pub attributes: Vec<(String, String)>,
    /// Outgoing relations to other knowledge graph entities.
    pub relations: Vec<GraphRelation>,
}

impl KnowledgeGraphRecord {
    /// Creates a new knowledge graph record.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        entity_type: impl Into<String>,
        name: impl Into<String>,
        attributes: Vec<(String, String)>,
        relations: Vec<GraphRelation>,
    ) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: id.into(),
            entity_type: entity_type.into(),
            name: name.into(),
            attributes,
            relations,
        }
    }
}

/// Structured error returned by tools or comparison harnesses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// Access permission was denied.
    PermissionDenied(String),
    /// I/O error occurred during inspection.
    IoError(String),
    /// A configured safety limit was exceeded.
    LimitExceeded(LimitationCode),
    /// User or harness input was invalid.
    InvalidInput(String),
    /// Other unclassified error condition.
    Other(String),
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied(msg) => write!(f, "permission denied: {msg}"),
            Self::IoError(msg) => write!(f, "I/O error: {msg}"),
            Self::LimitExceeded(code) => write!(f, "limit exceeded: {}", code.code()),
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::Other(msg) => write!(f, "tool error: {msg}"),
        }
    }
}

impl std::error::Error for ToolError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version_is_positive() {
        assert_eq!(CURRENT_SCHEMA_VERSION, 1);
    }

    #[test]
    fn test_confidence_strings() {
        assert_eq!(Confidence::High.as_str(), "HIGH");
        assert_eq!(Confidence::Medium.as_str(), "MEDIUM");
        assert_eq!(Confidence::Unverified.as_str(), "UNVERIFIED");
    }

    #[test]
    fn test_limitation_codes() {
        assert_eq!(
            LimitationCode::EditBudgetFallback.code(),
            "edit-budget-fallback"
        );
        assert_eq!(LimitationCode::DepthCapped.code(), "depth-capped");
        assert_eq!(LimitationCode::HunkCapped.code(), "hunk-capped");
        assert_eq!(
            LimitationCode::BinaryTreatedAsOpaque.code(),
            "binary-treated-as-opaque"
        );
        assert_eq!(LimitationCode::PermissionDenied.code(), "permission-denied");
        assert_eq!(LimitationCode::SymlinkSkipped.code(), "symlink-skipped");
        assert_eq!(LimitationCode::TruncatedOutput.code(), "truncated-output");
        assert_eq!(LimitationCode::UnsupportedCodec.code(), "unsupported-codec");
    }

    #[test]
    fn test_coverage_report_constructors() {
        let exact = CoverageReport::exact();
        assert_eq!(exact.schema_version, 1);
        assert!(exact.exact);
        assert!(!exact.truncated);
        assert!(exact.limitations.is_empty());

        let limited = CoverageReport::limited(vec![LimitationCode::DepthCapped], true);
        assert_eq!(limited.schema_version, 1);
        assert!(!limited.exact);
        assert!(limited.truncated);
        assert_eq!(limited.limitations.len(), 1);
    }

    #[test]
    fn test_evidence_record_instantiation() {
        let rec = EvidenceRecord::new(
            "obs-01",
            "Folder Compare",
            Confidence::High,
            CoverageReport::exact(),
            "Session Menu",
            "Directly observed in app menu",
        );
        assert_eq!(rec.schema_version, 1);
        assert_eq!(rec.id, "obs-01");
        assert_eq!(rec.confidence, Confidence::High);
        assert!(rec.coverage.exact);
    }

    #[test]
    fn test_hypothesis_record_instantiation() {
        let hyp = HypothesisRecord::new(
            "hyp-01",
            "Exit code is 1 when folders differ",
            HypothesisStatus::Confirmed,
            Confidence::High,
            vec!["obs-01".to_string()],
            "Verified via CLI runs",
        );
        assert_eq!(hyp.schema_version, 1);
        assert_eq!(hyp.status.as_str(), "confirmed");
        assert_eq!(hyp.supporting_evidence_ids.len(), 1);
    }

    #[test]
    fn test_knowledge_graph_record_instantiation() {
        let kg = KnowledgeGraphRecord::new(
            "node-text-compare",
            "session_type",
            "Text Compare",
            vec![("mode".to_string(), "side-by-side".to_string())],
            vec![GraphRelation::new("has_feature", "syntax_highlighting")],
        );
        assert_eq!(kg.schema_version, 1);
        assert_eq!(kg.name, "Text Compare");
        assert_eq!(kg.relations.len(), 1);
    }

    #[test]
    fn test_tool_error_display() {
        let err = ToolError::LimitExceeded(LimitationCode::DepthCapped);
        assert_eq!(err.to_string(), "limit exceeded: depth-capped");
    }
}
