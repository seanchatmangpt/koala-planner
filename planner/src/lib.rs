//! Reusable Rust API for Koala's FOND-HTN search engine.
//!
//! This library is intentionally a thin typed boundary over the existing search
//! implementation. It does not replace the legacy `solve.py` + PANDA pipeline;
//! that path remains the compatibility oracle while the Rust surface is proven.

extern crate bit_vec;

mod domain_description;
mod graph_lib;
mod heuristics;
mod relaxation;
mod search;
mod task_network;

use serde::{Deserialize, Serialize};

use crate::domain_description::read_json_domain;
pub use crate::domain_description::FONDProblem;
use crate::search::fixed_method::heuristic_factory;
use crate::search::htn_andstar::TiebreakerKind;
use crate::search::{HeuristicType, SearchResult};
use crate::search::{
    astar::AStarResult,
    goal_checks::is_goal_strong_od,
    search_node::get_successors_systematic,
};

/// Supported search modes at the stable library boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SolveMode {
    /// Existing AO* flexible FOND-HTN search.
    Flexible,
    /// Existing HTN-AND* FOND search.
    AndstarFond,
    /// Existing fixed-method strong-OD search.
    Fixed,
    /// Existing fixed-method strong-LD search.
    FixedLd,
}

/// Heuristic selection independent of the internal search-module enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Heuristic {
    Ff,
    Add,
    Max,
    Lmcut,
}

/// Secondary ordering used by HTN-AND* when f-values tie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tiebreaker {
    None,
    PolicySize,
    Closure,
    Combined,
}

/// Typed options for one planning invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolveOptions {
    pub mode: SolveMode,
    pub heuristic: Heuristic,
    pub tiebreaker: Tiebreaker,
}

impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            mode: SolveMode::Flexible,
            heuristic: Heuristic::Ff,
            tiebreaker: Tiebreaker::None,
        }
    }
}

/// Stable, serializable consequence of one search invocation.
///
/// `policy_text` is deliberately retained as a compatibility projection in
/// this first boundary. Later PRs may add richer typed policy projections,
/// but downstream consumers no longer need to depend on stdout parsing for
/// the common solve metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveReport {
    pub solved: bool,
    pub makespan: Option<u16>,
    pub policy_entries: usize,
    pub success_probability: Option<f64>,
    pub policy_text: Option<String>,
    pub stats_text: String,
}

impl SolveReport {
    fn no_solution(stats_text: String) -> Self {
        Self {
            solved: false,
            makespan: None,
            policy_entries: 0,
            success_probability: None,
            policy_text: None,
            stats_text,
        }
    }

    fn from_policy(policy: crate::search::StrongPolicy, stats_text: String) -> Self {
        Self {
            solved: true,
            makespan: Some(policy.makespan),
            policy_entries: policy.transitions.len(),
            success_probability: Some(policy.success_probability),
            policy_text: Some(policy.to_string()),
            stats_text,
        }
    }
}

impl Heuristic {
    fn internal(self) -> HeuristicType {
        match self {
            Self::Ff => HeuristicType::HFF,
            Self::Add => HeuristicType::HAdd,
            Self::Max => HeuristicType::HMax,
            Self::Lmcut => HeuristicType::HLMCut,
        }
    }
}

impl Tiebreaker {
    fn internal(self) -> TiebreakerKind {
        match self {
            Self::None => TiebreakerKind::NoTiebreak,
            Self::PolicySize => TiebreakerKind::PolicySize,
            Self::Closure => TiebreakerKind::ClosureFirst,
            Self::Combined => TiebreakerKind::Combined,
        }
    }
}

/// Solve an already-grounded Koala problem with the existing search engine.
///
/// This function is SELECT-only: it computes a candidate policy and has no
/// filesystem, network, deployment, or actuation authority.
pub fn solve(problem: &FONDProblem, options: SolveOptions) -> SolveReport {
    match options.mode {
        SolveMode::Flexible => {
            let (result, stats) =
                crate::search::AOStarSearch::run(problem, options.heuristic.internal());
            match result {
                SearchResult::Success(policy) => {
                    SolveReport::from_policy(policy, stats.to_string())
                }
                SearchResult::NoSolution => SolveReport::no_solution(stats.to_string()),
            }
        }
        SolveMode::AndstarFond => {
            let (result, stats) = crate::search::htn_andstar::run(
                problem,
                options.heuristic.internal(),
                options.tiebreaker.internal(),
            );
            match result {
                SearchResult::Success(policy) => {
                    SolveReport::from_policy(policy, stats.to_string())
                }
                SearchResult::NoSolution => SolveReport::no_solution(stats.to_string()),
            }
        }
        SolveMode::Fixed => solve_fixed(problem, options.heuristic, false),
        SolveMode::FixedLd => solve_fixed(problem, options.heuristic, true),
    }
}

/// Compatibility entry point for the JSON artifact produced by the current
/// PANDA/Python preprocessing pipeline.
pub fn solve_json_path(path: &str, options: SolveOptions) -> SolveReport {
    let problem = read_json_domain(path);
    solve(&problem, options)
}

fn solve_fixed(problem: &FONDProblem, heuristic: Heuristic, long_distance: bool) -> SolveReport {
    let h_type = heuristic.internal();
    let heuristic = heuristic_factory::create_function_with_heuristic(h_type.as_classical_fn());

    let (result, stats) = if long_distance {
        crate::search::fixed_method::astar::a_star_search(
            problem,
            heuristic,
            get_successors_systematic,
            || 1.0,
            crate::search::fixed_method::goal_checks::is_goal_strong_ld,
        )
    } else {
        crate::search::fixed_method::astar::a_star_search(
            problem,
            heuristic,
            get_successors_systematic,
            || 1.0,
            is_goal_strong_od,
        )
    };

    match result {
        AStarResult::Strong(policy) => SolveReport::from_policy(policy, stats.to_string()),
        AStarResult::Linear(_) | AStarResult::NoSolution => {
            SolveReport::no_solution(stats.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_preserve_the_existing_flexible_ff_frontier() {
        let options = SolveOptions::default();
        assert_eq!(options.mode, SolveMode::Flexible);
        assert_eq!(options.heuristic, Heuristic::Ff);
        assert_eq!(options.tiebreaker, Tiebreaker::None);
    }

    #[test]
    fn options_have_a_stable_json_contract() {
        let encoded = serde_json::to_string(&SolveOptions {
            mode: SolveMode::AndstarFond,
            heuristic: Heuristic::Lmcut,
            tiebreaker: Tiebreaker::Combined,
        })
        .unwrap();
        assert!(encoded.contains("andstar-fond"));
        assert!(encoded.contains("lmcut"));
        assert!(encoded.contains("combined"));
    }
}
