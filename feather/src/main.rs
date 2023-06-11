use std::path::PathBuf;

use database::FeatherDatabase;
use files::{Path, Source, SourceType, Str};

fn main() {
    let log_level = tracing::Level::TRACE;
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_writer(std::io::stderr)
        .with_max_level(log_level)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .with_timer(tracing_subscriber::fmt::time::uptime())
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

    if let Some(result) = files::source(&db, source).print_diagnostics() {
        println!("{}", result);
        fexpr::test(&result);
    }

    // TODO: <https://github.com/salsa-rs/salsa/blob/master/examples-2022/lazy-input/src/main.rs>
    // This helps us set up the main loop for language servers.
}
