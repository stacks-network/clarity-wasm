use std::rc::Rc;
use std::time::Duration;

use color_eyre::eyre::Result;
use diesel::{Connection, SqliteConnection};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::cli::DataArgs;
use crate::context::replay::ReplayOpts;
use crate::context::{ComparisonContext, Network, Runtime};
use crate::db::appdb::AppDb;
use crate::ok;

pub async fn exec(config: &crate::config::Config, data_args: DataArgs) -> Result<()> {
    let app_db_conn = SqliteConnection::establish(&config.app.db_path)?;
    let app_db = Rc::new(AppDb::new(app_db_conn));

    let _multi_pb = MultiProgress::new();
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")
            .unwrap()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );

    let replay_opts: ReplayOpts = data_args.into();

    let _compare_ctx = ComparisonContext::new(app_db.clone())
        .using_baseline(|from| from.stacks_node("baseline", &config.baseline.chainstate_path))?
        .instrument_into(|into| {
            into.instrumented(
                "interpreter_replay",
                Runtime::Interpreter,
                Network::Mainnet(1),
                "/home/cylwit/clarity-ab/replay",
            )?
            .instrumented(
                "wasm",
                Runtime::Wasm,
                Network::Mainnet(1),
                "/home/cylwit/clarity-ab/wasm",
            )
        })?
        .replay(&replay_opts)?;

    ok!()
}
