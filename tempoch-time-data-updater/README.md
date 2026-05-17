# tempoch-time-data-updater

`tempoch-time-data-updater` is the maintenance CLI that regenerates the
checked-in time-data tables used by `tempoch`.

It fetches authoritative upstream timekeeping data, parses it into the Rust
crate's internal representation, and rewrites the generated modules under
[`tempoch-time-data/src/generated`](/home/valles/workspace/siderust/rust/tempoch/tempoch-time-data/src/generated).
The updater is now a thin wrapper over the shared `tempoch-time-data`
support crate, so compile-time regeneration and runtime refresh use the same
fetch/parse/build pipeline. In particular, parsing is centralized in
`tempoch-time-data`; this crate owns generation orchestration and file output.

## What It Does

The updater downloads four upstream datasets:

- `UTC-TAI.history` from BIPM / IERS for the UTC minus TAI history.
- `deltat.data` from USNO for observed monthly `Delta T = TT - UT1`.
- `deltat.preds` from USNO for predicted future `Delta T`.
- `finals2000A.all` from IERS for daily Earth Orientation Parameters (EOP),
  including `UT1-UTC`.

From those sources it generates:

- `tempoch-time-data/src/generated/time_data.rs`
- `tempoch-time-data/src/generated/eop_data.rs`
- `tempoch-time-data/src/generated/time_data.provenance.json`

The generated Rust modules are the data that `tempoch` compiles into the
library. The updater does not add new public concepts on its own; it exists to
keep the checked-in generated tables aligned with the upstream sources.

## Why It Exists

`tempoch` intentionally keeps compiled tables as the default path.

That design has a few concrete benefits:

- Builds and tests stay deterministic once the generated files are committed.
- Consumers do not need network access to use UTC, TAI, TT, or UT1
  conversions.
- The crate can expose typed time conversions without depending on live remote
  services.
- Maintainers can verify whether committed generated data is stale with a
  simple reproducible check.

Separately, `tempoch` also offers runtime freshness through its ordinary UTC,
UT1, and context-backed conversion APIs. The updater exists to refresh the
checked-in, network-free defaults that ship with the crate.

The updater also records source provenance. It computes SHA-256 hashes for each
downloaded upstream file and stores them in
`time_data.provenance.json`, alongside the fetch timestamp. That makes it
possible to prove which exact upstream inputs produced the checked-in tables.

## How It Works

At a high level, the tool does the following:

1. Fetches the four upstream text files over HTTP.
2. Parses each file into structured Rust values via `tempoch-time-data`.
3. Builds the modern `Delta T` series by concatenating observed USNO values
   with future prediction values.
4. Applies a C0 continuity offset at the observed/predicted `Delta T`
   boundary so the generated series does not jump at the stitch point.
5. Hands that validated bundle to the generator.
6. Renders Rust source files for `tempoch-time-data`.
7. Writes a provenance sidecar containing the source hashes.

There are a few implementation details worth knowing:

- The UTC-TAI history is converted into piecewise segments that preserve the
  pre-1972 slope-based rules and the post-1972 leap-second-era flat steps.
- The EOP generator preserves the upstream observed/predicted flags instead of
  flattening them away.
- Optional EOP fields remain `None` when upstream leaves them blank; the tool
  does not fabricate zeroes.
- In `--check` mode the tool does not rewrite files. It compares the freshly
  rendered output against the committed generated files and exits non-zero if
  anything is out of date.

## How To Run It

From the Rust workspace root:

```bash
cargo run -p tempoch-time-data-updater
```

That refreshes the generated files in `tempoch-time-data/src/generated/`.

To verify whether the committed generated files are current without rewriting
them:

```bash
cargo run -p tempoch-time-data-updater -- --check
```

You can also run it from the crate directory:

```bash
cd tempoch/tempoch-time-data-updater
cargo run -- --check
```

## Typical Workflow

When upstream timekeeping data changes:

1. Run the updater.
2. Review the diff in the generated files.
3. Run the relevant tests, typically starting with:

```bash
cd tempoch
cargo test
```

If you need an explicit freshness verification outside the scheduled refresh,
`--check` is the maintainer-facing mode to run manually. Repository freshness
is maintained by the scheduled Monday GitHub Actions refresh, which runs at
05:23 UTC and pushes changes directly to `main` when the upstream datasets
change.

## Automated weekly publish

When the Monday refresh commits new generated files to `main`, a second
GitHub Actions job (`publish`) automatically:

1. Runs a **WIP guard** — checks for any commits on `main` since the last
   version tag that touch files outside `tempoch-time-data/src/generated/`.  If
   such commits exist, the publish is skipped with a warning so that
   unreviewed feature work is not shipped inadvertently.
2. **Bumps the patch version** across all four publishable crates
   (`tempoch-time-data`, `tempoch-core`, `tempoch`, `tempoch-ffi`) using
   `.github/scripts/bump-versions.sh`.  Dependency version constraints between
   crates are updated at the same time.
3. **Updates `CHANGELOG.md`** using `.github/scripts/update-changelog.sh`,
   which diffs the provenance SHA256 fields in `time_data.provenance.json`
   against the last tagged version to describe exactly which upstream datasets
   changed (UTC-TAI history, ΔT observed/predicted, EOP finals).
4. **Commits, tags, and pushes** the version bump as
   `chore(release): bump to vX.Y.Z` with an annotated tag `vX.Y.Z`.
5. **Publishes** the four crates to crates.io in dependency order with 30-second
   waits between each step to allow index propagation.

### Required repository secret

The publish job requires a `CARGO_REGISTRY_TOKEN` secret set in
**Settings → Secrets and variables → Actions**.  Without this secret the
publish steps will fail; the data-refresh commit will still land on `main`.

### Maintenance scripts

The two helper scripts are self-documented:

| Script | Purpose |
|--------|---------|
| `.github/scripts/bump-versions.sh` | Read and increment patch version in all Cargo.toml files |
| `.github/scripts/update-changelog.sh` | Generate a CHANGELOG entry from provenance diffs |

Both scripts can be run locally from the workspace root for testing or manual
releases:

```bash
bash .github/scripts/bump-versions.sh
bash .github/scripts/update-changelog.sh 0.4.3
```

### Manual trigger

The `update-time-data` workflow can be triggered manually from the Actions tab
(`workflow_dispatch`), which will run both the refresh and, if data changed,
the publish job.

## Notes

- The tool is a maintenance utility for repository authors, not a runtime
  dependency for downstream `tempoch` users.
- If you want runtime freshness inside an application instead of regenerating
  checked-in files, use the ordinary UT1 and UTC civil APIs. They will prefer
  the cached/refreshed bundle automatically, using `TEMPOCH_DATA_DIR` for
  cache placement.
- Network access is required when running the updater because it pulls current
  upstream datasets.
- GitHub cron schedules are defined in UTC; the repository's automatic refresh
  therefore guarantees a Monday run in UTC, not in every local timezone.
- If upstream formats change, parsing will fail rather than silently generating
  incorrect tables.
