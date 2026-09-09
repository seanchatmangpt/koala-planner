//! Pure-Rust serializer for the grounded format emitted by `pandaPIgrounder`.
//!
//! This crate replaces only the Python `serializer/` transformation boundary:
//!
//! ```text
//! PANDA grounded format -> Koala grounded JSON
//! ```
//!
//! It deliberately does **not** parse HDDL and does **not** replace PANDA's
//! parser or grounder. The existing Python serializer remains the differential
//! oracle until this implementation is qualified on the same fixture corpus.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalEffect {
    pub condition: Vec<String>,
    pub effect: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EffectSet {
    pub unconditional: Vec<String>,
    pub conditional: Vec<ConditionalEffect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    pub add_eff: EffectSet,
    pub del_eff: EffectSet,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub cost: u32,
    pub precond: Vec<String>,
    pub effects: Vec<Effect>,
    pub probability: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Method {
    pub task: String,
    pub subtasks: Vec<String>,
    pub orderings: Vec<(u32, u32)>,
}

/// JSON contract consumed by Koala's existing Rust `domain_reader`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundedProblem {
    pub state_features: Vec<String>,
    pub mutex_groups: Vec<String>,
    pub further_strict_mutex_groups: Vec<String>,
    pub further_non_strict_mutex_groups: Vec<String>,
    pub known_invariants: Vec<String>,
    pub actions: BTreeMap<String, Action>,
    pub initial_state: Vec<String>,
    pub goal: Vec<String>,
    pub initial_abstract_task: String,
    pub methods: BTreeMap<String, Method>,
    pub tasks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializationReport {
    pub output_path: String,
    pub state_features: usize,
    pub actions: usize,
    pub methods: usize,
    pub abstract_tasks: usize,
    pub probability_overrides: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializerError {
    message: String,
}

impl SerializerError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SerializerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SerializerError {}

impl From<std::io::Error> for SerializerError {
    fn from(value: std::io::Error) -> Self {
        Self::new(value.to_string())
    }
}

impl From<serde_json::Error> for SerializerError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(value.to_string())
    }
}

#[derive(Debug, Clone)]
struct PreAction {
    cost: u32,
    precond: Vec<String>,
    add_eff: EffectSet,
    del_eff: EffectSet,
}

#[derive(Debug)]
struct ParsedGrounded {
    state_features: Vec<String>,
    mutex_groups: Vec<String>,
    further_strict_mutex_groups: Vec<String>,
    further_non_strict_mutex_groups: Vec<String>,
    known_invariants: Vec<String>,
    actions: BTreeMap<String, PreAction>,
    initial_state: Vec<String>,
    goal: Vec<String>,
    initial_abstract_task: String,
    methods: BTreeMap<String, Method>,
    abstract_tasks: Vec<String>,
}

pub type ProbabilityMap = BTreeMap<String, Vec<f64>>;

/// Serialize one PANDA grounded document into Koala's current grounded JSON model.
pub fn serialize_grounded(
    input: &str,
    probability_map: Option<&ProbabilityMap>,
) -> Result<GroundedProblem, SerializerError> {
    let parsed = parse_grounded(input)?;
    merge_fond(parsed, probability_map.unwrap_or(&ProbabilityMap::new()))
}

/// Serialize to stable pretty JSON. Object keys are deterministic because the
/// Rust model uses `BTreeMap`; consumers must not rely on object key order.
pub fn serialize_grounded_json(
    input: &str,
    probability_map: Option<&ProbabilityMap>,
) -> Result<String, SerializerError> {
    let problem = serialize_grounded(input, probability_map)?;
    Ok(serde_json::to_string_pretty(&problem)?)
}

