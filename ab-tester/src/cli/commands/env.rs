use color_eyre::Result;
use comfy_table::{Row, Table};
use console::Color;
use diesel::{Connection, SqliteConnection};

use crate::{
    cli::{
        EnvArgs, EnvSubCommands, ListEnvArgs, NewEnvArgs, NewEnvSubCommands,
        NewInstrumentedEnvArgs, NewNetworkEnvArgs, NewStacksNodeEnvArgs,
    },
    config::Config,
    context::Runtime,
    db::appdb::AppDb,
    ok,
};

pub async fn exec(config: &Config, env_args: EnvArgs) -> Result<()> {
    match &env_args.commands {
        EnvSubCommands::List(args) => exec_list(config, args).await,
        EnvSubCommands::New(args) => exec_new(config, args).await,
    }
}

async fn exec_new(config: &Config, env_args: &NewEnvArgs) -> Result<()> {
    match &env_args.commands {
        NewEnvSubCommands::StacksNode(args) => exec_new_from_stacks_node(config, args).await,
        NewEnvSubCommands::Instrumented(args) => exec_new_instrumented(config, args).await,
        NewEnvSubCommands::Network(args) => exec_new_network(config, args).await,
    }
}

async fn exec_new_from_stacks_node(_config: &Config, _args: &NewStacksNodeEnvArgs) -> Result<()> {
    println!(
        "{} the specified stacks-node path does not exist",
        console::style("error:").bold().fg(Color::Red)
    );
    ok!()
}

async fn exec_new_instrumented(_config: &Config, _args: &NewInstrumentedEnvArgs) -> Result<()> {
    ok!()
}

async fn exec_new_network(_config: &Config, _args: &NewNetworkEnvArgs) -> Result<()> {
    ok!()
}

async fn exec_list(config: &Config, _args: &ListEnvArgs) -> Result<()> {
    println!();
    println!("Listing environments...");
    println!();

    let conn = SqliteConnection::establish(&config.app.db_path)?;
    let db = AppDb::new(conn);

    let envs = db.list_envs()?;

    let mut table = Table::new();

    table
        .set_header(Row::from(vec![
            console::style("id").bold(),
            console::style("name").bold(),
            console::style("runtime").bold(),
            console::style("path").bold(),
        ]))
        .set_width(80);

    if envs.is_empty() {
        println!("No environments have been created yet.");
        println!("Use the `env new` command to create a new environment.");
        return ok!();
    }

    for env in envs {
        let runtime: Runtime = env.runtime_id.try_into()?;

        let row = Row::from(vec![
            env.id.to_string(),
            env.name,
            runtime.to_string(),
            env.path,
        ]);

        table.add_row(row);
    }

    println!("{table}");

    ok!()
}
