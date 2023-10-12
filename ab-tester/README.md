# Clarity A/B Tester
_*...for lack of a better name...*_

This application can be used to walk through contract executions from a Stacks 
network (e.g. `testnet` or `mainnet`) and both inspect and compare results and 
performance before and after, using one or more runtimes.

You do need to have a copy of a Stacks node data directory as this tool produces
its testing data from a copy of a Stacks blockchain. A good source of this data
is the [Hiro Archive](https://archive.hiro.so/). If downloading archive data
then you will need to download the `stacks-blockchain` archive:

- **mainnet**: https://archive.hiro.so/mainnet/stacks-blockchain/
- **testnet**: https://archive.hiro.so/testnet/stacks-blockchain/

## Configuration

### Environment variables
Create a `.env` file in this directory with the following contents:
```
CONFIG_FILE="/path/to/your/config.toml"
DATABASE_URL="appdb.sqlite"
```
This file is gitignore'd and won't be checked into git. Any environment-specific 
settings should be placed in this file.

### Configuration File

The application requires a configuration file (default `config.toml`) which is 
used to configure baseline chainstate and runtime environments. See the example 
`config.toml` for more information.

## Database Migrations

This application uses [Diesel ORM](https://diesel.rs/) for managing database 
access and migrations. Visit their [getting-started guide](https://diesel.rs/guides/getting-started)
for more information on how to install and use the tooling.

- Schema declarations are found in `schema.rs`
- Model structures are found in `model.rs`
- Migrations are found in the `migrations` directory in the crate root.

To generate a new migration, use the command:
```$ diesel migration generate <name_of_migration>```
This will create a new folder within the `migrations` directory containing
both `up.sql` and `down.sql` files. Write your roll-up and roll-back SQL scripts
here.

Execute migrations with the command `diesel migration run`. Any pending migrations 
will be automatically run on application startup.