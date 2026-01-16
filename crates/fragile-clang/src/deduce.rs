//! Template argument deduction for C++ function templates.
//!
//! This module implements template argument deduction following C++ rules.
//! When a function template is called without explicit template arguments,
//! the compiler deduces the types from the call arguments.

use std::collections::HashMap;

use crate::types::CppType;
use crate::CppFunctionTemplate;

/// Error during template argument deduction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeductionError {
    /// Template parameter was deduced to different types from different arguments.
    Conflict {
        param: String,
        first: CppType,
        second: CppType,
    },
    /// Non-dependent types didn't match.
    TypeMismatch { expected: CppType, got: CppType },
    /// Not enough arguments to deduce all template parameters.
    InsufficientArguments {
        missing: Vec<String>,
    },
}

/// Template argument deducer.
pub struct TypeDeducer {
    /// Accumulated deductions: template param name → deduced type.
    deductions: HashMap<String, CppType>,
}

impl TypeDeducer {
    /// Create a new deducer.
    pub fn new() -> Self {
        Self {
            deductions: HashMap::new(),
        }
    }

    /// Deduce template arguments from call arguments.
    ///
    /// Given a function template and the types of the actual arguments at the call site,
    /// this deduces the template parameter bindings.
    ///
    /// # Example
    /// ```ignore
    /// template<typename T> T identity(T x);
    /// identity(42); // Deduces T = int
    /// ```
    pub fn deduce(
        template: &CppFunctionTemplate,
        arg_types: &[CppType],
    ) -> Result<HashMap<String, CppType>, DeductionError> {
        let mut deducer = Self::new();

        // Match each template parameter type against the corresponding argument type
        for (i, (_, param_type)) in template.params.iter().enumerate() {
            if let Some(arg_type) = arg_types.get(i) {
                deducer.deduce_from_types(param_type, arg_type)?;
            }
        }

        // Check that all template parameters were deduced
        let missing: Vec<String> = template
            .template_params
            .iter()
            .filter(|p| !deducer.deductions.contains_key(*p))
            .cloned()
            .collect();

        if !missing.is_empty() {
            return Err(DeductionError::InsufficientArguments { missing });
        }

        Ok(deducer.deductions)
    }

    /// Deduce template arguments by matching param type against arg type.
    fn deduce_from_types(
        &mut self,
        param_type: &CppType,
        arg_type: &CppType,
    ) -> Result<(), DeductionError> {
        match param_type {
            // Direct template parameter: T ← concrete type
            CppType::TemplateParam { name, .. } => {
                self.record_deduction(name, arg_type.clone())?;
            }

            // Reference to template param: T& or const T& or T&&
            // We deduce T from the referent
            CppType::Reference {
                referent, is_const, ..
            } => {
                if let CppType::TemplateParam { name, .. } = referent.as_ref() {
                    // For const T&, the arg type could be just T (lvalue)
                    // For T&, arg type could be int (lvalue of type int)
                    // For T&&, arg type could be int (rvalue of type int)
                    // In all cases, we strip references from arg to get the underlying type
                    let deduced = strip_reference(arg_type);
                    // If param is const T& and arg is const int, deduce T = int (not const int)
                    let deduced = if *is_const {
                        strip_const(&deduced)
                    } else {
                        deduced
                    };
                    self.record_deduction(name, deduced)?;
                } else {
                    // Reference to non-template-param type
                    // The referent types should match
                    self.deduce_from_types(referent, &strip_reference(arg_type))?;
                }
            }

            // Pointer to template param: T*
            CppType::Pointer { pointee, .. } => {
                if let CppType::Pointer { pointee: arg_pointee, .. } = arg_type {
                    self.deduce_from_types(pointee, arg_pointee)?;
                }
                // If arg is not a pointer, can't deduce
            }

            // Non-dependent types must match exactly
            _ if !param_type.is_dependent() => {
                // Allow implicit conversions for primitive types (simplified)
                // In real C++, there are complex conversion rules
                if param_type != arg_type {
                    // For now, we allow widening conversions for arithmetic types
                    if !is_compatible_types(param_type, arg_type) {
                        return Err(DeductionError::TypeMismatch {
                            expected: param_type.clone(),
                            got: arg_type.clone(),
                        });
                    }
                }
            }

            // Dependent types need more complex handling
            CppType::DependentType { .. } => {
                // Skip for now - this requires parsing the spelling
            }

            _ => {}
        }

        Ok(())
    }

    /// Record a deduction, checking for conflicts.
    fn record_deduction(&mut self, name: &str, deduced_type: CppType) -> Result<(), DeductionError> {
        if let Some(existing) = self.deductions.get(name) {
            if existing != &deduced_type {
                return Err(DeductionError::Conflict {
                    param: name.to_string(),
                    first: existing.clone(),
                    second: deduced_type,
                });
            }
        } else {
            self.deductions.insert(name.to_string(), deduced_type);
        }
        Ok(())
    }
}

impl Default for TypeDeducer {
    fn default() -> Self {
        Self::new()
    }
}

/// Strip reference wrapper from a type, returning the referent.
fn strip_reference(ty: &CppType) -> CppType {
    match ty {
        CppType::Reference { referent, .. } => referent.as_ref().clone(),
        _ => ty.clone(),
    }
}

