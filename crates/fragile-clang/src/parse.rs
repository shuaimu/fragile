//! Clang AST parsing using libclang.

use crate::ast::{
    AccessSpecifier, BinaryOp, CaptureDefault, CastKind, ClangAst, ClangNode, ClangNodeKind,
    ConstructorKind, CoroutineInfo, CoroutineKind, Requirement, SourceLocation, UnaryOp,
};
use crate::types::CppType;
use miette::{miette, Result};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;

/// Parser that uses libclang to parse C++ source files.
pub struct ClangParser {
    index: clang_sys::CXIndex,
    /// Additional include paths for header files (searched with -I)
    include_paths: Vec<String>,
    /// System include paths (searched with -isystem for angle-bracket includes)
    system_include_paths: Vec<String>,
    /// Preprocessor defines (-D flags)
    defines: Vec<String>,
    /// Error patterns to ignore (substring matches)
    ignored_error_patterns: Vec<String>,
    /// Use libc++ (LLVM's C++ standard library) instead of libstdc++
    use_libcxx: bool,
}

impl ClangParser {
    /// Create a new Clang parser with default settings.
    pub fn new() -> Result<Self> {
        Self::with_include_paths(Vec::new())
    }

    /// Create a new Clang parser with custom include paths.
    pub fn with_include_paths(include_paths: Vec<String>) -> Result<Self> {
        Self::with_paths(include_paths, Vec::new())
    }

    /// Create a new Clang parser with both regular and system include paths.
    /// System paths use -isystem and are searched for angle-bracket includes (<...>).
    /// Regular paths use -I and are searched for quoted includes ("...").
    pub fn with_paths(
        include_paths: Vec<String>,
        system_include_paths: Vec<String>,
    ) -> Result<Self> {
        Self::with_paths_and_defines(include_paths, system_include_paths, Vec::new())
    }

    /// Create a new Clang parser with include paths and preprocessor defines.
    /// Defines should be in the form "NAME" or "NAME=VALUE".
    pub fn with_paths_and_defines(
        include_paths: Vec<String>,
        system_include_paths: Vec<String>,
        defines: Vec<String>,
    ) -> Result<Self> {
        Self::with_paths_defines_and_ignored_errors(
            include_paths,
            system_include_paths,
            defines,
            Vec::new(),
        )
    }

    /// Create a new Clang parser with include paths, preprocessor defines, and error patterns to ignore.
    /// Ignored error patterns are substring matches against error messages.
    pub fn with_paths_defines_and_ignored_errors(
        include_paths: Vec<String>,
        system_include_paths: Vec<String>,
        defines: Vec<String>,
        ignored_error_patterns: Vec<String>,
    ) -> Result<Self> {
        Self::with_full_options(
            include_paths,
            system_include_paths,
            defines,
            ignored_error_patterns,
            false,
        )
    }

    /// Create a new Clang parser with all options including libc++ support.
    /// When `use_libcxx` is true, the parser will use LLVM's libc++ standard library
    /// instead of GCC's libstdc++. This requires libc++ to be installed
    /// (e.g., `apt install libc++-dev libc++abi-dev` on Debian/Ubuntu).
    pub fn with_full_options(
        include_paths: Vec<String>,
        system_include_paths: Vec<String>,
        defines: Vec<String>,
        ignored_error_patterns: Vec<String>,
        use_libcxx: bool,
    ) -> Result<Self> {
        unsafe {
            let index = clang_sys::clang_createIndex(0, 0);
            if index.is_null() {
                return Err(miette!("Failed to create clang index"));
            }
            Ok(Self {
                index,
                include_paths,
                system_include_paths,
                defines,
                ignored_error_patterns,
                use_libcxx,
            })
        }
    }

    /// Create a Clang parser with system C++ standard library include paths.
    /// This enables parsing code that includes headers like `<vector>`, `<string>`.
    pub fn with_system_includes() -> Result<Self> {
        // Common include paths for C++ standard library
        let system_paths = Self::detect_system_include_paths();
        Self::with_include_paths(system_paths)
    }

    /// Create a Clang parser configured to use libc++ (LLVM's C++ standard library).
    /// This is recommended for transpiling STL code as libc++ has cleaner, more
    /// transpiler-friendly code than libstdc++.
    ///
    /// Requires libc++ to be installed:
    /// - Debian/Ubuntu: `apt install libc++-dev libc++abi-dev`
    /// - Other systems: Install LLVM's C++ standard library package
    pub fn with_libcxx() -> Result<Self> {
        let system_paths = Self::detect_libcxx_include_paths();
        Self::with_full_options(Vec::new(), system_paths, Vec::new(), Vec::new(), true)
    }

    /// Create a Clang parser with libc++ and custom include paths.
    pub fn with_libcxx_and_paths(include_paths: Vec<String>) -> Result<Self> {
        let system_paths = Self::detect_libcxx_include_paths();
        Self::with_full_options(include_paths, system_paths, Vec::new(), Vec::new(), true)
    }

    /// Detect system C++ include paths by querying clang (libstdc++ paths).
    fn detect_system_include_paths() -> Vec<String> {
        // Common paths for libstdc++ (GCC)
        let possible_paths = vec![
            // GCC libstdc++ paths (common on Linux)
            "/usr/include/c++/14".to_string(),
            "/usr/include/c++/13".to_string(),
            "/usr/include/c++/12".to_string(),
            "/usr/include/c++/11".to_string(),
            "/usr/lib/gcc/x86_64-linux-gnu/14/include".to_string(),
            "/usr/lib/gcc/x86_64-linux-gnu/13/include".to_string(),
            // Platform-specific includes
            "/usr/include/x86_64-linux-gnu/c++/14".to_string(),
            "/usr/include/x86_64-linux-gnu/c++/13".to_string(),
            "/usr/include/x86_64-linux-gnu".to_string(),
            // Standard includes
            "/usr/include".to_string(),
            "/usr/local/include".to_string(),
            // LLVM/Clang includes
            "/usr/lib/llvm-19/lib/clang/19/include".to_string(),
            "/usr/lib/llvm-18/lib/clang/18/include".to_string(),
        ];

        // Filter to only existing paths
        possible_paths
            .into_iter()
            .filter(|p| std::path::Path::new(p).exists())
            .collect()
    }

    /// Detect libc++ (LLVM's C++ standard library) include paths.
    /// Returns paths where libc++ headers are installed.
    /// Requires: `apt install libc++-dev libc++abi-dev` on Debian/Ubuntu.
    pub fn detect_libcxx_include_paths() -> Vec<String> {
        let possible_paths = vec![
            // Standard libc++ location
            "/usr/include/c++/v1".to_string(),
            // Versioned LLVM installations
            "/usr/lib/llvm-19/include/c++/v1".to_string(),
            "/usr/lib/llvm-18/include/c++/v1".to_string(),
            "/usr/lib/llvm-17/include/c++/v1".to_string(),
            // Alternative locations
            "/usr/local/include/c++/v1".to_string(),
            // Platform-specific
            "/usr/include/x86_64-linux-gnu".to_string(),
            // Standard includes
            "/usr/include".to_string(),
            "/usr/local/include".to_string(),
            // LLVM/Clang builtin includes (needed for some intrinsics)
            "/usr/lib/llvm-19/lib/clang/19/include".to_string(),
            "/usr/lib/llvm-18/lib/clang/18/include".to_string(),
        ];

        // Filter to only existing paths
        possible_paths
            .into_iter()
            .filter(|p| std::path::Path::new(p).exists())
            .collect()
    }

    /// Check if libc++ headers are available on this system.
    pub fn is_libcxx_available() -> bool {
        std::path::Path::new("/usr/include/c++/v1").exists()
            || std::path::Path::new("/usr/lib/llvm-19/include/c++/v1").exists()
            || std::path::Path::new("/usr/lib/llvm-18/include/c++/v1").exists()
    }

    /// Detect vendored libc++ include path.
    /// Looks for vendor/llvm-project/libcxx/include/ relative to:
    /// 1. Environment variable FRAGILE_ROOT
    /// 2. Current working directory
    /// 3. Executable's parent directories (up to 5 levels)
    pub fn detect_vendored_libcxx_path() -> Option<String> {
        let vendored_subpath = "vendor/llvm-project/libcxx/include";

        // Try FRAGILE_ROOT environment variable
        if let Ok(root) = std::env::var("FRAGILE_ROOT") {
            let path = Path::new(&root).join(vendored_subpath);
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }

        // Try current working directory
        if let Ok(cwd) = std::env::current_dir() {
            let path = cwd.join(vendored_subpath);
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }

        // Try executable's parent directories (up to 5 levels)
        if let Ok(exe) = std::env::current_exe() {
            let mut dir = exe.parent().map(|p| p.to_path_buf());
            for _ in 0..5 {
                if let Some(ref parent) = dir {
                    let path = parent.join(vendored_subpath);
                    if path.exists() {
                        return Some(path.to_string_lossy().to_string());
                    }
                    dir = parent.parent().map(|p| p.to_path_buf());
                } else {
                    break;
                }
            }
        }

        None
    }

    /// Check if vendored libc++ is available.
    pub fn is_vendored_libcxx_available() -> bool {
        Self::detect_vendored_libcxx_path().is_some()
    }

    /// Create a Clang parser configured to use vendored libc++ from
    /// `vendor/llvm-project/libcxx/include/`. This uses the libc++ source
    /// code bundled with the fragile project instead of system-installed libc++.
    ///
    /// The vendored path is detected by looking for the directory relative to:
    /// 1. FRAGILE_ROOT environment variable
    /// 2. Current working directory
    /// 3. Executable's parent directories
    pub fn with_vendored_libcxx() -> Result<Self> {
        let vendored_path = Self::detect_vendored_libcxx_path().ok_or_else(|| {
            miette!(
                "Vendored libc++ not found. Expected at vendor/llvm-project/libcxx/include/\n\
                 Set FRAGILE_ROOT environment variable or run from the fragile project root."
            )
        })?;

        let system_paths = vec![vendored_path];
        // Add defines for libc++ compatibility
        let defines = vec!["_LIBCPP_HAS_NO_PRAGMA_SYSTEM_HEADER".to_string()];

        Self::with_full_options(Vec::new(), system_paths, defines, Vec::new(), true)
    }

    /// Create a Clang parser with vendored libc++ and custom include paths.
    pub fn with_vendored_libcxx_and_paths(include_paths: Vec<String>) -> Result<Self> {
        let vendored_path = Self::detect_vendored_libcxx_path().ok_or_else(|| {
            miette!(
                "Vendored libc++ not found. Expected at vendor/llvm-project/libcxx/include/\n\
                 Set FRAGILE_ROOT environment variable or run from the fragile project root."
            )
        })?;

        let system_paths = vec![vendored_path];
        let defines = vec!["_LIBCPP_HAS_NO_PRAGMA_SYSTEM_HEADER".to_string()];

        Self::with_full_options(include_paths, system_paths, defines, Vec::new(), true)
    }

    /// Build compiler arguments including include paths.
    fn build_compiler_args(&self) -> Vec<CString> {
        let mut args = vec![
            CString::new("-x").unwrap(),
            CString::new("c++").unwrap(),
            CString::new("-std=c++20").unwrap(),
            // Suppress some warnings that may cause issues with system headers
            CString::new("-w").unwrap(),
            // Don't limit the number of errors
            CString::new("-ferror-limit=0").unwrap(),
            // Disable builtin limits on stack depth for templates
            CString::new("-ftemplate-depth=1024").unwrap(),
        ];

        // Use libc++ if requested (LLVM's C++ standard library)
        // This enables cleaner transpilation of STL code compared to libstdc++
        if self.use_libcxx {
            args.push(CString::new("-stdlib=libc++").unwrap());
        }

        // If we have system include paths configured, disable the default C++ includes
        // so our stubs are used instead of system headers
        if !self.system_include_paths.is_empty() {
            args.push(CString::new("-nostdinc++").unwrap());
        }

        // Add system include paths first (-isystem for angle-bracket includes)
        for path in &self.system_include_paths {
            args.push(CString::new("-isystem").unwrap());
            args.push(CString::new(path.as_str()).unwrap());
        }

        // Add user include paths (-I for quoted includes)
        for path in &self.include_paths {
            args.push(CString::new(format!("-I{}", path)).unwrap());
        }

        // Add preprocessor defines (-D flags)
        for define in &self.defines {
            args.push(CString::new(format!("-D{}", define)).unwrap());
        }

        args
    }

    /// Parse a C++ source file into a Clang AST.
    pub fn parse_file(&self, path: &Path) -> Result<ClangAst> {
        let path_str = path.to_string_lossy();
        let c_path =
            CString::new(path_str.as_ref()).map_err(|_| miette!("Invalid path: {}", path_str))?;

        // Compiler arguments including include paths
        let args = self.build_compiler_args();
        let c_args: Vec<*const i8> = args.iter().map(|s| s.as_ptr()).collect();

        unsafe {
            // Use DetailedPreprocessingRecord to get better AST coverage,
            // and KeepGoing to continue past errors in system headers
            let options = clang_sys::CXTranslationUnit_DetailedPreprocessingRecord
                | clang_sys::CXTranslationUnit_KeepGoing;

            let tu = clang_sys::clang_parseTranslationUnit(
                self.index,
                c_path.as_ptr(),
                c_args.as_ptr(),
                c_args.len() as i32,
                ptr::null_mut(),
                0,
                options,
            );

            if tu.is_null() {
                return Err(miette!("Failed to parse file: {}", path_str));
            }

            // Check for errors - only fail on errors in user code, not system headers
            let num_diagnostics = clang_sys::clang_getNumDiagnostics(tu);
            let mut user_errors = Vec::new();

            for i in 0..num_diagnostics {
                let diag = clang_sys::clang_getDiagnostic(tu, i);
                let severity = clang_sys::clang_getDiagnosticSeverity(diag);

                if severity >= clang_sys::CXDiagnostic_Error {
                    let spelling = clang_sys::clang_getDiagnosticSpelling(diag);
                    let msg = cx_string_to_string(spelling);

                    // Get location for better error reporting
                    let location = clang_sys::clang_getDiagnosticLocation(diag);
                    let mut file: clang_sys::CXFile = ptr::null_mut();
                    let mut line: u32 = 0;
                    let mut column: u32 = 0;
                    let mut offset: u32 = 0;
                    clang_sys::clang_getExpansionLocation(
                        location,
                        &mut file,
                        &mut line,
                        &mut column,
                        &mut offset,
                    );

                    let file_name = if !file.is_null() {
                        let fname = clang_sys::clang_getFileName(file);
                        cx_string_to_string(fname)
                    } else {
                        String::from("<unknown>")
                    };

                    // Check if this error is from a system header
                    let is_system_header =
                        clang_sys::clang_Location_isInSystemHeader(location) != 0;

                    // Check if this error matches any ignored pattern
                    let is_ignored = self
                        .ignored_error_patterns
                        .iter()
                        .any(|pattern| msg.contains(pattern));

                    if !is_system_header && !is_ignored {
                        user_errors.push(format!("{}:{}:{}: {}", file_name, line, column, msg));
                    }
                }
                clang_sys::clang_disposeDiagnostic(diag);
            }

            // Only fail if there are errors in user code
            if !user_errors.is_empty() {
                clang_sys::clang_disposeTranslationUnit(tu);
                return Err(miette!(
                    "Clang errors in user code:\n{}",
                    user_errors.join("\n")
                ));
            }

            // Get the cursor for the translation unit
            let cursor = clang_sys::clang_getTranslationUnitCursor(tu);

            // Convert to our AST representation
            let root = self.convert_cursor(cursor);

            clang_sys::clang_disposeTranslationUnit(tu);

            Ok(ClangAst {
                translation_unit: root,
            })
        }
    }

    /// Parse C++ source code from a string.
    pub fn parse_string(&self, source: &str, filename: &str) -> Result<ClangAst> {
        let c_filename = CString::new(filename).unwrap();
        let c_source = CString::new(source).unwrap();

        // Create an unsaved file
        let unsaved_file = clang_sys::CXUnsavedFile {
            Filename: c_filename.as_ptr(),
            Contents: c_source.as_ptr(),
            Length: source.len() as u64,
        };

        // Compiler arguments including include paths
        let args = self.build_compiler_args();
        let c_args: Vec<*const i8> = args.iter().map(|s| s.as_ptr()).collect();

        unsafe {
            let tu = clang_sys::clang_parseTranslationUnit(
                self.index,
                c_filename.as_ptr(),
                c_args.as_ptr(),
                c_args.len() as i32,
                &unsaved_file as *const _ as *mut _,
                1,
                clang_sys::CXTranslationUnit_None,
            );

            if tu.is_null() {
                return Err(miette!("Failed to parse source code"));
            }

            let cursor = clang_sys::clang_getTranslationUnitCursor(tu);
            let root = self.convert_cursor(cursor);

            clang_sys::clang_disposeTranslationUnit(tu);

            Ok(ClangAst {
                translation_unit: root,
            })
        }
    }

