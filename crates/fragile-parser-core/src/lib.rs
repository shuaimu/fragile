use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

pub const PARSER_OUTPUT_SCHEMA_VERSION_V1: &str = "1.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserLanguage {
    C,
    Cpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeDirectiveKind {
    Include,
    System,
    Quote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDirective {
    pub kind: IncludeDirectiveKind,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRequest {
    pub source_path: PathBuf,
    pub language: ParserLanguage,
    pub frontend_args: Vec<String>,
    pub defines: Vec<String>,
    pub include_directives: Vec<IncludeDirective>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserOutputV1 {
    pub schema_version: String,
    pub translation_unit: ParserTranslationUnit,
    pub nodes: Vec<ParserNode>,
    pub diagnostics: Vec<ParserDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserTranslationUnit {
    pub source_path: PathBuf,
    pub language: ParserLanguage,
    pub parser_backend: String,
    pub frontend_args: Vec<String>,
    pub defines: Vec<String>,
    pub include_directives: Vec<IncludeDirective>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserNode {
    pub node_id: String,
    pub parent_id: Option<String>,
    pub node_kind: String,
    pub name: Option<String>,
    pub cpp_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserDiagnostic {
    pub level: ParserDiagnosticLevel,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserDiagnosticLevel {
    Warning,
    Error,
}

pub trait ParserBackend: Send + Sync {
    fn backend_id(&self) -> &'static str;

    fn parse(&self, request: &ParseRequest) -> Result<ParserOutputV1, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserCoreError {
    InvalidBackendId {
        backend_id: String,
    },
    DuplicateBackend {
        backend_id: String,
    },
    UnknownBackend {
        backend_id: String,
    },
    BackendFailure {
        backend_id: String,
        message: String,
    },
}

impl fmt::Display for ParserCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBackendId { backend_id } => {
                write!(f, "invalid parser backend id `{backend_id}`")
            }
            Self::DuplicateBackend { backend_id } => {
                write!(f, "duplicate parser backend id `{backend_id}`")
            }
            Self::UnknownBackend { backend_id } => {
                write!(f, "unknown parser backend id `{backend_id}`")
            }
            Self::BackendFailure {
                backend_id,
                message,
            } => write!(f, "parser backend `{backend_id}` failed: {message}"),
        }
    }
}

impl Error for ParserCoreError {}

// ---------------------------------------------------------------------------
// Deterministic error class for unsupported STL placeholder shapes (M6.1)
// ---------------------------------------------------------------------------

/// Deterministic error code prefix for unsupported STL placeholder shape errors.
pub const STL_SHAPE_ERROR_CODE_PREFIX: &str = "FRAGILE_STL_E";

/// Stable error codes for unsupported STL placeholder shape failure classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum UnsupportedStlShapeErrorCode {
    /// The placeholder node kind is not in the set of recognized STL placeholder kinds.
    UnrecognizedPlaceholderKind,
    /// The placeholder family is recognized but has no pre-generated contract mapping.
    MissingFamilyMapping,
    /// The placeholder family is mapped but the concrete shape (element/key/value type
    /// combination) is not supported by any pre-generated implementation.
    UnsupportedConcreteShape,
}

impl UnsupportedStlShapeErrorCode {
    /// Returns the stable deterministic error code string (e.g. `"FRAGILE_STL_E001"`).
    pub fn code_str(self) -> &'static str {
        match self {
            Self::UnrecognizedPlaceholderKind => "FRAGILE_STL_E001",
            Self::MissingFamilyMapping => "FRAGILE_STL_E002",
            Self::UnsupportedConcreteShape => "FRAGILE_STL_E003",
        }
    }

    /// Returns a short human-readable label for the error class.
    pub fn label(self) -> &'static str {
        match self {
            Self::UnrecognizedPlaceholderKind => "unrecognized STL placeholder kind",
            Self::MissingFamilyMapping => "missing STL family mapping",
            Self::UnsupportedConcreteShape => "unsupported STL concrete shape",
        }
    }
}

impl fmt::Display for UnsupportedStlShapeErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code_str())
    }
}

