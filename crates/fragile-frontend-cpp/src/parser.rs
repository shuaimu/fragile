use miette::{Result, IntoDiagnostic};
use tree_sitter::{Parser, Tree};

/// Parse C++ source code into a tree-sitter Tree.
pub fn parse(source: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    let language = tree_sitter_cpp::LANGUAGE;
    parser.set_language(&language.into()).into_diagnostic()?;

    parser
        .parse(source, None)
        .ok_or_else(|| miette::miette!("Failed to parse C++ source"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let source = r#"
int main() {
    int x = 42;
    return 0;
}
"#;
        let tree = parse(source).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn test_parse_function_with_params() {
        let source = r#"
int add(int a, int b) {
    return a + b;
}
"#;
        let tree = parse(source).unwrap();
        assert!(!tree.root_node().has_error());
    }
}
