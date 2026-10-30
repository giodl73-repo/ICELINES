// Library changes may revoke consent, but never enable saving in another tab.
// Policy belongs to the saved context; fresh unsaved revisions may retain it.
export function reconcileSavePolicy(
  dataset: { id: string; keepUpdated: boolean },
  entries: readonly { id: string; keepUpdated: boolean }[],
): void {
  dataset.keepUpdated = dataset.keepUpdated && entries.some(entry => entry.id === dataset.id && entry.keepUpdated);
}