/// Source location context for an unsupported STL shape occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StlShapeSourceLocation {
    /// Source file path where the unsupported STL shape was observed.
    pub file: Option<String>,
    /// 1-based line number, if available.
    pub line: Option<u32>,
    /// 1-based column number, if available.
    pub column: Option<u32>,
}

impl fmt::Display for StlShapeSourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.file, self.line, self.column) {
            (Some(file), Some(line), Some(col)) => write!(f, "{file}:{line}:{col}"),
            (Some(file), Some(line), None) => write!(f, "{file}:{line}"),
            (Some(file), None, _) => write!(f, "{file}"),
            (None, _, _) => write!(f, "<unknown>"),
        }
    }
}

/// Deterministic error for an unsupported STL placeholder shape.
///
/// This error is emitted when the parser or codegen encounters an STL placeholder
/// node whose shape (family, element types, or concrete instantiation) does not
/// have a pre-generated mapping target. The error captures enough metadata to
/// produce actionable diagnostics.
///
/// The `Display` format is deterministic and stable across runs for the same inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedStlShapeError {
    /// Stable deterministic error code.
    pub code: UnsupportedStlShapeErrorCode,
    /// The C++ symbol or type spelling that triggered the error.
    pub symbol: String,
    /// Source location where the unsupported shape was observed.
    pub location: StlShapeSourceLocation,
    /// The parser-output node kind that was emitted (e.g. `"stl_deque_placeholder"`).
    pub placeholder_kind: String,
    /// The STL family name extracted from the placeholder kind, if parseable
    /// (e.g. `"deque"`, `"map"`).
    pub family: Option<String>,
    /// A deterministic fingerprint of the shape (family + element/key/value types).
    /// Format: `"family(element_types)"` e.g. `"map(std::string, int)"`.
    pub shape_fingerprint: String,
    /// The mapping key that was looked up but not found, if applicable.
    pub missing_mapping_key: Option<String>,
    /// Sorted list of supported STL families for context in the error message.
    pub supported_families: Vec<String>,
}

impl fmt::Display for UnsupportedStlShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: `{}` at {} (placeholder_kind=`{}`, shape=`{}`",
            self.code.code_str(),
            self.code.label(),
            self.symbol,
            self.location,
            self.placeholder_kind,
            self.shape_fingerprint,
        )?;
        if let Some(ref key) = self.missing_mapping_key {
            write!(f, ", missing_key=`{key}`")?;
        }
        if !self.supported_families.is_empty() {
            write!(f, ", supported=[{}]", self.supported_families.join(", "))?;
        }
        write!(f, ")")
    }
}

impl Error for UnsupportedStlShapeError {}

impl UnsupportedStlShapeError {
    /// Convenience constructor for an unrecognized placeholder kind.
    pub fn unrecognized_placeholder_kind(
        symbol: impl Into<String>,
        placeholder_kind: impl Into<String>,
        supported_families: Vec<String>,
    ) -> Self {
        let placeholder_kind = placeholder_kind.into();
        let family = placeholder_kind
            .strip_prefix("stl_")
            .and_then(|s| s.strip_suffix("_placeholder"))
            .filter(|s| !s.is_empty())
            .map(String::from);
        let shape_fingerprint = family
            .as_deref()
            .unwrap_or(&placeholder_kind)
            .to_string();
        Self {
            code: UnsupportedStlShapeErrorCode::UnrecognizedPlaceholderKind,
            symbol: symbol.into(),
            location: StlShapeSourceLocation {
                file: None,
                line: None,
                column: None,
            },
            placeholder_kind,
            family,
            shape_fingerprint,
            missing_mapping_key: None,
            supported_families,
        }
    }