    /// Convert a Clang cursor to our AST node.
    fn convert_cursor(&self, cursor: clang_sys::CXCursor) -> ClangNode {
        unsafe {
            let kind = clang_sys::clang_getCursorKind(cursor);
            let location = self.get_location(cursor);

            let node_kind = self.convert_cursor_kind(cursor, kind);

            // Get children
            let mut children = Vec::new();

            // Create a context struct to pass both parser and children to visitor
            struct VisitorContext<'a> {
                parser: &'a ClangParser,
                children: &'a mut Vec<ClangNode>,
            }

            let mut ctx = VisitorContext {
                parser: self,
                children: &mut children,
            };
            let ctx_ptr: *mut VisitorContext = &mut ctx;

            extern "C" fn visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let ctx = &mut *(data as *mut VisitorContext);

                    // Skip null cursors
                    if clang_sys::clang_Cursor_isNull(child) != 0 {
                        return clang_sys::CXChildVisit_Continue;
                    }

                    ctx.children.push(ctx.parser.convert_cursor(child));
                    clang_sys::CXChildVisit_Continue
                }
            }

            // Visit children
            clang_sys::clang_visitChildren(cursor, visitor, ctx_ptr as clang_sys::CXClientData);

            ClangNode {
                kind: node_kind,
                children,
                location,
            }
        }
    }

    /// Get source location from cursor.
    fn get_location(&self, cursor: clang_sys::CXCursor) -> SourceLocation {
        unsafe {
            let loc = clang_sys::clang_getCursorLocation(cursor);
            let mut file: clang_sys::CXFile = ptr::null_mut();
            let mut line: u32 = 0;
            let mut column: u32 = 0;

            clang_sys::clang_getSpellingLocation(
                loc,
                &mut file,
                &mut line,
                &mut column,
                ptr::null_mut(),
            );

            let file_name = if !file.is_null() {
                let name = clang_sys::clang_getFileName(file);
                Some(cx_string_to_string(name))
            } else {
                None
            };

            SourceLocation {
                file: file_name,
                line,
                column,
            }
        }
    }

    /// Check if a function declaration has the noexcept specifier.
    /// Uses token-based detection to find 'noexcept' in the function signature.
    fn is_function_noexcept(&self, cursor: clang_sys::CXCursor) -> bool {
        unsafe {
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            let mut has_noexcept = false;
            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let spelling = clang_sys::clang_getTokenSpelling(tu, token);
                let token_str = cx_string_to_string(spelling);
                if token_str == "noexcept" {
                    has_noexcept = true;
                    break;
                }
            }

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            has_noexcept
        }
    }

    /// Check if a member reference expression uses arrow (->) or dot (.) access.
    fn is_arrow_access(&self, cursor: clang_sys::CXCursor) -> bool {
        unsafe {
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            let mut is_arrow = false;
            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let spelling = clang_sys::clang_getTokenSpelling(tu, token);
                let token_str = cx_string_to_string(spelling);
                if token_str == "->" {
                    is_arrow = true;
                    break;
                }
            }

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            is_arrow
        }
    }

    /// Get the declaring class name for a member expression.
    /// This is used to detect inherited member access.
    fn get_member_declaring_class(&self, cursor: clang_sys::CXCursor) -> Option<String> {
        unsafe {
            // Get the referenced cursor (the FieldDecl or CXXMethodDecl being accessed)
            let referenced = clang_sys::clang_getCursorReferenced(cursor);
            if clang_sys::clang_Cursor_isNull(referenced) != 0 {
                return None;
            }

            // Get the semantic parent (the class that declares this member)
            let parent = clang_sys::clang_getCursorSemanticParent(referenced);
            if clang_sys::clang_Cursor_isNull(parent) != 0 {
                return None;
            }

            // Check if parent is a struct/class
            let kind = clang_sys::clang_getCursorKind(parent);
            if kind == clang_sys::CXCursor_StructDecl || kind == clang_sys::CXCursor_ClassDecl {
                let name = cursor_spelling(parent);
                if !name.is_empty() {
                    return Some(name);
                }
            }

            None
        }
    }

    /// Check if a member expression references a static member.
    fn is_static_member(&self, cursor: clang_sys::CXCursor) -> bool {
        unsafe {
            // Get the referenced cursor (the FieldDecl or CXXMethodDecl being accessed)
            let referenced = clang_sys::clang_getCursorReferenced(cursor);
            if clang_sys::clang_Cursor_isNull(referenced) != 0 {
                return false;
            }

            // Check the storage class of the referenced member
            // CX_SC_Static = 3
            let storage = clang_sys::clang_Cursor_getStorageClass(referenced);
            storage == 3
        }
    }

    /// Get the namespace path for a referenced declaration.
    /// Returns a vector of namespace names from outermost to innermost.
    /// Returns empty for local variables, parameters, and other function-scoped declarations.
    fn get_namespace_path(&self, cursor: clang_sys::CXCursor) -> Vec<String> {
        unsafe {
            let mut path = Vec::new();

            // Get the referenced cursor (the actual declaration being referenced)
            let referenced = clang_sys::clang_getCursorReferenced(cursor);
            if clang_sys::clang_Cursor_isNull(referenced) != 0 {
                return path;
            }

            // Check if this is a local declaration (parameter, local variable)
            // Local declarations have a function as their immediate semantic parent
            let direct_parent = clang_sys::clang_getCursorSemanticParent(referenced);
            if clang_sys::clang_Cursor_isNull(direct_parent) == 0 {
                let parent_kind = clang_sys::clang_getCursorKind(direct_parent);
                // If the direct parent is a function or method, this is a local variable/parameter
                if parent_kind == clang_sys::CXCursor_FunctionDecl
                    || parent_kind == clang_sys::CXCursor_CXXMethod
                    || parent_kind == clang_sys::CXCursor_Constructor
                    || parent_kind == clang_sys::CXCursor_Destructor
                {
                    // Local variables don't need namespace qualification
                    return path;
                }
            }

            // Traverse up through semantic parents to collect namespace names
            let mut current = direct_parent;
            while clang_sys::clang_Cursor_isNull(current) == 0 {
                let kind = clang_sys::clang_getCursorKind(current);

                if kind == clang_sys::CXCursor_Namespace {
                    let name = cursor_spelling(current);
                    // Skip anonymous namespaces, std, and internal namespaces
                    if !name.is_empty() && !name.starts_with("__") && name != "std" {
                        path.push(name);
                    }
                } else if kind == clang_sys::CXCursor_EnumDecl {
                    // For enum constants, include the enum type name for scoped access
                    let name = cursor_spelling(current);
                    if !name.is_empty() {
                        path.push(name);
                    }
                } else if kind == clang_sys::CXCursor_ClassDecl
                    || kind == clang_sys::CXCursor_StructDecl
                {
                    // For static members/methods, include the class name for qualified access
                    // Check if the referenced item is static
                    let ref_kind = clang_sys::clang_getCursorKind(referenced);
                    let is_static = if ref_kind == clang_sys::CXCursor_CXXMethod
                        || ref_kind == clang_sys::CXCursor_FieldDecl
                        || ref_kind == clang_sys::CXCursor_VarDecl
                    {
                        // CX_SC_Static = 3
                        let storage = clang_sys::clang_Cursor_getStorageClass(referenced);
                        storage == 3
                    } else {
                        false
                    };
                    if is_static {
                        let name = cursor_spelling(current);
                        if !name.is_empty() {
                            path.push(name);
                        }
                    }
                } else if kind == clang_sys::CXCursor_TranslationUnit {
                    // Reached the top
                    break;
                }

                current = clang_sys::clang_getCursorSemanticParent(current);
            }

            // Reverse to get outermost namespace first
            path.reverse();
            path
        }
    }

    /// Convert a Clang cursor kind to our AST node kind.
    fn convert_cursor_kind(
        &self,
        cursor: clang_sys::CXCursor,
        kind: clang_sys::CXCursorKind,
    ) -> ClangNodeKind {
        unsafe {
            match kind {
                clang_sys::CXCursor_TranslationUnit => ClangNodeKind::TranslationUnit,

                // Declarations
                clang_sys::CXCursor_FunctionDecl => {
                    let name = cursor_spelling(cursor);
                    let mangled_name = cursor_mangled_name(cursor);
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let return_type =
                        self.convert_type(clang_sys::clang_getResultType(cursor_type));
                    let num_args = clang_sys::clang_Cursor_getNumArguments(cursor);

                    let mut params = Vec::new();
                    for i in 0..num_args {
                        let arg = clang_sys::clang_Cursor_getArgument(cursor, i as u32);
                        let arg_name = cursor_spelling(arg);
                        let arg_type = clang_sys::clang_getCursorType(arg);
                        params.push((arg_name, self.convert_type(arg_type)));
                    }

                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;
                    let is_variadic = clang_sys::clang_isFunctionTypeVariadic(cursor_type) != 0;
                    let is_noexcept = self.is_function_noexcept(cursor);
                    let is_coroutine = if is_definition {
                        self.contains_coroutine_expressions(cursor)
                    } else {
                        false
                    };

                    // Extract coroutine info from return type if this is a coroutine
                    let coroutine_info = if is_coroutine {
                        self.extract_coroutine_info(&return_type, cursor)
                    } else {
                        None
                    };

                    ClangNodeKind::FunctionDecl {
                        name,
                        mangled_name,
                        return_type,
                        params,
                        is_definition,
                        is_variadic,
                        is_noexcept,
                        is_coroutine,
                        coroutine_info,
                    }
                }

                // CXCursor_FunctionTemplate = 30
                30 => {
                    let name = cursor_spelling(cursor);
                    let (template_params, parameter_pack_indices) =
                        self.get_template_type_params_with_packs(cursor);

                    // Get the templated function's type info
                    // Use template-aware type conversion to detect template parameters
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let return_type = self.convert_type_with_template_ctx(
                        clang_sys::clang_getResultType(cursor_type),
                        &template_params,
                    );

                    // Extract params from ParmVarDecl children (clang_Cursor_getNumArguments
                    // returns 0 for templates)
                    let params = self.get_function_template_params(cursor, &template_params);

                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;
                    let is_noexcept = self.is_function_noexcept(cursor);

                    // Extract requires clause if present (C++20)
                    let requires_clause = self.get_requires_clause(cursor);

                    ClangNodeKind::FunctionTemplateDecl {
                        name,
                        template_params,
                        return_type,
                        params,
                        is_definition,
                        parameter_pack_indices,
                        requires_clause,
                        is_noexcept,
                    }
                }

                // CXCursor_ClassTemplate = 31
                31 => {
                    let name = cursor_spelling(cursor);
                    let (template_params, parameter_pack_indices) =
                        self.get_template_type_params_with_packs(cursor);

                    // Determine if this is a class or struct by checking the templated decl
                    // The spelling includes "class" or "struct" prefix
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let type_spelling = clang_sys::clang_getTypeSpelling(cursor_type);
                    let type_name = cx_string_to_string(type_spelling);
                    let is_class =
                        type_name.starts_with("class ") || !type_name.starts_with("struct ");

                    // Extract requires clause if present (C++20)
                    let requires_clause = self.get_requires_clause(cursor);

                    ClangNodeKind::ClassTemplateDecl {
                        name,
                        template_params,
                        is_class,
                        parameter_pack_indices,
                        requires_clause,
                    }
                }

                // CXCursor_ClassTemplatePartialSpecialization = 32
                32 => {
                    let name = cursor_spelling(cursor);
                    let (template_params, parameter_pack_indices) =
                        self.get_template_type_params_with_packs(cursor);

                    // Get the specialization arguments
                    let specialization_args = self.get_template_specialization_args(cursor);

                    // Determine if this is a class or struct
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let type_spelling = clang_sys::clang_getTypeSpelling(cursor_type);
                    let type_name = cx_string_to_string(type_spelling);
                    let is_class =
                        type_name.starts_with("class ") || !type_name.starts_with("struct ");

                    ClangNodeKind::ClassTemplatePartialSpecDecl {
                        name,
                        template_params,
                        specialization_args,
                        is_class,
                        parameter_pack_indices,
                    }
                }

                // CXCursor_TemplateTypeParameter = 27
                27 => {
                    let name = cursor_spelling(cursor);
                    // Check if this is a parameter pack (variadic template parameter)
                    // The cursor is variadic if it represents "typename... Args"
                    let is_pack = clang_sys::clang_Cursor_isVariadic(cursor) != 0;
                    // TODO: Extract proper depth and index from cursor
                    ClangNodeKind::TemplateTypeParmDecl {
                        name,
                        depth: 0,
                        index: 0,
                        is_pack,
                    }
                }

                clang_sys::CXCursor_ParmDecl => {
                    let name = cursor_spelling(cursor);
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::ParmVarDecl { name, ty }
                }

                clang_sys::CXCursor_VarDecl => {
                    let name = cursor_spelling(cursor);
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    let storage_class = clang_sys::clang_Cursor_getStorageClass(cursor);
                    let is_static = storage_class == clang_sys::CX_SC_Static;

                    // Check if this is a static member inside a class
                    let parent = clang_sys::clang_getCursorSemanticParent(cursor);
                    let parent_kind = clang_sys::clang_getCursorKind(parent);
                    let is_class_member = parent_kind == clang_sys::CXCursor_ClassDecl
                        || parent_kind == clang_sys::CXCursor_StructDecl;

                    if is_class_member && is_static {
                        // Static data member - treat as a static field (never a bit field)
                        let access = self.get_access_specifier(cursor);
                        ClangNodeKind::FieldDecl {
                            name,
                            ty,
                            access,
                            is_static: true,
                            bit_field_width: None,
                        }
                    } else {
                        // Regular variable declaration
                        let has_init = false; // Will be determined by children
                        ClangNodeKind::VarDecl { name, ty, has_init }
                    }
                }

                clang_sys::CXCursor_StructDecl | clang_sys::CXCursor_ClassDecl => {
                    let spelling = cursor_spelling(cursor);
                    let is_class = kind == clang_sys::CXCursor_ClassDecl;
                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;

                    // For template specializations, the cursor spelling is just "MyPair"
                    // but the type spelling gives us "MyPair<int>" which is what we need
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let type_spelling = clang_sys::clang_getTypeSpelling(cursor_type);
                    let type_name = cx_string_to_string(type_spelling);

                    // Use type spelling if it contains template args (e.g., "MyPair<int>")
                    // Otherwise fall back to cursor spelling
                    let name = if type_name.contains('<') && type_name.contains('>') {
                        // Strip "struct " or "class " prefix if present
                        type_name
                            .strip_prefix("struct ")
                            .unwrap_or(&type_name)
                            .strip_prefix("class ")
                            .unwrap_or(&type_name)
                            .to_string()
                    } else {
                        spelling
                    };

                    // For anonymous structs/classes, generate a synthetic name using location
                    let final_name = if name.is_empty() {
                        let loc = clang_sys::clang_getCursorLocation(cursor);
                        let mut line: u32 = 0;
                        let mut column: u32 = 0;
                        clang_sys::clang_getSpellingLocation(
                            loc,
                            std::ptr::null_mut(),
                            &mut line,
                            &mut column,
                            std::ptr::null_mut(),
                        );
                        format!("__anon_{}{}", line, column)
                    } else {
                        name
                    };
                    // Fields will be collected from children
                    ClangNodeKind::RecordDecl {
                        name: final_name,
                        is_class,
                        is_definition,
                        fields: Vec::new(),
                    }
                }

                // CXCursor_UnionDecl = 3
                clang_sys::CXCursor_UnionDecl => {
                    let spelling = cursor_spelling(cursor);

                    // For anonymous unions, generate a synthetic name using location
                    let name = if spelling.is_empty() {
                        let loc = clang_sys::clang_getCursorLocation(cursor);
                        let mut line: u32 = 0;
                        let mut column: u32 = 0;
                        clang_sys::clang_getSpellingLocation(
                            loc,
                            std::ptr::null_mut(),
                            &mut line,
                            &mut column,
                            std::ptr::null_mut(),
                        );
                        format!("__anon_union_{}{}", line, column)
                    } else {
                        spelling
                    };
                    // Fields will be collected from children
                    ClangNodeKind::UnionDecl {
                        name,
                        fields: Vec::new(),
                    }
                }

                clang_sys::CXCursor_FieldDecl => {
                    let name = cursor_spelling(cursor);
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    let access = self.get_access_specifier(cursor);
                    // Check if this is a bit field and get its width
                    let bit_field_width = if clang_sys::clang_Cursor_isBitField(cursor) != 0 {
                        Some(clang_sys::clang_getFieldDeclBitWidth(cursor) as u32)
                    } else {
                        None
                    };
                    // Regular field declarations are never static
                    ClangNodeKind::FieldDecl {
                        name,
                        ty,
                        access,
                        is_static: false,
                        bit_field_width,
                    }
                }

                clang_sys::CXCursor_EnumDecl => {
                    let name = cursor_spelling(cursor);
                    // Check if it's a scoped enum (enum class)
                    let is_scoped = clang_sys::clang_EnumDecl_isScoped(cursor) != 0;
                    // Get underlying type
                    let underlying_type =
                        self.convert_type(clang_sys::clang_getEnumDeclIntegerType(cursor));
                    ClangNodeKind::EnumDecl {
                        name,
                        is_scoped,
                        underlying_type,
                    }
                }

                clang_sys::CXCursor_EnumConstantDecl => {
                    let name = cursor_spelling(cursor);
                    let value = clang_sys::clang_getEnumConstantDeclValue(cursor);
                    ClangNodeKind::EnumConstantDecl {
                        name,
                        value: Some(value),
                    }
                }

                clang_sys::CXCursor_CXXMethod => {
                    let name = cursor_spelling(cursor);
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let return_type =
                        self.convert_type(clang_sys::clang_getResultType(cursor_type));
                    let params = self.extract_params(cursor);
                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;
                    let is_static = clang_sys::clang_CXXMethod_isStatic(cursor) != 0;
                    let is_virtual = clang_sys::clang_CXXMethod_isVirtual(cursor) != 0;
                    let is_pure_virtual = clang_sys::clang_CXXMethod_isPureVirtual(cursor) != 0;
                    let is_const = clang_sys::clang_CXXMethod_isConst(cursor) != 0;
                    let (is_override, is_final) = self.get_override_final_attrs(cursor);
                    let access = self.get_access_specifier(cursor);
                    ClangNodeKind::CXXMethodDecl {
                        name,
                        return_type,
                        params,
                        is_definition,
                        is_static,
                        is_virtual,
                        is_pure_virtual,
                        is_override,
                        is_final,
                        is_const,
                        access,
                    }
                }

                // Conversion functions (operator bool(), operator int(), etc.)
                clang_sys::CXCursor_ConversionFunction => {
                    let name = cursor_spelling(cursor);
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let return_type =
                        self.convert_type(clang_sys::clang_getResultType(cursor_type));
                    let params = self.extract_params(cursor);
                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;
                    let is_static = false; // Conversion operators are never static
                    let is_virtual = clang_sys::clang_CXXMethod_isVirtual(cursor) != 0;
                    let is_pure_virtual = clang_sys::clang_CXXMethod_isPureVirtual(cursor) != 0;
                    let is_const = clang_sys::clang_CXXMethod_isConst(cursor) != 0;
                    let (is_override, is_final) = self.get_override_final_attrs(cursor);
                    let access = self.get_access_specifier(cursor);
                    ClangNodeKind::CXXMethodDecl {
                        name,
                        return_type,
                        params,
                        is_definition,
                        is_static,
                        is_virtual,
                        is_pure_virtual,
                        is_override,
                        is_final,
                        is_const,
                        access,
                    }
                }

                clang_sys::CXCursor_Constructor => {
                    let class_name = self.get_parent_class_name(cursor);
                    let params = self.extract_params(cursor);
                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;
                    let ctor_kind = self.get_constructor_kind(cursor);
                    let access = self.get_access_specifier(cursor);
                    ClangNodeKind::ConstructorDecl {
                        class_name,
                        params,
                        is_definition,
                        ctor_kind,
                        access,
                    }
                }

                clang_sys::CXCursor_Destructor => {
                    let class_name = self.get_parent_class_name(cursor);
                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;
                    let access = self.get_access_specifier(cursor);
                    ClangNodeKind::DestructorDecl {
                        class_name,
                        is_definition,
                        access,
                    }
                }

                clang_sys::CXCursor_Namespace => {
                    let name = cursor_spelling(cursor);
                    let name_opt = if name.is_empty() { None } else { Some(name) };
                    ClangNodeKind::NamespaceDecl { name: name_opt }
                }

                // CXCursor_LinkageSpec = 23 (extern "C" { ... })
                // This is a language linkage specification that wraps declarations.
                // We treat it as a container and recurse into it to find the actual declarations.
                clang_sys::CXCursor_LinkageSpec => ClangNodeKind::LinkageSpecDecl,

                clang_sys::CXCursor_UsingDirective => {
                    // For UsingDirective, the child nodes contain a NamespaceRef
                    // that points to the nominated namespace
                    let namespace = self.get_using_directive_namespace(cursor);
                    ClangNodeKind::UsingDirective { namespace }
                }

                clang_sys::CXCursor_UsingDeclaration => {
                    // Get the qualified name from the using declaration
                    let qualified_name = self.get_qualified_name(cursor);
                    ClangNodeKind::UsingDeclaration { qualified_name }
                }

                // CXCursor_TypeAliasDecl = 36 (using X = Y;)
                clang_sys::CXCursor_TypeAliasDecl => {
                    let name = cursor_spelling(cursor);
                    let underlying_type = self.get_typedef_underlying_type(cursor);
                    ClangNodeKind::TypeAliasDecl {
                        name,
                        underlying_type,
                    }
                }

                // CXCursor_TypedefDecl = 20 (typedef Y X;)
                clang_sys::CXCursor_TypedefDecl => {
                    let name = cursor_spelling(cursor);
                    // Skip implicit/builtin typedefs (like __int128_t)
                    let loc = clang_sys::clang_getCursorLocation(cursor);
                    if clang_sys::clang_Location_isFromMainFile(loc) == 0 {
                        // Not from main file, likely a builtin typedef
                        ClangNodeKind::Unknown("implicit_typedef".to_string())
                    } else {
                        let underlying_type = self.get_typedef_underlying_type(cursor);
                        ClangNodeKind::TypedefDecl {
                            name,
                            underlying_type,
                        }
                    }
                }

                // CXCursor_TypeAliasTemplateDecl = 601 (template<typename T> using X = Y<T>;)
                601 => {
                    let name = cursor_spelling(cursor);
                    let template_params = self.get_template_type_params(cursor);
                    let underlying_type =
                        self.get_type_alias_template_underlying_type(cursor, &template_params);
                    ClangNodeKind::TypeAliasTemplateDecl {
                        name,
                        template_params,
                        underlying_type,
                    }
                }

                clang_sys::CXCursor_MemberRef => {
                    let name = cursor_spelling(cursor);
                    ClangNodeKind::MemberRef { name }
                }

                clang_sys::CXCursor_TypeRef => {
                    // TypeRef might be a base class initializer reference
                    // We'll mark it as such, but only use it as base init in constructor context
                    let type_name = cursor_spelling(cursor);
                    // Strip "class " or "struct " prefix if present
                    let type_name = type_name
                        .strip_prefix("class ")
                        .or_else(|| type_name.strip_prefix("struct "))
                        .unwrap_or(&type_name)
                        .to_string();
                    ClangNodeKind::Unknown(format!("TypeRef:{}", type_name))
                }

                clang_sys::CXCursor_FriendDecl => {
                    // Friend declaration - examine children to determine type
                    let (friend_class, friend_function) = self.get_friend_info(cursor);
                    ClangNodeKind::FriendDecl {
                        friend_class,
                        friend_function,
                    }
                }

                clang_sys::CXCursor_CXXBaseSpecifier => {
                    // Base class specifier (inheritance)
                    let base_type = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    let access = self.get_access_specifier(cursor);
                    let is_virtual = clang_sys::clang_isVirtualBase(cursor) != 0;
                    ClangNodeKind::CXXBaseSpecifier {
                        base_type,
                        access,
                        is_virtual,
                    }
                }

                // Statements
                clang_sys::CXCursor_CompoundStmt => ClangNodeKind::CompoundStmt,
                clang_sys::CXCursor_ReturnStmt => ClangNodeKind::ReturnStmt,
                clang_sys::CXCursor_IfStmt => ClangNodeKind::IfStmt,
                clang_sys::CXCursor_WhileStmt => ClangNodeKind::WhileStmt,
                clang_sys::CXCursor_ForStmt => ClangNodeKind::ForStmt,
                // CXCursor_CXXForRangeStmt = 225
                225 => {
                    // Range-based for: for (T x : container)
                    // Children: loop variable decl, range expression, body
                    let (var_name, var_type) = self.extract_range_for_var(cursor);
                    ClangNodeKind::CXXForRangeStmt { var_name, var_type }
                }
                clang_sys::CXCursor_DoStmt => ClangNodeKind::DoStmt,
                clang_sys::CXCursor_DeclStmt => ClangNodeKind::DeclStmt,
                clang_sys::CXCursor_BreakStmt => ClangNodeKind::BreakStmt,
                clang_sys::CXCursor_ContinueStmt => ClangNodeKind::ContinueStmt,
                clang_sys::CXCursor_SwitchStmt => ClangNodeKind::SwitchStmt,
                clang_sys::CXCursor_CaseStmt => {
                    // Evaluate the case constant
                    // The first child of CaseStmt is the constant expression
                    // This can be an IntegerLiteral, CharacterLiteral, DeclRefExpr (for const vars), etc.
                    // We need to handle all these cases by evaluating the first child

                    extern "C" fn find_case_value(
                        child: clang_sys::CXCursor,
                        _parent: clang_sys::CXCursor,
                        data: clang_sys::CXClientData,
                    ) -> clang_sys::CXChildVisitResult {
                        unsafe {
                            let child_kind = clang_sys::clang_getCursorKind(child);
                            // Try to evaluate any constant expression (IntegerLiteral, CharacterLiteral,
                            // DeclRefExpr to const vars, UnaryExpr with minus, etc.)
                            // clang_Cursor_Evaluate handles all these cases
                            if child_kind == clang_sys::CXCursor_IntegerLiteral
                                || child_kind == clang_sys::CXCursor_CharacterLiteral
                                || child_kind == clang_sys::CXCursor_DeclRefExpr
                                || child_kind == clang_sys::CXCursor_UnaryOperator
                                || child_kind == clang_sys::CXCursor_ParenExpr
                                || child_kind == 113
                            {
                                // CXCursor_ConstantExpr = 113
                                let eval = clang_sys::clang_Cursor_Evaluate(child);
                                if !eval.is_null() {
                                    let value_ptr = data as *mut i128;
                                    *value_ptr = clang_sys::clang_EvalResult_getAsInt(eval) as i128;
                                    clang_sys::clang_EvalResult_dispose(eval);
                                    return clang_sys::CXChildVisit_Break;
                                }
                            }
                            clang_sys::CXChildVisit_Recurse
                        }
                    }

                    let mut case_value: i128 = 0;
                    clang_sys::clang_visitChildren(
                        cursor,
                        find_case_value,
                        &mut case_value as *mut i128 as clang_sys::CXClientData,
                    );

                    ClangNodeKind::CaseStmt { value: case_value }
                }
                clang_sys::CXCursor_DefaultStmt => ClangNodeKind::DefaultStmt,

                // C++ Exception Handling
                clang_sys::CXCursor_CXXTryStmt => ClangNodeKind::TryStmt,

                clang_sys::CXCursor_CXXCatchStmt => {
                    // Get the exception type from the first child if it's a VarDecl
                    let exception_ty = self.get_catch_exception_type(cursor);
                    ClangNodeKind::CatchStmt { exception_ty }
                }

                clang_sys::CXCursor_CXXThrowExpr => {
                    // Get the type being thrown from the child expression
                    let exception_ty = self.get_throw_exception_type(cursor);
                    ClangNodeKind::ThrowExpr { exception_ty }
                }

                // C++ RTTI
                clang_sys::CXCursor_CXXTypeidExpr => {
                    let result_ty = self.convert_type(clang_sys::clang_getCursorType(cursor));

                    // Check if this is a type operand by looking at the first child
                    // For typeid(Type), there are no children or a TypeRef child
                    // For typeid(expr), there's an expression child
                    let mut is_type_operand = true;
                    let mut operand_ty = CppType::Void;

                    // Visit children to determine operand type
                    let mut first_child_kind: i32 = 0;
                    let mut first_child_cursor = clang_sys::clang_getNullCursor();
                    extern "C" fn find_first_child(
                        c: clang_sys::CXCursor,
                        _parent: clang_sys::CXCursor,
                        data: clang_sys::CXClientData,
                    ) -> clang_sys::CXChildVisitResult {
                        let (kind_ptr, cursor_ptr) =
                            unsafe { &mut *(data as *mut (i32, clang_sys::CXCursor)) };
                        *kind_ptr = unsafe { clang_sys::clang_getCursorKind(c) };
                        *cursor_ptr = c;
                        clang_sys::CXChildVisit_Break
                    }
                    let mut child_data = (first_child_kind, first_child_cursor);
                    clang_sys::clang_visitChildren(
                        cursor,
                        find_first_child,
                        &mut child_data as *mut (i32, clang_sys::CXCursor)
                            as clang_sys::CXClientData,
                    );
                    first_child_kind = child_data.0;
                    first_child_cursor = child_data.1;

                    if first_child_kind != 0 {
                        // If there's a child that's not a TypeRef, it's an expression operand
                        if first_child_kind != clang_sys::CXCursor_TypeRef {
                            is_type_operand = false;
                            // Get the type of the expression being evaluated
                            operand_ty = self
                                .convert_type(clang_sys::clang_getCursorType(first_child_cursor));
                        } else {
                            // TypeRef child - get the referenced type
                            operand_ty = self
                                .convert_type(clang_sys::clang_getCursorType(first_child_cursor));
                        }
                    }

                    ClangNodeKind::TypeidExpr {
                        result_ty,
                        is_type_operand,
                        operand_ty,
                    }
                }

                clang_sys::CXCursor_CXXDynamicCastExpr => {
                    // The result type is the target type of the cast
                    let target_ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::DynamicCastExpr { target_ty }
                }

                // Expressions
                clang_sys::CXCursor_IntegerLiteral => {
                    // Try to evaluate the literal
                    let eval = clang_sys::clang_Cursor_Evaluate(cursor);
                    let value = if eval.is_null() {
                        0i128
                    } else {
                        // Check if this is an unsigned integer to get the correct value
                        let is_unsigned = clang_sys::clang_EvalResult_isUnsignedInt(eval) != 0;
                        let result = if is_unsigned {
                            // Use getAsUnsigned for unsigned integers to avoid sign extension
                            clang_sys::clang_EvalResult_getAsUnsigned(eval) as i128
                        } else {
                            // Use getAsLongLong for signed integers (handles larger values)
                            clang_sys::clang_EvalResult_getAsLongLong(eval) as i128
                        };
                        clang_sys::clang_EvalResult_dispose(eval);
                        result
                    };

                    // Capture the type of the literal (e.g., int, unsigned int, long)
                    let clang_type = clang_sys::clang_getCursorType(cursor);
                    let cpp_type = Some(self.convert_type(clang_type));

                    ClangNodeKind::IntegerLiteral { value, cpp_type }
                }

                clang_sys::CXCursor_CharacterLiteral => {
                    // Character literals like 'a' - evaluate to get the ASCII value
                    let eval = clang_sys::clang_Cursor_Evaluate(cursor);
                    let value = if eval.is_null() {
                        0i128
                    } else {
                        let result = clang_sys::clang_EvalResult_getAsLongLong(eval) as i128;
                        clang_sys::clang_EvalResult_dispose(eval);
                        result
                    };

                    // In C++, character literals have type 'char' (i8 in Rust)
                    let cpp_type = Some(CppType::Char { signed: true });

                    ClangNodeKind::IntegerLiteral { value, cpp_type }
                }

                clang_sys::CXCursor_FloatingLiteral => {
                    let eval = clang_sys::clang_Cursor_Evaluate(cursor);
                    let value = if !eval.is_null() {
                        let val = clang_sys::clang_EvalResult_getAsDouble(eval);
                        clang_sys::clang_EvalResult_dispose(eval);
                        val
                    } else {
                        0.0
                    };

                    // Capture the type of the literal (float, double, long double)
                    let clang_type = clang_sys::clang_getCursorType(cursor);
                    let cpp_type = Some(self.convert_type(clang_type));

                    ClangNodeKind::FloatingLiteral { value, cpp_type }
                }

                clang_sys::CXCursor_CXXBoolLiteralExpr => {
                    // Evaluate the boolean literal using libclang
                    let eval = clang_sys::clang_Cursor_Evaluate(cursor);
                    let value = if !eval.is_null() {
                        // getAsInt returns 1 for true, 0 for false
                        let result = clang_sys::clang_EvalResult_getAsInt(eval) != 0;
                        clang_sys::clang_EvalResult_dispose(eval);
                        result
                    } else {
                        // Fallback: check the spelling ("true" or "false")
                        let spelling = cursor_spelling(cursor);
                        spelling == "true"
                    };
                    ClangNodeKind::BoolLiteral(value)
                }

                clang_sys::CXCursor_CXXNullPtrLiteralExpr => {
                    // C++ nullptr literal
                    ClangNodeKind::NullPtrLiteral
                }

                clang_sys::CXCursor_CXXNewExpr => {
                    // C++ new expression
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    // Check if this is array new by tokenizing the source
                    let is_array = self.check_new_is_array(cursor);
                    // Check if this is placement new by tokenizing the source
                    let is_placement = self.check_new_is_placement(cursor);
                    ClangNodeKind::CXXNewExpr {
                        ty,
                        is_array,
                        is_placement,
                    }
                }

                clang_sys::CXCursor_CXXDeleteExpr => {
                    // C++ delete expression
                    // Check if this is array delete by tokenizing the source
                    let is_array = self.check_delete_is_array(cursor);
                    ClangNodeKind::CXXDeleteExpr { is_array }
                }

                clang_sys::CXCursor_StringLiteral => {
                    // Get the string value using evaluation
                    let eval = clang_sys::clang_Cursor_Evaluate(cursor);
                    let value = if !eval.is_null() {
                        let str_ptr = clang_sys::clang_EvalResult_getAsStr(eval);
                        let result = if !str_ptr.is_null() {
                            std::ffi::CStr::from_ptr(str_ptr)
                                .to_string_lossy()
                                .into_owned()
                        } else {
                            String::new()
                        };
                        clang_sys::clang_EvalResult_dispose(eval);
                        result
                    } else {
                        // Fallback: use cursor spelling (may include quotes)
                        let spelling = cursor_spelling(cursor);
                        // Remove surrounding quotes if present
                        if spelling.starts_with('"')
                            && spelling.ends_with('"')
                            && spelling.len() >= 2
                        {
                            spelling[1..spelling.len() - 1].to_string()
                        } else {
                            spelling
                        }
                    };
                    ClangNodeKind::StringLiteral(value)
                }

                clang_sys::CXCursor_DeclRefExpr => {
                    let name = cursor_spelling(cursor);
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    let namespace_path = self.get_namespace_path(cursor);
                    ClangNodeKind::DeclRefExpr {
                        name,
                        ty,
                        namespace_path,
                    }
                }

                clang_sys::CXCursor_BinaryOperator => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    // Get the operator from tokens
                    let op = self.get_binary_op(cursor);
                    ClangNodeKind::BinaryOperator { op, ty }
                }

                // CXCursor_CompoundAssignOperator = 115 (+=, -=, *=, etc.)
                115 => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    let op = self.get_binary_op(cursor);
                    ClangNodeKind::BinaryOperator { op, ty }
                }

                clang_sys::CXCursor_UnaryOperator => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    let op = self.get_unary_op(cursor);
                    ClangNodeKind::UnaryOperator { op, ty }
                }

                clang_sys::CXCursor_CallExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::CallExpr { ty }
                }

                clang_sys::CXCursor_MemberRefExpr => {
                    let member_name = cursor_spelling(cursor);
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    // Check if arrow or dot using token-based detection
                    let is_arrow = self.is_arrow_access(cursor);
                    // Get the declaring class for inherited member detection
                    let declaring_class = self.get_member_declaring_class(cursor);
                    // Check if this is a static member access
                    let is_static = self.is_static_member(cursor);
                    ClangNodeKind::MemberExpr {
                        member_name,
                        is_arrow,
                        ty,
                        declaring_class,
                        is_static,
                    }
                }

                clang_sys::CXCursor_ArraySubscriptExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::ArraySubscriptExpr { ty }
                }

                clang_sys::CXCursor_ParenExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::ParenExpr { ty }
                }

                clang_sys::CXCursor_CStyleCastExpr | clang_sys::CXCursor_CXXFunctionalCastExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::CastExpr {
                        cast_kind: CastKind::Other,
                        ty,
                    }
                }

                clang_sys::CXCursor_CXXStaticCastExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::CastExpr {
                        cast_kind: CastKind::Static,
                        ty,
                    }
                }

                clang_sys::CXCursor_CXXReinterpretCastExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::CastExpr {
                        cast_kind: CastKind::Reinterpret,
                        ty,
                    }
                }

                clang_sys::CXCursor_CXXConstCastExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::CastExpr {
                        cast_kind: CastKind::Const,
                        ty,
                    }
                }

                clang_sys::CXCursor_ConditionalOperator => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::ConditionalOperator { ty }
                }

                // Initialization list expression: {1, 2, 3}
                clang_sys::CXCursor_InitListExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::InitListExpr { ty }
                }

                // C++ this expression (explicit or implicit)
                // CXCursor_CXXThisExpr = 132
                132 => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::CXXThisExpr { ty }
                }

                // C++20 Concepts
                // CXCursor_ConceptDecl = 604
                604 => {
                    let name = cursor_spelling(cursor);
                    let template_params = self.get_template_type_params(cursor);
                    let constraint_expr = self.get_concept_constraint_expr(cursor);

                    ClangNodeKind::ConceptDecl {
                        name,
                        template_params,
                        constraint_expr,
                    }
                }

                // CXCursor_RequiresExpr = 279
                279 => {
                    let params = self.get_requires_expr_params(cursor);
                    let requirements = self.get_requirements(cursor);

                    ClangNodeKind::RequiresExpr {
                        params,
                        requirements,
                    }
                }

                // CXCursor_ConceptSpecializationExpr = 602
                602 => {
                    let (concept_name, template_args) =
                        self.get_concept_specialization_info(cursor);

                    ClangNodeKind::ConceptSpecializationExpr {
                        concept_name,
                        template_args,
                    }
                }

                // C++11 Lambda expressions
                // CXCursor_LambdaExpr = 144
                144 => {
                    let (params, return_type, capture_default, captures) =
                        self.parse_lambda_info(cursor);
                    ClangNodeKind::LambdaExpr {
                        params,
                        return_type,
                        capture_default,
                        captures,
                    }
                }

                // C++20 Coroutines - libclang maps these to UnexposedExpr/UnexposedStmt
                // We detect them by tokenizing and looking for co_await, co_yield, co_return keywords
                clang_sys::CXCursor_UnexposedExpr => {
                    if let Some(coroutine_kind) = self.try_parse_coroutine_expr(cursor) {
                        coroutine_kind
                    } else if let Some(eval_kind) = self.try_evaluate_expr(cursor) {
                        // Try to evaluate the expression (for default arguments, etc.)
                        eval_kind
                    } else if let Some(implicit_cast) = self.try_parse_implicit_cast(cursor) {
                        // Try to detect implicit casts
                        implicit_cast
                    } else {
                        // Fall back to Unknown for non-coroutine unexposed expressions
                        let kind_spelling = clang_sys::clang_getCursorKindSpelling(kind);
                        ClangNodeKind::Unknown(cx_string_to_string(kind_spelling))
                    }
                }

                clang_sys::CXCursor_UnexposedStmt => {
                    if let Some(coroutine_kind) = self.try_parse_coroutine_stmt(cursor) {
                        coroutine_kind
                    } else {
                        // Fall back to Unknown for non-coroutine unexposed statements
                        let kind_spelling = clang_sys::clang_getCursorKindSpelling(kind);
                        ClangNodeKind::Unknown(cx_string_to_string(kind_spelling))
                    }
                }

                // CXCursor_TemplateRef = 45 - Reference to a template
                // These are just references and don't need special handling
                45 => {
                    let name = cursor_spelling(cursor);
                    ClangNodeKind::Unknown(format!("TemplateRef:{}", name))
                }

                // CXCursor_NamespaceRef = 46 - Reference to a namespace
                46 => {
                    let name = cursor_spelling(cursor);
                    ClangNodeKind::Unknown(format!("NamespaceRef:{}", name))
                }

                // CXCursor_OverloadedDeclRef = 49 - Reference to an overloaded declaration
                49 => {
                    let name = cursor_spelling(cursor);
                    ClangNodeKind::Unknown(format!("OverloadedDeclRef:{}", name))
                }

                // CXCursor_NonTypeTemplateParameter = 28 - Non-type template parameter
                28 => {
                    let name = cursor_spelling(cursor);
                    ClangNodeKind::Unknown(format!("NonTypeTemplateParameter:{}", name))
                }

                // CXCursor_MacroExpansion = 502 - Macro expansion
                502 => {
                    let name = cursor_spelling(cursor);
                    ClangNodeKind::Unknown(format!("macro_expansion:{}", name))
                }

                // CXCursor_InclusionDirective = 503 - #include directive
                503 => {
                    let name = cursor_spelling(cursor);
                    ClangNodeKind::Unknown(format!("inclusion:{}", name))
                }

                // CXCursor_ModuleImportDecl = 600 - C++20 import declaration
                600 => {
                    // Get module name via clang module API if available
                    let module = clang_sys::clang_Cursor_getModule(cursor);
                    let module_name = if !module.is_null() {
                        let full_name = clang_sys::clang_Module_getFullName(module);
                        cx_string_to_string(full_name)
                    } else {
                        // Fallback to cursor spelling if module API doesn't work
                        cursor_spelling(cursor)
                    };

                    // Check if this is a header unit import (import <header>)
                    // Header units typically have file paths or start with <
                    let is_header_unit = module_name.starts_with('<')
                        || module_name.ends_with(".h")
                        || module_name.ends_with(".hpp")
                        || module_name.contains('/');

                    ClangNodeKind::ModuleImportDecl {
                        module_name,
                        is_header_unit,
                    }
                }

                _ => {
                    let kind_spelling = clang_sys::clang_getCursorKindSpelling(kind);
                    ClangNodeKind::Unknown(cx_string_to_string(kind_spelling))
                }
            }
        }
    }

    /// Convert a Clang type to our type representation.
    fn convert_type(&self, ty: clang_sys::CXType) -> CppType {
        unsafe {
            let kind = ty.kind;
            match kind {
                clang_sys::CXType_Void => CppType::Void,
                clang_sys::CXType_Bool => CppType::Bool,
                clang_sys::CXType_Char_S | clang_sys::CXType_SChar => {
                    CppType::Char { signed: true }
                }
                clang_sys::CXType_Char_U | clang_sys::CXType_UChar => {
                    CppType::Char { signed: false }
                }
                clang_sys::CXType_Short => CppType::Short { signed: true },
                clang_sys::CXType_UShort => CppType::Short { signed: false },
                clang_sys::CXType_Int => CppType::Int { signed: true },
                clang_sys::CXType_UInt => CppType::Int { signed: false },
                clang_sys::CXType_Long => CppType::Long { signed: true },
                clang_sys::CXType_ULong => CppType::Long { signed: false },
                clang_sys::CXType_LongLong => CppType::LongLong { signed: true },
                clang_sys::CXType_ULongLong => CppType::LongLong { signed: false },
                clang_sys::CXType_Float => CppType::Float,
                clang_sys::CXType_Double => CppType::Double,

                clang_sys::CXType_Pointer => {
                    let pointee = clang_sys::clang_getPointeeType(ty);
                    let is_const = clang_sys::clang_isConstQualifiedType(pointee) != 0;
                    CppType::Pointer {
                        pointee: Box::new(self.convert_type(pointee)),
                        is_const,
                    }
                }

                clang_sys::CXType_LValueReference | clang_sys::CXType_RValueReference => {
                    let referent = clang_sys::clang_getPointeeType(ty);
                    let is_const = clang_sys::clang_isConstQualifiedType(referent) != 0;
                    let is_rvalue = kind == clang_sys::CXType_RValueReference;
                    CppType::Reference {
                        referent: Box::new(self.convert_type(referent)),
                        is_const,
                        is_rvalue,
                    }
                }

                clang_sys::CXType_ConstantArray => {
                    let element = clang_sys::clang_getArrayElementType(ty);
                    let size = clang_sys::clang_getArraySize(ty) as usize;
                    CppType::Array {
                        element: Box::new(self.convert_type(element)),
                        size: Some(size),
                    }
                }

                clang_sys::CXType_IncompleteArray => {
                    let element = clang_sys::clang_getArrayElementType(ty);
                    CppType::Array {
                        element: Box::new(self.convert_type(element)),
                        size: None,
                    }
                }

                clang_sys::CXType_Record | clang_sys::CXType_Elaborated => {
                    let spelling = clang_sys::clang_getTypeSpelling(ty);
                    let name = cx_string_to_string(spelling);
                    // Clean up the name (remove "struct " prefix, etc.)
                    let name = name
                        .trim_start_matches("struct ")
                        .trim_start_matches("class ")
                        .to_string();
                    CppType::Named(name)
                }

                clang_sys::CXType_FunctionProto => {
                    let return_type = clang_sys::clang_getResultType(ty);
                    let num_args = clang_sys::clang_getNumArgTypes(ty);
                    let mut params = Vec::new();
                    for i in 0..num_args {
                        let arg_type = clang_sys::clang_getArgType(ty, i as u32);
                        params.push(self.convert_type(arg_type));
                    }
                    let is_variadic = clang_sys::clang_isFunctionTypeVariadic(ty) != 0;

                    CppType::Function {
                        return_type: Box::new(self.convert_type(return_type)),
                        params,
                        is_variadic,
                    }
                }

                _ => {
                    let spelling = clang_sys::clang_getTypeSpelling(ty);
                    CppType::Named(cx_string_to_string(spelling))
                }
            }
        }
    }

    /// Get binary operator from cursor by tokenizing and finding the ROOT operator token.
    /// For `a + b * c`, the root operator depends on which BinaryOperator cursor we're examining.
    fn get_binary_op(&self, cursor: clang_sys::CXCursor) -> BinaryOp {
        unsafe {
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);

            // Get the two children (left and right operands)
            let mut children: Vec<clang_sys::CXCursor> = Vec::new();
            extern "C" fn child_visitor(
                cursor: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                client_data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                let children = unsafe { &mut *(client_data as *mut Vec<clang_sys::CXCursor>) };
                children.push(cursor);
                clang_sys::CXChildVisit_Continue
            }

            clang_sys::clang_visitChildren(
                cursor,
                child_visitor,
                &mut children as *mut Vec<clang_sys::CXCursor> as clang_sys::CXClientData,
            );

            // We need exactly 2 children for a binary operator
            if children.len() < 2 {
                return BinaryOp::Add; // Fallback
            }

            // Get the end of first child and start of second child
            let first_extent = clang_sys::clang_getCursorExtent(children[0]);
            let second_extent = clang_sys::clang_getCursorExtent(children[1]);
            let first_end = clang_sys::clang_getRangeEnd(first_extent);
            let second_start = clang_sys::clang_getRangeStart(second_extent);

            // Get file and offsets
            let mut first_end_offset: u32 = 0;
            let mut second_start_offset: u32 = 0;
            clang_sys::clang_getSpellingLocation(
                first_end,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut first_end_offset,
            );
            clang_sys::clang_getSpellingLocation(
                second_start,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut second_start_offset,
            );

            // Tokenize the region between children
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = std::ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            let mut result = BinaryOp::Add; // Default

            // Find operator token between first child end and second child start
            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let token_kind = clang_sys::clang_getTokenKind(token);

                // CXToken_Punctuation = 0
                if token_kind == 0 {
                    let token_loc = clang_sys::clang_getTokenLocation(tu, token);
                    let mut token_offset: u32 = 0;
                    clang_sys::clang_getSpellingLocation(
                        token_loc,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut token_offset,
                    );

                    // Only consider tokens between the two children
                    if token_offset >= first_end_offset && token_offset < second_start_offset {
                        let token_spelling = clang_sys::clang_getTokenSpelling(tu, token);
                        let token_str = cx_string_to_string(token_spelling);

                        if let Some(op) = str_to_binary_op(&token_str) {
                            result = op;
                            break;
                        }
                    }
                }
            }

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            result
        }
    }

    /// Get unary operator from cursor by tokenizing and finding the operator token.
    fn get_unary_op(&self, cursor: clang_sys::CXCursor) -> UnaryOp {
        unsafe {
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = std::ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            let mut result = UnaryOp::Minus; // Default

            // Unary operators: prefix (++x, --x, -x, +x, !x, ~x, *x, &x)
            //                  postfix (x++, x--)
            // Key insight: for prefix operators, the FIRST token is punctuation
            //              for postfix operators, the FIRST token is NOT punctuation
            let mut first_token_is_punct = false;
            let mut operator_str: Option<String> = None;

            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let token_kind = clang_sys::clang_getTokenKind(token);

                // CXToken_Punctuation = 0 (not 1!)
                if i == 0 {
                    first_token_is_punct = token_kind == 0;
                }

                if token_kind == 0 {
                    let token_spelling = clang_sys::clang_getTokenSpelling(tu, token);
                    let token_str = cx_string_to_string(token_spelling);
                    // Store the operator (for ++/--, it's the same for first/last)
                    if operator_str.is_none() || token_str == "++" || token_str == "--" {
                        operator_str = Some(token_str);
                    }
                }
            }

            // Determine operator based on what we found
            if let Some(ref op) = operator_str {
                result = match op.as_str() {
                    "++" => {
                        if first_token_is_punct {
                            UnaryOp::PreInc
                        } else {
                            UnaryOp::PostInc
                        }
                    }
                    "--" => {
                        if first_token_is_punct {
                            UnaryOp::PreDec
                        } else {
                            UnaryOp::PostDec
                        }
                    }
                    "-" => UnaryOp::Minus,
                    "+" => UnaryOp::Plus,
                    "!" => UnaryOp::LNot,
                    "~" => UnaryOp::Not,
                    "*" => UnaryOp::Deref,
                    "&" => UnaryOp::AddrOf,
                    _ => UnaryOp::Minus,
                };
            }

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            result
        }
    }

    /// Parse lambda expression information.
    /// Returns (params, return_type, capture_default, captures).
    fn parse_lambda_info(
        &self,
        cursor: clang_sys::CXCursor,
    ) -> (
        Vec<(String, CppType)>,
        CppType,
        CaptureDefault,
        Vec<(String, bool)>,
    ) {
        unsafe {
            let mut params = Vec::new();
            let mut return_type = CppType::Void;
            let mut capture_default = CaptureDefault::None;
            let mut captures = Vec::new();

            // Visit children to find parameters and body
            // Lambda structure: CXXRecordDecl (implicit class), then CompoundStmt (body)
            // The operator() method contains the parameters and return type
            let mut visit_data = (
                &mut params,
                &mut return_type,
                &mut capture_default,
                &mut captures,
                self,
            );

            extern "C" fn visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let (params, return_type, _capture_default, captures, parser): &mut (
                        &mut Vec<(String, CppType)>,
                        &mut CppType,
                        &mut CaptureDefault,
                        &mut Vec<(String, bool)>,
                        &ClangParser,
                    ) = &mut *(data as *mut _);

                    let kind = clang_sys::clang_getCursorKind(child);

                    match kind {
                        // ParmDecl - lambda parameters
                        clang_sys::CXCursor_ParmDecl => {
                            let name = cursor_spelling(child);
                            let ty = (*parser).convert_type(clang_sys::clang_getCursorType(child));
                            params.push((name, ty));
                        }
                        // CXXMethod - operator() contains return type
                        clang_sys::CXCursor_CXXMethod => {
                            let method_name = cursor_spelling(child);
                            if method_name == "operator()" {
                                let func_type = clang_sys::clang_getCursorType(child);
                                let ret_type = clang_sys::clang_getResultType(func_type);
                                **return_type = (*parser).convert_type(ret_type);
                            }
                        }
                        // Check for capture variables
                        clang_sys::CXCursor_VarDecl => {
                            // Captured variables appear as VarDecl children
                            let name = cursor_spelling(child);
                            if !name.is_empty() {
                                // Check if captured by reference
                                let ty = clang_sys::clang_getCursorType(child);
                                let is_ref = ty.kind == clang_sys::CXType_LValueReference;
                                captures.push((name, is_ref));
                            }
                        }
                        _ => {}
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                visitor,
                &mut visit_data as *mut _ as clang_sys::CXClientData,
            );

            // Try to determine capture default from tokens
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            if !tokens.is_null() && num_tokens > 0 {
                // Look for [=] or [&] at the start
                let mut found_bracket = false;
                for i in 0..num_tokens {
                    let token = *tokens.add(i as usize);
                    let spelling = clang_sys::clang_getTokenSpelling(tu, token);
                    let s = cx_string_to_string(spelling);

                    if s == "[" {
                        found_bracket = true;
                    } else if found_bracket {
                        if s == "=" {
                            capture_default = CaptureDefault::ByCopy;
                            break;
                        } else if s == "&" && i + 1 < num_tokens {
                            // Check if next token is ] for [&] default capture
                            let next_token = *tokens.add((i + 1) as usize);
                            let next_spelling = clang_sys::clang_getTokenSpelling(tu, next_token);
                            let next_s = cx_string_to_string(next_spelling);
                            if next_s == "]" {
                                capture_default = CaptureDefault::ByRef;
                            }
                            break;
                        } else if s == "]" {
                            // Empty capture []
                            break;
                        } else {
                            // Some specific capture, not default
                            break;
                        }
                    }
                }

                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            (params, return_type, capture_default, captures)
        }
    }

    /// Extract loop variable name and type from a range-based for statement.
    fn extract_range_for_var(&self, cursor: clang_sys::CXCursor) -> (String, CppType) {
        unsafe {
            let mut var_name = String::new();
            let mut var_type = CppType::Int { signed: true };

            // Visit the first VarDecl child to get the loop variable
            extern "C" fn visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let (var_name, var_type, parser): &mut (
                        &mut String,
                        &mut CppType,
                        &ClangParser,
                    ) = &mut *(data as *mut _);

                    let kind = clang_sys::clang_getCursorKind(child);

                    // The loop variable is in a VarDecl
                    if kind == clang_sys::CXCursor_VarDecl {
                        let name = cursor_spelling(child);
                        // Skip internal variables like __range1, __begin1, __end1
                        if !name.starts_with("__") {
                            **var_name = name;
                            **var_type =
                                (*parser).convert_type(clang_sys::clang_getCursorType(child));
                            return clang_sys::CXChildVisit_Break;
                        }
                    }
                    clang_sys::CXChildVisit_Continue
                }
            }

            let mut visit_data = (&mut var_name, &mut var_type, self);
            clang_sys::clang_visitChildren(
                cursor,
                visitor,
                &mut visit_data as *mut _ as clang_sys::CXClientData,
            );

            (var_name, var_type)
        }
    }

    /// Try to parse a coroutine expression (co_await or co_yield) from an UnexposedExpr.
    /// Returns Some(ClangNodeKind) if this is a coroutine expression, None otherwise.
    fn try_parse_coroutine_expr(&self, cursor: clang_sys::CXCursor) -> Option<ClangNodeKind> {
        unsafe {
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            if tokens.is_null() || num_tokens == 0 {
                return None;
            }

            // Get the first token to check for co_await or co_yield
            let first_token = *tokens;
            let first_spelling = clang_sys::clang_getTokenSpelling(tu, first_token);
            let first_str = cx_string_to_string(first_spelling);

            let result = match first_str.as_str() {
                "co_await" => {
                    // Get the result type from the cursor type
                    let result_ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    // For operand type, we'll use the result type as a placeholder
                    // In practice, we'd need to examine the child expression
                    let operand_ty = self.get_coroutine_operand_type(cursor);
                    Some(ClangNodeKind::CoawaitExpr {
                        operand_ty,
                        result_ty,
                    })
                }
                "co_yield" => {
                    // Get the result type from the cursor type
                    let result_ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    // For value type, examine the child expression
                    let value_ty = self.get_coroutine_operand_type(cursor);
                    Some(ClangNodeKind::CoyieldExpr {
                        value_ty,
                        result_ty,
                    })
                }
                _ => None,
            };

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            result
        }
    }

    /// Try to parse a coroutine statement (co_return) from an UnexposedStmt.
    /// Returns Some(ClangNodeKind) if this is a coroutine statement, None otherwise.
    fn try_parse_coroutine_stmt(&self, cursor: clang_sys::CXCursor) -> Option<ClangNodeKind> {
        unsafe {
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            if tokens.is_null() || num_tokens == 0 {
                return None;
            }

            // Get the first token to check for co_return
            let first_token = *tokens;
            let first_spelling = clang_sys::clang_getTokenSpelling(tu, first_token);
            let first_str = cx_string_to_string(first_spelling);

            let result = if first_str == "co_return" {
                // Check if there's a value being returned
                // If num_tokens > 2 (co_return + ; + something), there's a value
                let value_ty = if num_tokens > 2 {
                    Some(self.get_coroutine_operand_type(cursor))
                } else {
                    None
                };
                Some(ClangNodeKind::CoreturnStmt { value_ty })
            } else {
                None
            };

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            result
        }
    }

    /// Get the operand type for a coroutine expression by examining its first child.
    fn get_coroutine_operand_type(&self, cursor: clang_sys::CXCursor) -> CppType {
        unsafe {
            // Visit children to find the operand expression
            struct OperandTypeData {
                ty: CppType,
                parser: *const ClangParser,
            }

            extern "C" fn find_operand_type(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(data as *mut OperandTypeData);
                    let child_type = clang_sys::clang_getCursorType(child);
                    data.ty = (*data.parser).convert_type(child_type);
                    clang_sys::CXChildVisit_Break
                }
            }

            let mut data = OperandTypeData {
                ty: CppType::Void,
                parser: self as *const ClangParser,
            };

            clang_sys::clang_visitChildren(
                cursor,
                find_operand_type,
                &mut data as *mut OperandTypeData as clang_sys::CXClientData,
            );

            data.ty
        }
    }

    /// Extract coroutine information from the function's return type.
    /// This analyzes the return type to determine the coroutine kind (Async, Generator, Task)
    /// and extract the value type (T in Task<T> or Generator<T>).
    fn extract_coroutine_info(
        &self,
        return_type: &CppType,
        cursor: clang_sys::CXCursor,
    ) -> Option<CoroutineInfo> {
        // Get the return type spelling for pattern matching
        let type_spelling = match return_type {
            CppType::Named(name) => name.clone(),
            _ => {
                // For non-named types, fall back to checking what coroutine expressions are used
                return Some(self.infer_coroutine_kind_from_body(cursor));
            }
        };

        // Check for common coroutine return type patterns
        // Task-like types (async coroutines)
        let async_patterns = [
            "Task",
            "task",
            "std::task",
            "cppcoro::task",
            "folly::coro::Task",
            "boost::asio::awaitable",
        ];

        // Generator-like types
        let generator_patterns = [
            "Generator",
            "generator",
            "std::generator",
            "cppcoro::generator",
        ];

        // Check async patterns first
        for pattern in &async_patterns {
            if let Some(value_type) = self.extract_template_value_type(&type_spelling, pattern) {
                return Some(CoroutineInfo {
                    kind: CoroutineKind::Async,
                    value_type: Some(value_type),
                    return_type_spelling: type_spelling,
                });
            }
        }

        // Check generator patterns
        for pattern in &generator_patterns {
            if let Some(value_type) = self.extract_template_value_type(&type_spelling, pattern) {
                return Some(CoroutineInfo {
                    kind: CoroutineKind::Generator,
                    value_type: Some(value_type),
                    return_type_spelling: type_spelling,
                });
            }
        }

        // If no pattern matched, infer from the coroutine body
        Some(self.infer_coroutine_kind_from_body(cursor))
    }

    /// Extract template value type from a type name like "Task<int>" or "Generator<std::string>".
    /// Returns Some(CppType) if the pattern matches and type can be extracted.
    fn extract_template_value_type(&self, type_name: &str, pattern: &str) -> Option<CppType> {
        // Check if type name starts with the pattern
        if !type_name.starts_with(pattern) {
            return None;
        }

        // Find the template argument
        let rest = &type_name[pattern.len()..];
        if !rest.starts_with('<') {
            return None;
        }

        // Extract content between < and > respecting nested templates
        let mut depth = 0;
        let mut start_idx = None;
        let mut end_idx = None;

        for (i, ch) in rest.chars().enumerate() {
            match ch {
                '<' => {
                    if depth == 0 {
                        start_idx = Some(i + 1);
                    }
                    depth += 1;
                }
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end_idx = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let (Some(start), Some(end)) = (start_idx, end_idx) {
            let arg = rest[start..end].trim();
            if arg.is_empty() {
                return None;
            }
            return Some(self.parse_type_from_string(arg));
        }

        None
    }

    /// Parse a type from its string representation.
    /// Used for extracting template arguments from type spellings.
    fn parse_type_from_string(&self, type_str: &str) -> CppType {
        let type_str = type_str.trim();

        // Check for pointer types
        if type_str.ends_with('*') {
            let pointee = self.parse_type_from_string(&type_str[..type_str.len() - 1]);
            return CppType::Pointer {
                pointee: Box::new(pointee),
                is_const: type_str.contains("const "),
            };
        }

        // Check for reference types
        if type_str.ends_with('&') {
            let without_ref = type_str[..type_str.len() - 1].trim();
            let is_rvalue = without_ref.ends_with('&');
            let referent_str = if is_rvalue {
                &without_ref[..without_ref.len() - 1]
            } else {
                without_ref
            };
            let referent = self.parse_type_from_string(referent_str);
            return CppType::Reference {
                referent: Box::new(referent),
                is_const: type_str.contains("const "),
                is_rvalue,
            };
        }

        // Check for common primitives
        match type_str {
            "void" => CppType::Void,
            "bool" => CppType::Bool,
            "char" => CppType::Char { signed: true },
            "signed char" => CppType::Char { signed: true },
            "unsigned char" => CppType::Char { signed: false },
            "short" | "short int" => CppType::Short { signed: true },
            "unsigned short" | "unsigned short int" => CppType::Short { signed: false },
            "int" => CppType::Int { signed: true },
            "unsigned int" | "unsigned" => CppType::Int { signed: false },
            "long" | "long int" => CppType::Long { signed: true },
            "unsigned long" | "unsigned long int" => CppType::Long { signed: false },
            "long long" | "long long int" => CppType::LongLong { signed: true },
            "unsigned long long" | "unsigned long long int" => CppType::LongLong { signed: false },
            "float" => CppType::Float,
            "double" => CppType::Double,
            _ => {
                // Named type (struct, class, typedef, etc.)
                CppType::Named(type_str.to_string())
            }
        }
    }

    /// Infer coroutine kind by examining the function body for co_await, co_yield, co_return.
    fn infer_coroutine_kind_from_body(&self, cursor: clang_sys::CXCursor) -> CoroutineInfo {
        unsafe {
            struct CoroutineExprData {
                has_co_await: bool,
                has_co_yield: bool,
                has_co_return: bool,
                parser: *const ClangParser,
            }

            extern "C" fn find_coroutine_exprs(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(data as *mut CoroutineExprData);
                    let parser = &*data.parser;

                    // Check if this is an UnexposedExpr or UnexposedStmt that might be a coroutine construct
                    let cursor_kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_UnexposedExpr = 100
                    if cursor_kind == 100 {
                        if let Some(kind) = parser.try_parse_coroutine_expr(child) {
                            match kind {
                                ClangNodeKind::CoawaitExpr { .. } => data.has_co_await = true,
                                ClangNodeKind::CoyieldExpr { .. } => data.has_co_yield = true,
                                _ => {}
                            }
                        }
                    }

                    // CXCursor_UnexposedStmt = 200
                    if cursor_kind == 200 {
                        if let Some(kind) = parser.try_parse_coroutine_stmt(child) {
                            if matches!(kind, ClangNodeKind::CoreturnStmt { .. }) {
                                data.has_co_return = true;
                            }
                        }
                    }

                    clang_sys::CXChildVisit_Recurse
                }
            }

            let mut data = CoroutineExprData {
                has_co_await: false,
                has_co_yield: false,
                has_co_return: false,
                parser: self as *const ClangParser,
            };

            clang_sys::clang_visitChildren(
                cursor,
                find_coroutine_exprs,
                &mut data as *mut CoroutineExprData as clang_sys::CXClientData,
            );

            // Determine coroutine kind based on what expressions are used
            let kind = if data.has_co_yield {
                CoroutineKind::Generator
            } else if data.has_co_await {
                CoroutineKind::Async
            } else if data.has_co_return {
                CoroutineKind::Task
            } else {
                CoroutineKind::Custom
            };

            // Get return type spelling from cursor
            let cursor_type = clang_sys::clang_getCursorType(cursor);
            let result_type = clang_sys::clang_getResultType(cursor_type);
            let type_spelling = clang_sys::clang_getTypeSpelling(result_type);
            let return_type_spelling = cx_string_to_string(type_spelling);

            CoroutineInfo {
                kind,
                value_type: None, // Can't determine without return type pattern matching
                return_type_spelling,
            }
        }
    }

    /// Try to evaluate an expression to a constant value.
    /// This is used for default arguments and other compile-time constants
    /// that appear as UnexposedExpr without children.
    fn try_evaluate_expr(&self, cursor: clang_sys::CXCursor) -> Option<ClangNodeKind> {
        unsafe {
            // Try to evaluate the expression
            let eval_result = clang_sys::clang_Cursor_Evaluate(cursor);
            if eval_result.is_null() {
                return None;
            }

            let kind = clang_sys::clang_EvalResult_getKind(eval_result);
            let cursor_ty = clang_sys::clang_getCursorType(cursor);
            let ty = self.convert_type(cursor_ty);

            // CXEval_Int = 1, CXEval_Float = 2
            let result = match kind {
                1 => {
                    // Integer result
                    let value = clang_sys::clang_EvalResult_getAsLongLong(eval_result);
                    Some(ClangNodeKind::EvaluatedExpr {
                        int_value: Some(value),
                        float_value: None,
                        ty,
                    })
                }
                2 => {
                    // Float result
                    let value = clang_sys::clang_EvalResult_getAsDouble(eval_result);
                    Some(ClangNodeKind::EvaluatedExpr {
                        int_value: None,
                        float_value: Some(value),
                        ty,
                    })
                }
                _ => None,
            };

            clang_sys::clang_EvalResult_dispose(eval_result);
            result
        }
    }

    /// Try to detect an implicit cast from an UnexposedExpr.
    /// libclang often exposes implicit casts as UnexposedExpr.
    /// We detect them by comparing the expression type with its child's type.
    fn try_parse_implicit_cast(&self, cursor: clang_sys::CXCursor) -> Option<ClangNodeKind> {
        unsafe {
            // Get the type of this expression
            let expr_ty = clang_sys::clang_getCursorType(cursor);
            let expr_type = self.convert_type(expr_ty);

            // Check if this cursor has exactly one child (typical for implicit casts)
            struct ChildInfo {
                count: usize,
                child_ty: Option<CppType>,
                parser: *const ClangParser,
            }

            extern "C" fn count_children(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut ChildInfo);
                    info.count += 1;
                    if info.count == 1 {
                        // Get the type of the first child
                        let child_ty = clang_sys::clang_getCursorType(child);
                        let parser = &*info.parser;
                        info.child_ty = Some(parser.convert_type(child_ty));
                    }
                    clang_sys::CXChildVisit_Continue
                }
            }

            let mut info = ChildInfo {
                count: 0,
                child_ty: None,
                parser: self as *const ClangParser,
            };

            clang_sys::clang_visitChildren(
                cursor,
                count_children,
                &mut info as *mut ChildInfo as clang_sys::CXClientData,
            );

            // Only treat as implicit cast if there's exactly one child with a different type
            if info.count == 1 {
                if let Some(child_type) = info.child_ty {
                    // Check if the types differ and need a cast
                    // Helper function to check if a named type is a size type
                    fn is_size_type(name: &str) -> bool {
                        matches!(
                            name,
                            "ptrdiff_t"
                                | "std::ptrdiff_t"
                                | "ssize_t"
                                | "size_t"
                                | "std::size_t"
                                | "intptr_t"
                                | "std::intptr_t"
                                | "uintptr_t"
                                | "std::uintptr_t"
                                | "difference_type"
                                | "size_type"
                        )
                    }

                    let needs_cast = match (&expr_type, &child_type) {
                        // Integral to integral (char to int, short to long, etc.)
                        (CppType::Int { .. }, CppType::Char { .. })
                        | (CppType::Int { .. }, CppType::Short { .. })
                        | (CppType::Long { .. }, CppType::Int { .. })
                        | (CppType::Long { .. }, CppType::Short { .. })
                        | (CppType::Long { .. }, CppType::Char { .. })
                        | (CppType::LongLong { .. }, CppType::Int { .. })
                        | (CppType::LongLong { .. }, CppType::Long { .. }) => {
                            Some(CastKind::IntegralCast)
                        }
                        // Named size types (ptrdiff_t, size_t) from integral
                        (CppType::Named(name), CppType::Int { .. })
                        | (CppType::Named(name), CppType::Short { .. })
                        | (CppType::Named(name), CppType::Char { .. })
                        | (CppType::Named(name), CppType::Long { .. })
                        | (CppType::Named(name), CppType::LongLong { .. })
                            if is_size_type(name) =>
                        {
                            Some(CastKind::IntegralCast)
                        }
                        // Integral to named size types
                        (CppType::Long { .. }, CppType::Named(name))
                        | (CppType::Int { .. }, CppType::Named(name))
                        | (CppType::LongLong { .. }, CppType::Named(name))
                            if is_size_type(name) =>
                        {
                            Some(CastKind::IntegralCast)
                        }
                        // Floating to floating
                        (CppType::Double, CppType::Float) | (CppType::Float, CppType::Double) => {
                            Some(CastKind::FloatingCast)
                        }
                        // Integral to floating
                        (CppType::Float, CppType::Int { .. })
                        | (CppType::Float, CppType::Long { .. })
                        | (CppType::Float, CppType::Char { .. })
                        | (CppType::Double, CppType::Int { .. })
                        | (CppType::Double, CppType::Long { .. })
                        | (CppType::Double, CppType::Char { .. }) => {
                            Some(CastKind::IntegralToFloating)
                        }
                        // Floating to integral
                        (CppType::Int { .. }, CppType::Float)
                        | (CppType::Int { .. }, CppType::Double)
                        | (CppType::Long { .. }, CppType::Float)
                        | (CppType::Long { .. }, CppType::Double) => {
                            Some(CastKind::FloatingToIntegral)
                        }
                        // Function to pointer (function pointer initialization/assignment)
                        (CppType::Pointer { pointee, .. }, CppType::Function { .. })
                            if matches!(pointee.as_ref(), CppType::Function { .. }) =>
                        {
                            Some(CastKind::FunctionToPointerDecay)
                        }
                        // Derived-to-base pointer cast (e.g., Dog* to Animal*)
                        (
                            CppType::Pointer {
                                pointee: base_pointee,
                                ..
                            },
                            CppType::Pointer {
                                pointee: derived_pointee,
                                ..
                            },
                        ) if matches!(base_pointee.as_ref(), CppType::Named(_))
                            && matches!(derived_pointee.as_ref(), CppType::Named(_)) =>
                        {
                            // Check if base and derived are different class names
                            if let (CppType::Named(base_name), CppType::Named(derived_name)) =
                                (base_pointee.as_ref(), derived_pointee.as_ref())
                            {
                                if base_name != derived_name {
                                    // This is a derived-to-base cast
                                    Some(CastKind::Other)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };

                    if let Some(cast_kind) = needs_cast {
                        return Some(ClangNodeKind::ImplicitCastExpr {
                            cast_kind,
                            ty: expr_type,
                        });
                    }
                }
            }

            None
        }
    }

    /// Get the exception type for a catch statement.
    /// Returns None for `catch(...)` (catch all).
    fn get_catch_exception_type(&self, cursor: clang_sys::CXCursor) -> Option<CppType> {
        unsafe {
            // The first child of a catch statement is the exception declaration (VarDecl)
            // or nothing for catch(...)
            struct CatchTypeData {
                ty: Option<CppType>,
                parser: *const ClangParser,
            }

            extern "C" fn find_catch_type(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(data as *mut CatchTypeData);
                    let child_kind = clang_sys::clang_getCursorKind(child);

                    // Look for VarDecl which contains the exception type
                    if child_kind == clang_sys::CXCursor_VarDecl {
                        let child_type = clang_sys::clang_getCursorType(child);
                        data.ty = Some((*data.parser).convert_type(child_type));
                        return clang_sys::CXChildVisit_Break;
                    }
                    clang_sys::CXChildVisit_Continue
                }
            }

            let mut data = CatchTypeData {
                ty: None,
                parser: self as *const ClangParser,
            };

            clang_sys::clang_visitChildren(
                cursor,
                find_catch_type,
                &mut data as *mut CatchTypeData as clang_sys::CXClientData,
            );

            data.ty
        }
    }

    /// Get the exception type for a throw expression.
    /// Returns None for `throw;` (rethrow).
    fn get_throw_exception_type(&self, cursor: clang_sys::CXCursor) -> Option<CppType> {
        unsafe {
            // The child of a throw expression is the expression being thrown
            // No child means it's a rethrow (throw;)
            struct ThrowTypeData {
                ty: Option<CppType>,
                parser: *const ClangParser,
            }

            extern "C" fn find_throw_type(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(data as *mut ThrowTypeData);
                    let child_type = clang_sys::clang_getCursorType(child);
                    data.ty = Some((*data.parser).convert_type(child_type));
                    clang_sys::CXChildVisit_Break
                }
            }

            let mut data = ThrowTypeData {
                ty: None,
                parser: self as *const ClangParser,
            };

            clang_sys::clang_visitChildren(
                cursor,
                find_throw_type,
                &mut data as *mut ThrowTypeData as clang_sys::CXClientData,
            );

            data.ty
        }
    }

    /// Check if a function body contains coroutine expressions (co_await, co_yield, co_return).
    fn contains_coroutine_expressions(&self, cursor: clang_sys::CXCursor) -> bool {
        unsafe {
            let mut found = false;
            let found_ptr: *mut bool = &mut found;

            extern "C" fn visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                client_data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                let found_ptr = client_data as *mut bool;
                let kind = unsafe { clang_sys::clang_getCursorKind(child) };

                // Check for coroutine expressions
                // CXCursor_CoawaitExpr = 281, CXCursor_CoyieldExpr = 282, CXCursor_CoreturnStmt = 279
                if kind == 281 || kind == 282 || kind == 279 {
                    unsafe { *found_ptr = true };
                    return clang_sys::CXChildVisit_Break;
                }
                clang_sys::CXChildVisit_Recurse
            }

            clang_sys::clang_visitChildren(cursor, visitor, found_ptr as clang_sys::CXClientData);
            found
        }
    }

    /// Get the access specifier for a cursor (field, method, etc.).
    fn get_access_specifier(&self, cursor: clang_sys::CXCursor) -> AccessSpecifier {
        unsafe {
            let access = clang_sys::clang_getCXXAccessSpecifier(cursor);
            match access {
                clang_sys::CX_CXXPublic => AccessSpecifier::Public,
                clang_sys::CX_CXXProtected => AccessSpecifier::Protected,
                clang_sys::CX_CXXPrivate => AccessSpecifier::Private,
                _ => AccessSpecifier::Private, // Default to private for invalid/unknown
            }
        }
    }

    /// Get the constructor kind for a constructor cursor.
    fn get_constructor_kind(&self, cursor: clang_sys::CXCursor) -> ConstructorKind {
        unsafe {
            if clang_sys::clang_CXXConstructor_isDefaultConstructor(cursor) != 0 {
                ConstructorKind::Default
            } else if clang_sys::clang_CXXConstructor_isCopyConstructor(cursor) != 0 {
                ConstructorKind::Copy
            } else if clang_sys::clang_CXXConstructor_isMoveConstructor(cursor) != 0 {
                ConstructorKind::Move
            } else {
                ConstructorKind::Other
            }
        }
    }

    /// Check if a CXXNewExpr is an array new (new T[n]).
    fn check_new_is_array(&self, cursor: clang_sys::CXCursor) -> bool {
        unsafe {
            // Get the translation unit from the cursor
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            // Get the source range and check tokens for '['
            let range = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = std::ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, range, &mut tokens, &mut num_tokens);

            let mut found_bracket = false;
            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let spelling = clang_sys::clang_getTokenSpelling(tu, token);
                let s = std::ffi::CStr::from_ptr(clang_sys::clang_getCString(spelling))
                    .to_string_lossy()
                    .to_string();
                clang_sys::clang_disposeString(spelling);

                if s == "[" {
                    found_bracket = true;
                    break;
                }
            }

            clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            found_bracket
        }
    }

    /// Check if a CXXNewExpr is placement new (new (ptr) T()).
    /// Placement new has the pattern: new (placement_args) Type(constructor_args)
    /// We detect this by looking for '(' immediately after 'new' keyword.
    fn check_new_is_placement(&self, cursor: clang_sys::CXCursor) -> bool {
        unsafe {
            // Get the translation unit from the cursor
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            // Get the source range and check tokens for placement syntax
            let range = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = std::ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, range, &mut tokens, &mut num_tokens);

            // Placement new pattern: "new" followed by "(" before any identifier (the type)
            let mut saw_new = false;
            let mut is_placement = false;

            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let spelling = clang_sys::clang_getTokenSpelling(tu, token);
                let s = std::ffi::CStr::from_ptr(clang_sys::clang_getCString(spelling))
                    .to_string_lossy()
                    .to_string();
                clang_sys::clang_disposeString(spelling);

                if s == "new" {
                    saw_new = true;
                } else if saw_new {
                    // After 'new', if we see '(' immediately, it's placement new
                    // If we see an identifier (type) first, it's regular new
                    if s == "(" {
                        is_placement = true;
                        break;
                    } else if s == "[" {
                        // Array new: new T[n] - skip, not placement
                        break;
                    } else {
                        // Type name or other token - not placement new
                        break;
                    }
                }
            }

            clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            is_placement
        }
    }

    /// Check if a CXXDeleteExpr is an array delete (delete[]).
    fn check_delete_is_array(&self, cursor: clang_sys::CXCursor) -> bool {
        unsafe {
            // Get the translation unit from the cursor
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            // Get the source range and check tokens for '[]'
            let range = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = std::ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, range, &mut tokens, &mut num_tokens);

            let mut found_bracket = false;
            let mut prev_was_open = false;
            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let spelling = clang_sys::clang_getTokenSpelling(tu, token);
                let s = std::ffi::CStr::from_ptr(clang_sys::clang_getCString(spelling))
                    .to_string_lossy()
                    .to_string();
                clang_sys::clang_disposeString(spelling);

                if s == "[" {
                    prev_was_open = true;
                } else if s == "]" && prev_was_open {
                    found_bracket = true;
                    break;
                } else {
                    prev_was_open = false;
                }
            }

            clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            found_bracket
        }
    }

    /// Get the parent class name for a member cursor (constructor, destructor, method).
    fn get_parent_class_name(&self, cursor: clang_sys::CXCursor) -> String {
        unsafe {
            let parent = clang_sys::clang_getCursorSemanticParent(cursor);
            cursor_spelling(parent)
        }
    }

    /// Extract function parameters from a cursor (function, constructor, etc.).
    fn extract_params(&self, cursor: clang_sys::CXCursor) -> Vec<(String, CppType)> {
        unsafe {
            let num_args = clang_sys::clang_Cursor_getNumArguments(cursor);
            let mut params = Vec::new();
            for i in 0..num_args {
                let arg = clang_sys::clang_Cursor_getArgument(cursor, i as u32);
                let arg_name = cursor_spelling(arg);
                let arg_type = clang_sys::clang_getCursorType(arg);
                params.push((arg_name, self.convert_type(arg_type)));
            }
            params
        }
    }

    /// Get the namespace path from a UsingDirective cursor by examining its children.
    fn get_using_directive_namespace(&self, cursor: clang_sys::CXCursor) -> Vec<String> {
        unsafe {
            // The using directive has NamespaceRef children pointing to the namespace(s)
            // We need to visit children and collect namespace names
            let mut namespace_path = Vec::new();
            let namespace_path_ptr: *mut Vec<String> = &mut namespace_path;

            extern "C" fn namespace_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let path = &mut *(data as *mut Vec<String>);
                    let kind = clang_sys::clang_getCursorKind(child);

                    if kind == clang_sys::CXCursor_NamespaceRef {
                        let name = cursor_spelling(child);
                        if !name.is_empty() {
                            path.push(name);
                        }
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                namespace_visitor,
                namespace_path_ptr as clang_sys::CXClientData,
            );

            namespace_path
        }
    }

    /// Get the qualified name from a UsingDeclaration cursor.
    fn get_qualified_name(&self, cursor: clang_sys::CXCursor) -> Vec<String> {
        unsafe {
            // Get the referenced declaration
            let referenced = clang_sys::clang_getCursorReferenced(cursor);
            if clang_sys::clang_Cursor_isNull(referenced) != 0 {
                return Vec::new();
            }

            // Build the qualified name path
            self.build_namespace_path(referenced)
        }
    }

    /// Build a namespace path by traversing semantic parents.
    /// Returns the full qualified path for the given cursor.
    fn build_namespace_path(&self, cursor: clang_sys::CXCursor) -> Vec<String> {
        unsafe {
            let mut path = Vec::new();
            let mut current = cursor;

            // First, add the cursor itself if it's a namespace
            let kind = clang_sys::clang_getCursorKind(current);
            if kind == clang_sys::CXCursor_Namespace {
                let name = cursor_spelling(current);
                if !name.is_empty() {
                    path.push(name);
                }
            }

            // Traverse up through parents to build the full path
            loop {
                // Move to parent
                let parent = clang_sys::clang_getCursorSemanticParent(current);
                let parent_kind = clang_sys::clang_getCursorKind(parent);

                // Stop at translation unit or if parent is same as current
                if parent_kind == clang_sys::CXCursor_TranslationUnit
                    || clang_sys::clang_Cursor_isNull(parent) != 0
                    || clang_sys::clang_equalCursors(current, parent) != 0
                {
                    break;
                }

                // Add parent namespace to path
                if parent_kind == clang_sys::CXCursor_Namespace {
                    let name = cursor_spelling(parent);
                    if !name.is_empty() {
                        path.push(name);
                    }
                }

                current = parent;
            }

            // Reverse to get outermost first (outer::inner -> ["outer", "inner"])
            path.reverse();
            path
        }
    }

    /// Get override and final attributes from a method cursor.
    /// Returns (is_override, is_final).
    fn get_override_final_attrs(&self, cursor: clang_sys::CXCursor) -> (bool, bool) {
        unsafe {
            struct AttrInfo {
                is_override: bool,
                is_final: bool,
            }
            let mut info = AttrInfo {
                is_override: false,
                is_final: false,
            };
            let info_ptr: *mut AttrInfo = &mut info;

            extern "C" fn attr_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut AttrInfo);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_CXXOverrideAttr = 405
                    // CXCursor_CXXFinalAttr = 404
                    match kind {
                        404 => info.is_final = true,
                        405 => info.is_override = true,
                        _ => {}
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                attr_visitor,
                info_ptr as clang_sys::CXClientData,
            );

            (info.is_override, info.is_final)
        }
    }

    /// Convert a type with template parameter awareness.
    ///
    /// If the type spelling matches a known template parameter, returns a
    /// TemplateParam variant instead of Named.
    fn convert_type_with_template_ctx(
        &self,
        ty: clang_sys::CXType,
        template_params: &[String],
    ) -> CppType {
        unsafe {
            let kind = ty.kind;

            // First, handle structural types (pointers, references, arrays) by
            // recursively processing the inner type with template context
            match kind {
                clang_sys::CXType_Pointer => {
                    let pointee = clang_sys::clang_getPointeeType(ty);
                    let is_const = clang_sys::clang_isConstQualifiedType(pointee) != 0;
                    return CppType::Pointer {
                        pointee: Box::new(
                            self.convert_type_with_template_ctx(pointee, template_params),
                        ),
                        is_const,
                    };
                }

                clang_sys::CXType_LValueReference | clang_sys::CXType_RValueReference => {
                    let referent = clang_sys::clang_getPointeeType(ty);
                    let is_const = clang_sys::clang_isConstQualifiedType(referent) != 0;
                    let is_rvalue = kind == clang_sys::CXType_RValueReference;
                    return CppType::Reference {
                        referent: Box::new(
                            self.convert_type_with_template_ctx(referent, template_params),
                        ),
                        is_const,
                        is_rvalue,
                    };
                }

                clang_sys::CXType_ConstantArray => {
                    let element = clang_sys::clang_getArrayElementType(ty);
                    let size = clang_sys::clang_getArraySize(ty) as usize;
                    return CppType::Array {
                        element: Box::new(
                            self.convert_type_with_template_ctx(element, template_params),
                        ),
                        size: Some(size),
                    };
                }

                clang_sys::CXType_IncompleteArray => {
                    let element = clang_sys::clang_getArrayElementType(ty);
                    return CppType::Array {
                        element: Box::new(
                            self.convert_type_with_template_ctx(element, template_params),
                        ),
                        size: None,
                    };
                }

                _ => {}
            }

            // For non-structural types, check the spelling
            let spelling = clang_sys::clang_getTypeSpelling(ty);
            let type_name = cx_string_to_string(spelling);

            // Strip "const " prefix for matching purposes
            // The const qualifier is already captured in the parent type
            let base_name = type_name.trim_start_matches("const ").to_string();

            // Check if this is a template parameter by name
            if let Some(index) = template_params.iter().position(|p| p == &base_name) {
                return CppType::TemplateParam {
                    name: base_name,
                    depth: 0,
                    index: index as u32,
                };
            }

            // Check for internal template parameter format: "type-parameter-{depth}-{index}"
            // libclang uses this format for template parameters in partial specializations
            if base_name.starts_with("type-parameter-") {
                let parts: Vec<_> = base_name["type-parameter-".len()..].split('-').collect();
                if parts.len() == 2 {
                    if let (Ok(depth), Ok(index)) =
                        (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                    {
                        // Map the index to the template parameter name
                        let name = if (index as usize) < template_params.len() {
                            template_params[index as usize].clone()
                        } else {
                            base_name.clone()
                        };
                        return CppType::TemplateParam { name, depth, index };
                    }
                }
            }

            // Check for dependent types (types that contain template params)
            let is_dependent = template_params.iter().any(|p| base_name.contains(p));

            if is_dependent {
                // For now, store dependent types with their full spelling
                // A more sophisticated approach would parse and reconstruct the type
                CppType::DependentType {
                    spelling: type_name,
                }
            } else {
                self.convert_type(ty)
            }
        }
    }

    /// Get function parameters from a function template cursor.
    ///
    /// For function templates, `clang_Cursor_getNumArguments` returns 0, so we need
    /// to extract parameters from the ParmVarDecl children.
    fn get_function_template_params(
        &self,
        cursor: clang_sys::CXCursor,
        template_params: &[String],
    ) -> Vec<(String, CppType)> {
        unsafe {
            struct ParamData<'a> {
                params: Vec<(String, CppType)>,
                parser: &'a ClangParser,
                template_params: &'a [String],
            }

            let mut data = ParamData {
                params: Vec::new(),
                parser: self,
                template_params,
            };
            let data_ptr: *mut ParamData = &mut data;

            extern "C" fn param_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                client_data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(client_data as *mut ParamData);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_ParmDecl = 10
                    if kind == 10 {
                        let name = cursor_spelling(child);
                        let ty = clang_sys::clang_getCursorType(child);
                        let cpp_type = data
                            .parser
                            .convert_type_with_template_ctx(ty, data.template_params);
                        data.params.push((name, cpp_type));
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                param_visitor,
                data_ptr as clang_sys::CXClientData,
            );

            data.params
        }
    }

    /// Get template type parameters from a function/class template cursor.
    /// Returns the names of the template type parameters (e.g., ["T", "U"]).
    fn get_template_type_params(&self, cursor: clang_sys::CXCursor) -> Vec<String> {
        let (names, _) = self.get_template_type_params_with_packs(cursor);
        names
    }

    /// Get template type parameters with pack info from a function/class template cursor.
    /// Returns (parameter names, indices of parameter packs).
    ///
    /// Note: We need to tokenize each template type parameter's extent to detect "..."
    /// because libclang doesn't expose isParameterPack() in its C API.
    fn get_template_type_params_with_packs(
        &self,
        cursor: clang_sys::CXCursor,
    ) -> (Vec<String>, Vec<usize>) {
        unsafe {
            // Get the translation unit for tokenization
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);

            struct ParamInfo {
                names: Vec<String>,
                pack_indices: Vec<usize>,
                tu: clang_sys::CXTranslationUnit,
            }
            let mut info = ParamInfo {
                names: Vec::new(),
                pack_indices: Vec::new(),
                tu,
            };
            let info_ptr: *mut ParamInfo = &mut info;

            extern "C" fn param_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut ParamInfo);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_TemplateTypeParameter = 27
                    if kind == 27 {
                        let name = cursor_spelling(child);
                        if !name.is_empty() {
                            let index = info.names.len();

                            // Tokenize the extent to detect "..." token
                            let extent = clang_sys::clang_getCursorExtent(child);
                            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
                            let mut num_tokens: u32 = 0;

                            clang_sys::clang_tokenize(
                                info.tu,
                                extent,
                                &mut tokens,
                                &mut num_tokens,
                            );

                            let mut is_pack = false;
                            for i in 0..num_tokens {
                                let token = *tokens.add(i as usize);
                                let token_spelling =
                                    clang_sys::clang_getTokenSpelling(info.tu, token);
                                let token_str = cx_string_to_string(token_spelling);
                                if token_str == "..." {
                                    is_pack = true;
                                    break;
                                }
                            }

                            if !tokens.is_null() {
                                clang_sys::clang_disposeTokens(info.tu, tokens, num_tokens);
                            }

                            if is_pack {
                                info.pack_indices.push(index);
                            }
                            info.names.push(name);
                        }
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                param_visitor,
                info_ptr as clang_sys::CXClientData,
            );

            (info.names, info.pack_indices)
        }
    }

    /// Get the template specialization arguments from a partial specialization cursor.
    /// Returns the types used in the specialization pattern (e.g., [T, T] for Pair<T, T>).
    fn get_template_specialization_args(&self, cursor: clang_sys::CXCursor) -> Vec<CppType> {
        unsafe {
            // Get the specialized type (e.g., Pair<T, T>)
            let cursor_type = clang_sys::clang_getCursorType(cursor);

            // Get the number of template arguments
            let num_args = clang_sys::clang_Type_getNumTemplateArguments(cursor_type);
            if num_args < 0 {
                return Vec::new();
            }

            // Get the template parameters for this partial specialization
            let template_params = self.get_template_type_params(cursor);

            // Extract each template argument
            let mut args = Vec::new();
            for i in 0..num_args {
                let arg_type =
                    clang_sys::clang_Type_getTemplateArgumentAsType(cursor_type, i as u32);
                // Use template-aware conversion to detect template parameter references
                let cpp_type = self.convert_type_with_template_ctx(arg_type, &template_params);
                args.push(cpp_type);
            }

            args
        }
    }

    /// Get friend information from a FriendDecl cursor.
    /// Returns (friend_class, friend_function) where one will be Some and the other None.
    fn get_friend_info(&self, cursor: clang_sys::CXCursor) -> (Option<String>, Option<String>) {
        unsafe {
            // Structure to pass both options through the visitor
            struct FriendInfo {
                friend_class: Option<String>,
                friend_function: Option<String>,
            }
            let mut info = FriendInfo {
                friend_class: None,
                friend_function: None,
            };
            let info_ptr: *mut FriendInfo = &mut info;

            extern "C" fn friend_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut FriendInfo);
                    let kind = clang_sys::clang_getCursorKind(child);

                    match kind {
                        // Friend class declaration
                        // CXCursor_ClassDecl = 4, CXCursor_StructDecl = 2
                        // Also check for ClassTemplate (31) which can appear for forward-declared friends
                        clang_sys::CXCursor_ClassDecl
                        | clang_sys::CXCursor_StructDecl
                        | clang_sys::CXCursor_ClassTemplate => {
                            let name = cursor_spelling(child);
                            if !name.is_empty() {
                                info.friend_class = Some(name);
                            }
                        }
                        // Friend function declaration
                        clang_sys::CXCursor_FunctionDecl => {
                            let name = cursor_spelling(child);
                            if !name.is_empty() {
                                info.friend_function = Some(name);
                            }
                        }
                        // For types, sometimes the type reference appears instead
                        clang_sys::CXCursor_TypeRef => {
                            let name = cursor_spelling(child);
                            if !name.is_empty() && info.friend_class.is_none() {
                                // Strip "class " or "struct " prefix if present
                                let name = name
                                    .strip_prefix("class ")
                                    .or_else(|| name.strip_prefix("struct "))
                                    .unwrap_or(&name)
                                    .to_string();
                                info.friend_class = Some(name);
                            }
                        }
                        _ => {}
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                friend_visitor,
                info_ptr as clang_sys::CXClientData,
            );

            (info.friend_class, info.friend_function)
        }
    }

    // ========== C++20 Concepts Support ==========

    /// Get the requires clause from a template cursor, if present.
    /// Returns the constraint expression as a string, or None if no requires clause.
    fn get_requires_clause(&self, cursor: clang_sys::CXCursor) -> Option<String> {
        unsafe {
            struct RequiresInfo {
                constraint: Option<String>,
                tu: clang_sys::CXTranslationUnit,
            }
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let mut info = RequiresInfo {
                constraint: None,
                tu,
            };
            let info_ptr: *mut RequiresInfo = &mut info;

            extern "C" fn requires_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut RequiresInfo);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_RequiresExpr = 279, CXCursor_ConceptSpecializationExpr = 602
                    // These are the only cursors that represent requires clauses
                    if kind == 279 || kind == 602 {
                        // Found a constraint - extract the text from the source range
                        let extent = clang_sys::clang_getCursorExtent(child);
                        let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
                        let mut num_tokens: u32 = 0;

                        clang_sys::clang_tokenize(info.tu, extent, &mut tokens, &mut num_tokens);

                        let mut constraint_parts = Vec::new();
                        for i in 0..num_tokens {
                            let token = *tokens.add(i as usize);
                            let spelling = clang_sys::clang_getTokenSpelling(info.tu, token);
                            constraint_parts.push(cx_string_to_string(spelling));
                        }

                        if !tokens.is_null() {
                            clang_sys::clang_disposeTokens(info.tu, tokens, num_tokens);
                        }

                        if !constraint_parts.is_empty() {
                            info.constraint = Some(constraint_parts.join(" "));
                            return clang_sys::CXChildVisit_Break;
                        }
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                requires_visitor,
                info_ptr as clang_sys::CXClientData,
            );

            info.constraint
        }
    }

    /// Get the constraint expression from a concept declaration.
    fn get_concept_constraint_expr(&self, cursor: clang_sys::CXCursor) -> String {
        unsafe {
            // Tokenize the entire cursor extent to get the constraint expression
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            let mut all_tokens = Vec::new();
            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let spelling = clang_sys::clang_getTokenSpelling(tu, token);
                all_tokens.push(cx_string_to_string(spelling));
            }

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            // The constraint is everything after "=" in the concept definition
            // e.g., "template < typename T > concept Integral = __is_integral ( T )"
            // We want: "__is_integral ( T )"
            if let Some(eq_pos) = all_tokens.iter().position(|t| t == "=") {
                all_tokens[eq_pos + 1..].join(" ")
            } else {
                String::new()
            }
        }
    }

    /// Get parameters from a requires expression.
    fn get_requires_expr_params(&self, cursor: clang_sys::CXCursor) -> Vec<(String, CppType)> {
        unsafe {
            struct ParamData<'a> {
                params: Vec<(String, CppType)>,
                parser: &'a ClangParser,
            }
            let mut data = ParamData {
                params: Vec::new(),
                parser: self,
            };
            let data_ptr: *mut ParamData = &mut data;

            extern "C" fn param_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                client_data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(client_data as *mut ParamData);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_ParmDecl = 10
                    if kind == 10 {
                        let name = cursor_spelling(child);
                        let ty = clang_sys::clang_getCursorType(child);
                        let cpp_type = data.parser.convert_type(ty);
                        data.params.push((name, cpp_type));
                    }

                    // Only visit direct children for parameters
                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                param_visitor,
                data_ptr as clang_sys::CXClientData,
            );

            data.params
        }
    }

    /// Get requirements from a requires expression.
    fn get_requirements(&self, cursor: clang_sys::CXCursor) -> Vec<Requirement> {
        unsafe {
            struct ReqData<'a> {
                requirements: Vec<Requirement>,
                tu: clang_sys::CXTranslationUnit,
                #[allow(dead_code)]
                parser: &'a ClangParser,
            }
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let mut data = ReqData {
                requirements: Vec::new(),
                tu,
                parser: self,
            };
            let data_ptr: *mut ReqData = &mut data;

            extern "C" fn req_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                client_data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(client_data as *mut ReqData);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // Skip parameter declarations
                    if kind == 10 {
                        return clang_sys::CXChildVisit_Continue;
                    }

                    // Extract the text of each requirement from tokens
                    let extent = clang_sys::clang_getCursorExtent(child);
                    let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
                    let mut num_tokens: u32 = 0;

                    clang_sys::clang_tokenize(data.tu, extent, &mut tokens, &mut num_tokens);

                    let mut token_strs = Vec::new();
                    for i in 0..num_tokens {
                        let token = *tokens.add(i as usize);
                        let spelling = clang_sys::clang_getTokenSpelling(data.tu, token);
                        token_strs.push(cx_string_to_string(spelling));
                    }

                    if !tokens.is_null() {
                        clang_sys::clang_disposeTokens(data.tu, tokens, num_tokens);
                    }

                    if !token_strs.is_empty() {
                        let expr = token_strs.join(" ");

                        // Detect requirement type based on tokens
                        if token_strs.first().map(|s| s.as_str()) == Some("typename") {
                            // Type requirement
                            data.requirements.push(Requirement::Type {
                                type_name: token_strs[1..].join(" "),
                            });
                        } else if token_strs.first().map(|s| s.as_str()) == Some("{") {
                            // Compound requirement
                            let is_noexcept = token_strs.contains(&"noexcept".to_string());
                            let return_constraint = if let Some(arrow_pos) =
                                token_strs.iter().position(|t| t == "->")
                            {
                                Some(token_strs[arrow_pos + 1..].join(" "))
                            } else {
                                None
                            };
                            data.requirements.push(Requirement::Compound {
                                expr,
                                is_noexcept,
                                return_constraint,
                            });
                        } else if token_strs.first().map(|s| s.as_str()) == Some("requires") {
                            // Nested requirement
                            data.requirements.push(Requirement::Nested {
                                constraint: token_strs[1..].join(" "),
                            });
                        } else {
                            // Simple requirement
                            data.requirements.push(Requirement::Simple { expr });
                        }
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                req_visitor,
                data_ptr as clang_sys::CXClientData,
            );

            data.requirements
        }
    }

    /// Get concept specialization information (concept name and template arguments).
    fn get_concept_specialization_info(
        &self,
        cursor: clang_sys::CXCursor,
    ) -> (String, Vec<CppType>) {
        unsafe {
            // Get the concept name from the cursor spelling
            let concept_name = cursor_spelling(cursor);

            // Get template arguments from the type
            let cursor_type = clang_sys::clang_getCursorType(cursor);
            let num_args = clang_sys::clang_Type_getNumTemplateArguments(cursor_type);

            let mut template_args = Vec::new();
            if num_args > 0 {
                for i in 0..num_args {
                    let arg_type =
                        clang_sys::clang_Type_getTemplateArgumentAsType(cursor_type, i as u32);
                    template_args.push(self.convert_type(arg_type));
                }
            }

            (concept_name, template_args)
        }
    }

    // ========== Type Alias Support ==========

    /// Get the underlying type of a typedef or type alias declaration.
    fn get_typedef_underlying_type(&self, cursor: clang_sys::CXCursor) -> CppType {
        unsafe {
            let typedef_type = clang_sys::clang_getTypedefDeclUnderlyingType(cursor);
            self.convert_type(typedef_type)
        }
    }

    /// Get the underlying type of a type alias template declaration.
    /// This needs special handling because the underlying type may reference template parameters.
    fn get_type_alias_template_underlying_type(
        &self,
        cursor: clang_sys::CXCursor,
        template_params: &[String],
    ) -> CppType {
        unsafe {
            // For type alias templates, we need to find the TypeAliasDecl child
            // and get its underlying type with template parameter context
            struct AliasInfo {
                underlying_type: Option<CppType>,
                parser: *const ClangParser,
                template_params: *const [String],
            }

            let mut info = AliasInfo {
                underlying_type: None,
                parser: self,
                template_params,
            };
            let info_ptr: *mut AliasInfo = &mut info;

            extern "C" fn alias_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut AliasInfo);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_TypeAliasDecl = 36
                    if kind == clang_sys::CXCursor_TypeAliasDecl {
                        let typedef_type = clang_sys::clang_getTypedefDeclUnderlyingType(child);
                        let parser = &*info.parser;
                        let template_params = &*info.template_params;
                        info.underlying_type = Some(
                            parser.convert_type_with_template_ctx(typedef_type, template_params),
                        );
                        return clang_sys::CXChildVisit_Break;
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                alias_visitor,
                info_ptr as clang_sys::CXClientData,
            );

            info.underlying_type.unwrap_or(CppType::Void)
        }
    }
}

