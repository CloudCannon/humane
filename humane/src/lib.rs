use cucumber::{Cucumber, WorldInit};

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
        Cucumber::new()
            .steps(Civilization::collection())
            .max_concurrent_scenarios(Some(8))
            .after(|_, _, _, maybe_world| {
                Box::pin(async move {
                    if let Some(world) = maybe_world {
                        world.shutdown().await;
                    }
                })
            })
            .filter_run(&self.options.test_file_root, |_, _, sc| {
                !sc.tags.iter().any(|t| t == "skip")
            })
            .await;
    }
}
