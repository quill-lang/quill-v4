pub fn test(code: &str) {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_feather::language())
        .expect("Error loading feather grammar");
    let tree = parser.parse(code, None).unwrap();
    println!("{}", tree.root_node().to_sexp());
}
