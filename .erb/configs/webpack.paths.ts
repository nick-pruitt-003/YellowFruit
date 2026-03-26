import path from 'path';

// All paths are derived from the project root (process.cwd()).
// npm scripts always run from the project root, so this is reliable
// and works in both CJS (ts-node) and ESM (webpack-cli native TS import) contexts.
const rootPath = process.cwd();

const erbPath = path.join(rootPath, '.erb');
const erbNodeModulesPath = path.join(erbPath, 'node_modules');

const dllPath = path.join(erbPath, 'dll');

const srcPath = path.join(rootPath, 'src');
const srcMainPath = path.join(srcPath, 'main');
const srcRendererPath = path.join(srcPath, 'renderer');

const releasePath = path.join(rootPath, 'release');
const appPath = path.join(releasePath, 'app');
const appPackagePath = path.join(appPath, 'package.json');
const appNodeModulesPath = path.join(appPath, 'node_modules');
const srcNodeModulesPath = path.join(srcPath, 'node_modules');

const distPath = path.join(appPath, 'dist');
const distMainPath = path.join(distPath, 'main');
const distRendererPath = path.join(distPath, 'renderer');

const buildPath = path.join(releasePath, 'build');

export default {
  rootPath,
  erbNodeModulesPath,
  dllPath,
  srcPath,
  srcMainPath,
  srcRendererPath,
  releasePath,
  appPath,
  appPackagePath,
  appNodeModulesPath,
  srcNodeModulesPath,
  distPath,
  distMainPath,
  distRendererPath,
  buildPath,
};
