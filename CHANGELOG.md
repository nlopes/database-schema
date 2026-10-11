# Changelog

## Unreleased

- Return Diesel migration failures before writing a dump.
- Keep valid SQLite names such as `sqlitex_users` and restore tables and views
  before their indexes and triggers.
