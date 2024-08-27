use std::process::ExitCode;

use clap::Parser;
use datafusion::execution::context::SessionContext;
use datafusion_cli::{
    exec,
    print_format::PrintFormat,
    print_options::{MaxRows, PrintOptions},
};
use datafusion_iceberg::error::Error;
use rustyline::{error::ReadlineError, DefaultEditor};

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

    let print_options = PrintOptions {
        format: PrintFormat::Automatic,
        quiet: true,
        maxrows: MaxRows::Unlimited,
        color: true,
    };

    let mut ctx = SessionContext::new();

    let mut rl = DefaultEditor::new().unwrap();

    loop {
        let readline = rl.readline(">> ");

        match readline {
            Ok(line) => {
                exec::exec_from_commands(&mut ctx, vec![line], &print_options).await?;
            }
            Err(ReadlineError::Interrupted) => break,
            Err(err) => {
                println!("{:?}", err)
            }
        }
    }

    Ok(())
}
