/// Planner module — interprets user instructions and produces structured plans.
///
/// In Phase 1 this returns hardcoded plans. LLM integration comes in Phase 7.
use actions::{ActionParams, ActionType};
use common::LapisResult;

/// A single step in a plan.
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub id: u32,
    pub action_type: ActionType,
    pub parameters: ActionParams,
    pub description: String,
    pub expected_outcome: String,
}

/// A full plan composed of ordered steps.
#[derive(Debug, Clone)]
pub struct Plan {
    pub instruction: String,
    pub steps: Vec<PlanStep>,
}

/// Creates a plan from a user instruction. Phase 1: returns an empty plan.
pub fn create_plan(instruction: &str) -> LapisResult<Plan> {
    Ok(Plan {
        instruction: instruction.to_string(),
        steps: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_plan_preserves_instruction() {
        let plan = create_plan("open notepad").unwrap();
        assert_eq!(plan.instruction, "open notepad");
    }

    #[test]
    fn stub_plan_has_no_steps() {
        let plan = create_plan("anything").unwrap();
        assert!(plan.steps.is_empty());
    }
}
