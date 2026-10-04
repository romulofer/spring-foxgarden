use super::*;

/// Parses with this extension's own contributed grammar — the editor does
/// the parsing in real use, so what these tests need is any tree of the
/// right grammar, not the editor's parser.
fn parse(language: tree_sitter::Language, source: &str) -> Tree {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).expect("a bundled grammar must load");
    parser.parse(source, None).expect("parse")
}

fn endpoints(source: &str) -> Vec<HttpRoute> {
    let tree = parse(tree_sitter_java::LANGUAGE.into(), source);
    http_routes(&tree, source, JAVA)
}

fn kotlin_endpoints(source: &str) -> Vec<HttpRoute> {
    let tree = parse(tree_sitter_kotlin_ng::LANGUAGE.into(), source);
    http_routes(&tree, source, KOTLIN)
}

#[test]
fn positional_string_path() {
    let source = "class Foo {\n    @GetMapping(\"/x\")\n    public void run() {}\n}\n";
    let eps = endpoints(source);
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].method, "GET");
    assert_eq!(eps[0].path, "/x");
    assert_eq!(eps[0].owner, "Foo");
    assert_eq!(eps[0].handler, "run");
}

#[test]
fn value_equals_path() {
    let source = "class Foo {\n    @GetMapping(value = \"/x\")\n    public void run() {}\n}\n";
    let eps = endpoints(source);
    assert_eq!(eps[0].path, "/x");
}

#[test]
fn path_equals_path() {
    let source = "class Foo {\n    @GetMapping(path = \"/x\")\n    public void run() {}\n}\n";
    let eps = endpoints(source);
    assert_eq!(eps[0].path, "/x");
}

#[test]
fn method_equals_combined_with_class_level_base_path() {
    let source = "@RequestMapping(\"/api\")\nclass Foo {\n    @RequestMapping(method = RequestMethod.DELETE)\n    public void run() {}\n}\n";
    let eps = endpoints(source);
    assert_eq!(eps[0].method, "DELETE");
    assert_eq!(eps[0].path, "/api");
}

#[test]
fn bare_marker_annotation_with_no_args() {
    let source = "class Foo {\n    @PostMapping\n    public void run() {}\n}\n";
    let eps = endpoints(source);
    assert_eq!(eps[0].method, "POST");
    assert_eq!(eps[0].path, "/");
}

#[test]
fn no_class_level_base_path() {
    let source = "class Foo {\n    @GetMapping(\"/x\")\n    public void run() {}\n}\n";
    let eps = endpoints(source);
    assert_eq!(eps[0].path, "/x");
}

#[test]
fn nested_class() {
    let source =
        "class Outer {\n    class Inner {\n        @GetMapping(\"/inner\")\n        public void run() {}\n    }\n}\n";
    let eps = endpoints(source);
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].owner, "Inner");
    assert_eq!(eps[0].path, "/inner");
}

#[test]
fn multiple_unrelated_annotations_only_recognized_one_counts() {
    let source =
        "class Foo {\n    @Override\n    @GetMapping(\"/x\")\n    @Transactional\n    public void run() {}\n}\n";
    let eps = endpoints(source);
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].method, "GET");
    assert_eq!(eps[0].path, "/x");
}

#[test]
fn no_recognized_annotation_at_all() {
    let source = "class Foo {\n    @Override\n    public void run() {}\n}\n";
    assert_eq!(endpoints(source), vec![]);
}

#[test]
fn handler_byte_points_at_the_method_name() {
    let source = "class Foo {\n    @GetMapping(\"/x\")\n    public void run() {}\n}\n";
    let eps = endpoints(source);
    let byte = eps[0].handler_byte;
    assert_eq!(&source[byte..byte + 3], "run");
}

#[test]
fn class_with_request_mapping_but_no_recognized_method_annotations_contributes_nothing() {
    let source = "@RequestMapping(\"/api\")\nclass Foo {\n    public void run() {}\n}\n";
    assert_eq!(endpoints(source), vec![]);
}

#[test]
fn multiple_endpoints_in_source_order() {
    let source = "@RequestMapping(\"/api\")\nclass Foo {\n    @GetMapping(\"/a\")\n    public void a() {}\n\n    @PostMapping(\"/b\")\n    public void b() {}\n}\n";
    let eps = endpoints(source);
    assert_eq!(eps.len(), 2);
    assert_eq!(eps[0].handler, "a");
    assert_eq!(eps[0].path, "/api/a");
    assert_eq!(eps[1].handler, "b");
    assert_eq!(eps[1].path, "/api/b");
}

#[test]
fn kotlin_positional_string_path() {
    let source = "class Foo {\n    @GetMapping(\"/x\")\n    fun run() {}\n}\n";
    let eps = kotlin_endpoints(source);
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].method, "GET");
    assert_eq!(eps[0].path, "/x");
    assert_eq!(eps[0].owner, "Foo");
    assert_eq!(eps[0].handler, "run");
}

