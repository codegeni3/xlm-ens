use clap::ValueEnum;
use serde_json::Value;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    #[value(name = "csv")]
    Csv,
}

pub fn emit(format: OutputFormat, human: &str, json: Value) {
    match format {
        OutputFormat::Human => println!("{human}"),
        OutputFormat::Json | OutputFormat::Csv => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json).expect("json output should always serialize")
            );
        }
    }
}

pub fn emit_error(format: OutputFormat, human: &str, json: Value) {
    match format {
        OutputFormat::Human => eprintln!("{human}"),
        OutputFormat::Json | OutputFormat::Csv => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&json).expect("json output should always serialize")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OutputFormat;
    use clap::ValueEnum;

    #[test]
    fn test_invalid_output_mode_exits_with_error() {
        assert!(OutputFormat::from_str("tsv", false).is_err());
    }
}
