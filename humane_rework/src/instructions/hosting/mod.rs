use std::collections::HashMap;

use super::{HumaneInstruction, InstructionArgs};
use crate::civilization::Civilization;
use crate::errors::{HumaneInputError, HumaneStepError};

use async_trait::async_trait;

mod host_dir {
    use std::time::Duration;

    use actix_web::{App, HttpServer};
    use tokio::time::sleep;

    use super::*;

    pub struct HostDir;

    inventory::submit! {
        &HostDir as &dyn HumaneInstruction
    }

    #[async_trait]
    impl HumaneInstruction for HostDir {
        fn instruction(&self) -> &'static str {
            "I serve the directory {dir}"
        }

        async fn run(
            &self,
            args: &InstructionArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<(), HumaneStepError> {
            let dir = args.get_string("dir")?;

            let mut attempts = 0;
            let mut running = false;
            while !running && attempts < 5 {
                let port = civ.ensure_port();
                let dir = civ.tmp_file_path(&dir);
                match HttpServer::new(move || {
                    App::new().service(actix_files::Files::new("/", &dir).index_file("index.html"))
                })
                .bind(("127.0.0.1", port))
                {
                    Ok(bound) => {
                        let server = bound.run();
                        let handle = server.handle();
                        civ.handles.push(handle);
                        civ.threads.push(tokio::task::spawn(async { server.await }));
                        running = true;
                    }
                    Err(_) => {
                        civ.purge_port();
                        attempts += 1;
                    }
                }
            }

            assert!(running);
            // Wait a beat to make sure the server is ready to roll
            sleep(Duration::from_millis(100)).await;

            Ok(())
        }
    }
}
