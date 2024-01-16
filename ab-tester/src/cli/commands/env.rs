use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use color_eyre::Result;
use color_eyre::eyre::{bail, anyhow};
use comfy_table::{Row, Table};
use diesel::{Connection, SqliteConnection};
use log::*;
use chrono::prelude::*;

use crate::cli::{
    EnvArgs, EnvSubCommands, ListEnvArgs, NewEnvArgs, NewEnvSubCommands, NewInstrumentedEnvArgs,
    NewNetworkEnvArgs, NewStacksNodeEnvArgs, SnapshotEnvArgs, NetworkChoice,
};
use crate::config::Config;
use crate::context::{Runtime, Network};
use crate::db::appdb::AppDb;
use crate::environments::{RuntimeEnvBuilder, ReadableEnv};
use crate::{ok, DeleteEnvArgs};
use crate::utils::{zstd_compress, append_to_path};

pub async fn exec(
    config: &Config, 
    env_args: EnvArgs
) -> Result<()> {
    let app_db_conn = SqliteConnection::establish(&config.app.db_path)?;
    let app_db = AppDb::new(app_db_conn);

    match &env_args.commands {
        EnvSubCommands::List(args) => exec_list(app_db, config, args).await,
        EnvSubCommands::New(args) => exec_new(app_db, config, args).await,
        EnvSubCommands::Snapshot(args) => exec_snapshot(app_db, config, args).await,
        EnvSubCommands::Delete(args) => exec_delete(app_db, config, args).await,
    }
}

async fn exec_delete(
    app_db: AppDb,
    _config: &Config,
    args: &DeleteEnvArgs
) -> Result<()> {
    let app_db = Rc::new(app_db);

    // Handle id vs. name environment identifiers
    let env = if let Some(id) = args.env_id {
        app_db.get_env_by_id(id)?
            .ok_or(anyhow!("environment could not be found"))
    } else if let Some(name) = &args.env_name {
        app_db.get_env_by_name(name)?
            .ok_or(anyhow!("environment could not be found"))
    } else {
        bail!("one of env-id or env-name must be provided.")
    }?;

    app_db.delete_environment(env.id)?;

    todo!()
}

async fn exec_snapshot(
    app_db: AppDb, 
    config: &Config, 
    args: &SnapshotEnvArgs
) -> Result<()> {
    let app_db = Rc::new(app_db);

    // Handle id vs. name environment identifiers
    let env = if let Some(id) = args.env_id {
            app_db.get_env_by_id(id)?
                .ok_or(anyhow!("environment could not be found"))
        } else if let Some(name) = &args.env_name {
            app_db.get_env_by_name(name)?
                .ok_or(anyhow!("environment could not be found"))
        } else {
            bail!("one of env-id or env-name must be provided.")
        }?;

    // Attempt to parse the type of environment.
    let env_type: EnvironmentType = env.environment_type_id.try_into()?;

    // Instantiate an environment builder.
    let builder = RuntimeEnvBuilder::new(app_db);

    // Create a `ReadableEnv` instance from the environment's stored configuration.
    let mut env_instance: Box<dyn ReadableEnv> = match env_type {
        EnvironmentType::StacksNode => {
            Box::new(builder.stacks_node(env.name.clone(), PathBuf::try_from(env.base_path)?)?)
        },
        EnvironmentType::NetworkSynced => todo!(),
        EnvironmentType::Instrumented => {
            Box::new(builder.instrumented(
                env.name.clone(), 
                env.runtime_id.try_into()?, 
                Network::new(env.network_id as u32, env.chain_id as u32)?, 
                env.is_read_only, 
                env.base_path)?)
        },
    };

    // Attempt to open the environment for reading.
    env_instance.open()?;

    let alias = args.alias.clone().unwrap_or_else(|| {
        let mut alias = env.name;
        alias.push_str("-");
        alias.push_str(&Utc::now().format("%Y%m%d%H%M%S").to_string());
        alias.to_string()
    });
    
    // Attempt to snapshot the environment's current state.
    snapshot_environment(&*env_instance, config, alias)?;

    todo!()
}

async fn exec_new(
    app_db: AppDb, 
    config: &Config, 
    env_args: &NewEnvArgs
) -> Result<()> {
    match &env_args.commands {
        NewEnvSubCommands::StacksNode(args) => exec_new_from_stacks_node(
            app_db, 
            config, 
            args, 
            &env_args.name).await,
        NewEnvSubCommands::Instrumented(args) => exec_new_instrumented(
            app_db, 
            config, 
            args, 
            &env_args.name
        ).await,
        NewEnvSubCommands::Network(args) => exec_new_network(
            app_db, 
            config, 
            args, 
            &env_args.name).await,
    }
}

async fn exec_new_from_stacks_node(
    app_db: AppDb,  
    _config: &Config, 
    args: &NewStacksNodeEnvArgs, 
    name: &str
) -> Result<()> {
    if !args.path.exists() {
        bail!("the specified stacks-node path does not exist")
    } else if !args.path.is_dir() {
        bail!("the specified stacks-node path is not a directory")
    }

    let app_db = Rc::new(app_db);

    let builder = RuntimeEnvBuilder::new(app_db);

    info!("attempting to read stacks node at path: {:?}", args.path);
    let mut env = builder.stacks_node(
        name.to_string(), 
        args.path.clone())?;

    let x: &mut dyn ReadableEnv = &mut env;
    x.open()?;
    info!("node successfully opened and validated");

    ok!()
}

