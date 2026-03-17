use fragile_parser_clang::{FragileParserClangBackend, FRAGILE_PARSER_CLANG_BACKEND_ID};
use fragile_parser_core::{
    BackendRegistry, IncludeDirective, IncludeDirectiveKind, ParseRequest, ParserLanguage,
    PARSER_OUTPUT_SCHEMA_VERSION_V1,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

struct CorpusFixtureCase {
    label: &'static str,
    request: ParseRequest,
    min_nodes: usize,
    required_names: &'static [&'static str],
    required_node_kinds: &'static [&'static str],
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("m2_a1")
}

fn build_fixture_cases(root: &Path) -> Vec<CorpusFixtureCase> {
    let include_dir = root.join("include");
    let include_path = include_dir
        .to_str()
        .expect("fixture include path should be valid UTF-8")
        .to_string();

    vec![
        CorpusFixtureCase {
            label: "pipeline_cpp",
            request: ParseRequest {
                source_path: root.join("src").join("pipeline.cpp"),
                language: ParserLanguage::Cpp,
                frontend_args: Vec::new(),
                defines: vec!["CORPUS_SCALE=4".to_string()],
                include_directives: vec![IncludeDirective {
                    kind: IncludeDirectiveKind::Include,
                    path: include_path.clone(),
                }],
            },
            min_nodes: 20,
            required_names: &["run_pipeline", "classify_status", "fold_packets", "Packet"],
            required_node_kinds: &["translation_unit", "function_decl", "record_decl", "statement"],
        },
        CorpusFixtureCase {
            label: "dispatch_cpp",
            request: ParseRequest {
                source_path: root.join("src").join("dispatch.cpp"),
                language: ParserLanguage::Cpp,
                frontend_args: vec![
                    "-I".to_string(),
                    include_path,
                    "-DUNUSED_FRONTEND_FLAG=1".to_string(),
                ],
                defines: Vec::new(),
                include_directives: Vec::new(),
            },
            min_nodes: 15,
            required_names: &["dispatch_status", "wrap_dispatch", "HandlerFn", "PacketBox"],
            required_node_kinds: &["translation_unit", "function_decl", "type_ref", "expression"],
        },
        CorpusFixtureCase {
            label: "metrics_c",
            request: ParseRequest {
                source_path: root.join("src").join("metrics.c"),
                language: ParserLanguage::C,
                frontend_args: Vec::new(),
                defines: vec!["CORPUS_C_SHIFT=5".to_string()],
                include_directives: Vec::new(),
            },
            min_nodes: 12,
            required_names: &[
                "MetricSample",
                "clamp_non_negative",
                "accumulate_weighted",
                "stable_partition_score",
            ],
            required_node_kinds: &["translation_unit", "function_decl", "record_decl", "statement"],
        },
    ]
}

fn assert_sequential_node_ids(case_label: &str, node_ids: &[String]) {
    for (idx, node_id) in node_ids.iter().enumerate() {
        assert_eq!(
            node_id,
            &format!("n{idx}"),
            "{case_label}: node ids should be deterministic pre-order indices"
        );
    }
}

#[test]
fn parser_core_registry_parses_non_trivial_fixture_corpus_with_deterministic_ir() {
    let fixture_root = fixture_root();
    assert!(
        fixture_root.is_dir(),
        "expected fixture root directory at {}",
        fixture_root.display()
    );

    let cases = build_fixture_cases(&fixture_root);
    let mut registry = BackendRegistry::new();
    registry
        .register(FragileParserClangBackend)
        .expect("failed to register fragile-parser-clang backend");

    let mut total_nodes = 0usize;
    for case in &cases {
        let output = registry
            .parse_with(FRAGILE_PARSER_CLANG_BACKEND_ID, &case.request)
            .unwrap_or_else(|err| {
                panic!("{}: parse failed for {}: {}", case.label, case.request.source_path.display(), err)
            });
        let output_second = registry
            .parse_with(FRAGILE_PARSER_CLANG_BACKEND_ID, &case.request)
            .unwrap_or_else(|err| {
                panic!(
                    "{}: second parse failed for {}: {}",
                    case.label,
                    case.request.source_path.display(),
                    err
                )
            });

        assert_eq!(
            output, output_second,
            "{}: parse output should be deterministic across repeated runs",
            case.label
        );
        assert_eq!(
            output.schema_version,
            PARSER_OUTPUT_SCHEMA_VERSION_V1,
            "{}: schema version mismatch",
            case.label
        );
        assert_eq!(
            output.translation_unit.source_path, case.request.source_path,
            "{}: translation unit source path mismatch",
            case.label
        );
        assert_eq!(
            output.translation_unit.language, case.request.language,
            "{}: translation unit language mismatch",
            case.label
        );
        assert_eq!(
            output.translation_unit.parser_backend,
            FRAGILE_PARSER_CLANG_BACKEND_ID,
            "{}: parser backend id mismatch",
            case.label
        );
        assert!(
            output.nodes.len() >= case.min_nodes,
            "{}: expected at least {} nodes for non-trivial fixture corpus entry, got {}",
            case.label,
            case.min_nodes,
            output.nodes.len()
        );

        assert_sequential_node_ids(
            case.label,
            &output
                .nodes
                .iter()
                .map(|node| node.node_id.clone())
                .collect::<Vec<_>>(),
        );

        let node_kinds: HashSet<&str> = output.nodes.iter().map(|node| node.node_kind.as_str()).collect();
        for required_kind in case.required_node_kinds {
            assert!(
                node_kinds.contains(required_kind),
                "{}: missing required node kind `{}` in parser output",
                case.label,
                required_kind
            );
        }

        let node_names: HashSet<&str> = output
            .nodes
            .iter()
            .filter_map(|node| node.name.as_deref())
            .collect();
        for required_name in case.required_names {
            assert!(
                node_names.contains(required_name),
                "{}: missing required named node `{}` in parser output",
                case.label,
                required_name
            );
        }

        total_nodes += output.nodes.len();
    }

    assert!(
        total_nodes >= 70,
        "fixture corpus should emit a non-trivial aggregate node volume, got {}",
        total_nodes
    );
}
