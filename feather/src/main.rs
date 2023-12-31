use std::path::PathBuf;

use database::FeatherDatabase;
use files::{Path, Source, SourceType, Str};
use kernel::Db;

fn main() {
    let log_level = tracing::Level::TRACE;
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_writer(std::io::stderr)
        .with_max_level(log_level)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_env_filter("database=trace,diagnostic=trace,feather=trace,feather_parser=trace,files=trace,kernel=trace")
        .pretty()
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("could not set default tracing subscriber");
    tracing::info!("initialised logging with verbosity level {}", log_level);

    let (db, _rx) = FeatherDatabase::new(PathBuf::new());
    let path = Path::new(
        &db,
        vec![
            Str::new(&db, "test".to_string()),
            Str::new(&db, "test".to_string()),
        ],
    );
    let source = Source::new(&db, path, SourceType::Feather);

    if let Some(module) = feather_parser::parse_module(&db, source)
        .to_dynamic()
        .print_reports()
    {
        tracing::info!("successfully parsed module");
        for definition in &module.definitions {
            tracing::info!(
                "def {}: {} =\n    {}",
                definition.contents.name.contents.text(&db),
                db.format_expression(definition.contents.ty),
                definition
                    .contents
                    .body
                    .map(|body| db.format_expression(body))
                    .unwrap_or_else(|| "<no body>".to_owned()),
            );
        }
        // tracing::info!("{:#?}", result);
    }

    // TODO: <https://github.com/salsa-rs/salsa/blob/master/examples-2022/lazy-input/src/main.rs>
    // This helps us set up the main loop for language servers.
}
