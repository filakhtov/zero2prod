use std::fmt::{Debug, Display};

use tokio::task::JoinError;
use zero2prod::{
    configuration::get_configuration,
    issue_delivery_worker::run_worker_until_stopped,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

fn get_configuration_path() -> String {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1) {
        Some(path) => path.to_owned(),
        None => {
            eprintln!("usage: {} <configuration_file>", args[0]);
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration(&get_configuration_path())
        .expect("Failed to read the `{}` configuration file");
    let server = Application::build(configuration.clone()).await?;
    let worker = run_worker_until_stopped(configuration);

    let server_task = tokio::spawn(server.run_until_stopped());
    let worker_task = tokio::spawn(worker);

    tokio::select! {
        o = server_task => report_exit("API", o),
        o = worker_task => report_exit("Background worker", o),
    };

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!("{} has exited", task_name),
        Ok(Err(e)) => tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "{} failed",
            task_name,
        ),
        Err(e) => tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "{} task failed to complete",
            task_name,
        ),
    }
}
