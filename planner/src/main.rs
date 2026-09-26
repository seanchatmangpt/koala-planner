use std::env;
use std::process;

extern crate bit_vec;

mod domain_description;
mod graph_lib;
mod heuristics;
mod relaxation;
mod search;
mod task_network;

use crate::search::fixed_method::heuristic_factory;
use crate::search::htn_andstar::TiebreakerKind;
use crate::search::{HeuristicType, SearchResult};
use domain_description::{read_json_domain, FONDProblem};
use search::{
    astar::AStarResult, goal_checks::is_goal_strong_od, search_node::get_successors_systematic,
};

/// Search mode selected by the CLI (`args[2]` historically).
#[derive(Debug, PartialEq)]
enum Mode {
    Fixed,
    Flexible,
    AndStarFond,
    FixedLd,
}

/// Everything the binary needs, derived from argv by `CliConfig::from_args`.
/// Contract: any argument that is not recognized is a usage error (exit code 2),
/// never a silent ignore — a documented-but-unimplemented flag must be refused,
/// not swallowed.
#[derive(Debug)]
struct CliConfig {
    problem_path: String,
    mode: Mode,
    heuristic: HeuristicType,
    tiebreaker: TiebreakerKind,
}

impl CliConfig {
    fn from_args(args: &[String]) -> Result<CliConfig, String> {
        if args.len() < 2 {
            return Err(
                "The path to the problem file is not given.\nUsage: planner <problem.json> \
                 [--fixed|--flexible|--andstar-fond|--fixed-ld] --ff|--add|--max|--lmcut \
                 [--tiebreak policy-size|closure|combined]"
                    .to_string(),
            );
        }
        let problem_path = args[1].clone();

        let mut mode: Option<Mode> = None;
        let mut heuristic: Option<HeuristicType> = None;
        let mut tiebreaker = TiebreakerKind::NoTiebreak;

        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--fixed" | "--flexible" | "--andstar-fond" | "--fixed-ld" => {
                    if mode.is_some() {
                        return Err(format!("Mode flag given more than once: '{}'", args[i]));
                    }
                    mode = Some(match args[i].as_str() {
                        "--fixed" => Mode::Fixed,
                        "--flexible" => Mode::Flexible,
                        "--andstar-fond" => Mode::AndStarFond,
                        _ => Mode::FixedLd,
                    });
                }
                "--add" | "--max" | "--ff" | "--lmcut" => {
                    if heuristic.is_some() {
                        return Err(format!(
                            "Heuristic flag given more than once: '{}'",
                            args[i]
                        ));
                    }
                    heuristic = Some(match args[i].as_str() {
                        "--add" => HeuristicType::HAdd,
                        "--max" => HeuristicType::HMax,
                        "--ff" => HeuristicType::HFF,
                        _ => HeuristicType::HLMCut,
                    });
                }
                "--tiebreak" => {
                    let value = args.get(i + 1).ok_or_else(|| {
                        "--tiebreak requires a value: policy-size | closure | combined".to_string()
                    })?;
                    tiebreaker = match value.as_str() {
                        "policy-size" => TiebreakerKind::PolicySize,
                        "closure" => TiebreakerKind::ClosureFirst,
                        "combined" => TiebreakerKind::Combined,
                        other => {
                            return Err(format!(
                                "Unknown tiebreak '{}': use policy-size | closure | combined",
                                other
                            ))
                        }
                    };
                    i += 1;
                }
                other => {
                    return Err(format!(
                        "Unknown argument '{}': expected a mode (--fixed|--flexible|\
                         --andstar-fond|--fixed-ld), a heuristic (--ff|--add|--max|--lmcut), \
                         or --tiebreak <value>",
                        other
                    ));
                }
            }
            i += 1;
        }

        let heuristic = heuristic.ok_or_else(|| {
            "Expected a heuristic flag: --ff | --add | --max | --lmcut".to_string()
        })?;

        Ok(CliConfig {
            problem_path,
            mode: mode.unwrap_or(Mode::Flexible),
            heuristic,
            tiebreaker,
        })
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = match CliConfig::from_args(&args) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{}", message);
            process::exit(2);
        }
    };
    let problem = read_json_domain(&config.problem_path);
    let h_type = config.heuristic;
    match config.mode {
        Mode::Fixed => {
            println!("Running fixed method solver");
            fixed_method(
                &problem,
                heuristic_factory::create_function_with_heuristic(h_type.as_classical_fn()),
            )
        }
        Mode::Flexible => {
            println!("Running AO* flexible solver");
            ao_star(&problem, h_type)
        }
        Mode::AndStarFond => {
            println!("Running HTN-AND* FOND solver (min-cost)");
            println!("Tiebreaker: {:?}", config.tiebreaker);
            htn_andstar_fond(&problem, h_type, config.tiebreaker)
        }
        Mode::FixedLd => {
            println!("Running fixed strong-LD solver");
            fixed_ld(
                &problem,
                heuristic_factory::create_function_with_heuristic(h_type.as_classical_fn()),
            )
        }
    }
}

fn ao_star(problem: &FONDProblem, h_type: HeuristicType) {
    let (solution, stats) = search::AOStarSearch::run(problem, h_type);
    print!("{}", stats);
    match solution {
        SearchResult::Success(policy) => {
            println!("makespan: {}", policy.makespan);
            println!("policy entries: {}", policy.transitions.len());
            print!("{}", policy);
        }
        SearchResult::NoSolution => {
            println!("Problem has no solution");
        }
    }
}