async fn exec_new_instrumented(
    app_db: AppDb, 
    config: &Config, 
    args: &NewInstrumentedEnvArgs, 
    name: &str
) -> Result<()> {
    let app_db = Rc::new(app_db);
    let builder = RuntimeEnvBuilder::new(app_db);

    let network = match args.network {
        NetworkChoice::Testnet => Network::Testnet(args.chain_id),
        NetworkChoice::Mainnet => Network::Mainnet(args.chain_id)
    };

    let working_dir = if let Some(dir) = &args.path {
        dir.display().to_string()
    } else {
        PathBuf::try_from(&config.app.working_dir)?
            .join("environments")
            .join(name)
            .display()
            .to_string()
    };

    let mut env = builder.instrumented(
        name.to_string(), 
        args.runtime, 
        network,
        args.is_read_only, 
        working_dir
    )?;

    let x: &mut dyn ReadableEnv = &mut env;
    x.open()?;

    ok!()
}

async fn exec_new_network(
    _app_db: AppDb, 
    _config: &Config, 
    _args: &NewNetworkEnvArgs, 
    _name: &str
) -> Result<()> {
    todo!("network node not implemented");
}

async fn exec_list(
    app_db: AppDb, 
    _config: &Config, 
    _args: &ListEnvArgs
) -> Result<()> {
    println!();
    println!("Listing environments...");
    println!();

    let envs = app_db.list_envs()?;

    let mut table = Table::new();

    table
        .set_header(Row::from(vec![
            console::style("id").bold(),
            console::style("name").bold(),
            console::style("runtime").bold(),
            console::style("network").bold(),
            console::style("chain-id").bold(),
            console::style("read-only").bold(),
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
        let network = Network::new(env.network_id as u32, env.chain_id as u32)?;

        let row = Row::from(vec![
            env.id.to_string(),
            env.name,
            runtime.to_string(),
            network.to_string(),
            env.chain_id.to_string(),
            env.is_read_only.to_string(),
            env.base_path,
        ]);

        table.add_row(row);
    }

    println!("{table}");

    ok!()
}

fn snapshot_environment(
    target: &dyn ReadableEnv,
    config: &Config,
    snapshot_name: String
) -> Result<()> {
    let name = target.name();
    let working_dir = target.cfg().working_dir();

    debug!("creating new tar archive of the environment's working directory: '{:?}'", working_dir);
    let mut tar_file = tempfile::tempfile()?;
    {
        let mut tar = tar::Builder::new(&mut tar_file);
        tar.append_path(working_dir)?;
        tar.finish()?;
    }
    tar_file.flush()?;

    let target_path = PathBuf::from_str(&config.app.working_dir)?
        .join(format!("snapshots/{:?}/{:?}.tar.zstd", name, snapshot_name));

    debug!("opening target file for compression: {}", target_path.display());
    let mut target_file = File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(target_path)?;
    let target_writer = BufWriter::new(&mut target_file);

    debug!("compressing file");
    zstd::stream::copy_encode(tar_file, target_writer, 5)?;
    debug!("compression finished, flushing file");
    target_file.sync_all()?;

    std::thread::sleep(Duration::from_millis(500));

    // TODO: Load environment from src-target.backup if exists and --reset-env
    // is set.
    let init_chainstate_snapshot_path =
        append_to_path(target.cfg().chainstate_index_db_path(), ".zstd");
    let init_chainstate_snapshot_exists =
        std::fs::metadata(init_chainstate_snapshot_path).is_ok();
    let init_burnstate_snapshot_path =
        append_to_path(target.cfg().sortition_db_path(), ".zstd");
    let init_burnstate_snapshot_exists =
        std::fs::metadata(&init_burnstate_snapshot_path).is_ok();

    // TODO: Backup environment
    if !init_chainstate_snapshot_exists {
        let chainstate_index_path = target.cfg().chainstate_index_db_path().parent().unwrap().to_path_buf();
        let chainstate_index_sqlite_path = &chainstate_index_path.join("index.sqlite");
        let chainstate_index_blobs_path = &chainstate_index_path.join("index.sqlite.blobs");
        let chainstate_clarity_sqlite_path = &chainstate_index_path.join("clarity/marf.sqlite");
        let chainstate_clarity_blobs_path = &chainstate_index_path.join("clarity/marf.sqlite.blobs");

        // Chainstate Index DB
        info!("[{name}] chainstate index snapshot does not exist, creating it...");
        zstd_compress(chainstate_index_sqlite_path)?;
        zstd_compress(chainstate_index_blobs_path)?;
        zstd_compress(chainstate_clarity_sqlite_path)?;
        zstd_compress(chainstate_clarity_blobs_path)?;
    }

    if !init_burnstate_snapshot_exists && !target.cfg().is_sortition_app_indexed() {
        // Sortition DB
        std::fs::create_dir_all(target.cfg().sortition_dir())?;
        let db_file = File::open(target.cfg().sortition_db_path())?;
        let db_reader = BufReader::new(db_file);
        std::fs::create_dir_all(&init_burnstate_snapshot_path)?;
        let file = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&init_burnstate_snapshot_path)?;
        let file_writer = BufWriter::new(file);
        zstd::stream::copy_encode(db_reader, file_writer, 5)?;
    }

    ok!()
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EnvironmentType {
    StacksNode = 0,
    NetworkSynced = 1,
    Instrumented = 2
}

impl TryFrom<i32> for EnvironmentType {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: i32) -> Result<Self> {
        match value {
            0 => Ok(EnvironmentType::StacksNode),
            1 => Ok(EnvironmentType::NetworkSynced),
            2 => Ok(EnvironmentType::Instrumented),
            _ => bail!("failed to parse environment type from integer: {}", value),
        }
    }
}