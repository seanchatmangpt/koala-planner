use clap_noun_verb::{NounVerbError, Result};
use planner::{solve_json_path, Heuristic, SolveMode, SolveOptions, SolveReport, Tiebreaker};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;

pub fn plan_solve(
    grounded_json: String,
    mode: Option<String>,
    heuristic: Option<String>,
    tiebreaker: Option<String>,
) -> Result<SolveReport> {
    let options = SolveOptions {
        mode: parse_mode(mode.as_deref().unwrap_or("flexible"))?,
        heuristic: parse_heuristic(heuristic.as_deref().unwrap_or("ff"))?,
        tiebreaker: parse_tiebreaker(tiebreaker.as_deref().unwrap_or("none"))?,
    };

    Ok(solve_json_path(&grounded_json, options))
}

/// Semantic comparison receipt for two grounded planner artifacts.
#[derive(Debug, Serialize)]
pub struct OracleReport {
    pub equivalent: bool,
    pub first_difference: Option<String>,
    pub legacy_bytes: usize,
    pub candidate_bytes: usize,
}

/// Compare the legacy serializer consequence with a candidate replacement.
///
/// JSON object key order is intentionally ignored by `serde_json::Value`
/// equality; array ordering remains significant because grounded fact/task
/// indices can depend on it.
pub fn oracle_compare(legacy_json: String, candidate_json: String) -> Result<OracleReport> {
    let legacy_bytes = fs::read(&legacy_json).map_err(|error| {
        NounVerbError::execution_error(format!(
            "unable to read legacy artifact '{legacy_json}': {error}"
        ))
    })?;
    let candidate_bytes = fs::read(&candidate_json).map_err(|error| {
        NounVerbError::execution_error(format!(
            "unable to read candidate artifact '{candidate_json}': {error}"
        ))
    })?;

    let legacy: Value = serde_json::from_slice(&legacy_bytes).map_err(|error| {
        NounVerbError::execution_error(format!(
            "legacy artifact '{legacy_json}' is not valid JSON: {error}"
        ))
    })?;
    let candidate: Value = serde_json::from_slice(&candidate_bytes).map_err(|error| {
        NounVerbError::execution_error(format!(
            "candidate artifact '{candidate_json}' is not valid JSON: {error}"
        ))
    })?;

    let first_difference = first_difference("$", &legacy, &candidate);

    Ok(OracleReport {
        equivalent: first_difference.is_none(),
        first_difference,
        legacy_bytes: legacy_bytes.len(),
        candidate_bytes: candidate_bytes.len(),
    })
}

fn first_difference(path: &str, left: &Value, right: &Value) -> Option<String> {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            let keys: BTreeSet<&str> = left
                .keys()
                .map(String::as_str)
                .chain(right.keys().map(String::as_str))
                .collect();

            for key in keys {
                let child_path = format!("{path}.{key}");
                match (left.get(key), right.get(key)) {
                    (Some(left), Some(right)) => {
                        if let Some(diff) = first_difference(&child_path, left, right) {
                            return Some(diff);
                        }
                    }
                    (None, Some(_)) => return Some(format!("{child_path}: missing from legacy")),
                    (Some(_), None) => return Some(format!("{child_path}: missing from candidate")),
                    (None, None) => unreachable!(),
                }
            }
            None
        }
        (Value::Array(left), Value::Array(right)) => {
            if left.len() != right.len() {
                return Some(format!(
                    "{path}: array length differs (legacy={}, candidate={})",
                    left.len(),
                    right.len()
                ));
            }
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                if let Some(diff) = first_difference(&format!("{path}[{index}]"), left, right) {
                    return Some(diff);
                }
            }
            None
        }
        _ if left == right => None,
        _ => Some(format!("{path}: value differs")),
    }
}

fn parse_mode(value: &str) -> Result<SolveMode> {
    match value {
        "flexible" => Ok(SolveMode::Flexible),
        "andstar-fond" => Ok(SolveMode::AndstarFond),
        "fixed" => Ok(SolveMode::Fixed),
        "fixed-ld" => Ok(SolveMode::FixedLd),
        other => Err(NounVerbError::execution_error(format!(
            "unsupported solve mode '{other}'; expected flexible | andstar-fond | fixed | fixed-ld"
        ))),
    }
}

fn parse_heuristic(value: &str) -> Result<Heuristic> {
    match value {
        "ff" => Ok(Heuristic::Ff),
        "add" => Ok(Heuristic::Add),
        "max" => Ok(Heuristic::Max),
        "lmcut" => Ok(Heuristic::Lmcut),
        other => Err(NounVerbError::execution_error(format!(
            "unsupported heuristic '{other}'; expected ff | add | max | lmcut"
        ))),
    }
}

fn parse_tiebreaker(value: &str) -> Result<Tiebreaker> {
    match value {
        "none" => Ok(Tiebreaker::None),
        "policy-size" => Ok(Tiebreaker::PolicySize),
        "closure" => Ok(Tiebreaker::Closure),
        "combined" => Ok(Tiebreaker::Combined),
        other => Err(NounVerbError::execution_error(format!(
            "unsupported tiebreaker '{other}'; expected none | policy-size | closure | combined"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_unknown_mode_instead_of_silently_falling_back() {
        assert!(parse_mode("magic").is_err());
    }

    #[test]
    fn parses_the_fond_htn_frontier_explicitly() {
        assert_eq!(parse_mode("andstar-fond").unwrap(), SolveMode::AndstarFond);
        assert_eq!(parse_heuristic("lmcut").unwrap(), Heuristic::Lmcut);
        assert_eq!(parse_tiebreaker("combined").unwrap(), Tiebreaker::Combined);
    }

    #[test]
    fn oracle_ignores_object_key_order_but_preserves_array_order() {
        let left = json!({"b": [1, 2], "a": true});
        let right = json!({"a": true, "b": [1, 2]});
        assert!(first_difference("$", &left, &right).is_none());

        let wrong = json!({"a": true, "b": [2, 1]});
        assert_eq!(first_difference("$", &left, &wrong), Some("$.b[0]: value differs".to_string()));
    }
}
