#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreGeneratedStlSurfaceStatus {
    Available,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreGeneratedStlModuleContract {
    pub module_id: &'static str,
    pub source_file: &'static str,
    pub sentinel: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreGeneratedStlFamilyContract {
    pub family: &'static str,
    pub module_id: &'static str,
    pub canonical_type_prefix: &'static str,
    pub status: PreGeneratedStlSurfaceStatus,
}

pub const PREGENERATED_STL_LAYOUT_VERSION_V1: &str = "v1";
pub const PREGENERATED_STL_LAYOUT_NAMESPACE_V1: &str = "fragile_stl::v1";

pub const PREGENERATED_STL_MODULES_V1: &[PreGeneratedStlModuleContract] = &[
    PreGeneratedStlModuleContract {
        module_id: "file_header",
        source_file: "file_header.rs",
        sentinel: "pub struct FragileOpaqueField;",
    },
    PreGeneratedStlModuleContract {
        module_id: "array_helpers",
        source_file: "array_helpers.rs",
        sentinel: "unsafe fn fragile_new_array<T: Clone>(len: usize, init: T) -> *mut T {",
    },
    PreGeneratedStlModuleContract {
        module_id: "comparison",
        source_file: "comparison.rs",
        sentinel: "pub struct partial_ordering { pub _M_value: __cmp_cat_type }",
    },
    PreGeneratedStlModuleContract {
        module_id: "vector",
        source_file: "vector.rs",
        sentinel: "pub struct std_vector<T> {",
    },
    PreGeneratedStlModuleContract {
        module_id: "container_adapters",
        source_file: "container_adapters.rs",
        sentinel: "pub struct std_deque<T> {",
    },
    PreGeneratedStlModuleContract {
        module_id: "string",
        source_file: "string.rs",
        sentinel: "pub struct std_string {",
    },
    PreGeneratedStlModuleContract {
        module_id: "smart_ptr",
        source_file: "smart_ptr.rs",
        sentinel: "pub struct std_unique_ptr<T> {",
    },
    PreGeneratedStlModuleContract {
        module_id: "algorithm",
        source_file: "algorithm.rs",
        sentinel: "pub fn std_sort_int(first: *mut i32, last: *mut i32) {",
    },
    PreGeneratedStlModuleContract {
        module_id: "tree",
        source_file: "tree.rs",
        sentinel: "pub struct __tree_end_node {",
    },
    PreGeneratedStlModuleContract {
        module_id: "hash",
        source_file: "hash.rs",
        sentinel: "pub fn _Hash_bytes(_ptr: *const (), _len: u64, _seed: u64) -> u64 {",
    },
    PreGeneratedStlModuleContract {
        module_id: "numeric",
        source_file: "numeric.rs",
        sentinel: "pub mod numeric_limits {",
    },
    PreGeneratedStlModuleContract {
        module_id: "locale",
        source_file: "locale.rs",
        sentinel: "pub struct locale_facet_vtable {",
    },
    PreGeneratedStlModuleContract {
        module_id: "io",
        source_file: "io.rs",
        sentinel: "pub struct basic_format_parse_context_char;",
    },
    PreGeneratedStlModuleContract {
        module_id: "exception",
        source_file: "exception.rs",
        sentinel: "pub struct exception_vtable {",
    },
    PreGeneratedStlModuleContract {
        module_id: "math",
        source_file: "math.rs",
        sentinel: "pub fn __builtin_addressof<T>(x: &T) -> *const T { x as *const T }",
    },
    PreGeneratedStlModuleContract {
        module_id: "clib",
        source_file: "clib.rs",
        sentinel: "pub fn strtof_l(_s: *const i8, _endptr: *mut *mut i8, _loc: *mut std::ffi::c_void) -> f32 { 0.0 }",
    },
    PreGeneratedStlModuleContract {
        module_id: "runtime",
        source_file: "runtime.rs",
        sentinel: "pub mod fragile_runtime {",
    },
];

pub const PREGENERATED_STL_FAMILY_CONTRACT_V1: &[PreGeneratedStlFamilyContract] = &[
    PreGeneratedStlFamilyContract {
        family: "vector",
        module_id: "vector",
        canonical_type_prefix: "std_vector",
        status: PreGeneratedStlSurfaceStatus::Available,
    },
    PreGeneratedStlFamilyContract {
        family: "map",
        module_id: "tree",
        canonical_type_prefix: "std_map",
        status: PreGeneratedStlSurfaceStatus::Planned,
    },
    PreGeneratedStlFamilyContract {
        family: "unordered_map",
        module_id: "hash",
        canonical_type_prefix: "std_unordered_map",
        status: PreGeneratedStlSurfaceStatus::Planned,
    },
    PreGeneratedStlFamilyContract {
        family: "string",
        module_id: "string",
        canonical_type_prefix: "std_string",
        status: PreGeneratedStlSurfaceStatus::Available,
    },
    PreGeneratedStlFamilyContract {
        family: "optional",
        module_id: "comparison",
        canonical_type_prefix: "std_optional",
        status: PreGeneratedStlSurfaceStatus::Planned,
    },
    PreGeneratedStlFamilyContract {
        family: "variant",
        module_id: "comparison",
        canonical_type_prefix: "std_variant",
        status: PreGeneratedStlSurfaceStatus::Planned,
    },
    PreGeneratedStlFamilyContract {
        family: "tuple",
        module_id: "comparison",
        canonical_type_prefix: "std_tuple",
        status: PreGeneratedStlSurfaceStatus::Planned,
    },
    PreGeneratedStlFamilyContract {
        family: "shared_ptr",
        module_id: "smart_ptr",
        canonical_type_prefix: "std_shared_ptr",
        status: PreGeneratedStlSurfaceStatus::Available,
    },
    PreGeneratedStlFamilyContract {
        family: "unique_ptr",
        module_id: "smart_ptr",
        canonical_type_prefix: "std_unique_ptr",
        status: PreGeneratedStlSurfaceStatus::Available,
    },
];

pub fn pre_generated_stl_modules_v1() -> &'static [PreGeneratedStlModuleContract] {
    PREGENERATED_STL_MODULES_V1
}

