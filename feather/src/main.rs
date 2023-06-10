use diagnostic::{
    miette::{diagnostic, MietteDiagnostic},
    Dr,
};

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

    let dr: Dr<(), MietteDiagnostic, MietteDiagnostic> =
        Dr::new(()).with(diagnostic!("test diagnostic"));
    dr.print_diagnostics();
}
