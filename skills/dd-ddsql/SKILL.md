---
name: dd-ddsql
description: DDSQL (Datadog SQL) - query metrics, logs, spans, RUM, security findings, and reference tables with SQL via pup. Use when writing, running, or debugging a ddsql query.
metadata:
  version: "1.0.0"
  author: datadog-labs
  repository: https://github.com/datadog-labs/agent-skills
  tags: datadog,ddsql,sql,query,metrics,logs,dd-ddsql
  alwaysApply: "false"
---

# DDSQL (Datadog SQL)

Query Datadog data with a PostgreSQL-style SQL dialect via `pup ddsql`. DDSQL is **not** full SQL — it supports a specific, fixed subset, and queries against unknown tables/columns fail. Most failures come from inventing table or column names, or from using standard-SQL idioms DDSQL rejects. This skill exists to prevent both.

## 🥇 The golden rule

**Never invent table or column names, and never assume standard-SQL syntax.** Discover the schema first, consult the authoritative spec when unsure, then write the query. The spec itself repeats this: *"Never invent table/column names."*

## Workflow (discovery-first — do not skip)

```bash
# 1. (When unsure of syntax/functions) Print the authoritative, always-current spec.
#    This is better than any static doc: server-maintained, lists every function and
#    table function, and includes an explicit "Incorrect Usage" anti-pattern list.
pup ddsql spec

# 2. Find the table. Names are namespaced (e.g. aws.ec2_instance, reference_tables.my_table).
pup ddsql schema tables --query ec2          # case-insensitive substring filter

# 3. Get EXACT column names + types for that table.
pup ddsql schema columns --table-id public.aws.ec2_instance

# 4. Write the query using only discovered columns.

# 5. Run it.
pup ddsql table       --query "SELECT ..."   # tabular results
pup ddsql time-series --query "SELECT ..."   # time-series results (timestamp/value rows)

# 6. On HTTP 400 ("column X cannot be resolved" / "non-existent dataset"):
#    re-run schema discovery, fix the name/type, retry. Do NOT guess again.
```

## Running queries (pup mechanics)

```bash
# Query from a flag, or from stdin with --query -
echo "SELECT * FROM aws.ec2_instance LIMIT 5" | pup ddsql table --query -

# Output formats: json (default), yaml, table, csv
pup ddsql table --query "SELECT * FROM aws.ec2_instance LIMIT 5" -o csv > out.csv

# Time window flags (for the tabular query layer; default --from 1h --to now)
pup ddsql table --query "..." --from 24h --to now --limit 100 --offset 0
```

| Flag | Purpose |
|------|---------|
| `--query <STRING>` / `--query -` | Query string, or `-` to read from stdin |
| `--from` / `--to` | Time window (`1h`, `30m`, `7d`, `now`, unix ts). Default `1h`/`now` |
| `--limit` / `--offset` | Row cap / pagination (`table` default 50, `time-series` default 5000) |
| `--interval` | Aggregation interval in **milliseconds** |
| `-o json\|yaml\|table\|csv` | Output format |

- **Queries run async**: pup submits, then polls until the result is ready. A short pause is normal.
- **Agent envelope**: in agent sessions, output is wrapped as `{status, data, metadata}`. When writing a command the user will run themselves outside this session, append `--no-agent` so the output matches what they'll see.
- **Auth**: query commands work with OAuth2 (`pup auth login`) or `DD_API_KEY`+`DD_APP_KEY`. On 401/403, run `pup auth refresh`.

## Two ways to name data

1. **Direct namespaced tables** — cloud/infra inventory and your reference tables. Discover with `schema tables`/`schema columns`.
   - `aws.ec2_instance`, `aws.iam_user`, ... (default namespace is `dd`; `public.` is the schema kind in table-ids)
   - `reference_tables.<your_table>` — user-uploaded lookup tables
2. **Table functions** — telemetry that isn't a static table (logs, metrics, spans, RUM, security). These take a `columns => ARRAY[...]` argument **and** an `AS (col type, ...)` clause that declares the exact returned types.

## Table functions

Every event/log table function needs `columns => ARRAY[...]` **and** a matching `AS (name TYPE, ...)` clause. Types must be explicit: `TIMESTAMP` for timestamps, `VARCHAR` for text, `BIGINT` for whole numbers.

