//! Query overrides for C++ MIR injection.

use fragile_clang::{CppFunction, CppModule, MirBody};
use rustc_hash::FxHashMap;
use std::sync::RwLock;

/// Registry for C++ MIR bodies.
///
/// This stores the MIR bodies generated from C++ source files,
/// which are then injected into rustc via query overrides.
pub struct CppMirRegistry {
    /// Map from function name to MIR body
    functions: RwLock<FxHashMap<String, CppFunctionEntry>>,
    /// Map from struct name to struct info
    structs: RwLock<FxHashMap<String, CppStructEntry>>,
}

/// Entry for a C++ function in the registry.
#[derive(Debug, Clone)]
pub struct CppFunctionEntry {
    /// Mangled name for linking
    pub mangled_name: String,
    /// Display name for diagnostics
    pub display_name: String,
    /// The MIR body
    pub mir_body: MirBody,
    /// Whether this function has been referenced
    pub referenced: bool,
}

/// Entry for a C++ struct in the registry.
#[derive(Debug, Clone)]
pub struct CppStructEntry {
    /// Struct name
    pub name: String,
    /// Field names and types (as Rust type strings)
    pub fields: Vec<(String, String)>,
}

impl CppMirRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            functions: RwLock::new(FxHashMap::default()),
            structs: RwLock::new(FxHashMap::default()),
        }
    }

    /// Register all functions and structs from a C++ module.
    pub fn register_module(&self, module: &CppModule) {
        let mut functions = self.functions.write().unwrap();
        let mut structs = self.structs.write().unwrap();

        // Register functions
        for func in &module.functions {
            functions.insert(
                func.mangled_name.clone(),
                CppFunctionEntry {
                    mangled_name: func.mangled_name.clone(),
                    display_name: func.display_name.clone(),
                    mir_body: func.mir_body.clone(),
                    referenced: false,
                },
            );
        }

        // Register structs
        for struct_def in &module.structs {
            let fields: Vec<(String, String)> = struct_def
                .fields
                .iter()
                .map(|(name, ty)| (name.clone(), ty.to_rust_type_str()))
                .collect();

            structs.insert(
                struct_def.name.clone(),
                CppStructEntry {
                    name: struct_def.name.clone(),
                    fields,
                },
            );
        }
    }

    /// Get the MIR body for a function by name.
    pub fn get_mir(&self, name: &str) -> Option<MirBody> {
        let functions = self.functions.read().unwrap();
        functions.get(name).map(|entry| entry.mir_body.clone())
    }

    /// Check if a function is from C++.
    pub fn is_cpp_function(&self, name: &str) -> bool {
        let functions = self.functions.read().unwrap();
        functions.contains_key(name)
    }

    /// Get the number of registered functions.
    pub fn function_count(&self) -> usize {
        let functions = self.functions.read().unwrap();
        functions.len()
    }

    /// Get the number of registered structs.
    pub fn struct_count(&self) -> usize {
        let structs = self.structs.read().unwrap();
        structs.len()
    }

    /// Get all registered function names.
    pub fn function_names(&self) -> Vec<String> {
        let functions = self.functions.read().unwrap();
        functions.keys().cloned().collect()
    }

    /// Get all registered struct names.
    pub fn struct_names(&self) -> Vec<String> {
        let structs = self.structs.read().unwrap();
        structs.keys().cloned().collect()
    }

    /// Get struct info by name.
    pub fn get_struct(&self, name: &str) -> Option<CppStructEntry> {
        let structs = self.structs.read().unwrap();
        structs.get(name).cloned()
    }

    /// Mark a function as referenced.
    pub fn mark_referenced(&self, name: &str) {
        let mut functions = self.functions.write().unwrap();
        if let Some(entry) = functions.get_mut(name) {
            entry.referenced = true;
        }
    }

    /// Get all referenced functions.
    pub fn get_referenced_functions(&self) -> Vec<CppFunctionEntry> {
        let functions = self.functions.read().unwrap();
        functions
            .values()
            .filter(|entry| entry.referenced)
            .cloned()
            .collect()
    }
}

impl Default for CppMirRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fragile_clang::{CppModule, CppFunction, MirBody};
    use fragile_clang::CppType;

    #[test]
    fn test_registry_register_and_lookup() {
        let registry = CppMirRegistry::new();

        let mut module = CppModule::new();
        module.functions.push(CppFunction {
            mangled_name: "add".to_string(),
            display_name: "add".to_string(),
            namespace: Vec::new(),
            params: vec![
                ("a".to_string(), CppType::int()),
                ("b".to_string(), CppType::int()),
            ],
            return_type: CppType::int(),
            mir_body: MirBody::new(),
        });

        registry.register_module(&module);

        assert_eq!(registry.function_count(), 1);
        assert!(registry.is_cpp_function("add"));
        assert!(!registry.is_cpp_function("sub"));
    }
}
