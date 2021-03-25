# Agent Migrations

Migration files for the `agent.db` SQLite database live in the agent
`migrations/sql/` directory.

## Conventions

- Each migration should contain one SQL statement

- Migration files should be named according to the scheme `migrations/sql/NNNNNN_name.sql`,
  where `NNNNNN` is an incrementing number starting from `000001`.

## Troubleshooting

In the event a migration fails to run and we need to bump the schema version
(maintained by SQlite's `user_version` PRAGMA), we can check the last successful
version that was run with the command:

```
$ DISABLE_MIGRATIONS=1 pennsieve config schema-version

> 3
```

If we need to manually set the schema version in order to re-run a failed
migration, we can run:

```
DISABLE_MIGRATIONS=1 pennsieve config schema-version <new-version>
```

The `DISABLE_MIGRATIONS` environment variable will prevent migrations from 
running as part of the application's initialization step in `main`. This
is needed if the database is "stuck" on a specific failed migration.

The schema version given by `config schema-version` will refer to the last
successful migration file run, e.g. `3 => 000003_add_name_column.sql`.

Note: Although the `config schema-version` option will *NOT* be displayed as 
part of the `--help` menu of a production release, it will still be available.