impl Drop for ClangParser {
    fn drop(&mut self) {
        unsafe {
            clang_sys::clang_disposeIndex(self.index);
        }
    }
}

/// Convert a CXString to a Rust String.
fn cx_string_to_string(cx_string: clang_sys::CXString) -> String {
    unsafe {
        let c_str = clang_sys::clang_getCString(cx_string);
        let result = if c_str.is_null() {
            String::new()
        } else {
            CStr::from_ptr(c_str).to_string_lossy().into_owned()
        };
        clang_sys::clang_disposeString(cx_string);
        result
    }
}

/// Get the spelling of a cursor.
fn cursor_spelling(cursor: clang_sys::CXCursor) -> String {
    unsafe {
        let spelling = clang_sys::clang_getCursorSpelling(cursor);
        cx_string_to_string(spelling)
    }
}

/// Get the mangled name of a cursor (for function declarations).
///
/// Uses libclang's `clang_Cursor_getMangling` to get the platform-specific
/// mangled name (e.g., "_Z3addii" for `int add(int, int)` on Linux/Itanium ABI).
fn cursor_mangled_name(cursor: clang_sys::CXCursor) -> String {
    unsafe {
        let mangled = clang_sys::clang_Cursor_getMangling(cursor);
        let name = cx_string_to_string(mangled);
        // If mangling returns empty string (e.g., for extern "C"), use display name
        if name.is_empty() {
            cursor_spelling(cursor)
        } else {
            name
        }
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)] // Test values that happen to be close to PI aren't using PI
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int add(int a, int b) {
                    return a + b;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Check that we got a translation unit
        match &ast.translation_unit.kind {
            ClangNodeKind::TranslationUnit => {}
            _ => panic!("Expected TranslationUnit"),
        }
    }

    #[test]
    fn test_parse_namespace() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                namespace foo {
                    int bar() {
                        return 42;
                    }
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Check that we got a translation unit
        assert!(matches!(
            ast.translation_unit.kind,
            ClangNodeKind::TranslationUnit
        ));

        // Should have a namespace child
        assert!(!ast.translation_unit.children.is_empty());
        let ns = &ast.translation_unit.children[0];
        match &ns.kind {
            ClangNodeKind::NamespaceDecl { name } => {
                assert_eq!(name.as_deref(), Some("foo"));
            }
            _ => panic!("Expected NamespaceDecl, got {:?}", ns.kind),
        }

        // Namespace should have a function child
        assert!(!ns.children.is_empty());
        match &ns.children[0].kind {
            ClangNodeKind::FunctionDecl {
                name, mangled_name, ..
            } => {
                assert_eq!(name, "bar");
                // C++ mangling: namespace foo, function bar() -> _ZN3foo3barEv
                assert_eq!(mangled_name, "_ZN3foo3barEv", "Expected C++ mangled name");
            }
            _ => panic!("Expected FunctionDecl inside namespace"),
        }
    }

    #[test]
    fn test_mangled_name_for_simple_function() {
        // Test that mangled names are correctly extracted for functions
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int add_cpp(int a, int b) {
                    return a + b;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the function declaration
        for child in &ast.translation_unit.children {
            if let ClangNodeKind::FunctionDecl {
                name, mangled_name, ..
            } = &child.kind
            {
                assert_eq!(name, "add_cpp");
                // C++ mangling: add_cpp(int, int) -> _Z7add_cppii
                assert_eq!(
                    mangled_name, "_Z7add_cppii",
                    "Expected C++ mangled name for add_cpp(int, int)"
                );
                return;
            }
        }
        panic!("Expected to find function declaration");
    }

    #[test]
    fn test_parse_anonymous_namespace() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                namespace {
                    int hidden() {
                        return 0;
                    }
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        assert!(!ast.translation_unit.children.is_empty());
        let ns = &ast.translation_unit.children[0];
        match &ns.kind {
            ClangNodeKind::NamespaceDecl { name } => {
                assert!(name.is_none(), "Expected anonymous namespace");
            }
            _ => panic!("Expected NamespaceDecl"),
        }
    }

    #[test]
    fn test_parse_using_namespace() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                namespace foo {
                    int x;
                }
                using namespace foo;
                "#,
                "test.cpp",
            )
            .unwrap();

        // Should have namespace and using directive as children
        assert!(ast.translation_unit.children.len() >= 2);

        // Find the using directive
        let using_dir = ast
            .translation_unit
            .children
            .iter()
            .find(|c| matches!(&c.kind, ClangNodeKind::UsingDirective { .. }));

        assert!(using_dir.is_some(), "Expected UsingDirective");
        match &using_dir.unwrap().kind {
            ClangNodeKind::UsingDirective { namespace } => {
                assert_eq!(namespace, &vec!["foo"]);
            }
            _ => panic!("Expected UsingDirective"),
        }
    }

    #[test]
    fn test_parse_using_nested_namespace() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                namespace outer {
                    namespace inner {
                        int x;
                    }
                }
                using namespace outer::inner;
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the using directive
        let using_dir = ast
            .translation_unit
            .children
            .iter()
            .find(|c| matches!(&c.kind, ClangNodeKind::UsingDirective { .. }));

        assert!(using_dir.is_some(), "Expected UsingDirective");
        match &using_dir.unwrap().kind {
            ClangNodeKind::UsingDirective { namespace } => {
                assert_eq!(namespace, &vec!["outer", "inner"]);
            }
            _ => panic!("Expected UsingDirective"),
        }
    }

    #[test]
    fn test_parse_constructor_with_initializer_list() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                class Test {
                public:
                    int x;
                    int y;
                    Test(int a, int b) : x(a), y(b) { }
                };
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the class
        let class =
            ast.translation_unit.children.iter().find(
                |c| matches!(&c.kind, ClangNodeKind::RecordDecl { name, .. } if name == "Test"),
            );
        assert!(class.is_some(), "Expected Test class");
        let class = class.unwrap();

        // Find the constructor
        let ctor = class
            .children
            .iter()
            .find(|c| matches!(&c.kind, ClangNodeKind::ConstructorDecl { .. }));
        assert!(ctor.is_some(), "Expected constructor");
        let ctor = ctor.unwrap();

        // Should have MemberRef nodes for x and y
        let member_refs: Vec<_> = ctor
            .children
            .iter()
            .filter(|c| matches!(&c.kind, ClangNodeKind::MemberRef { .. }))
            .collect();
        assert_eq!(
            member_refs.len(),
            2,
            "Expected 2 MemberRef nodes for x and y"
        );

        // Verify the member names
        let member_names: Vec<String> = member_refs
            .iter()
            .filter_map(|c| {
                if let ClangNodeKind::MemberRef { name } = &c.kind {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(member_names.contains(&"x".to_string()));
        assert!(member_names.contains(&"y".to_string()));
    }

    #[test]
    fn test_integer_literal_type_int() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int foo() {
                    return 42;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the integer literal
        fn find_int_literal(node: &ClangNode) -> Option<&ClangNode> {
            if matches!(&node.kind, ClangNodeKind::IntegerLiteral { .. }) {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_int_literal(child) {
                    return Some(found);
                }
            }
            None
        }

        let literal = find_int_literal(&ast.translation_unit).expect("Expected IntegerLiteral");
        if let ClangNodeKind::IntegerLiteral { value, cpp_type } = &literal.kind {
            assert_eq!(*value, 42);
            let ty = cpp_type.as_ref().expect("Expected type info");
            assert_eq!(ty.bit_width(), Some(32));
            assert_eq!(ty.is_signed(), Some(true));
        } else {
            panic!("Expected IntegerLiteral");
        }
    }

    #[test]
    fn test_integer_literal_type_unsigned() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                unsigned int foo() {
                    return 4294967295u;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the integer literal
        fn find_int_literal(node: &ClangNode) -> Option<&ClangNode> {
            if matches!(&node.kind, ClangNodeKind::IntegerLiteral { .. }) {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_int_literal(child) {
                    return Some(found);
                }
            }
            None
        }

        let literal = find_int_literal(&ast.translation_unit).expect("Expected IntegerLiteral");
        if let ClangNodeKind::IntegerLiteral { value, cpp_type } = &literal.kind {
            assert_eq!(*value, 4294967295i128);
            let ty = cpp_type.as_ref().expect("Expected type info");
            assert_eq!(ty.bit_width(), Some(32));
            assert_eq!(ty.is_signed(), Some(false));
        } else {
            panic!("Expected IntegerLiteral");
        }
    }

    #[test]
    fn test_integer_literal_type_long() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                long foo() {
                    return 9223372036854775807L;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the integer literal
        fn find_int_literal(node: &ClangNode) -> Option<&ClangNode> {
            if matches!(&node.kind, ClangNodeKind::IntegerLiteral { .. }) {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_int_literal(child) {
                    return Some(found);
                }
            }
            None
        }

        let literal = find_int_literal(&ast.translation_unit).expect("Expected IntegerLiteral");
        if let ClangNodeKind::IntegerLiteral { value, cpp_type } = &literal.kind {
            assert_eq!(*value, 9223372036854775807i128);
            let ty = cpp_type.as_ref().expect("Expected type info");
            assert_eq!(ty.bit_width(), Some(64));
            assert_eq!(ty.is_signed(), Some(true));
        } else {
            panic!("Expected IntegerLiteral");
        }
    }

    #[test]
    fn test_integer_literal_type_unsigned_long() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                unsigned long foo() {
                    return 18446744073709551615UL;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the integer literal
        fn find_int_literal(node: &ClangNode) -> Option<&ClangNode> {
            if matches!(&node.kind, ClangNodeKind::IntegerLiteral { .. }) {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_int_literal(child) {
                    return Some(found);
                }
            }
            None
        }

        let literal = find_int_literal(&ast.translation_unit).expect("Expected IntegerLiteral");
        if let ClangNodeKind::IntegerLiteral { value, cpp_type } = &literal.kind {
            // Note: value is stored as i128, so max u64 fits
            assert_eq!(*value as u64, 18446744073709551615u64);
            let ty = cpp_type.as_ref().expect("Expected type info");
            assert_eq!(ty.bit_width(), Some(64));
            assert_eq!(ty.is_signed(), Some(false));
        } else {
            panic!("Expected IntegerLiteral");
        }
    }

    #[test]
    fn test_float_literal_type_float() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                float foo() {
                    return 3.14f;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the floating literal
        fn find_float_literal(node: &ClangNode) -> Option<&ClangNode> {
            if matches!(&node.kind, ClangNodeKind::FloatingLiteral { .. }) {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_float_literal(child) {
                    return Some(found);
                }
            }
            None
        }

        let literal = find_float_literal(&ast.translation_unit).expect("Expected FloatingLiteral");
        if let ClangNodeKind::FloatingLiteral { value, cpp_type } = &literal.kind {
            assert!((*value - 3.14).abs() < 0.01);
            let ty = cpp_type.as_ref().expect("Expected type info");
            assert_eq!(ty.bit_width(), Some(32), "float should be 32 bits");
        } else {
            panic!("Expected FloatingLiteral");
        }
    }

    #[test]
    fn test_float_literal_type_double() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                double foo() {
                    return 3.14159265358979;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the floating literal
        fn find_float_literal(node: &ClangNode) -> Option<&ClangNode> {
            if matches!(&node.kind, ClangNodeKind::FloatingLiteral { .. }) {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_float_literal(child) {
                    return Some(found);
                }
            }
            None
        }

        let literal = find_float_literal(&ast.translation_unit).expect("Expected FloatingLiteral");
        if let ClangNodeKind::FloatingLiteral { value, cpp_type } = &literal.kind {
            assert!((*value - 3.14159265358979).abs() < 0.0000001);
            let ty = cpp_type.as_ref().expect("Expected type info");
            assert_eq!(ty.bit_width(), Some(64), "double should be 64 bits");
        } else {
            panic!("Expected FloatingLiteral");
        }
    }

    #[test]
    fn test_bool_literal_true() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                bool foo() {
                    return true;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the bool literal
        fn find_bool_literal(node: &ClangNode) -> Option<&ClangNode> {
            if matches!(&node.kind, ClangNodeKind::BoolLiteral(_)) {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_bool_literal(child) {
                    return Some(found);
                }
            }
            None
        }

        let literal = find_bool_literal(&ast.translation_unit).expect("Expected BoolLiteral");
        if let ClangNodeKind::BoolLiteral(value) = &literal.kind {
            assert!(*value, "Expected true");
        } else {
            panic!("Expected BoolLiteral");
        }
    }

    #[test]
    fn test_bool_literal_false() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                bool foo() {
                    return false;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the bool literal
        fn find_bool_literal(node: &ClangNode) -> Option<&ClangNode> {
            if matches!(&node.kind, ClangNodeKind::BoolLiteral(_)) {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_bool_literal(child) {
                    return Some(found);
                }
            }
            None
        }

        let literal = find_bool_literal(&ast.translation_unit).expect("Expected BoolLiteral");
        if let ClangNodeKind::BoolLiteral(value) = &literal.kind {
            assert!(!*value, "Expected false");
        } else {
            panic!("Expected BoolLiteral");
        }
    }

    #[test]
    fn test_string_literal() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                const char* foo() {
                    return "hello world";
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the string literal
        fn find_string_literal(node: &ClangNode) -> Option<&ClangNode> {
            if matches!(&node.kind, ClangNodeKind::StringLiteral(_)) {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_string_literal(child) {
                    return Some(found);
                }
            }
            None
        }

        let literal = find_string_literal(&ast.translation_unit).expect("Expected StringLiteral");
        if let ClangNodeKind::StringLiteral(value) = &literal.kind {
            assert_eq!(value, "hello world");
        } else {
            panic!("Expected StringLiteral");
        }
    }

    #[test]
    fn test_string_literal_empty() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                const char* foo() {
                    return "";
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the string literal
        fn find_string_literal(node: &ClangNode) -> Option<&ClangNode> {
            if matches!(&node.kind, ClangNodeKind::StringLiteral(_)) {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_string_literal(child) {
                    return Some(found);
                }
            }
            None
        }

        let literal = find_string_literal(&ast.translation_unit).expect("Expected StringLiteral");
        if let ClangNodeKind::StringLiteral(value) = &literal.kind {
            assert_eq!(value, "");
        } else {
            panic!("Expected StringLiteral");
        }
    }

    #[test]
    fn test_parse_lvalue_reference_parameter() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                void increment(int& x) {
                    x++;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the function declaration
        for child in &ast.translation_unit.children {
            if let ClangNodeKind::FunctionDecl { name, params, .. } = &child.kind {
                assert_eq!(name, "increment");
                assert_eq!(params.len(), 1);
                let (param_name, param_type) = &params[0];
                assert_eq!(param_name, "x");
                // Check that param_type is CppType::Reference
                if let CppType::Reference {
                    is_const,
                    is_rvalue,
                    ..
                } = param_type
                {
                    assert!(!is_const, "Expected non-const reference");
                    assert!(!is_rvalue, "Expected lvalue reference (not rvalue)");
                } else {
                    panic!("Expected Reference type, got {:?}", param_type);
                }
                return;
            }
        }
        panic!("Expected to find function declaration");
    }

    #[test]
    fn test_parse_const_lvalue_reference_parameter() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int read_only(const int& x) {
                    return x;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the function declaration
        for child in &ast.translation_unit.children {
            if let ClangNodeKind::FunctionDecl { name, params, .. } = &child.kind {
                assert_eq!(name, "read_only");
                assert_eq!(params.len(), 1);
                let (param_name, param_type) = &params[0];
                assert_eq!(param_name, "x");
                // Check that param_type is CppType::Reference with is_const=true
                if let CppType::Reference {
                    is_const,
                    is_rvalue,
                    ..
                } = param_type
                {
                    assert!(is_const, "Expected const reference");
                    assert!(!is_rvalue, "Expected lvalue reference (not rvalue)");
                } else {
                    panic!("Expected Reference type, got {:?}", param_type);
                }
                return;
            }
        }
        panic!("Expected to find function declaration");
    }

    #[test]
    fn test_parse_rvalue_reference_parameter() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                void take_ownership(int&& x) {
                    // Move semantics
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the function declaration
        for child in &ast.translation_unit.children {
            if let ClangNodeKind::FunctionDecl { name, params, .. } = &child.kind {
                assert_eq!(name, "take_ownership");
                assert_eq!(params.len(), 1);
                let (param_name, param_type) = &params[0];
                assert_eq!(param_name, "x");
                // Check that param_type is CppType::Reference with is_rvalue=true
                if let CppType::Reference {
                    is_const,
                    is_rvalue,
                    ..
                } = param_type
                {
                    assert!(!is_const, "Expected non-const rvalue reference");
                    assert!(is_rvalue, "Expected rvalue reference (T&&)");
                } else {
                    panic!("Expected Reference type, got {:?}", param_type);
                }
                return;
            }
        }
        panic!("Expected to find function declaration");
    }

    #[test]
    fn test_module_import_decl_variant() {
        // Test that ModuleImportDecl variant can be created and matched
        // Note: Actual C++20 module parsing requires special Clang flags and module interface files,
        // so we just test that our AST representation is correct.
        let node = ClangNode::new(ClangNodeKind::ModuleImportDecl {
            module_name: "std.core".to_string(),
            is_header_unit: false,
        });

        if let ClangNodeKind::ModuleImportDecl {
            module_name,
            is_header_unit,
        } = &node.kind
        {
            assert_eq!(module_name, "std.core");
            assert!(!is_header_unit);
        } else {
            panic!("Expected ModuleImportDecl");
        }

        // Test header unit variant
        let header_node = ClangNode::new(ClangNodeKind::ModuleImportDecl {
            module_name: "<iostream>".to_string(),
            is_header_unit: true,
        });

        if let ClangNodeKind::ModuleImportDecl {
            module_name,
            is_header_unit,
        } = &header_node.kind
        {
            assert_eq!(module_name, "<iostream>");
            assert!(is_header_unit);
        } else {
            panic!("Expected ModuleImportDecl for header unit");
        }
    }
}

/// Convert string to binary operator.
fn str_to_binary_op(s: &str) -> Option<BinaryOp> {
    match s {
        "+" => Some(BinaryOp::Add),
        "-" => Some(BinaryOp::Sub),
        "*" => Some(BinaryOp::Mul),
        "/" => Some(BinaryOp::Div),
        "%" => Some(BinaryOp::Rem),
        "&" => Some(BinaryOp::And),
        "|" => Some(BinaryOp::Or),
        "^" => Some(BinaryOp::Xor),
        "<<" => Some(BinaryOp::Shl),
        ">>" => Some(BinaryOp::Shr),
        "==" => Some(BinaryOp::Eq),
        "!=" => Some(BinaryOp::Ne),
        "<" => Some(BinaryOp::Lt),
        "<=" => Some(BinaryOp::Le),
        ">" => Some(BinaryOp::Gt),
        ">=" => Some(BinaryOp::Ge),
        "<=>" => Some(BinaryOp::Spaceship),
        "&&" => Some(BinaryOp::LAnd),
        "||" => Some(BinaryOp::LOr),
        "=" => Some(BinaryOp::Assign),
        "+=" => Some(BinaryOp::AddAssign),
        "-=" => Some(BinaryOp::SubAssign),
        "*=" => Some(BinaryOp::MulAssign),
        "/=" => Some(BinaryOp::DivAssign),
        "%=" => Some(BinaryOp::RemAssign),
        "&=" => Some(BinaryOp::AndAssign),
        "|=" => Some(BinaryOp::OrAssign),
        "^=" => Some(BinaryOp::XorAssign),
        "<<=" => Some(BinaryOp::ShlAssign),
        ">>=" => Some(BinaryOp::ShrAssign),
        "," => Some(BinaryOp::Comma),
        _ => None,
    }
}
