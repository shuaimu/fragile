//! Name resolution for C++ namespace lookups.
//!
//! This module implements C++ name lookup rules, including:
//! - Unqualified lookup (searching current and enclosing scopes)
//! - Using directive resolution (importing namespaces)
//! - Using declaration resolution (importing specific names)

use crate::{CppFunction, CppModule, CppStruct, UsingDeclaration, UsingDirective};
use rustc_hash::FxHashMap;

/// Resolves unqualified names to their fully qualified forms.
///
/// Uses C++ lookup rules:
/// 1. Search current scope
/// 2. Search enclosing scopes (walk up namespace hierarchy)
/// 3. Search namespaces imported via `using namespace`
/// 4. Search names imported via `using` declaration
pub struct NameResolver<'a> {
    /// Functions indexed by fully qualified name
    functions: FxHashMap<Vec<String>, Vec<&'a CppFunction>>,
    /// Structs indexed by fully qualified name
    structs: FxHashMap<Vec<String>, &'a CppStruct>,
    /// Using namespace directives
    using_directives: &'a [UsingDirective],
    /// Using declarations
    using_declarations: &'a [UsingDeclaration],
}

impl<'a> NameResolver<'a> {
    /// Create a new name resolver from a C++ module.
    pub fn new(module: &'a CppModule) -> Self {
        let mut functions: FxHashMap<Vec<String>, Vec<&'a CppFunction>> = FxHashMap::default();
        let mut structs: FxHashMap<Vec<String>, &'a CppStruct> = FxHashMap::default();

        // Index all functions by their qualified name
        for func in &module.functions {
            let mut qualified = func.namespace.clone();
            qualified.push(func.display_name.clone());
            functions.entry(qualified).or_default().push(func);
        }

        // Index extern declarations too
        for ext in &module.externs {
            let mut qualified = ext.namespace.clone();
            qualified.push(ext.display_name.clone());
            // Create a temporary wrapper - we don't actually store it but record the path
            // For externs we just track that the name exists
            functions.entry(qualified).or_default();
        }

        // Index all structs by their qualified name
        for st in &module.structs {
            let mut qualified = st.namespace.clone();
            qualified.push(st.name.clone());
            structs.insert(qualified, st);
        }

        Self {
            functions,
            structs,
            using_directives: &module.using_directives,
            using_declarations: &module.using_declarations,
        }
    }

    /// Resolve an unqualified function name to its fully qualified form.
    ///
    /// # Arguments
    /// * `name` - The unqualified function name (e.g., "helper")
    /// * `scope` - The current scope as a namespace path (e.g., ["foo", "bar"])
    ///
    /// # Returns
    /// The fully qualified name if found (e.g., ["foo", "bar", "helper"]), or None.
    pub fn resolve_function(&self, name: &str, scope: &[String]) -> Option<Vec<String>> {
        // Handle already-qualified names (contains ::)
        if name.contains("::") {
            let parts: Vec<String> = name.split("::").map(String::from).collect();
            if self.functions.contains_key(&parts) {
                return Some(parts);
            }
            // Try resolving relative to scope
            let mut qualified = scope.to_vec();
            qualified.extend(parts);
            if self.functions.contains_key(&qualified) {
                return Some(qualified);
            }
        }

        // 1. Search current scope and all enclosing scopes
        let mut search_scope = scope.to_vec();
        loop {
            let mut candidate = search_scope.clone();
            candidate.push(name.to_string());
            if self.functions.contains_key(&candidate) {
                return Some(candidate);
            }

            if search_scope.is_empty() {
                break;
            }
            search_scope.pop();
        }

        // 2. Search via using namespace directives applicable to this scope
        for directive in self.using_directives {
            // Check if directive is visible from current scope
            if self.is_scope_visible(&directive.scope, scope) {
                let mut candidate = directive.namespace.clone();
                candidate.push(name.to_string());
                if self.functions.contains_key(&candidate) {
                    return Some(candidate);
                }
            }
        }

        // 3. Search via using declarations
        for decl in self.using_declarations {
            // Check if declaration is visible from current scope
            if self.is_scope_visible(&decl.scope, scope) {
                // Check if the last component matches the name
                if let Some(last) = decl.qualified_name.last() {
                    if last == name {
                        if self.functions.contains_key(&decl.qualified_name) {
                            return Some(decl.qualified_name.clone());
                        }
                    }
                }
            }
        }

        None
    }

