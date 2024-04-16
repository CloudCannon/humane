use console::style;

use crate::{HumaneTestStep, HumaneTestStepState};

pub fn log_step_runs(steps: &Vec<HumaneTestStep>, indent: usize) {
    for step in steps {
        use HumaneTestStepState::*;
        let prefix = if indent > 0 {
            format!("{: <1$}↳ ", "", indent)
        } else {
            "".to_string()
        };

        println!(
            "{prefix}{}",
            match step.state() {
                Dormant => style(format!("⦸ {step}")).dim(),
                Failed => style(format!("✘ {step}")).red(),
                Passed => style(format!("✓ {step}")).green(),
            }
        );
        match step {
            HumaneTestStep::Ref {
                hydrated_steps: Some(inner_steps),
                ..
            } => {
                log_step_runs(inner_steps, indent + 2);
            }
            _ => {}
        }
    }
}
