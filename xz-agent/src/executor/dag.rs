use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::sync::Arc;

use crate::error::AgentError;
use crate::types::agent_def::{AgentDef, AgentRunResult};
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

/// Internal generic Kahn's algorithm implementation.
///
/// Accepts a slice of items and two closures to extract the item's
/// identifier and its dependency list. Returns layers where each layer
/// contains items with no dependencies on other layers.
fn generic_topological_layers<T, F1, F2>(
    items: &[T],
    get_id: F1,
    get_deps: F2,
) -> Result<Vec<Vec<T>>, AgentError>
where
    T: Clone,
    F1: Fn(&T) -> &str,
    F2: Fn(&T) -> &[String],
{
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let item_map: HashMap<&str, &T> = items.iter().map(|item| (get_id(item), item)).collect();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

    for item in items {
        let id = get_id(item);
        in_degree.entry(id).or_insert(0);
        for dep in get_deps(item) {
            if item_map.contains_key(dep.as_str()) {
                adj.entry(dep.as_str()).or_default().push(id);
                *in_degree.entry(id).or_default() += 1;
            }
        }
    }

    // Kahn's algorithm
    let mut queue: VecDeque<&str> = VecDeque::new();
    for item in items {
        if *in_degree.get(get_id(item)).unwrap_or(&0) == 0 {
            queue.push_back(get_id(item));
        }
    }

    let mut layers: Vec<Vec<T>> = Vec::new();
    let mut sorted_count = 0;

    while !queue.is_empty() {
        let layer_size = queue.len();
        let mut layer: Vec<T> = Vec::with_capacity(layer_size);

        for _ in 0..layer_size {
            let id = queue.pop_front().unwrap();
            if let Some(item) = item_map.get(id) {
                layer.push((*item).clone());
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

    // Check for cycles
    if sorted_count != items.len() {
        let cycle_ids: Vec<String> = items
            .iter()
            .filter(|item| *in_degree.get(get_id(item)).unwrap_or(&0) > 0)
            .map(|item| get_id(item).to_string())
            .collect();
        return Err(AgentError::CircularDependency(cycle_ids));
    }

    Ok(layers)
}

/// Topological sort the steps, detecting circular dependencies.
///
/// Returns layers of steps where each layer can be executed in parallel.
pub fn topological_sort(steps: &[AgentStep]) -> Result<Vec<Vec<AgentStep>>, AgentError> {
    generic_topological_layers(steps, |s| s.id.as_str(), |s| &s.depends_on)
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
    generic_topological_layers(agents, |a| a.name.as_str(), |a| &a.depends_on)
}


/// Execute agents in topological order, running each layer concurrently.
///
/// Agents within the same layer have no dependencies on each other and
/// are executed in parallel via `tokio::spawn`. Results are returned in
/// the same order as the input `agents` vector.
///
/// # Errors
///
/// Returns [`AgentError::CircularDependency`](crate::error::AgentError::CircularDependency)
/// if the DAG has cycles.
/// Returns [`AgentError::ExecutionFailed`](crate::error::AgentError::ExecutionFailed)
/// if any spawned task panics.
pub async fn schedule_parallel<F, Fut>(
    agents: Vec<AgentDef>,
    executor: F,
) -> Result<Vec<AgentRunResult>, AgentError>
where
    F: Fn(AgentDef) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<String, AgentError>> + Send,
{
    let layers = topological_layers(&agents)?;
    let mut results_map: HashMap<String, AgentRunResult> = HashMap::new();
    let exec_arc = Arc::new(executor);

    for layer in &layers {
        let handles: Vec<_> = layer
            .iter()
            .map(|agent| {
                let agent = agent.clone();
                let exec = Arc::clone(&exec_arc);
                tokio::spawn(async move {
                    let result = (&*exec)(agent.clone()).await;
                    let (success, output) = match result {
                        Ok(out) => (true, out),
                        Err(e) => (false, e.to_string()),
                    };
                    AgentRunResult {
                        agent_name: agent.name,
                        output,
                        success,
                    }
                })
            })
            .collect();

        let layer_results = futures::future::join_all(handles).await;

        for handle_result in layer_results {
            match handle_result {
                Ok(result) => {
                    results_map.insert(result.agent_name.clone(), result);
                }
                Err(join_err) => {
                    return Err(AgentError::ExecutionFailed(format!(
                        "Task panicked: {}",
                        join_err
                    )));
                }
            }
        }
    }

    // Return results in original agent order
    let mut results: Vec<AgentRunResult> = Vec::with_capacity(agents.len());
    for agent in &agents {
        if let Some(result) = results_map.remove(&agent.name) {
            results.push(result);
        }
    }

    Ok(results)
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

    #[tokio::test]
    async fn schedule_parallel_linear() {
        let agents = vec![
            make_agent("a", vec![]),
            make_agent("b", vec![]),
            make_agent("c", vec![]),
        ];
        let results = schedule_parallel(agents, |agent| async move {
            Ok(format!("done: {}", agent.task))
        })
        .await
        .unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.success));
    }

    #[tokio::test]
    async fn schedule_parallel_diamond() {
        let agents = vec![
            make_agent("a", vec![]),
            make_agent("b", vec!["a"]),
            make_agent("c", vec!["a"]),
            make_agent("d", vec!["b", "c"]),
        ];
        let results = schedule_parallel(agents, |agent| async move {
            Ok(format!("done: {}", agent.task))
        })
        .await
        .unwrap();
        assert_eq!(results.len(), 4);
        assert!(results.iter().all(|r| r.success));
        assert_eq!(results[0].agent_name, "a");
        assert_eq!(results[1].agent_name, "b");
        assert_eq!(results[2].agent_name, "c");
        assert_eq!(results[3].agent_name, "d");
    }

    #[tokio::test]
    async fn schedule_parallel_failure_propagates() {
        let agents = vec![
            make_agent("a", vec![]),
            make_agent("b", vec!["a"]),
        ];
        let results = schedule_parallel(agents, |agent| async move {
            if agent.name == "b" {
                Err(AgentError::ExecutionFailed("b failed".into()))
            } else {
                Ok("ok".into())
            }
        })
        .await
        .unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(!results[1].success);
        assert!(results[1].output.contains("b failed"));
    }
}
