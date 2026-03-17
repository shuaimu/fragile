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
}
