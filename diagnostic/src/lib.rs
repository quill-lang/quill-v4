use miette::{miette, LabeledSpan, Report, Severity};

pub fn generate_report() -> Report {
    miette!(
        severity = Severity::Error,
        code = "expected::rparen",
        help = "always close your parens",
        labels = vec![LabeledSpan::at_offset(6, "here")],
        url = "https://example.com",
        "expected closing ')'",
    )
    .with_source_code("(2 + 2 ".to_string())
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
