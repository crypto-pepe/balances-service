use std::sync::Arc;
use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

use crate::error::Error as AppError;

pub fn init_tracing() -> Result<(), AppError> {
    LogTracer::init().map_err(|e| Arc::new(e))?;

    let env_filter = EnvFilter::from_default_env();
    let fmt_layer = fmt::Layer::default();
    let subscriber = Registry::default().with(env_filter).with(fmt_layer);
    set_global_default(subscriber).map_err(|e| Arc::new(e))?;

    Ok(())
}