| Function | Notes |
|----------|-------|
| `dd.logs(columns => ARRAY[...], filter? , indexes? , from_timestamp? , to_timestamp?) AS (...)` | Logs. `filter` uses Datadog log search syntax. |
| `dd.metric_scalar(query, reducer [, from, to])` | **Singular.** One aggregated value per group. `reducer` = `avg`/`max`/`min`/`sum`/... |
| `dd.metrics_timeseries(query [, from, to])` | **Plural.** Full time series; no reducer. |
| `dd.spans(columns => ARRAY[...], filter? , from_timestamp? , to_timestamp?) AS (...)` | APM spans. |
| `dd.rum(columns => ARRAY[...], event_type? , filter? , ...) AS (...)` | RUM events. `event_type` = `session`/`view`/`action`/`error`/... |
| `dd.security_findings(columns => ARRAY[...], finding_types? , filter?) AS (...)` | **Always past 24h** (no timestamp params). Use `@`-prefixed fields (`@severity`). |

⚠️ **Watch the naming asymmetry**: scalar is `dd.metric_scalar` (singular "metric"), timeseries is `dd.metrics_timeseries` (plural "metrics").

**Timestamp params are both-or-neither.** You may pass both `from`/`to` or neither (defaults to past 1h) — never just one. Supported forms: `TIMESTAMP '2025-12-11 10:00:00'`, `now() - INTERVAL '1 hour'`, `date_trunc('day', now())`. **Not** supported: `to_timestamp()`, bare string literals, `current_timestamp`.

## ⚠️ DDSQL ≠ standard SQL

| Don't | Do | Why |
|-------|----|----|
| `current_timestamp` | `NOW()` | `current_timestamp` is unsupported |
| `SELECT #tagname` | `tags -> 'tagname'` | `#` tag syntax is not supported; tags are an hstore accessed with `->` (returns text) |
| `json ->> 'k'`, `json @> ...` | `JSON_EXTRACT_PATH_TEXT(json, 'k')` | `->>` and `@>` are not supported |
| `ANY(col)` | `WHERE col IS NOT NULL` / explicit conditions | `ANY()` is not supported |
| `GROUP BY my_alias` | `GROUP BY tags->'name'` (repeat the expression) | SELECT aliases are only usable *after* SELECT (e.g. `ORDER BY`), not in `WHERE`/`GROUP BY`/`HAVING` |
| `SELECT a, COUNT(*) ... GROUP BY` w/o `a` | put every non-aggregated SELECT column in `GROUP BY` | required for grouped queries |
| `CAST(x AS INT)` | `CAST(x AS BIGINT)` | CAST targets are `BIGINT`, `DECIMAL`, `TIMESTAMP`, `VARCHAR` only |

For substring matching, lowercase both sides and split compound terms: `WHERE LOWER(message) LIKE '%intense%' OR LOWER(message) LIKE '%api%'`.

## Verified examples

```bash
# Cloud inventory — public table (discover columns first with `schema columns`)
pup ddsql table --query "SELECT instance_id, instance_type FROM aws.ec2_instance LIMIT 5"

# Logs — note columns => ARRAY[...] paired with AS (... types ...)
pup ddsql table --query "SELECT timestamp, service, status FROM dd.logs(
  columns => ARRAY['timestamp','service','status'],
  filter  => 'status:error'
) AS (timestamp TIMESTAMP, service VARCHAR, status VARCHAR) LIMIT 20"

# Metrics, aggregated scalar (one value per host)
pup ddsql table --query "SELECT * FROM dd.metric_scalar('avg:system.cpu.user{*} by {host}', 'avg') LIMIT 10"

# Metrics, time series (use the time-series subcommand)
pup ddsql time-series --query "SELECT timestamp, value, tags->'host' AS host
  FROM dd.metrics_timeseries('avg:system.cpu.user{*} by {host}')"

# Reference table — names are org-specific, so discover first
pup ddsql schema tables --query my_lookup
pup ddsql table --query "SELECT * FROM reference_tables.my_lookup LIMIT 10"
```

## Reference

- **`pup ddsql spec`** — the authoritative, always-current spec (functions, table functions, anti-patterns, worked translations). Prefer it over any static doc.
- [DDSQL reference](https://docs.datadoghq.com/ddsql_reference/) — web documentation.

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `depends on non-existent dataset "X"` | Queried a table that doesn't exist (e.g. bare `metrics`) | Use a real table from `schema tables`, or a `dd.*` table function |
| `Column 'X' cannot be resolved` | Invented/misspelled column | `schema columns --table-id <id>`; use exact names |
| `target table ... does not exist` | Reference table not present in this org | `schema tables --query <name>` to find a real one |
| 401 / 403 | Token expired / missing scope | `pup auth refresh`, then `pup auth login` |
| Empty `[]` results | No data in window, or filter too narrow | Widen `--from`/`--to` or loosen `filter` |
