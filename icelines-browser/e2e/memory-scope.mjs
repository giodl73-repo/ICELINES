// Once an intervention occurs, subsequent measurements cannot be controls.
export function normalWorkloadPhases(phases) {
  const intervention = phases.findIndex(phase => phase.phase.startsWith('diagnostic_'));
  return intervention < 0 ? phases : phases.slice(0, intervention);
}
