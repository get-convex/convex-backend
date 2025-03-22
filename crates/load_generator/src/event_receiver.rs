use ::metrics::StaticMetricLabel;
use common::errors::report_error;
use serde::Deserialize;

use crate::{
    metrics,
    stats::Stats,
};

#[derive(Deserialize, Debug)]
pub enum MetricType {
    Latency,
    Count,
}

#[derive(Deserialize, Debug)]
pub struct Event {
    #[serde(flatten)]
    pub metadata: EventMetadata,
    #[serde(flatten)]
    pub metric: Metric,
}

#[derive(Eq, PartialEq, Debug, Deserialize, Ord, PartialOrd)]
pub struct EventMetadata {
    pub scenario: String,
    pub name: String,
    pub path: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Metric {
    Error { msg: String },
    Metric { r#type: MetricType, value: f64 },
}

pub struct EventProcessor {
    /// Receiver for events from ScenarioRunner
    pub rx: tokio::sync::mpsc::Receiver<Result<Event, serde_json::Error>>,
    /// Stats object to track event metrics
    pub stats: Stats,
    pub metric_labels: Vec<StaticMetricLabel>,
}

impl EventProcessor {
    /// Process events from ScenarioRunner
    pub async fn receive_events(&mut self) {
        while let Some(event) = self.rx.recv().await {
            match event {
                Ok(event) => {
                    if let Err(mut e) = metrics::log_event(self.metric_labels.clone(), &event) {
                        report_error(&mut e).await;
                    }
                    tracing::debug!(
                        "Deserialized event: {event:?}. Log with tags {:?}",
                        self.metric_labels
                    );
                    self.stats.clone().process_event(event);
                },
                Err(e) => {
                    tracing::error!("Could not deserialize event: {e}")
                },
            }
        }
    }
}
