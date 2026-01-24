//! Diagnostic reporting infrastructure for the Fragile transpiler.
//!
//! This module provides error and warning reporting using the miette crate.
//! Currently prepared for future integration; the Diagnostic struct fields
//! are consumed by miette derive macros when displayed.

// Suppress warnings: fields are assigned in constructors but appear "unused" to the linter
// because they're consumed by miette derive macros for diagnostic display, not direct reads.
#![allow(dead_code, unused)]

use crate::span::Span;
use miette::{Diagnostic as MietteDiagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Error, MietteDiagnostic)]
#[error("{message}")]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    #[label("{label}")]
    pub span: Option<SourceSpan>,
    pub label: String,
    #[help]
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            span: None,
            label: String::new(),
            help: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            span: None,
            label: String::new(),
            help: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(SourceSpan::new((span.start as usize).into(), span.len() as usize));
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}