fn htn_andstar_fond(problem: &FONDProblem, h_type: HeuristicType, tiebreaker: TiebreakerKind) {
    let (solution, stats) = search::htn_andstar::run(problem, h_type, tiebreaker);
    print!("{}", stats);
    match solution {
        SearchResult::Success(x) => {
            println!("makespan: {}", x.makespan);
            println!("policy entries: {}", x.transitions.len());
            print!("{}", x);
        }
        SearchResult::NoSolution => {
            println!("Problem has no solution");
        }
    }
}

fn fixed_method(problem: &FONDProblem, heuristic: heuristic_factory::HeuristicFn) {
    let (solution, stats) = search::fixed_method::astar::a_star_search(
        &problem,
        heuristic,
        get_successors_systematic,
        || 1.0,
        is_goal_strong_od,
    );
    println!("{}", stats);
    if let AStarResult::Strong(policy) = solution {
        println!("Solution was found");
        println!("# of policy entries: {}", policy.transitions.len());
        println!("Success probability: {:.4}", policy.success_probability);
    } else {
        println!("Problem has no solution");
    }
}

fn fixed_ld(problem: &FONDProblem, heuristic: heuristic_factory::HeuristicFn) {
    use search::fixed_method::goal_checks::is_goal_strong_ld;
    let (solution, stats) = search::fixed_method::astar::a_star_search(
        &problem,
        heuristic,
        get_successors_systematic,
        || 1.0,
        is_goal_strong_ld,
    );
    println!("{}", stats);
    if let AStarResult::Strong(policy) = solution {
        println!("Solution was found");
        println!("# of policy entries: {}", policy.transitions.len());
        println!("Success probability: {:.4}", policy.success_probability);
        print!("{}", policy);
    } else {
        println!("Problem has no solution");
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("planner")
            .chain(list.iter().copied())
            .map(String::from)
            .collect()
    }

    fn heuristic_of(config: &CliConfig) -> &'static str {
        match config.heuristic {
            HeuristicType::HFF => "--ff",
            HeuristicType::HAdd => "--add",
            HeuristicType::HMax => "--max",
            HeuristicType::HLMCut => "--lmcut",
        }
    }

    #[test]
    fn missing_problem_path_is_a_usage_error() {
        let err = CliConfig::from_args(&args(&[])).unwrap_err();
        assert!(err.contains("path to the problem file is not given"));
    }

    #[test]
    fn missing_heuristic_is_a_usage_error() {
        let err = CliConfig::from_args(&args(&["p.json"])).unwrap_err();
        assert!(err.contains("Expected a heuristic flag"));
    }

    #[test]
    fn default_mode_is_flexible() {
        let config = CliConfig::from_args(&args(&["p.json", "--ff"])).unwrap();
        assert_eq!(config.mode, Mode::Flexible);
        assert_eq!(heuristic_of(&config), "--ff");
        assert_eq!(config.tiebreaker, TiebreakerKind::NoTiebreak);
    }

    #[test]
    fn every_mode_and_heuristic_combination_parses() {
        for mode_flag in ["--fixed", "--flexible", "--andstar-fond", "--fixed-ld"] {
            for h_flag in ["--ff", "--add", "--max", "--lmcut"] {
                let config = CliConfig::from_args(&args(&["p.json", mode_flag, h_flag])).unwrap();
                assert_eq!(
                    config.mode,
                    match mode_flag {
                        "--fixed" => Mode::Fixed,
                        "--flexible" => Mode::Flexible,
                        "--andstar-fond" => Mode::AndStarFond,
                        _ => Mode::FixedLd,
                    }
                );
                assert_eq!(heuristic_of(&config), h_flag);
            }
        }
    }

    #[test]
    fn unknown_flag_is_refused_not_swallowed() {
        // Regression: `--threshold` was documented and forwarded by solve.py but
        // never parsed here — it used to be silently ignored.
        let err = CliConfig::from_args(&args(&["p.json", "--ff", "--threshold", "5"])).unwrap_err();
        assert!(err.contains("Unknown argument '--threshold'"), "{}", err);
    }

    #[test]
    fn non_flag_garbage_is_refused() {
        let err = CliConfig::from_args(&args(&["p.json", "--ff", "stray"])).unwrap_err();
        assert!(err.contains("Unknown argument 'stray'"), "{}", err);
    }

    #[test]
    fn unknown_tiebreak_value_is_refused() {
        let err =
            CliConfig::from_args(&args(&["p.json", "--ff", "--tiebreak", "bogus"])).unwrap_err();
        assert!(err.contains("Unknown tiebreak 'bogus'"), "{}", err);
    }

    #[test]
    fn tiebreak_without_value_is_refused() {
        let err = CliConfig::from_args(&args(&["p.json", "--ff", "--tiebreak"])).unwrap_err();
        assert!(err.contains("--tiebreak requires a value"), "{}", err);
    }

    #[test]
    fn all_tiebreak_values_parse() {
        for (flag, expected) in [
            ("policy-size", TiebreakerKind::PolicySize),
            ("closure", TiebreakerKind::ClosureFirst),
            ("combined", TiebreakerKind::Combined),
        ] {
            let config = CliConfig::from_args(&args(&[
                "p.json",
                "--andstar-fond",
                "--ff",
                "--tiebreak",
                flag,
            ]))
            .unwrap();
            assert_eq!(config.tiebreaker, expected);
        }
    }

    #[test]
    fn duplicate_mode_is_refused() {
        let err =
            CliConfig::from_args(&args(&["p.json", "--fixed", "--flexible", "--ff"])).unwrap_err();
        assert!(err.contains("Mode flag given more than once"), "{}", err);
    }

    #[test]
    fn duplicate_heuristic_is_refused() {
        let err = CliConfig::from_args(&args(&["p.json", "--ff", "--add"])).unwrap_err();
        assert!(
            err.contains("Heuristic flag given more than once"),
            "{}",
            err
        );
    }
}