pub fn pre_generated_stl_family_contract_v1() -> &'static [PreGeneratedStlFamilyContract] {
    PREGENERATED_STL_FAMILY_CONTRACT_V1
}

pub fn pre_generated_stl_family_contract_entry_v1(
    family: &str,
) -> Option<&'static PreGeneratedStlFamilyContract> {
    PREGENERATED_STL_FAMILY_CONTRACT_V1
        .iter()
        .find(|entry| entry.family == family)
}

pub fn pre_generated_stl_module_source_v1(module_id: &str) -> Option<&'static str> {
    match module_id {
        "file_header" => Some(include_str!("file_header.rs")),
        "array_helpers" => Some(include_str!("array_helpers.rs")),
        "comparison" => Some(include_str!("comparison.rs")),
        "vector" => Some(include_str!("vector.rs")),
        "container_adapters" => Some(include_str!("container_adapters.rs")),
        "string" => Some(include_str!("string.rs")),
        "smart_ptr" => Some(include_str!("smart_ptr.rs")),
        "algorithm" => Some(include_str!("algorithm.rs")),
        "tree" => Some(include_str!("tree.rs")),
        "hash" => Some(include_str!("hash.rs")),
        "numeric" => Some(include_str!("numeric.rs")),
        "locale" => Some(include_str!("locale.rs")),
        "io" => Some(include_str!("io.rs")),
        "exception" => Some(include_str!("exception.rs")),
        "math" => Some(include_str!("math.rs")),
        "clib" => Some(include_str!("clib.rs")),
        "runtime" => Some(include_str!("runtime.rs")),
        _ => None,
    }
}
