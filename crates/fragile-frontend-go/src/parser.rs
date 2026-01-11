use miette::{Result, IntoDiagnostic};
use tree_sitter::{Parser, Tree};

/// Parse Go source code into a tree-sitter Tree.
pub fn parse(source: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    let language = tree_sitter_go::LANGUAGE;
    parser.set_language(&language.into()).into_diagnostic()?;

    parser
        .parse(source, None)
        .ok_or_else(|| miette::miette!("Failed to parse Go source"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let source = r#"
package main

func main() {
    x := 42
}
"#;
        let tree = parse(source).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn test_parse_function_with_return() {
        let source = r#"
package main

func add(a int, b int) int {
    return a + b
}
"#;
        let tree = parse(source).unwrap();
        assert!(!tree.root_node().has_error());
    }
}
