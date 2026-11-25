# Production mix v1

Playing Tendencies has a compact **Point mix** disclosure. Its headline shows
goals as a share of all recorded points. Expanded rows show goal and assist rates
and shares. Matched primary/secondary assist detail is a breakdown of the assist
row, not extra production. This complements the feat library and documentary
class; it does not replace or assign either.

Use the selected skater's existing all-strength NHL context. With goal rate G and
assist rate A, goal share = G / (G + A) and assist share = A / (G + A). The common
TOI denominator cancels. Display rates to two decimals and shares to one decimal;
compute from unrounded values. These are observed shares, not season projections.
No recorded points means no percentage. Missing either rate means no record.

Optional primary/secondary credits must be whole, nonnegative counts. Convert to
per-60 rates using the same parent minutes and require their combined rate to
match the NHL assist rate within 1e-8. Otherwise omit this detail while retaining
the supported goal/assist composition. Detail shares divide by all points, which
differs from the existing Passing record's primary-share-of-assists measure.

Require a skater, positive GP and finite positive all-strength minutes. There is
no ten-GP gate. Bind actor, season, group, GP and minutes; catalog validation
rejects altered or stale records. Rust builds the optional record during native
and WASM profile admission from existing refresh inputs. TypeScript renders it;
older serialized cards can omit it. No additional fetch or enrichment is needed.

The disclosure stays closed initially and is hidden in compact print. Existing
Playing Tendencies feats and sourced descriptions remain. These point credits
do not enumerate passes, chances created or touches and do not establish a
finisher/playmaker class, assigned role, technical skill or simulation bonus.
