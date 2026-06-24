import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';

const [wasmPath, metadataPath] = process.argv.slice(2);
if (!wasmPath || !metadataPath) {
  throw new Error('usage: write-fixture-metadata <plugin.wasm> <metadata.json>');
}
const bytes = readFileSync(wasmPath);
const sha256 = createHash('sha256').update(bytes).digest('hex');
const metadata = {
  assemblyscript: '0.27.31',
  extismAsPdk: '1.0.0',
  wasmSha256: `sha256:${sha256}`,
  wasmBytes: bytes.length,
  export: 'handle'
};
writeFileSync(metadataPath, `${JSON.stringify(metadata, null, 2)}\n`);
