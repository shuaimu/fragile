use fragile_stl::layout_contract::{
    pre_generated_stl_family_contract_entry_v1, pre_generated_stl_family_contract_v1,
    pre_generated_stl_module_manifest_text_v1, pre_generated_stl_module_manifest_v1,
    pre_generated_stl_module_source_v1, pre_generated_stl_modules_v1, PreGeneratedStlSurfaceStatus,
    PREGENERATED_STL_LAYOUT_NAMESPACE_V1, PREGENERATED_STL_LAYOUT_VERSION_V1,
};
use std::collections::HashSet;

#[test]
fn pregenerated_layout_contract_v1_module_manifest_is_deterministic_and_unique() {
    let modules = pre_generated_stl_modules_v1();
    let expected_module_ids = [
        "file_header",
        "array_helpers",
        "comparison",
        "vector",
        "container_adapters",
        "string",
        "smart_ptr",
        "algorithm",
        "tree",
        "hash",
        "numeric",
        "locale",
        "io",
        "exception",
        "math",
        "clib",
        "runtime",
    ];

    let module_ids = modules.iter().map(|module| module.module_id).collect::<Vec<_>>();
    assert_eq!(
        module_ids, expected_module_ids,
        "pre-generated STL module order is part of layout contract v1"
    );

    let mut seen_ids = HashSet::new();
    let mut seen_sources = HashSet::new();
    for module in modules {
        assert!(
            seen_ids.insert(module.module_id),
            "duplicate module id in contract: {}",
            module.module_id
        );
        assert!(
            seen_sources.insert(module.source_file),
            "duplicate source file in contract: {}",
            module.source_file
        );
        assert!(
            !module.sentinel.trim().is_empty(),
            "module `{}` must provide a non-empty sentinel",
            module.module_id
        );
    }
}

#[test]
fn pregenerated_layout_contract_v1_module_sources_resolve_with_sentinels() {
    for module in pre_generated_stl_modules_v1() {
        let source = pre_generated_stl_module_source_v1(module.module_id).unwrap_or_else(|| {
            panic!(
                "contract module `{}` should resolve to source text",
                module.module_id
            )
        });
        assert!(
            source.contains(module.sentinel),
            "module `{}` source should contain sentinel `{}`",
            module.module_id,
            module.sentinel
        );
    }
}

#[test]
fn pregenerated_layout_contract_v1_family_contract_covers_required_families() {
    assert_eq!(PREGENERATED_STL_LAYOUT_VERSION_V1, "v1");
    assert_eq!(PREGENERATED_STL_LAYOUT_NAMESPACE_V1, "fragile_stl::v1");

    let required_families = [
        "vector",
        "map",
        "unordered_map",
        "string",
        "optional",
        "variant",
        "tuple",
        "shared_ptr",
        "unique_ptr",
    ];

    let mut seen_families = HashSet::new();
    let modules = pre_generated_stl_modules_v1()
        .iter()
        .map(|module| module.module_id)
        .collect::<HashSet<_>>();
    for entry in pre_generated_stl_family_contract_v1() {
        assert!(
            seen_families.insert(entry.family),
            "duplicate STL family contract entry: {}",
            entry.family
        );
        assert!(
            modules.contains(entry.module_id),
            "family `{}` maps to unknown module `{}`",
            entry.family,
            entry.module_id
        );
        assert!(
            entry.canonical_type_prefix.starts_with("std_"),
            "family `{}` canonical prefix should use std_* naming",
            entry.family
        );
    }

    for family in required_families {
        assert!(
            pre_generated_stl_family_contract_entry_v1(family).is_some(),
            "required STL family `{}` must exist in v1 naming contract",
            family
        );
    }
}

#[test]
fn pregenerated_layout_contract_v1_available_family_prefixes_exist_in_module_sources() {
    for family in pre_generated_stl_family_contract_v1() {
        if family.status != PreGeneratedStlSurfaceStatus::Available {
            continue;
        }
        let source = pre_generated_stl_module_source_v1(family.module_id).unwrap_or_else(|| {
            panic!(
                "family `{}` expected source for module `{}`",
                family.family, family.module_id
            )
        });
        assert!(
            source.contains(family.canonical_type_prefix),
            "available family `{}` should have prefix `{}` in module `{}` source",
            family.family,
            family.canonical_type_prefix,
            family.module_id
        );
    }
}

#[test]
fn pregenerated_layout_contract_v1_module_manifest_entries_are_deterministic() {
    let first = pre_generated_stl_module_manifest_v1();
    let second = pre_generated_stl_module_manifest_v1();
    assert_eq!(
        first, second,
        "manifest entries should be deterministic across repeated generation"
    );

    let modules = pre_generated_stl_modules_v1();
    assert_eq!(
        first.len(),
        modules.len(),
        "manifest should include one entry per module contract item"
    );

    for (entry, module) in first.iter().zip(modules.iter()) {
        assert_eq!(entry.module_id, module.module_id);
        assert_eq!(entry.source_file, module.source_file);
        assert_eq!(entry.sentinel, module.sentinel);

        let source = pre_generated_stl_module_source_v1(module.module_id).unwrap_or_else(|| {
            panic!(
                "contract module `{}` should resolve to source text",
                module.module_id
            )
        });
        assert_eq!(entry.source_byte_len, source.len());
        assert_eq!(entry.source_line_count, source.lines().count());
    }
}

#[test]
fn pregenerated_layout_contract_v1_module_manifest_text_is_stable_and_complete() {
    let first = pre_generated_stl_module_manifest_text_v1();
    let second = pre_generated_stl_module_manifest_text_v1();
    assert_eq!(
        first, second,
        "manifest text should be deterministic across repeated generation"
    );

    assert!(
        first.contains(&format!("layout_version={}", PREGENERATED_STL_LAYOUT_VERSION_V1)),
        "manifest text should include layout version header"
    );
    assert!(
        first.contains(&format!(
            "layout_namespace={}",
            PREGENERATED_STL_LAYOUT_NAMESPACE_V1
        )),
        "manifest text should include layout namespace header"
    );
    assert!(
        first.contains(&format!("module_count={}", pre_generated_stl_modules_v1().len())),
        "manifest text should include module count header"
    );

    for entry in pre_generated_stl_module_manifest_v1() {
        let line = format!(
            "module={}|source_file={}|bytes={}|lines={}|fnv1a64={:016x}",
            entry.module_id,
            entry.source_file,
            entry.source_byte_len,
            entry.source_line_count,
            entry.source_fnv1a64
        );
        assert!(
            first.contains(line.as_str()),
            "manifest text should include module fingerprint line `{}`",
            line
        );
    }
}
