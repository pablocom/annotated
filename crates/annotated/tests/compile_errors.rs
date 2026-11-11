//! What `#[derive(Validate)]` refuses, with the compile error that explains why.

#[test]
fn compile_errors_explain_what_the_derive_refuses() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
    #[cfg(feature = "regex")]
    cases.compile_fail("tests/ui/regex/*.rs");
    #[cfg(not(feature = "regex"))]
    cases.compile_fail("tests/ui/without-regex/*.rs");
    #[cfg(not(feature = "unicode"))]
    cases.compile_fail("tests/ui/without-unicode/*.rs");
}