#[test]
fn kotlin_value_equals_path() {
    let source = "class Foo {\n    @GetMapping(value = \"/x\")\n    fun run() {}\n}\n";
    let eps = kotlin_endpoints(source);
    assert_eq!(eps[0].path, "/x");
}

#[test]
fn kotlin_path_equals_path() {
    let source = "class Foo {\n    @GetMapping(path = \"/x\")\n    fun run() {}\n}\n";
    let eps = kotlin_endpoints(source);
    assert_eq!(eps[0].path, "/x");
}

#[test]
fn kotlin_method_equals_combined_with_class_level_base_path() {
    let source = "@RequestMapping(\"/api\")\nclass Foo {\n    @RequestMapping(method = [RequestMethod.DELETE])\n    fun run() {}\n}\n";
    let eps = kotlin_endpoints(source);
    assert_eq!(eps[0].method, "DELETE");
    assert_eq!(eps[0].path, "/api");
}

#[test]
fn kotlin_array_literal_unwrapping_for_path_too() {
    let source = "class Foo {\n    @GetMapping(path = [\"/x\"])\n    fun run() {}\n}\n";
    let eps = kotlin_endpoints(source);
    assert_eq!(eps[0].path, "/x");
}

#[test]
fn kotlin_bare_marker_annotation_with_no_args() {
    let source = "class Foo {\n    @PostMapping\n    fun run() {}\n}\n";
    let eps = kotlin_endpoints(source);
    assert_eq!(eps[0].method, "POST");
    assert_eq!(eps[0].path, "/");
}

#[test]
fn kotlin_no_class_level_base_path() {
    let source = "class Foo {\n    @GetMapping(\"/x\")\n    fun run() {}\n}\n";
    let eps = kotlin_endpoints(source);
    assert_eq!(eps[0].path, "/x");
}

#[test]
fn kotlin_nested_class() {
    let source = "class Outer {\n    class Inner {\n        @GetMapping(\"/inner\")\n        fun run() {}\n    }\n}\n";
    let eps = kotlin_endpoints(source);
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].owner, "Inner");
    assert_eq!(eps[0].path, "/inner");
}

#[test]
fn kotlin_multiple_unrelated_annotations_only_recognized_one_counts() {
    let source =
        "class Foo {\n    @Suppress(\"unused\")\n    @GetMapping(\"/x\")\n    @JvmStatic\n    fun run() {}\n}\n";
    let eps = kotlin_endpoints(source);
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].method, "GET");
    assert_eq!(eps[0].path, "/x");
}

#[test]
fn kotlin_no_recognized_annotation_at_all() {
    let source = "class Foo {\n    @Suppress(\"unused\")\n    fun run() {}\n}\n";
    assert_eq!(kotlin_endpoints(source), vec![]);
}

#[test]
fn kotlin_handler_byte_points_at_the_function_name() {
    let source = "class Foo {\n    @GetMapping(\"/x\")\n    fun run() {}\n}\n";
    let eps = kotlin_endpoints(source);
    let byte = eps[0].handler_byte;
    assert_eq!(&source[byte..byte + 3], "run");
}

#[test]
fn kotlin_class_with_request_mapping_but_no_recognized_function_annotations_contributes_nothing() {
    let source = "@RequestMapping(\"/api\")\nclass Foo {\n    fun run() {}\n}\n";
    assert_eq!(kotlin_endpoints(source), vec![]);
}

#[test]
fn kotlin_multiple_endpoints_in_source_order() {
    let source = "@RequestMapping(\"/api\")\nclass Foo {\n    @GetMapping(\"/a\")\n    fun a() {}\n\n    @PostMapping(\"/b\")\n    fun b() {}\n}\n";
    let eps = kotlin_endpoints(source);
    assert_eq!(eps.len(), 2);
    assert_eq!(eps[0].handler, "a");
    assert_eq!(eps[0].path, "/api/a");
    assert_eq!(eps[1].handler, "b");
    assert_eq!(eps[1].path, "/api/b");
}

/// A Java tree handed over under another language's id is not this
/// extension's to read, even though it would find a route in it.
#[test]
fn a_language_this_extension_does_not_route_has_no_routes() {
    let source = "class Foo {\n    @GetMapping(\"/x\")\n    public void run() {}\n}\n";
    let tree = parse(tree_sitter_java::LANGUAGE.into(), source);
    assert_eq!(http_routes(&tree, source, "yaml"), vec![]);
}

/// Spring takes an array wherever it takes one path; the first one is used,
/// as on the Kotlin side.
#[test]
fn array_initializer_paths() {
    let source = "@RequestMapping(value = {\"/api\"})\nclass Foo {\n    @GetMapping({\"/users\", \"/people\"})\n    public void run() {}\n}\n";
    let eps = endpoints(source);
    assert_eq!(eps[0].path, "/api/users");
}