/// Strip const qualifier from a type (for deduction purposes).
fn strip_const(ty: &CppType) -> CppType {
    match ty {
        CppType::Reference { referent, is_rvalue, .. } => {
            CppType::Reference {
                referent: Box::new(strip_const(referent)),
                is_const: false,
                is_rvalue: *is_rvalue,
            }
        }
        CppType::Pointer { pointee, .. } => {
            CppType::Pointer {
                pointee: Box::new(strip_const(pointee)),
                is_const: false,
            }
        }
        _ => ty.clone(),
    }
}

/// Check if two types are compatible (simplified version).
fn is_compatible_types(expected: &CppType, got: &CppType) -> bool {
    match (expected, got) {
        // Same types are always compatible
        (a, b) if a == b => true,
        // Allow char → int (integral promotion)
        (CppType::Int { signed: true }, CppType::Char { .. }) => true,
        (CppType::Int { signed: true }, CppType::Short { .. }) => true,
        // Allow float → double
        (CppType::Double, CppType::Float) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_template(
        name: &str,
        template_params: Vec<&str>,
        params: Vec<(&str, CppType)>,
        return_type: CppType,
    ) -> CppFunctionTemplate {
        CppFunctionTemplate {
            name: name.to_string(),
            namespace: vec![],
            template_params: template_params.into_iter().map(|s| s.to_string()).collect(),
            params: params
                .into_iter()
                .map(|(n, t)| (n.to_string(), t))
                .collect(),
            return_type,
            is_definition: true,
        }
    }

    #[test]
    fn test_deduce_simple_int() {
        // template<typename T> T identity(T x);
        // identity(42); // T = int
        let template = make_template(
            "identity",
            vec!["T"],
            vec![("x", CppType::template_param("T", 0, 0))],
            CppType::template_param("T", 0, 0),
        );

        let arg_types = vec![CppType::Int { signed: true }];
        let result = TypeDeducer::deduce(&template, &arg_types).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.get("T"), Some(&CppType::Int { signed: true }));
    }

    #[test]
    fn test_deduce_simple_double() {
        // template<typename T> T identity(T x);
        // identity(3.14); // T = double
        let template = make_template(
            "identity",
            vec!["T"],
            vec![("x", CppType::template_param("T", 0, 0))],
            CppType::template_param("T", 0, 0),
        );

        let arg_types = vec![CppType::Double];
        let result = TypeDeducer::deduce(&template, &arg_types).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.get("T"), Some(&CppType::Double));
    }

    #[test]
    fn test_deduce_multiple_params_same_type() {
        // template<typename T> T max(T a, T b);
        // max(1, 2); // T = int (consistent)
        let template = make_template(
            "max",
            vec!["T"],
            vec![
                ("a", CppType::template_param("T", 0, 0)),
                ("b", CppType::template_param("T", 0, 0)),
            ],
            CppType::template_param("T", 0, 0),
        );

        let arg_types = vec![CppType::Int { signed: true }, CppType::Int { signed: true }];
        let result = TypeDeducer::deduce(&template, &arg_types).unwrap();

        assert_eq!(result.get("T"), Some(&CppType::Int { signed: true }));
    }

    #[test]
    fn test_deduce_conflict() {
        // template<typename T> T max(T a, T b);
        // max(1, 3.14); // Error: T = int vs T = double
        let template = make_template(
            "max",
            vec!["T"],
            vec![
                ("a", CppType::template_param("T", 0, 0)),
                ("b", CppType::template_param("T", 0, 0)),
            ],
            CppType::template_param("T", 0, 0),
        );

        let arg_types = vec![CppType::Int { signed: true }, CppType::Double];
        let result = TypeDeducer::deduce(&template, &arg_types);

        assert!(matches!(result, Err(DeductionError::Conflict { .. })));
    }

    #[test]
    fn test_deduce_two_different_params() {
        // template<typename T, typename U> T convert(U x);
        // convert<int>(3.14); // T = explicit, U = double
        let template = make_template(
            "convert",
            vec!["T", "U"],
            vec![("x", CppType::template_param("U", 0, 1))],
            CppType::template_param("T", 0, 0),
        );

        let arg_types = vec![CppType::Double];
        let result = TypeDeducer::deduce(&template, &arg_types);

        // This should fail because T cannot be deduced from arguments
        assert!(matches!(
            result,
            Err(DeductionError::InsufficientArguments { .. })
        ));
    }

    #[test]
    fn test_deduce_pointer() {
        // template<typename T> void process(T* ptr);
        // int x; process(&x); // T = int
        let template = make_template(
            "process",
            vec!["T"],
            vec![(
                "ptr",
                CppType::Pointer {
                    pointee: Box::new(CppType::template_param("T", 0, 0)),
                    is_const: false,
                },
            )],
            CppType::Void,
        );

        let arg_types = vec![CppType::Pointer {
            pointee: Box::new(CppType::Int { signed: true }),
            is_const: false,
        }];
        let result = TypeDeducer::deduce(&template, &arg_types).unwrap();

        assert_eq!(result.get("T"), Some(&CppType::Int { signed: true }));
    }

    #[test]
    fn test_deduce_const_ref() {
        // template<typename T> void print(const T& x);
        // int x = 42; print(x); // T = int
        let template = make_template(
            "print",
            vec!["T"],
            vec![(
                "x",
                CppType::Reference {
                    referent: Box::new(CppType::template_param("T", 0, 0)),
                    is_const: true,
                    is_rvalue: false,
                },
            )],
            CppType::Void,
        );

        let arg_types = vec![CppType::Int { signed: true }];
        let result = TypeDeducer::deduce(&template, &arg_types).unwrap();

        assert_eq!(result.get("T"), Some(&CppType::Int { signed: true }));
    }
}
