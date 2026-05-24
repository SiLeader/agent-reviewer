use std::sync::Arc;
use tokio::sync::SemaphorePermit;

#[derive(Clone)]
pub struct ConcurrencyLimiter {
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl ConcurrencyLimiter {
    pub fn new(concurrency: usize) -> Self {
        Self {
            semaphore: Arc::new(tokio::sync::Semaphore::new(concurrency)),
        }
    }

    pub async fn acquire<'a>(&'a self) -> anyhow::Result<SemaphorePermit<'a>> {
        let sp = self.semaphore.acquire().await?;
        Ok(sp)
    }

    pub async fn release(&self, permit: SemaphorePermit<'_>) -> anyhow::Result<()> {
        permit.forget();
        Ok(())
    }
}
