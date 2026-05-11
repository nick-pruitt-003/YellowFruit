/**
 * Patches electron-devtools-installer to use the non-deprecated
 * session.extensions.* API introduced in Electron 32+.
 *
 * electron-devtools-installer v4.0.0 still calls session.getAllExtensions()
 * and session.loadExtension() directly, which are deprecated in favor of
 * session.extensions.getAllExtensions() and session.extensions.loadExtension().
 */
const fs = require('fs');
const path = require('path');

const targetFile = path.join(
  __dirname,
  '../../node_modules/electron-devtools-installer/dist/index.js',
);

if (!fs.existsSync(targetFile)) {
  console.log('patch-electron-devtools: package not found, skipping');
  process.exit(0);
}

let src = fs.readFileSync(targetFile, 'utf8');

// Use the exact original strings from the unpatched package so we never double-apply
const oldGetAll = 'const installedExtension = targetSession.getAllExtensions().find';
const newGetAll =
  'const installedExtension = (targetSession.extensions ? targetSession.extensions.getAllExtensions() : targetSession.getAllExtensions()).find';

const oldLoad = 'return targetSession.loadExtension(extensionFolder, loadExtensionOptions);';
const newLoad =
  'return targetSession.extensions\n        ? targetSession.extensions.loadExtension(extensionFolder, loadExtensionOptions)\n        : targetSession.loadExtension(extensionFolder, loadExtensionOptions);';

let changed = false;

if (src.includes(oldGetAll)) {
  src = src.replace(oldGetAll, newGetAll);
  changed = true;
}

if (src.includes(oldLoad)) {
  src = src.replace(oldLoad, newLoad);
  changed = true;
}

if (changed) {
  fs.writeFileSync(targetFile, src);
  console.log('patch-electron-devtools: patched session.extensions.* API');
} else {
  console.log('patch-electron-devtools: already patched or not needed');
}
