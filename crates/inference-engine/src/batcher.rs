//! Inference Batcher

use feature_engine::FeatureVector;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tracing::{debug, info};

use crate::engine::InferenceEngine;
use crate::InferenceError;

/// Inference batcher for batching multiple feature vectors
pub struct InferenceBatcher {
    /// Channel receiver for incoming feature vectors
    receiver: mpsc::Receiver<FeatureVector>,
    /// Batch size threshold
    batch_size: usize,
    /// Timeout for batch collection (ms)
    timeout_ms: u64,
}

impl InferenceBatcher {
    /// Create a new batcher
    pub fn new(receiver: mpsc::Receiver<FeatureVector>, batch_size: usize, timeout_ms: u64) -> Self {
        info!("Creating inference batcher: batch_size={}, timeout={}ms", batch_size, timeout_ms);
        Self {
            receiver,
            batch_size,
            timeout_ms,
        }
    }

    /// Create a channel pair for the batcher
    pub fn channel(batch_size: usize, timeout_ms: u64) -> (mpsc::Sender<FeatureVector>, Self) {
        let (tx, rx) = mpsc::channel(batch_size * 2);
        (tx, Self::new(rx, batch_size, timeout_ms))
    }

    /// Run the batcher loop
    pub async fn run(&mut self, engine: &InferenceEngine) -> Result<(), InferenceError> {
        info!("Starting inference batcher");

        loop {
            // Collect batch
            let mut batch = Vec::with_capacity(self.batch_size);
            let timeout_duration = Duration::from_millis(self.timeout_ms);

            // Wait for first item
            match self.receiver.recv().await {
                Some(features) => batch.push(features),
                None => {
                    debug!("Batcher channel closed");
                    break;
                }
            }

            // Try to collect more until batch is full or timeout
            while batch.len() < self.batch_size {
                match timeout(timeout_duration, self.receiver.recv()).await {
                    Ok(Some(features)) => batch.push(features),
                    Ok(None) => break, // Channel closed
                    Err(_) => break, // Timeout
                }
            }

            debug!("Processing batch of {} feature vectors", batch.len());

            // Process batch
            for features in &batch {
                match engine.predict(features).await {
                    Ok(result) => {
                        debug!(
                            "Prediction: {:?} (conf={:.2}, latency={}ms)",
                            result.prediction.fault_type,
                            result.prediction.confidence,
                            result.latency_ms
                        );
                    }
                    Err(e) => {
                        debug!("Inference error: {}", e);
                    }
                }
            }
        }

        info!("Inference batcher stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batcher_creation() {
        let (tx, _batcher) = InferenceBatcher::channel(16, 5000);
        
        // Send a feature vector
        tx.send(FeatureVector::default()).await.unwrap();
    }
}
