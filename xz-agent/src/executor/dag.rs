use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::AgentError;
use crate::types::agent_def::AgentDef;
use crate::types::step::AgentStep;

/// Execution context passed between steps.
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    pub variables: HashMap<String, String>,
    pub step_outputs: HashMap<String, String>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    pub fn set_step_output(&mut self, step_id: impl Into<String>, output: impl Into<String>) {
        self.step_outputs.insert(step_id.into(), output.into());
    }

    pub fn get_step_output(&self, step_id: &str) -> Option<&String> {
        self.step_outputs.get(step_id)
    }

    /// Resolve template variables like {{ steps.step_id.output }}
    pub fn resolve_template(&self, template: &str) -> String {
        let mut result = template.to_string();
        for (step_id, output) in &self.step_outputs {
            let placeholder = format!("{{{{ steps.{}.output }}}}", step_id);
            result = result.replace(&placeholder, output);
        }
        for (key, value) in &self.variables {
            let placeholder = format!("{{{{ variables.{} }}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

/// Topological sort the steps, detecting circular dependencies.
///
/// Returns layers of steps where each layer can be executed in parallel.
pub fn topological_sort(steps: &[AgentStep]) -> Result<Vec<Vec<AgentStep>>, AgentError> {
    if steps.is_empty() {
        return Ok(vec![]);
    }

    let step_map: HashMap<&str, &AgentStep> = steps.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

    for step in steps {
        in_degree.entry(&step.id).or_insert(0);
        for dep in &step.depends_on {
            if step_map.contains_key(dep.as_str()) {
                adj.entry(dep.as_str()).or_default().push(&step.id);
                *in_degree.entry(&step.id).or_default() += 1;
            }
        }
    }

    // Kahn's algorithm
    let mut queue: VecDeque<&str> = VecDeque::new();
    for step in steps {
        if in_degree.get(step.id.as_str()).copied().unwrap_or(0) == 0 {
            queue.push_back(&step.id);
        }
    }

    let mut layers: Vec<Vec<AgentStep>> = Vec::new();
    let mut sorted_count = 0;

    while !queue.is_empty() {
        let layer_size = queue.len();
        let mut layer: Vec<AgentStep> = Vec::new();

        for _ in 0..layer_size {
            let id = queue.pop_front().unwrap();
            layer.push(step_map[id].clone());
            sorted_count += 1;

            if let Some(children) = adj.get(id) {
                for child in children {
                    let entry = in_degree.get_mut(child).unwrap();
                    *entry = entry.saturating_sub(1);
                    if *entry == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }

        if !layer.is_empty() {
            layers.push(layer);
        }
    }

    if sorted_count != steps.len() {
        let sorted_ids: HashSet<&str> = layers
            .iter()
            .flatten()
            .map(|s| s.id.as_str())
            .collect();
        let cycle_ids: Vec<String> = steps
            .iter()
            .filter(|s| !sorted_ids.contains(s.id.as_str()))
            .map(|s| s.id.clone())
            .collect();
        return Err(AgentError::CircularDependency(cycle_ids));
    }

    Ok(layers)
}

/// Validate that a set of agent definitions form a valid DAG (no cycles).
///
/// Uses Kahn's algorithm internally via [`topological_layers`]. Returns
/// `Ok(())` if the graph is a valid DAG, or
/// [`Err(AgentError::CircularDependency)`](crate::error::AgentError::CircularDependency)
/// listing agents involved in cycles.
pub fn validate_dag(agents: &[AgentDef]) -> Result<(), AgentError> {
    topological_layers(agents)?;
    Ok(())
}

/// Compute topological layers for parallel execution of agent definitions.
///
/// Returns agent groups organized into layers where each layer contains
/// agents that can run in parallel (no inter-dependencies within a layer).
/// Uses Kahn's algorithm for cycle detection.
///
/// # Errors
///
/// Returns [`AgentError::CircularDependency`](crate::error::AgentError::CircularDependency)
/// if a cycle is detected in the dependency graph.
pub fn topological_layers(agents: &[AgentDef]) -> Result<Vec<Vec<AgentDef>>, AgentError> {
    if agents.is_empty() {
        return Ok(Vec::new());
    }

    let agent_map: HashMap<&str, &AgentDef> =
        agents.iter().map(|a| (a.name.as_str(), a)).collect();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

    for agent in agents {
        in_degree.entry(&agent.name).or_insert(0);
        for dep in &agent.depends_on {
            if agent_map.contains_key(dep.as_str()) {
                adj.entry(dep.as_str()).or_default().push(&agent.name);
                *in_degree.entry(&agent.name).or_default() += 1;
            }
        }
    }

    let mut queue: VecDeque<&str> = VecDeque::new();
    for agent in agents {
        if *in_degree.get(agent.name.as_str()).unwrap_or(&0) == 0 {
            queue.push_back(&agent.name);
        }
    }

    let mut layers: Vec<Vec<AgentDef>> = Vec::new();
    let mut sorted_count = 0;

    while !queue.is_empty() {
        let layer_size = queue.len();
        let mut layer: Vec<AgentDef> = Vec::with_capacity(layer_size);

        for _ in 0..layer_size {
            let id = queue.pop_front().unwrap();
            if let Some(agent) = agent_map.get(id) {
                layer.push((*agent).clone());
            }
            sorted_count += 1;

            if let Some(children) = adj.get(id) {
                for child in children {
                    if let Some(deg) = in_degree.get_mut(child) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push_back(child);
                        }
                    }
                }
            }
        }

        if !layer.is_empty() {
            layers.push(layer);
        }
    }

    // Check for cycles: if we couldn't sort all agents, the unsorted ones are in cycles
    if sorted_count != agents.len() {
        let cycle_ids: Vec<String> = agents
            .iter()
            .filter(|a| {
                in_degree
                    .get(a.name.as_str())
                    .copied()
                    .unwrap_or(0)
                    > 0
            })
            .map(|a| a.name.clone())
            .collect();
        return Err(AgentError::CircularDependency(cycle_ids));
    }

    Ok(layers)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::step::AgentAction;

    fn make_step(id: &str, deps: Vec<&str>) -> AgentStep {
        AgentStep::new(
            id,
            format!("Step {}", id),
            AgentAction::Notify {
                method: crate::types::step::NotificationMethod::InApp,
                title_template: "test".into(),
                body_template: "test".into(),
            },
        )
        .with_deps(deps)
    }

    // Helper so that `make_step` can take dependencies
    impl AgentStep {
        fn with_deps(mut self, deps: Vec<&str>) -> Self {
            self.depends_on = deps.into_iter().map(|d| d.to_string()).collect();
            self
        }
    }

    fn make_agent(name: &str, deps: Vec<&str>) -> AgentDef {
        AgentDef {
            name: name.into(),
            task: format!("Task {}", name),
            depends_on: deps.into_iter().map(|d| d.to_string()).collect(),
        }
    }

    #[test]
    fn test_topological_sort_linear() {
        let steps = vec![
            make_step("c", vec!["b"]),
            make_step("b", vec!["a"]),
            make_step("a", vec![]),
        ];
        let layers = topological_sort(&steps).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0][0].id, "a");
        assert_eq!(layers[1][0].id, "b");
        assert_eq!(layers[2][0].id, "c");
    }

    #[test]
    fn test_topological_sort_diamond() {
        let steps = vec![
            make_step("a", vec![]),
            make_step("b", vec!["a"]),
            make_step("c", vec!["a"]),
            make_step("d", vec!["b", "c"]),
        ];
        let layers = topological_sort(&steps).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].len(), 1); // a
        assert_eq!(layers[1].len(), 2); // b, c
        assert_eq!(layers[2].len(), 1); // d
    }

    #[test]
    fn test_circular_dependency() {
        let steps = vec![
            make_step("a", vec!["b"]),
            make_step("b", vec!["a"]),
        ];
        let result = topological_sort(&steps);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentError::CircularDependency(_)
        ));
    }

    #[test]
    fn validate_dag_linear_chain() {
        let agents = vec![
            make_agent("a", vec![]),
            make_agent("b", vec!["a"]),
            make_agent("c", vec!["b"]),
        ];
        assert!(validate_dag(&agents).is_ok());
    }

    #[test]
    fn validate_dag_diamond() {
        let agents = vec![
            make_agent("a", vec![]),
            make_agent("b", vec!["a"]),
            make_agent("c", vec!["a"]),
            make_agent("d", vec!["b", "c"]),
        ];
        assert!(validate_dag(&agents).is_ok());
    }

    #[test]
    fn validate_dag_circular() {
        let agents = vec![
            make_agent("a", vec!["b"]),
            make_agent("b", vec!["a"]),
        ];
        let result = validate_dag(&agents);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentError::CircularDependency(_)
        ));
    }

    #[test]
    fn topological_layers_diamond() {
        let agents = vec![
            make_agent("a", vec![]),
            make_agent("b", vec!["a"]),
            make_agent("c", vec!["a"]),
            make_agent("d", vec!["b", "c"]),
        ];
        let layers = topological_layers(&agents).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].len(), 1);
        assert_eq!(layers[0][0].name, "a");
        assert_eq!(layers[1].len(), 2);
        assert_eq!(layers[2].len(), 1);
        assert_eq!(layers[2][0].name, "d");
    }
}
