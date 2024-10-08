use super::{Cron, Error, Job, JobHandler};
use crate::colors::MaybeColorize;

use std::sync::Arc;
use time::OffsetDateTime;

use tokio::time::{interval, Duration};
use tracing::error;

pub struct ScheduledJob {
    job: Box<JobHandler>,
    args: serde_json::Value,
    cron: Cron,
}

impl ScheduledJob {
    pub async fn schedule(&self) -> Result<(), Error> {
        self.job.job.execute_async(self.args.clone()).await?;

        Ok(())
    }

    pub fn should_run(&self, time: &OffsetDateTime) -> bool {
        self.cron.should_run(time)
    }

    pub fn job(&self) -> &Box<dyn Job> {
        &self.job.job
    }
}

pub struct Clock {
    jobs: Arc<Vec<ScheduledJob>>,
}

impl Clock {
    pub async fn run(&self) {
        let mut clock = interval(Duration::from_secs(1));

        loop {
            clock.tick().await;
            let now = OffsetDateTime::now_utc();
            let jobs = self.jobs.clone();

            tokio::spawn(async move {
                for job in jobs.iter() {
                    if job.should_run(&now) {
                        match job.schedule().await {
                            Ok(_) => (),
                            Err(err) => {
                                error!(
                                    "job {} failed to schedule: {:?}",
                                    job.job().job_name().green(),
                                    err
                                );
                            }
                        }
                    }
                }
            });
        }
    }
}