    /// Resolve an unqualified type name to its fully qualified form.
    ///
    /// Similar to function resolution but for struct/class types.
    pub fn resolve_type(&self, name: &str, scope: &[String]) -> Option<Vec<String>> {
        // Handle already-qualified names
        if name.contains("::") {
            let parts: Vec<String> = name.split("::").map(String::from).collect();
            if self.structs.contains_key(&parts) {
                return Some(parts);
            }
        }

        // 1. Search current scope and enclosing scopes
        let mut search_scope = scope.to_vec();
        loop {
            let mut candidate = search_scope.clone();
            candidate.push(name.to_string());
            if self.structs.contains_key(&candidate) {
                return Some(candidate);
            }

            if search_scope.is_empty() {
                break;
            }
            search_scope.pop();
        }

        // 2. Search via using namespace directives
        for directive in self.using_directives {
            if self.is_scope_visible(&directive.scope, scope) {
                let mut candidate = directive.namespace.clone();
                candidate.push(name.to_string());
                if self.structs.contains_key(&candidate) {
                    return Some(candidate);
                }
            }
        }

        // 3. Search via using declarations
        for decl in self.using_declarations {
            if self.is_scope_visible(&decl.scope, scope) {
                if let Some(last) = decl.qualified_name.last() {
                    if last == name && self.structs.contains_key(&decl.qualified_name) {
                        return Some(decl.qualified_name.clone());
                    }
                }
            }
        }

        None
    }

    /// Check if a using directive/declaration scope is visible from the current scope.
    ///
    /// A directive in scope `[A, B]` is visible from `[A, B, C]` (nested deeper)
    /// and from `[A, B]` (same scope), but not from `[A]` (enclosing scope).
    fn is_scope_visible(&self, directive_scope: &[String], current_scope: &[String]) -> bool {
        // The directive must be in the same scope or an enclosing scope
        if directive_scope.len() > current_scope.len() {
            return false;
        }
        directive_scope
            .iter()
            .zip(current_scope.iter())
            .all(|(d, c)| d == c)
    }

    /// Format a qualified name as a string with :: separators.
    pub fn format_qualified_name(qualified: &[String]) -> String {
        qualified.join("::")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CppType, MirBody};

    fn make_function(name: &str, namespace: Vec<String>) -> CppFunction {
        CppFunction {
            mangled_name: name.to_string(),
            display_name: name.to_string(),
            namespace,
            params: vec![],
            return_type: CppType::int(),
            mir_body: MirBody::new(),
        }
    }

    fn make_struct(name: &str, namespace: Vec<String>) -> CppStruct {
        CppStruct {
            name: name.to_string(),
            is_class: false,
            namespace,
            bases: vec![],
            fields: vec![],
            static_fields: vec![],
            constructors: vec![],
            destructor: None,
            methods: vec![],
            friends: vec![],
        }
    }

    #[test]
    fn test_resolve_same_namespace() {
        // namespace foo { int helper(); int main() { return helper(); } }
        let mut module = CppModule::new();
        module.functions.push(make_function("helper", vec!["foo".into()]));
        module.functions.push(make_function("main", vec!["foo".into()]));

        let resolver = NameResolver::new(&module);

        // Resolve "helper" from scope ["foo"]
        let result = resolver.resolve_function("helper", &["foo".into()]);
        assert_eq!(result, Some(vec!["foo".into(), "helper".into()]));
    }

    #[test]
    fn test_resolve_global_from_namespace() {
        // int global_fn();
        // namespace foo { int main() { return global_fn(); } }
        let mut module = CppModule::new();
        module.functions.push(make_function("global_fn", vec![]));
        module.functions.push(make_function("main", vec!["foo".into()]));

        let resolver = NameResolver::new(&module);

        // Resolve "global_fn" from scope ["foo"] - should find it at global scope
        let result = resolver.resolve_function("global_fn", &["foo".into()]);
        assert_eq!(result, Some(vec!["global_fn".into()]));
    }

    #[test]
    fn test_resolve_via_using_namespace() {
        // namespace bar { int helper(); }
        // using namespace bar;
        // int main() { return helper(); }
        let mut module = CppModule::new();
        module.functions.push(make_function("helper", vec!["bar".into()]));
        module.functions.push(make_function("main", vec![]));
        module.using_directives.push(UsingDirective {
            namespace: vec!["bar".into()],
            scope: vec![], // At global scope
        });

        let resolver = NameResolver::new(&module);

        // Resolve "helper" from global scope - should find it via using directive
        let result = resolver.resolve_function("helper", &[]);
        assert_eq!(result, Some(vec!["bar".into(), "helper".into()]));
    }

