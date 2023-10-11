# Clarity A/B Tester
_*...for lack of a better name...*_

This application can be used to walk through contract executions from a Stacks network (e.g. `testnet` or `mainnet`) and both inspect and compare results and performance before and after, using one or more runtimes.

## Configuration
### Environment variables
Create a `.env` file in this directory with the following contents:
```
CONFIG_FILE="/path/to/your/config.toml"
DATABASE_URL="appdb.sqlite"
```
This file is gitignore'd and won't be checked into git. Any environment-specific settings should be placed in this file.

### Configuration File
The application requires a configuration file (default `config.toml`) which is used to configure baseline chainstate and runtime environments. See the example `config.toml` for more information.

## Database Migrations
This application uses [Diesel ORM](https://diesel.rs/) for managing database access and migrations.
- Schema declarations are found in `schema.rs`
- Model structures are found in `model.rs`
- Migrations are found in the `migrations` directory in the crate root.