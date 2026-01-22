//! Debug helper to understand default argument handling

fn main() {
    let source = r#"
        int add(int a, int b = 10, int c = 20) {
            return a + b + c;
        }
        int main() {
            int x = add(1);
            return 0;
        }
    "#;
    
    unsafe {
        let index = clang_sys::clang_createIndex(0, 0);
        
        let unsaved = clang_sys::CXUnsavedFile {
            Filename: b"test.cpp\0".as_ptr() as *const i8,
            Contents: source.as_ptr() as *const i8,
            Length: source.len() as u64,
        };
        
        let tu = clang_sys::clang_parseTranslationUnit(
            index,
            b"test.cpp\0".as_ptr() as *const i8,
            std::ptr::null(),
            0,
            &unsaved as *const _ as *mut _,
            1,
            clang_sys::CXTranslationUnit_None,
        );
        
        if tu.is_null() {
            eprintln!("Failed to parse");
            return;
        }
        
        let cursor = clang_sys::clang_getTranslationUnitCursor(tu);
        
        extern "C" fn visitor(
            cursor: clang_sys::CXCursor,
            _parent: clang_sys::CXCursor,
            depth: clang_sys::CXClientData,
        ) -> clang_sys::CXChildVisitResult {
            unsafe {
                let depth = *(depth as *const i32);
                let kind = clang_sys::clang_getCursorKind(cursor);
                let kind_spelling = clang_sys::clang_getCursorKindSpelling(kind);
                let kind_str = std::ffi::CStr::from_ptr(clang_sys::clang_getCString(kind_spelling))
                    .to_string_lossy();
                clang_sys::clang_disposeString(kind_spelling);
                
                let name = clang_sys::clang_getCursorSpelling(cursor);
                let name_str = std::ffi::CStr::from_ptr(clang_sys::clang_getCString(name))
                    .to_string_lossy();
                clang_sys::clang_disposeString(name);
                
                let indent = "  ".repeat(depth as usize);
                println!("{}kind={} ({}) name='{}'", indent, kind, kind_str, name_str);
                
                let new_depth = depth + 1;
                clang_sys::clang_visitChildren(
                    cursor,
                    visitor,
                    &new_depth as *const i32 as clang_sys::CXClientData,
                );
                
                clang_sys::CXChildVisit_Continue
            }
        }
        
        let depth: i32 = 0;
        clang_sys::clang_visitChildren(cursor, visitor, &depth as *const i32 as clang_sys::CXClientData);
        
        clang_sys::clang_disposeTranslationUnit(tu);
        clang_sys::clang_disposeIndex(index);
    }
}