    #[test]
    fn test_resolve_via_using_declaration() {
        // namespace bar { int helper(); }
        // using bar::helper;
        // int main() { return helper(); }
        let mut module = CppModule::new();
        module.functions.push(make_function("helper", vec!["bar".into()]));
        module.functions.push(make_function("main", vec![]));
        module.using_declarations.push(UsingDeclaration {
            qualified_name: vec!["bar".into(), "helper".into()],
            scope: vec![],
        });

        let resolver = NameResolver::new(&module);

        // Resolve "helper" from global scope
        let result = resolver.resolve_function("helper", &[]);
        assert_eq!(result, Some(vec!["bar".into(), "helper".into()]));
    }

    #[test]
    fn test_resolve_nested_namespace() {
        // namespace outer { namespace inner { int func(); } }
        // namespace outer { int main() { return inner::func(); } }
        let mut module = CppModule::new();
        module.functions.push(make_function(
            "func",
            vec!["outer".into(), "inner".into()],
        ));
        module.functions.push(make_function("main", vec!["outer".into()]));

        let resolver = NameResolver::new(&module);

        // Resolve "inner::func" from scope ["outer"]
        let result = resolver.resolve_function("inner::func", &["outer".into()]);
        assert_eq!(
            result,
            Some(vec!["outer".into(), "inner".into(), "func".into()])
        );
    }

    #[test]
    fn test_resolve_type() {
        // namespace foo { struct Bar {}; }
        // using namespace foo;
        // Bar make_bar();  // Should resolve Bar to foo::Bar
        let mut module = CppModule::new();
        module.structs.push(make_struct("Bar", vec!["foo".into()]));
        module.using_directives.push(UsingDirective {
            namespace: vec!["foo".into()],
            scope: vec![],
        });

        let resolver = NameResolver::new(&module);

        let result = resolver.resolve_type("Bar", &[]);
        assert_eq!(result, Some(vec!["foo".into(), "Bar".into()]));
    }

    #[test]
    fn test_using_directive_scope_visibility() {
        // namespace outer {
        //     using namespace bar;  // Only visible inside outer
        //     int func();
        // }
        // int main() { return bar_func(); }  // Should NOT find bar_func via outer's using
        let mut module = CppModule::new();
        module.functions.push(make_function("bar_func", vec!["bar".into()]));
        module.functions.push(make_function("main", vec![]));
        module.using_directives.push(UsingDirective {
            namespace: vec!["bar".into()],
            scope: vec!["outer".into()], // Using directive is inside outer
        });

        let resolver = NameResolver::new(&module);

        // From global scope, the using directive in "outer" should not be visible
        let result = resolver.resolve_function("bar_func", &[]);
        assert_eq!(result, None);

        // From inside "outer", the using directive should be visible
        let result = resolver.resolve_function("bar_func", &["outer".into()]);
        assert_eq!(result, Some(vec!["bar".into(), "bar_func".into()]));
    }

    #[test]
    fn test_local_shadows_using() {
        // namespace bar { int helper(); }
        // namespace foo {
        //     using namespace bar;
        //     int helper();  // This shadows bar::helper
        //     int main() { return helper(); }
        // }
        let mut module = CppModule::new();
        module.functions.push(make_function("helper", vec!["bar".into()]));
        module
            .functions
            .push(make_function("helper", vec!["foo".into()]));
        module.functions.push(make_function("main", vec!["foo".into()]));
        module.using_directives.push(UsingDirective {
            namespace: vec!["bar".into()],
            scope: vec!["foo".into()],
        });

        let resolver = NameResolver::new(&module);

        // Local foo::helper should shadow bar::helper
        let result = resolver.resolve_function("helper", &["foo".into()]);
        assert_eq!(result, Some(vec!["foo".into(), "helper".into()]));
    }

    #[test]
    fn test_format_qualified_name() {
        assert_eq!(
            NameResolver::format_qualified_name(&["foo".into(), "bar".into(), "baz".into()]),
            "foo::bar::baz"
        );
        assert_eq!(NameResolver::format_qualified_name(&["single".into()]), "single");
        assert_eq!(NameResolver::format_qualified_name(&[]), "");
    }
}
