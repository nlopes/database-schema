# Changelog

## Unreleased

- Return Diesel migration failures before writing a dump.
- Use `./migrations` and `./structure.sql` as the builder defaults.
- Keep valid SQLite names such as `sqlitex_users` and restore tables and views
  before their indexes and triggers.
- Support macros in expression position and prefer Tokio when both runtimes are
  enabled.
- Fix MySQL password authentication, socket options, and malformed URL errors.
- Remove credentials from tracing logs and `Debug` output.
- Keep MySQL and PostgreSQL dump passwords out of command arguments.
- Verify repeated schema generation, including regeneration from fresh databases.
- Correct usage documentation and compile its examples.
- Test runtime, TLS, and workspace example configurations in CI.
- Restore server dumps and verify views, indexes, foreign keys, generated IDs,
  triggers, and functions with both migration drivers.
