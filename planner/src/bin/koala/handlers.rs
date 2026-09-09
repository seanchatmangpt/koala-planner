use clap_noun_verb::{NounVerbError, Result};
use planner::{solve_json_path, Heuristic, SolveMode, SolveOptions, SolveReport, Tiebreaker};

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
}
