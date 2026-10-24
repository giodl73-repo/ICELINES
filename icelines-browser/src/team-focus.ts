// Only generate a single shared-Rust predicate, never evaluate team membership here.
export function teamSeasonFilter(code: string): string {
  const team = code.trim().toUpperCase();
  if (!/^[A-Z][A-Z0-9]{1,7}$/.test(team)) throw new Error('Enter a team code of 2–8 letters or digits starting with a letter, such as MTL.');
  return `team.any=${team}`;
}

// The Rust projection uses slash-separated season stints. Do not interpret arbitrary
// imported text as query syntax or offer unknown/missing-team placeholders.
export function playerTeamCodes(display: string): string[] {
  return [...new Set(display.split('/').map(value => value.trim()).filter(value => /^[A-Z][A-Z0-9]{1,7}$/.test(value) && !['RET', 'UNK', 'UNKNOWN', 'TOT'].includes(value)))];
}