/// File-oriented adapter used by the noun-verb CLI and by differential tests.
pub fn convert_file(
    grounded_input: impl AsRef<Path>,
    output_json: impl AsRef<Path>,
    probability_map_path: Option<&Path>,
) -> Result<SerializationReport, SerializerError> {
    let input = fs::read_to_string(grounded_input.as_ref())?;
    let probability_map = match probability_map_path {
        Some(path) => Some(read_probability_map(path)?),
        None => None,
    };
    let problem = serialize_grounded(&input, probability_map.as_ref())?;
    let json = serde_json::to_string_pretty(&problem)?;
    fs::write(output_json.as_ref(), json)?;

    Ok(SerializationReport {
        output_path: output_json.as_ref().display().to_string(),
        state_features: problem.state_features.len(),
        actions: problem.actions.len(),
        methods: problem.methods.len(),
        abstract_tasks: problem.tasks.len(),
        probability_overrides: probability_map.as_ref().map_or(0, BTreeMap::len),
    })
}

pub fn read_probability_map(path: impl AsRef<Path>) -> Result<ProbabilityMap, SerializerError> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn parse_grounded(input: &str) -> Result<ParsedGrounded, SerializerError> {
    let sections = split_sections(input)?;
    if sections.len() != 11 {
        return Err(SerializerError::new(format!(
            "expected 11 PANDA grounded sections, found {}",
            sections.len()
        )));
    }

    let state_features = counted_lines(&sections[0], "state features")?;
    let mutex_groups = counted_lines(&sections[1], "mutex groups")?;
    let further_strict_mutex_groups = counted_lines(&sections[2], "further strict mutex groups")?;
    let further_non_strict_mutex_groups =
        counted_lines(&sections[3], "further non-strict mutex groups")?;
    let known_invariants = counted_lines(&sections[4], "invariants")?;

    let action_count = count_prefix(&sections[5], "actions")?;
    let action_lines = &sections[5][1..];
    let expected_action_lines = action_count
        .checked_mul(4)
        .ok_or_else(|| SerializerError::new("action count overflow"))?;
    if action_lines.len() != expected_action_lines {
        return Err(SerializerError::new(format!(
            "actions section declares {action_count} actions but contains {} payload lines (expected {expected_action_lines})",
            action_lines.len()
        )));
    }

    let tasks_payload = counted_lines(&sections[8], "tasks")?;
    if tasks_payload.len() < action_count {
        return Err(SerializerError::new(format!(
            "tasks section contains {} tasks but actions section contains {action_count} primitives",
            tasks_payload.len()
        )));
    }

    let task_names: Vec<String> = tasks_payload
        .iter()
        .enumerate()
        .map(|(index, line)| parse_task_name(index, line))
        .collect::<Result<_, _>>()?;

    let mut actions = BTreeMap::new();
    for action_index in 0..action_count {
        let offset = action_index * 4;
        let cost = parse_u32(&action_lines[offset], "action cost")?;
        let precond_ids = parse_terminated_ids(&action_lines[offset + 1], "action preconditions")?;
        let precond = ids_to_names(&precond_ids, &state_features, "action precondition")?;
        let add_eff = parse_effect_line(&action_lines[offset + 2], &state_features)?;
        let del_eff = parse_effect_line(&action_lines[offset + 3], &state_features)?;
        actions.insert(
            task_names[action_index].clone(),
            PreAction {
                cost,
                precond,
                add_eff,
                del_eff,
            },
        );
    }

    let initial_state_line = singleton_line(&sections[6], "initial state")?;
    let initial_state_ids = parse_terminated_ids(initial_state_line, "initial state")?;
    let initial_state = ids_to_names(&initial_state_ids, &state_features, "initial state")?;

    // Preserve the legacy JSON contract here. The Python serializer never maps
    // the goal IDs back to fact names; the Rust planner currently ignores this
    // field as well. Changing that semantic is a separate migration.
    let goal = vec![singleton_line(&sections[7], "goal")?.to_string()];

    let initial_task_id = parse_usize(
        singleton_line(&sections[9], "initial abstract task")?,
        "initial abstract task",
    )?;
    let initial_abstract_task = task_names
        .get(initial_task_id)
        .cloned()
        .ok_or_else(|| SerializerError::new(format!(
            "initial abstract task id {initial_task_id} is outside task table of length {}",
            task_names.len()
        )))?;

    let method_count = count_prefix(&sections[10], "methods")?;
    let method_lines = &sections[10][1..];
    let expected_method_lines = method_count
        .checked_mul(4)
        .ok_or_else(|| SerializerError::new("method count overflow"))?;
    if method_lines.len() != expected_method_lines {
        return Err(SerializerError::new(format!(
            "methods section declares {method_count} methods but contains {} payload lines (expected {expected_method_lines})",
            method_lines.len()
        )));
    }

    let mut methods = BTreeMap::new();
    for method_index in 0..method_count {
        let offset = method_index * 4;
        let raw_name = method_lines[offset].clone();
        let task_id = parse_usize(&method_lines[offset + 1], "method task id")?;
        let task = task_names
            .get(task_id)
            .cloned()
            .ok_or_else(|| SerializerError::new(format!(
                "method '{raw_name}' references task id {task_id}, outside task table"
            )))?;
        let subtask_ids = parse_terminated_ids(&method_lines[offset + 2], "method subtasks")?;
        let subtasks = subtask_ids
            .iter()
            .map(|id| {
                task_names.get(*id as usize).cloned().ok_or_else(|| {
                    SerializerError::new(format!(
                        "method '{raw_name}' references subtask id {id}, outside task table"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let ordering_ids = parse_terminated_ids(&method_lines[offset + 3], "method orderings")?;
        if ordering_ids.len() % 2 != 0 {
            return Err(SerializerError::new(format!(
                "method '{raw_name}' has an odd number of ordering indices"
            )));
        }
        let orderings = ordering_ids
            .chunks_exact(2)
            .map(|pair| (pair[0], pair[1]))
            .collect();
        let name = format!("{raw_name}_{method_index}");
        methods.insert(
            name,
            Method {
                task,
                subtasks,
                orderings,
            },
        );
    }

    let abstract_tasks = task_names[action_count..].to_vec();

    Ok(ParsedGrounded {
        state_features,
        mutex_groups,
        further_strict_mutex_groups,
        further_non_strict_mutex_groups,
        known_invariants,
        actions,
        initial_state,
        goal,
        initial_abstract_task,
        methods,
        abstract_tasks,
    })
}

fn merge_fond(
    mut parsed: ParsedGrounded,
    probability_map: &ProbabilityMap,
) -> Result<GroundedProblem, SerializerError> {
    let expected_counts = extract_nondeterministic_counts(&parsed.methods)?;
    let translator = build_nondeterministic_translator(&expected_counts)?;

    let mut actions: BTreeMap<String, Action> = BTreeMap::new();
    for (name, action) in &parsed.actions {
        let effect = Effect {
            add_eff: action.add_eff.clone(),
            del_eff: action.del_eff.clone(),
        };

        if name.starts_with("fond_act__") {
            let translated = translator.get(name).ok_or_else(|| {
                SerializerError::new(format!(
                    "synthetic FOND action '{name}' has no method-derived translation"
                ))
            })?;
            match actions.get_mut(translated) {
                Some(existing) => existing.effects.push(effect),
                None => {
                    actions.insert(
                        translated.clone(),
                        Action {
                            cost: action.cost,
                            precond: action.precond.clone(),
                            effects: vec![effect],
                            probability: Vec::new(),
                        },
                    );
                }
            }
        } else {
            actions.insert(
                name.clone(),
                Action {
                    cost: action.cost,
                    precond: action.precond.clone(),
                    effects: vec![effect],
                    probability: Vec::new(),
                },
            );
        }
    }

    // Mirror the legacy FOND pruning rule: if the number of synthetic methods
    // for a nondeterministic task does not equal the encoded outcome count, the
    // combined action is not admitted.
    let mut actual_counts: BTreeMap<String, usize> =
        expected_counts.keys().cloned().map(|key| (key, 0)).collect();
    for method in parsed.methods.values() {
        if let Some(count) = actual_counts.get_mut(&method.task) {
            *count += 1;
        }
    }
    for (task, expected) in &expected_counts {
        if actual_counts.get(task).copied().unwrap_or(0) != *expected {
            actions.remove(task);
        }
    }

    actions.retain(|name, _| !name.starts_with("__method_precondition_fond_act__"));
    parsed
        .methods
        .retain(|name, _| !name.starts_with("fond_act__"));
    parsed
        .abstract_tasks
        .retain(|task| !expected_counts.contains_key(task));

    inject_probabilities(&mut actions, probability_map)?;

    Ok(GroundedProblem {
        state_features: parsed.state_features,
        mutex_groups: parsed.mutex_groups,
        further_strict_mutex_groups: parsed.further_strict_mutex_groups,
        further_non_strict_mutex_groups: parsed.further_non_strict_mutex_groups,
        known_invariants: parsed.known_invariants,
        actions,
        initial_state: parsed.initial_state,
        goal: parsed.goal,
        initial_abstract_task: parsed.initial_abstract_task,
        methods: parsed.methods,
        tasks: parsed.abstract_tasks,
    })
}

fn inject_probabilities(
    actions: &mut BTreeMap<String, Action>,
    probability_map: &ProbabilityMap,
) -> Result<(), SerializerError> {
    for (action_name, action) in actions {
        let outcome_count = action.effects.len();
        if outcome_count == 0 {
            return Err(SerializerError::new(format!(
                "action '{action_name}' has zero effects and cannot receive a probability distribution"
            )));
        }
        let base_name = action_name.split('[').next().unwrap_or(action_name);
        if let Some(probabilities) = probability_map.get(base_name) {
            if probabilities.len() != outcome_count {
                return Err(SerializerError::new(format!(
                    "action '{action_name}' has {outcome_count} effects but probability map supplies {} values",
                    probabilities.len()
                )));
            }
            action.probability = probabilities.clone();
        } else {
            action.probability = vec![1.0 / outcome_count as f64; outcome_count];
        }
    }
    Ok(())
}

fn extract_nondeterministic_counts(
    methods: &BTreeMap<String, Method>,
) -> Result<BTreeMap<String, usize>, SerializerError> {
    let mut counts = BTreeMap::new();
    for (name, method) in methods {
        if !name.starts_with("fond_act__") {
            continue;
        }
        let synthetic = method
            .subtasks
            .iter()
            .find(|task| task.starts_with("fond_act__"))
            .ok_or_else(|| {
                SerializerError::new(format!(
                    "synthetic FOND method '{name}' has no synthetic FOND subtask"
                ))
            })?;
        let count = parse_synthetic_outcome_count(synthetic)?;
        counts.insert(method.task.clone(), count);
    }
    Ok(counts)
}

fn parse_synthetic_outcome_count(name: &str) -> Result<usize, SerializerError> {
    let stem = name.split('[').next().unwrap_or(name);
    let (_, count) = stem.rsplit_once("of").ok_or_else(|| {
        SerializerError::new(format!(
            "synthetic FOND task '{name}' does not contain an outcome count"
        ))
    })?;
    parse_usize(count, "synthetic FOND outcome count")
}

fn build_nondeterministic_translator(
    counts: &BTreeMap<String, usize>,
) -> Result<BTreeMap<String, String>, SerializerError> {
    let mut translator = BTreeMap::new();
    for (task, count) in counts {
        if *count == 0 {
            return Err(SerializerError::new(format!(
                "nondeterministic task '{task}' declares zero outcomes"
            )));
        }
        let (base, params) = match task.split_once('[') {
            Some((base, rest)) => (base, Some(rest)),
            None => (task.as_str(), None),
        };
        for index in 1..=*count {
            let synthetic = match params {
                Some(params) => format!("fond_act__{base}_{index}of{count}[{params}"),
                None => format!("fond_act__{base}_{index}of{count}"),
            };
            translator.insert(synthetic, task.clone());
        }
    }
    Ok(translator)
}

fn split_sections(input: &str) -> Result<Vec<Vec<String>>, SerializerError> {
    let mut sections = Vec::new();
    for raw in input.split(";;").skip(1) {
        let mut lines = raw
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with(';'));
        let _header = lines
            .next()
            .ok_or_else(|| SerializerError::new("grounded section is missing its header"))?;
        sections.push(lines.map(ToOwned::to_owned).collect());
    }
    Ok(sections)
}

fn count_prefix(section: &[String], name: &str) -> Result<usize, SerializerError> {
    let first = section
        .first()
        .ok_or_else(|| SerializerError::new(format!("{name} section is empty")))?;
    parse_usize(first, &format!("{name} count"))
}

fn counted_lines(section: &[String], name: &str) -> Result<Vec<String>, SerializerError> {
    let count = count_prefix(section, name)?;
    let payload = &section[1..];
    if payload.len() != count {
        return Err(SerializerError::new(format!(
            "{name} section declares {count} entries but contains {}",
            payload.len()
        )));
    }
    Ok(payload.to_vec())
}

fn singleton_line<'a>(section: &'a [String], name: &str) -> Result<&'a str, SerializerError> {
    if section.len() != 1 {
        return Err(SerializerError::new(format!(
            "{name} section must contain exactly one line, found {}",
            section.len()
        )));
    }
    Ok(&section[0])
}

fn parse_task_name(index: usize, line: &str) -> Result<String, SerializerError> {
    let mut parts = line.split_whitespace();
    let kind = parts
        .next()
        .ok_or_else(|| SerializerError::new(format!("task {index} has no kind marker")))?;
    let name = parts
        .next()
        .ok_or_else(|| SerializerError::new(format!("task {index} has no name")))?;
    if parts.next().is_some() {
        return Err(SerializerError::new(format!(
            "task {index} contains whitespace in its name, which PANDA grounded format forbids"
        )));
    }
    if kind != "0" && kind != "1" {
        return Err(SerializerError::new(format!(
            "task {index} has unknown kind marker '{kind}'"
        )));
    }
    Ok(name.to_string())
}

fn parse_effect_line(line: &str, features: &[String]) -> Result<EffectSet, SerializerError> {
    let tokens = parse_i64_tokens(line, "effect")?;
    let mut cursor = 0usize;
    let mut result = EffectSet::default();
    let mut terminated = false;

    while cursor < tokens.len() {
        let condition_count = tokens[cursor];
        cursor += 1;
        if condition_count == -1 {
            terminated = true;
            break;
        }
        if condition_count < 0 {
            return Err(SerializerError::new(format!(
                "effect block has invalid negative condition count {condition_count}"
            )));
        }
        let condition_count = condition_count as usize;
        let needed = condition_count + 1;
        if cursor + needed > tokens.len() {
            return Err(SerializerError::new(
                "effect block ends before its conditions/effect are complete",
            ));
        }
        let condition_ids = &tokens[cursor..cursor + condition_count];
        let effect_id = tokens[cursor + condition_count];
        cursor += needed;

        let condition = condition_ids
            .iter()
            .map(|id| feature_name(*id, features, "conditional effect condition"))
            .collect::<Result<Vec<_>, _>>()?;
        let effect = feature_name(effect_id, features, "effect")?;
        if condition.is_empty() {
            result.unconditional.push(effect);
        } else {
            result
                .conditional
                .push(ConditionalEffect { condition, effect });
        }
    }

    if !terminated {
        return Err(SerializerError::new("effect line is missing -1 terminator"));
    }
    if cursor != tokens.len() {
        return Err(SerializerError::new(
            "effect line contains tokens after its -1 terminator",
        ));
    }
    Ok(result)
}

fn parse_terminated_ids(line: &str, context: &str) -> Result<Vec<u32>, SerializerError> {
    let tokens = parse_i64_tokens(line, context)?;
    let terminator = tokens.iter().position(|value| *value == -1).ok_or_else(|| {
        SerializerError::new(format!("{context} line is missing -1 terminator"))
    })?;
    if terminator + 1 != tokens.len() {
        return Err(SerializerError::new(format!(
            "{context} line contains tokens after its -1 terminator"
        )));
    }
    tokens[..terminator]
        .iter()
        .map(|value| {
            u32::try_from(*value).map_err(|_| {
                SerializerError::new(format!(
                    "{context} contains invalid negative id {value}"
                ))
            })
        })
        .collect()
}

fn ids_to_names(
    ids: &[u32],
    features: &[String],
    context: &str,
) -> Result<Vec<String>, SerializerError> {
    ids.iter()
        .map(|id| feature_name(*id as i64, features, context))
        .collect()
}

fn feature_name(
    id: i64,
    features: &[String],
    context: &str,
) -> Result<String, SerializerError> {
    let id = usize::try_from(id).map_err(|_| {
        SerializerError::new(format!("{context} contains invalid negative fact id {id}"))
    })?;
    features.get(id).cloned().ok_or_else(|| {
        SerializerError::new(format!(
            "{context} references fact id {id}, outside feature table of length {}",
            features.len()
        ))
    })
}

fn parse_i64_tokens(line: &str, context: &str) -> Result<Vec<i64>, SerializerError> {
    line.split_whitespace()
        .map(|token| {
            token.parse::<i64>().map_err(|error| {
                SerializerError::new(format!(
                    "{context} contains non-integer token '{token}': {error}"
                ))
            })
        })
        .collect()
}

fn parse_usize(value: &str, context: &str) -> Result<usize, SerializerError> {
    value.parse::<usize>().map_err(|error| {
        SerializerError::new(format!("invalid {context} '{value}': {error}"))
    })
}

fn parse_u32(value: &str, context: &str) -> Result<u32, SerializerError> {
    value.parse::<u32>().map_err(|error| {
        SerializerError::new(format!("invalid {context} '{value}': {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
;; State Features
1
f
;; Mutex Groups
1
0 0 f
;; Further Strict Mutex Groups
0
;; Further Non-Strict Mutex Groups
0
;; Invariants
0
;; Actions
1
1
-1
0 0 -1
-1
;; Initial State
-1
;; Goal
-1
;; Tasks and Tasks Names
2
0 a
1 top
;; Initial Abstract Task
1
;; Decomposition Methods
1
m
1
0 -1
-1
"#;

    #[test]
    fn serializes_minimal_grounded_problem() {
        let problem = serialize_grounded(MINIMAL, None).unwrap();
        assert_eq!(problem.state_features, vec!["f"]);
        assert_eq!(problem.tasks, vec!["top"]);
        assert_eq!(problem.initial_abstract_task, "top");
        assert_eq!(problem.methods["m_0"].task, "top");
        let action = &problem.actions["a"];
        assert_eq!(action.precond, Vec::<String>::new());
        assert_eq!(action.effects.len(), 1);
        assert_eq!(action.effects[0].add_eff.unconditional, vec!["f"]);
        assert_eq!(action.probability, vec![1.0]);
    }

    #[test]
    fn effect_parser_preserves_conditional_effects() {
        let features = vec!["f0".to_string(), "f1".to_string(), "f2".to_string()];
        let effect = parse_effect_line("0 0 1 1 2 -1", &features).unwrap();
        assert_eq!(effect.unconditional, vec!["f0"]);
        assert_eq!(
            effect.conditional,
            vec![ConditionalEffect {
                condition: vec!["f1".to_string()],
                effect: "f2".to_string(),
            }]
        );
    }

    #[test]
    fn parses_multi_digit_fond_outcome_counts() {
        assert_eq!(
            parse_synthetic_outcome_count("fond_act__observe_10of12[x]").unwrap(),
            12
        );
    }

    #[test]
    fn rejects_probability_arity_drift() {
        let mut map = ProbabilityMap::new();
        map.insert("a".to_string(), vec![0.5, 0.5]);
        let error = serialize_grounded(MINIMAL, Some(&map)).unwrap_err();
        assert!(error.to_string().contains("probability map supplies 2 values"));
    }
}
