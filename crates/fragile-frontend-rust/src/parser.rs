use miette::{Result, IntoDiagnostic};
use tree_sitter::{Parser, Tree};

/// Parse Rust source code into a tree-sitter Tree.
pub fn parse(source: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    parser.set_language(&language.into()).into_diagnostic()?;

    parser
        .parse(source, None)
        .ok_or_else(|| miette::miette!("Failed to parse Rust source"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let source = r#"
fn main() {
    let x = 42;
}
"#;
        let tree = parse(source).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn test_parse_function_with_return() {
        let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let tree = parse(source).unwrap();
        assert!(!tree.root_node().has_error());
    }
}
