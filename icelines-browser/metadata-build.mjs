// A function replacement preserves literal $&, $', $` and $$ in source strings.
export function injectCatalogMetadata(source, headers) {
  const marker = 'const knownMetadata = {}; // BUILD_CATALOG_METADATA';
  if (source.split(marker).length !== 2) throw new Error('Missing or repeated catalog metadata build marker');
  return source.replace(marker, () => `const knownMetadata = ${JSON.stringify(headers)}; // BUILD_CATALOG_METADATA`);
}
