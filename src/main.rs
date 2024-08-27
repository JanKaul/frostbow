use std::process::ExitCode;

use clap::Parser;
use datafusion::execution::context::SessionContext;
use datafusion_cli::{
    exec,
    print_format::PrintFormat,
    print_options::{MaxRows, PrintOptions},
};
use datafusion_iceberg::error::Error;

#[derive(Debug, Parser)]
#[clap(version, about)]
struct Args {}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = main_inner().await {
        println!("Error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn main_inner() -> Result<(), Error> {
    let _args = Args::parse();

    let mut print_options = PrintOptions {
        format: PrintFormat::Automatic,
        quiet: true,
        maxrows: MaxRows::Unlimited,
        color: true,
    };

    let mut ctx = SessionContext::new();

    exec::exec_from_repl(&mut ctx, &mut print_options)
        .await
        .unwrap();

    Ok(())
}
