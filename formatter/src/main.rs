use formatter::{formatter, Configuration, FormatterError, Operation};

fn main() {
    let input = std::fs::read_to_string("test/test.ftr").unwrap();
    let mut input = input.as_bytes();
    let mut output = Vec::new();
    let query = include_str!("feather.scm");

    let config = Configuration::parse_default_config();
    let language = config.get_language("feather").unwrap();
    let grammars = language.grammars().expect("grammars");

    match formatter(
        &mut input,
        &mut output,
        query,
        language,
        &grammars,
        Operation::Format {
            skip_idempotence: true,
        },
    ) {
        Ok(()) => {
            let formatted = String::from_utf8(output).expect("valid utf-8");
            println!("{}", formatted);
        }
        Err(FormatterError::Query(message, _)) => {
            panic!("Error in query file: {message}");
        }
        Err(err) => {
            panic!("An error occurred: {err}");
        }
    }
}