    /// Convenience constructor for a recognized family without a pre-generated mapping.
    pub fn missing_family_mapping(
        symbol: impl Into<String>,
        placeholder_kind: impl Into<String>,
        family: impl Into<String>,
    ) -> Self {
        let family_str = family.into();
        Self {
            code: UnsupportedStlShapeErrorCode::MissingFamilyMapping,
            symbol: symbol.into(),
            location: StlShapeSourceLocation {
                file: None,
                line: None,
                column: None,
            },
            placeholder_kind: placeholder_kind.into(),
            family: Some(family_str.clone()),
            shape_fingerprint: family_str.clone(),
            missing_mapping_key: Some(family_str),
            supported_families: Vec::new(),
        }
    }

    /// Convenience constructor for an unsupported concrete shape.
    pub fn unsupported_concrete_shape(
        symbol: impl Into<String>,
        placeholder_kind: impl Into<String>,
        family: impl Into<String>,
        shape_fingerprint: impl Into<String>,
        missing_mapping_key: impl Into<String>,
    ) -> Self {
        let family_str = family.into();
        Self {
            code: UnsupportedStlShapeErrorCode::UnsupportedConcreteShape,
            symbol: symbol.into(),
            location: StlShapeSourceLocation {
                file: None,
                line: None,
                column: None,
            },
            placeholder_kind: placeholder_kind.into(),
            family: Some(family_str),
            shape_fingerprint: shape_fingerprint.into(),
            missing_mapping_key: Some(missing_mapping_key.into()),
            supported_families: Vec::new(),
        }
    }

    /// Set the source location on the error.
    pub fn with_location(mut self, location: StlShapeSourceLocation) -> Self {
        self.location = location;
        self
    }

    /// Set the supported families list on the error.
    pub fn with_supported_families(mut self, families: Vec<String>) -> Self {
        self.supported_families = families;
        self
    }

    /// Convert this error into a `ParserDiagnostic`.
    pub fn to_parser_diagnostic(&self) -> ParserDiagnostic {
        ParserDiagnostic {
            level: ParserDiagnosticLevel::Error,
            code: self.code.code_str().to_string(),
            message: self.to_string(),
        }
    }
}

#[derive(Default)]
pub struct BackendRegistry {
    backends: BTreeMap<String, Arc<dyn ParserBackend>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<B>(&mut self, backend: B) -> Result<(), ParserCoreError>
    where
        B: ParserBackend + 'static,
    {
        self.register_arc(Arc::new(backend))
    }

    pub fn register_arc(
        &mut self,
        backend: Arc<dyn ParserBackend>,
    ) -> Result<(), ParserCoreError> {
        let backend_id = backend.backend_id().trim();
        if backend_id.is_empty() {
            return Err(ParserCoreError::InvalidBackendId {
                backend_id: backend.backend_id().to_string(),
            });
        }
        if self.backends.contains_key(backend_id) {
            return Err(ParserCoreError::DuplicateBackend {
                backend_id: backend_id.to_string(),
            });
        }
        self.backends.insert(backend_id.to_string(), backend);
        Ok(())
    }

    pub fn get(&self, backend_id: &str) -> Option<Arc<dyn ParserBackend>> {
        let backend_id = backend_id.trim();
        self.backends.get(backend_id).map(Arc::clone)
    }

    pub fn backend_ids(&self) -> Vec<String> {
        self.backends.keys().cloned().collect()
    }

