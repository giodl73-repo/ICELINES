# Phase Wild Admin Verify Inventory

## Purpose

Inventory admin data verify route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Wild Admin Verify posture |
|---|---|---|
| JSON verify | `POST /api/v1/admin/data/verify` | Keep safe `DataMutationIntent::verify`, known target validation, unknown-target rejection, and `MutationResultView`. |
| HTML verify | `POST /admin/data/verify` | Keep verify controls limited to manifest rows, shared intent, `/admin` redirect, and install/remove deferrals. |

## Risks to Avoid

- Claiming web release bundle install.
- Claiming web destructive data remove.
- Claiming arbitrary filesystem mutation from verify.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused admin data verify tests cover JSON
   result, unknown-target rejection, HTML form rendering, and redirect behavior.
3. Matrix wording. Result: passed; data verify rows now carry scoped
   safe-verification wording.
4. Closeout. Result: passed; Phase Wild Admin Verify is closed with final
   route-row claims and non-claims recorded.
