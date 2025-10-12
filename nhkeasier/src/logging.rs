use std::sync::Arc;

use tracing::{Event, Subscriber};
use tracing_panic::panic_hook;
use tracing_subscriber::{
    Layer,
    filter::{LevelFilter, Targets},
    fmt,
    layer::Context,
    prelude::*,
    registry,
};

use crate::DEBUG;

struct EmailLayer;

#[derive(Default)]
struct VecCollectVisitor {
    fields: Vec<(String, String)>,
}

impl VecCollectVisitor {
    fn new() -> Self {
        Default::default()
    }
}

impl tracing::field::Visit for VecCollectVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields
            .push((field.name().to_string(), format!("{value:?}")));
    }
}

impl<S: Subscriber> Layer<S> for EmailLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = meta.level();
        let file = meta.file().unwrap_or("<none>");
        let line = meta.line().unwrap_or(0);
        let mut visitor = VecCollectVisitor::new();
        event.record(&mut visitor);
        let (_, message) = &visitor.fields[0];
        crate::send_email_sync(
            &format!("{level} {message}"),
            format!("{file}:{line}\n{:#?}", visitor.fields),
        )
    }
}

pub fn init_logging() {
    // debug to file
    let file = std::fs::File::options()
        .append(true)
        .create(true)
        .open("nhkeasier.com.log")
        .expect("Failed to open nhkeasier.com.log");
    let file_filter = Targets::new()
        .with_target("nhkeasier", LevelFilter::DEBUG)
        .with_default(LevelFilter::WARN);
    let file_layer = fmt::layer()
        .with_writer(Arc::new(file))
        .with_filter(file_filter);

    // info to stdout
    let stdout_layer = fmt::layer().with_filter(LevelFilter::INFO);

    if DEBUG {
        registry().with(stdout_layer).with(file_layer).init();
    } else {
        // warn to email
        let email_layer = EmailLayer.with_filter(LevelFilter::WARN);

        registry()
            .with(stdout_layer)
            .with(file_layer)
            .with(email_layer)
            .init();
    }

    // also log panics (especially for email notifications)
    // NOTE: only set the hook after the subscriber, to avoid silently swallowing panics
    std::panic::set_hook(Box::new(panic_hook));
}