    pub fn parse_with(
        &self,
        backend_id: &str,
        request: &ParseRequest,
    ) -> Result<ParserOutputV1, ParserCoreError> {
        let backend_key = backend_id.trim();
        let Some(backend) = self.backends.get(backend_key) else {
            return Err(ParserCoreError::UnknownBackend {
                backend_id: backend_key.to_string(),
            });
        };
        backend
            .parse(request)
            .map_err(|message| ParserCoreError::BackendFailure {
                backend_id: backend_key.to_string(),
                message,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BackendRegistry, ParseRequest, ParserBackend, ParserCoreError, ParserLanguage, ParserNode,
        ParserOutputV1, ParserTranslationUnit, PARSER_OUTPUT_SCHEMA_VERSION_V1,
    };
    use std::path::PathBuf;

    struct FakeBackend {
        id: &'static str,
        should_fail: bool,
    }

    impl ParserBackend for FakeBackend {
        fn backend_id(&self) -> &'static str {
            self.id
        }

        fn parse(&self, request: &ParseRequest) -> Result<ParserOutputV1, String> {
            if self.should_fail {
                return Err("synthetic parse failure".to_string());
            }
            Ok(ParserOutputV1 {
                schema_version: PARSER_OUTPUT_SCHEMA_VERSION_V1.to_string(),
                translation_unit: ParserTranslationUnit {
                    source_path: request.source_path.clone(),
                    language: request.language,
                    parser_backend: self.id.to_string(),
                    frontend_args: request.frontend_args.clone(),
                    defines: request.defines.clone(),
                    include_directives: request.include_directives.clone(),
                },
                nodes: vec![ParserNode {
                    node_id: "n0".to_string(),
                    parent_id: None,
                    node_kind: "translation_unit".to_string(),
                    name: Some("tu".to_string()),
                    cpp_type: None,
                }],
                diagnostics: Vec::new(),
            })
        }
    }

    fn sample_request() -> ParseRequest {
        ParseRequest {
            source_path: PathBuf::from("tests/fixtures/sample.cpp"),
            language: ParserLanguage::Cpp,
            frontend_args: vec!["-std=c++20".to_string()],
            defines: vec!["UNIT_TEST=1".to_string()],
            include_directives: Vec::new(),
        }
    }

    #[test]
    fn registry_register_and_parse_with_backend() {
        let mut registry = BackendRegistry::new();
        registry
            .register(FakeBackend {
                id: "fake",
                should_fail: false,
            })
            .expect("register fake backend");
        let output = registry
            .parse_with("fake", &sample_request())
            .expect("parse_with should dispatch");
        assert_eq!(output.schema_version, PARSER_OUTPUT_SCHEMA_VERSION_V1);
        assert_eq!(output.translation_unit.parser_backend, "fake");
        assert_eq!(output.nodes.len(), 1);
    }

    #[test]
    fn registry_rejects_duplicate_backend_registration() {
        let mut registry = BackendRegistry::new();
        registry
            .register(FakeBackend {
                id: "dup",
                should_fail: false,
            })
            .expect("first registration should succeed");
        let err = registry
            .register(FakeBackend {
                id: "dup",
                should_fail: false,
            })
            .expect_err("duplicate registration should fail");
        assert_eq!(
            err,
            ParserCoreError::DuplicateBackend {
                backend_id: "dup".to_string()
            }
        );
    }

    #[test]
    fn registry_reports_unknown_backend() {
        let registry = BackendRegistry::new();
        let err = registry
            .parse_with("missing", &sample_request())
            .expect_err("missing backend should error");
        assert_eq!(
            err,
            ParserCoreError::UnknownBackend {
                backend_id: "missing".to_string()
            }
        );
    }

    #[test]
    fn registry_wraps_backend_failure_with_backend_id() {
        let mut registry = BackendRegistry::new();
        registry
            .register(FakeBackend {
                id: "failing",
                should_fail: true,
            })
            .expect("register failing backend");
        let err = registry
            .parse_with("failing", &sample_request())
            .expect_err("failing backend should bubble as backend failure");
        assert_eq!(
            err,
            ParserCoreError::BackendFailure {
                backend_id: "failing".to_string(),
                message: "synthetic parse failure".to_string(),
            }
        );
    }

    #[test]
    fn registry_backend_ids_are_deterministic_and_sorted() {
        let mut registry = BackendRegistry::new();
        registry
            .register(FakeBackend {
                id: "z_backend",
                should_fail: false,
            })
            .expect("register z");
        registry
            .register(FakeBackend {
                id: "a_backend",
                should_fail: false,
            })
            .expect("register a");
        registry
            .register(FakeBackend {
                id: "m_backend",
                should_fail: false,
            })
            .expect("register m");
        assert_eq!(
            registry.backend_ids(),
            vec![
                "a_backend".to_string(),
                "m_backend".to_string(),
                "z_backend".to_string()
            ]
        );
    }

    #[test]
    fn registry_rejects_empty_backend_id() {
        let mut registry = BackendRegistry::new();
        let err = registry
            .register(FakeBackend {
                id: "   ",
                should_fail: false,
            })
            .expect_err("empty backend id should fail");
        assert_eq!(
            err,
            ParserCoreError::InvalidBackendId {
                backend_id: "   ".to_string()
            }
        );
    }

    // --- UnsupportedStlShapeError tests (M6.1) ---

    use super::{
        StlShapeSourceLocation, UnsupportedStlShapeError, UnsupportedStlShapeErrorCode,
        STL_SHAPE_ERROR_CODE_PREFIX,
    };

    #[test]
    fn error_code_strings_are_stable_and_prefixed() {
        assert_eq!(
            UnsupportedStlShapeErrorCode::UnrecognizedPlaceholderKind.code_str(),
            "FRAGILE_STL_E001"
        );
        assert_eq!(
            UnsupportedStlShapeErrorCode::MissingFamilyMapping.code_str(),
            "FRAGILE_STL_E002"
        );
        assert_eq!(
            UnsupportedStlShapeErrorCode::UnsupportedConcreteShape.code_str(),
            "FRAGILE_STL_E003"
        );
        // All codes share the declared prefix.
        for code in [
            UnsupportedStlShapeErrorCode::UnrecognizedPlaceholderKind,
            UnsupportedStlShapeErrorCode::MissingFamilyMapping,
            UnsupportedStlShapeErrorCode::UnsupportedConcreteShape,
        ] {
            assert!(
                code.code_str().starts_with(STL_SHAPE_ERROR_CODE_PREFIX),
                "code `{}` must start with `{}`",
                code.code_str(),
                STL_SHAPE_ERROR_CODE_PREFIX,
            );
        }
    }

    #[test]
    fn error_code_labels_are_nonempty() {
        for code in [
            UnsupportedStlShapeErrorCode::UnrecognizedPlaceholderKind,
            UnsupportedStlShapeErrorCode::MissingFamilyMapping,
            UnsupportedStlShapeErrorCode::UnsupportedConcreteShape,
        ] {
            assert!(!code.label().is_empty(), "label for {code:?} must be nonempty");
        }
    }

    #[test]
    fn unrecognized_placeholder_kind_display_is_deterministic() {
        let err = UnsupportedStlShapeError::unrecognized_placeholder_kind(
            "std::deque<int>",
            "stl_deque_placeholder",
            vec!["map".to_string(), "vector".to_string()],
        );
        let msg = err.to_string();
        assert!(msg.contains("FRAGILE_STL_E001"), "must contain error code: {msg}");
        assert!(msg.contains("stl_deque_placeholder"), "must contain placeholder kind: {msg}");
        assert!(msg.contains("std::deque<int>"), "must contain symbol: {msg}");
        assert!(msg.contains("supported=[map, vector]"), "must list supported families: {msg}");

        // Deterministic: same inputs produce identical output.
        let err2 = UnsupportedStlShapeError::unrecognized_placeholder_kind(
            "std::deque<int>",
            "stl_deque_placeholder",
            vec!["map".to_string(), "vector".to_string()],
        );
        assert_eq!(err.to_string(), err2.to_string());
    }

    #[test]
    fn missing_family_mapping_display_is_deterministic() {
        let err = UnsupportedStlShapeError::missing_family_mapping(
            "std::optional<int>",
            "stl_optional_placeholder",
            "optional",
        );
        let msg = err.to_string();
        assert!(msg.contains("FRAGILE_STL_E002"), "must contain error code: {msg}");
        assert!(msg.contains("optional"), "must contain family: {msg}");
        assert!(msg.contains("missing_key=`optional`"), "must contain missing key: {msg}");
    }

    #[test]
    fn unsupported_concrete_shape_display_is_deterministic() {
        let err = UnsupportedStlShapeError::unsupported_concrete_shape(
            "std::map<std::string, int>",
            "stl_map_placeholder",
            "map",
            "map(std::string, int)",
            "std_map_string__int",
        );
        let msg = err.to_string();
        assert!(msg.contains("FRAGILE_STL_E003"), "must contain error code: {msg}");
        assert!(msg.contains("map(std::string, int)"), "must contain shape: {msg}");
        assert!(
            msg.contains("missing_key=`std_map_string__int`"),
            "must contain missing key: {msg}"
        );
    }

    #[test]
    fn with_location_sets_source_location() {
        let err = UnsupportedStlShapeError::unrecognized_placeholder_kind(
            "std::deque<int>",
            "stl_deque_placeholder",
            vec![],
        )
        .with_location(StlShapeSourceLocation {
            file: Some("test.cpp".to_string()),
            line: Some(42),
            column: Some(5),
        });
        let msg = err.to_string();
        assert!(msg.contains("test.cpp:42:5"), "must contain location: {msg}");
    }

    #[test]
    fn source_location_display_variants() {
        let full = StlShapeSourceLocation {
            file: Some("a.cpp".to_string()),
            line: Some(10),
            column: Some(3),
        };
        assert_eq!(full.to_string(), "a.cpp:10:3");

        let no_col = StlShapeSourceLocation {
            file: Some("b.cpp".to_string()),
            line: Some(7),
            column: None,
        };
        assert_eq!(no_col.to_string(), "b.cpp:7");

        let file_only = StlShapeSourceLocation {
            file: Some("c.cpp".to_string()),
            line: None,
            column: None,
        };
        assert_eq!(file_only.to_string(), "c.cpp");

        let unknown = StlShapeSourceLocation {
            file: None,
            line: None,
            column: None,
        };
        assert_eq!(unknown.to_string(), "<unknown>");
    }

    #[test]
    fn to_parser_diagnostic_produces_error_level() {
        let err = UnsupportedStlShapeError::unrecognized_placeholder_kind(
            "std::deque<int>",
            "stl_deque_placeholder",
            vec![],
        );
        let diag = err.to_parser_diagnostic();
        assert_eq!(diag.level, super::ParserDiagnosticLevel::Error);
        assert_eq!(diag.code, "FRAGILE_STL_E001");
        assert!(!diag.message.is_empty());
    }

    #[test]
    fn unrecognized_placeholder_kind_extracts_family_from_kind_string() {
        let err = UnsupportedStlShapeError::unrecognized_placeholder_kind(
            "std::deque<int>",
            "stl_deque_placeholder",
            vec![],
        );
        assert_eq!(err.family.as_deref(), Some("deque"));
        assert_eq!(err.shape_fingerprint, "deque");
    }

    #[test]
    fn unrecognized_placeholder_kind_handles_nonstandard_kind_string() {
        let err = UnsupportedStlShapeError::unrecognized_placeholder_kind(
            "SomeType",
            "unknown_kind",
            vec![],
        );
        // Cannot parse family from non-standard format.
        assert_eq!(err.family, None);
        assert_eq!(err.shape_fingerprint, "unknown_kind");
    }

    #[test]
    fn error_implements_std_error_trait() {
        let err = UnsupportedStlShapeError::unrecognized_placeholder_kind(
            "std::deque<int>",
            "stl_deque_placeholder",
            vec![],
        );
        // Verify it can be used as a trait object.
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn error_codes_are_ordered() {
        // Codes have a deterministic ordering matching severity/priority.
        assert!(
            UnsupportedStlShapeErrorCode::UnrecognizedPlaceholderKind
                < UnsupportedStlShapeErrorCode::MissingFamilyMapping
        );
        assert!(
            UnsupportedStlShapeErrorCode::MissingFamilyMapping
                < UnsupportedStlShapeErrorCode::UnsupportedConcreteShape
        );
    }
}
