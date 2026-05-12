use crate::executor::dag::ExecutionContext;

pub fn evaluate_condition(expression: &str, ctx: &ExecutionContext) -> bool {
    let resolved = ctx.resolve_template(expression);

    if resolved.is_empty() || resolved == "false" || resolved == "0" {
        return false;
    }

    if resolved.contains("==") {
        let parts: Vec<&str> = resolved.split("==").map(|s| s.trim()).collect();
        return parts.len() == 2 && parts[0] == parts[1];
    }
    if resolved.contains("!=") {
        let parts: Vec<&str> = resolved.split("!=").map(|s| s.trim()).collect();
        return parts.len() == 2 && parts[0] != parts[1];
    }

    true
}
