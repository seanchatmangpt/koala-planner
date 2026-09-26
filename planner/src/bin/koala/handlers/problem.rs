use clap_noun_verb::{NounVerbError, Result};
use koala_serializer::{convert_file, SerializationReport};
use std::path::Path;

pub fn problem_serialize(
    grounded_input: String,
    output_json: String,
    probability_map: Option<String>,
) -> Result<SerializationReport> {
    convert_file(
        &grounded_input,
        &output_json,
        probability_map.as_deref().map(Path::new),
    )
    .map_err(|error| NounVerbError::execution_error(error.to_string()))
}
