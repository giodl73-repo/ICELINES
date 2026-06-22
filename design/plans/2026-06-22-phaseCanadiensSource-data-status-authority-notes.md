# Phase Canadiens Source - Data status authority notes

Status: Closed

## Intent

Carry advanced-source authority into operational freshness surfaces so operators
can see what optional snapshots do and do not prove.

## Scope

- Add shared `DataStatusView.authority_notes`.
- Add a MoneyPuck skater snapshot authority note covering optional xG/CF/FF
  metrics.
- Name blocked adjacent claims: goalie xGA/GSAx, goalie high-danger SV%,
  skater high-danger chance %, zone entries, and deployment recommendations.
- Render the note in CLI `data-status`, Web `/admin`, and
  `/api/v1/admin/data-status` without creating local cache state.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core data_status_view_exposes_moneypuck_snapshot_authority_note`
- `cargo test -p icelines-web --test l1_router admin_data_status`
- `git diff --check`
