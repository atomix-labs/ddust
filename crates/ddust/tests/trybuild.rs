//! Every misuse the types refuse, as a program that must not compile, with the message it gets.

#[cfg(test)]
mod tests {
    use trybuild::TestCases;

    #[test]
    fn every_refused_misuse_fails_to_compile() {
        TestCases::new().compile_fail("tests/compile_fail/*.rs");
    }
}
