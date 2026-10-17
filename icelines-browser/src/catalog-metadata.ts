import { packageMetadata } from './library.js';
import type { PackageData, PackageMetadata } from './types.js';

// The builder replaces this marker with headers keyed by the exact digest of
// the package bytes it validates. Unknown revisions retain the full parser.
const knownMetadata: Record<string, PackageMetadata> = {}; // BUILD_CATALOG_METADATA
const decoder = new TextDecoder('utf-8', { fatal: true });

// Call only after network/storage validation authenticates bytes against revision.
// A known digest identifies the whole validated payload, not just its prefix.
export function metadataForVerifiedCatalogBytes(bytes: Uint8Array, revision: string): PackageMetadata {
  if (Object.hasOwn(knownMetadata,revision)) return { ...knownMetadata[revision] };
  return packageMetadata(JSON.parse(decoder.decode(bytes)) as PackageData);
}
