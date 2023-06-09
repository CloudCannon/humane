use std::env;

use cucumber::{gherkin::Scenario, Cucumber, WorldInit};

use civilization::Civilization;
use options::RobotHumaneConfig;

mod civilization;
pub mod options;

pub struct Humane {
    options: RobotHumaneConfig,
}

impl Humane {
    pub fn new(options: RobotHumaneConfig) -> Self {
        Self { options }
    }

    pub async fn go(&mut self) {
        let has_tag = |sc: &Scenario, tag| sc.tags.iter().any(|t| t == tag);

        let r = Cucumber::new()
            .steps(Civilization::collection())
            .max_concurrent_scenarios(Some(8))
            .after(|_, _, _, maybe_world| {
                Box::pin(async move {
                    if let Some(world) = maybe_world {
                        world.shutdown().await;
                    }
                })
            })
            .filter_run(&self.options.test_file_root, move |_, _, sc| {
                if has_tag(sc, "skip") {
                    return false;
                }
                let is_platform_limited = sc.tags.iter().any(|t| t.starts_with("platform-"));
                if is_platform_limited {
                    match env::consts::OS {
                        "linux" => has_tag(sc, "platform-linux") || has_tag(sc, "platform-unix"),
                        "macos" => has_tag(sc, "platform-macos") || has_tag(sc, "platform-unix"),
                        "windows" => has_tag(sc, "platform-windows"),
                        _ => false,
                    }
                } else {
                    true
                }
            })
            .await;
        if r.parsing_errors > 0
            || r.failed_hooks > 0
            || r.scenarios.failed > 0
            || r.steps.failed > 0
        {
            std::process::exit(1);
        }
    }
}
