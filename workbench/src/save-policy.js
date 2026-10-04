// Library changes may revoke consent, but never enable saving in another tab.
// Policy belongs to the saved context; fresh unsaved revisions may retain it.
export function reconcileSavePolicy(dataset, entries) {
    dataset.keepUpdated = dataset.keepUpdated && entries.some(entry => entry.id === dataset.id && entry.keepUpdated);
}
