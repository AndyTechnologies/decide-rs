#!/usr/bin/env node
/**
 * Post-install script for pi-decide extension
 * Downloads decide-rs binaries and Julia-1 model automatically with verification
 */

import { existsSync, mkdirSync, chmodSync, writeFileSync, readFileSync } from 'fs';
import { join, dirname, basename } from 'path';
import { fileURLToPath } from 'url';
import https from 'https';
import { createWriteStream } from 'fs';
import crypto from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const EXTENSION_ROOT = join(__dirname, '..');
const BIN_DIR = join(EXTENSION_ROOT, 'bin');
const HOME_DIR = process.env.HOME || process.env.USERPROFILE || '/home/' + (process.env.USER || 'user');
const MODEL_DIR = join(process.env.XDG_DATA_HOME || join(HOME_DIR, '.local', 'share'), 'decide-rs', 'julia-checkpoint');

const VERSION = '0.1.0';
const REPO = 'AndyTechnologies/decide-rs';

// Platform detection
const platform = process.platform;
const arch = process.arch;

let platformKey = '';
let binarySuffix = '';

if (platform === 'linux' && arch === 'x64') {
  platformKey = 'linux-x86_64';
  binarySuffix = 'x86_64-unknown-linux-gnu';
} else if (platform === 'darwin' && arch === 'x64') {
  platformKey = 'macos-x86_64';
  binarySuffix = 'x86_64-apple-darwin';
} else if (platform === 'darwin' && arch === 'arm64') {
  platformKey = 'macos-arm64';
  binarySuffix = 'aarch64-apple-darwin';
} else if (platform === 'win32' && arch === 'x64') {
  platformKey = 'windows-x86_64';
  binarySuffix = 'x86_64-pc-windows-msvc';
} else {
  console.error('[pi-decide] Unsupported platform:', platform, arch);
  console.error('[pi-decide] Manual installation required. See: https://github.com/AndyTechnologies/decide-rs/releases');
  process.exit(0); // Don't fail install, just warn
}

const BINARIES = [
  { name: 'decide-rs', localName: platform === 'win32' ? 'decide-rs.exe' : 'decide-rs' },
  { name: 'decide-rs-mcp', localName: platform === 'win32' ? 'decide-rs-mcp.exe' : 'decide-rs-mcp' },
  { name: 'decide-rs-download', localName: platform === 'win32' ? 'decide-rs-download.exe' : 'decide-rs-download' },
];

// SHA256 checksums for each binary (to be populated after release)
// Format: { platformKey: { binaryName: sha256 } }
const CHECKSUMS = {
  'linux-x86_64': {
    'decide-rs': '',
    'decide-rs-mcp': '',
    'decide-rs-download': '',
  },
  'macos-x86_64': {
    'decide-rs': '',
    'decide-rs-mcp': '',
    'decide-rs-download': '',
  },
  'macos-arm64': {
    'decide-rs': '',
    'decide-rs-mcp': '',
    'decide-rs-download': '',
  },
  'windows-x86_64': {
    'decide-rs': '',
    'decide-rs-mcp': '',
    'decide-rs-download': '',
  },
};

const MODEL_FILES = [
  'julia_config.json',
  'encoder/config.json',
  'tokenizer/tokenizer.json',
  'tokenizer/tokenizer_config.json',
  'model.safetensors',
];

const MODEL_BASE_URL = 'https://huggingface.co/SupersonicLabs/Julia-1/resolve/a85b127321d580d65176c89ced8273f305745d85';

function log(message) {
  console.log('[pi-decide] ' + message);
}

function warn(message) {
  console.warn('[pi-decide] WARN: ' + message);
}

function error(message) {
  console.error('[pi-decide] ERROR: ' + message);
}

function getBinaryUrl(binary) {
  return `https://github.com/${REPO}/releases/download/v${VERSION}/${binary.name}-${binarySuffix}`;
}

function verifyChecksum(filePath, expectedSha256) {
  if (!expectedSha256) {
    warn('No checksum available for ' + basename(filePath) + ', skipping verification');
    return true;
  }
  const hash = crypto.createHash('sha256');
  const content = readFileSync(filePath);
  hash.update(content);
  const actual = hash.digest('hex');
  if (actual.toLowerCase() !== expectedSha256.toLowerCase()) {
    error('Checksum mismatch for ' + basename(filePath) + ': expected ' + expectedSha256 + ', got ' + actual);
    return false;
  }
  log('  ✓ Checksum verified: ' + basename(filePath));
  return true;
}

function downloadFile(url, destPath, expectedSha256 = '') {
  return new Promise((resolve, reject) => {
    log('Downloading ' + basename(destPath) + '...');
    const file = createWriteStream(destPath);

    const req = https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        const redirectUrl = response.headers.location;
        if (redirectUrl) {
          log('  Redirecting...');
          file.close();
          downloadFile(redirectUrl, destPath, expectedSha256).then(resolve).catch(reject);
          return;
        }
      }
      if (response.statusCode !== 200) {
        file.close();
        reject(new Error('HTTP ' + response.statusCode + ': ' + response.statusMessage));
        return;
      }
      const total = parseInt(response.headers['content-length'] || '0', 10);
      let downloaded = 0;
      let lastReport = 0;
      response.on('data', (chunk) => {
        downloaded += chunk.length;
        if (total > 0 && downloaded - lastReport >= 1024 * 1024) { // Report every 1MB
          lastReport = downloaded;
          const percent = ((downloaded / total) * 100).toFixed(1);
          process.stdout.write('\r  Progress: ' + percent + '% (' + (downloaded / 1024 / 1024).toFixed(1) + ' MB / ' + (total / 1024 / 1024).toFixed(1) + ' MB)');
        }
      });
      response.pipe(file);
      file.on('finish', () => {
        file.close();
        process.stdout.write('\n');
        if (!verifyChecksum(destPath, expectedSha256)) {
          reject(new Error('Checksum verification failed'));
          return;
        }
        log('Downloaded and verified: ' + destPath);
        resolve();
      });
    });

    req.on('error', (err) => {
      file.destroy();
      reject(err);
    });
    req.setTimeout(300000, () => { // 5 min timeout
      file.destroy();
      reject(new Error('Download timeout'));
    });
  });
}

function downloadModelFile(file, destDir) {
  return new Promise((resolve, reject) => {
    const url = MODEL_BASE_URL + '/' + file;
    const destPath = join(destDir, file);
    const destDirPath = dirname(destPath);

    if (!existsSync(destDirPath)) {
      mkdirSync(destDirPath, { recursive: true });
    }

    if (existsSync(destPath)) {
      log('  ' + file + ': already exists, skipping');
      resolve();
      return;
    }

    log('Downloading model file: ' + file);
    const fileStream = createWriteStream(destPath);

    const req = https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        const redirectUrl = response.headers.location;
        if (redirectUrl) {
          log('  Redirecting...');
          fileStream.close();
          downloadFile(redirectUrl, destPath).then(resolve).catch(reject);
          return;
        }
      }
      if (response.statusCode !== 200) {
        fileStream.close();
        reject(new Error('HTTP ' + response.statusCode + ' for ' + file));
        return;
      }
      const total = parseInt(response.headers['content-length'] || '0', 10);
      let downloaded = 0;
      let lastReport = 0;
      response.on('data', (chunk) => {
        downloaded += chunk.length;
        if (total > 0 && downloaded - lastReport >= 10 * 1024 * 1024) { // Report every 10MB
          lastReport = downloaded;
          const percent = ((downloaded / total) * 100).toFixed(1);
          process.stdout.write('\r  Progress: ' + percent + '% (' + (downloaded / 1024 / 1024).toFixed(1) + ' MB / ' + (total / 1024 / 1024).toFixed(1) + ' MB)');
        }
      });
      response.pipe(fileStream);
      fileStream.on('finish', () => {
        fileStream.close();
        process.stdout.write('\n');
        log('  ' + file + ': downloaded');
        resolve();
      });
    });

    req.on('error', (err) => {
      fileStream.destroy();
      reject(err);
    });
    req.setTimeout(600000, () => { // 10 min timeout for model files
      fileStream.destroy();
      reject(new Error('Download timeout'));
    });
  });
}

async function setupBinaries() {
  log('Setting up binaries for ' + platformKey + '...');

  if (!existsSync(BIN_DIR)) {
    mkdirSync(BIN_DIR, { recursive: true });
  }

  const platformChecksums = CHECKSUMS[platformKey] || {};
  let anyFailed = false;

  for (const binary of BINARIES) {
    const destPath = join(BIN_DIR, binary.localName);
    if (existsSync(destPath)) {
      log('Binary ' + binary.localName + ' already exists, skipping');
      continue;
    }
    try {
      const expectedSha256 = platformChecksums[binary.name] || '';
      await downloadFile(getBinaryUrl(binary), destPath, expectedSha256);
      if (platform !== 'win32') {
        chmodSync(destPath, 0o755);
      }
    } catch (err) {
      error('Failed to download ' + binary.localName + ': ' + err.message);
      anyFailed = true;
    }
  }

  if (anyFailed) {
    warn('Some binaries failed to download. Manual installation may be required.');
    log('See: https://github.com/' + REPO + '/releases/tag/v' + VERSION);
    return false;
  }

  log('Binaries installed to ' + BIN_DIR);
  return true;
}

async function setupModel() {
  log('Setting up Julia-1 model...');

  if (!existsSync(MODEL_DIR)) {
    mkdirSync(MODEL_DIR, { recursive: true });
  }

  const modelPath = join(MODEL_DIR, 'model.safetensors');
  if (existsSync(modelPath)) {
    log('Model already exists, skipping download');
    return true;
  }

  log('Downloading Julia-1 model files (~1.5 GB)...');
  for (const file of MODEL_FILES) {
    try {
      await downloadModelFile(file, MODEL_DIR);
    } catch (err) {
      error('Failed to download ' + file + ': ' + err.message);
      return false;
    }
  }

  log('Model downloaded successfully to ' + MODEL_DIR);
  return true;
}

function printNextSteps(binariesOk, modelOk) {
  console.log('');
  console.log('╔══════════════════════════════════════════════════════════════╗');
  console.log('║  pi-decide post-install complete                             ║');
  console.log('╚══════════════════════════════════════════════════════════════╝');
  console.log('');

  if (!binariesOk) {
    console.log('⚠️  Binaries: Some failed to auto-download');
    console.log('   Manual installation:');
    console.log('   1. Download from: https://github.com/' + REPO + '/releases/tag/v' + VERSION);
    console.log('   2. Place in: ' + BIN_DIR);
    console.log('   3. Make executable: chmod +x ' + BIN_DIR + '/*');
    console.log('');
  } else {
    console.log('✅ Binaries installed to: ' + BIN_DIR);
    console.log('   Add to PATH (add to ~/.bashrc, ~/.zshrc, or ~/.config/fish/config.fish):');
    if (platform === 'win32') {
      console.log('   $env:PATH += ";' + BIN_DIR + '"');
    } else {
      console.log('   export PATH="' + BIN_DIR + ':$PATH"');
    }
    console.log('');
  }

  if (!modelOk) {
    console.log('⚠️  Model: Auto-download failed or incomplete');
    console.log('   Manual: decide-rs fetch (after adding binaries to PATH)');
    console.log('');
  } else {
    console.log('✅ Model installed to: ' + MODEL_DIR);
    console.log('');
  }

  console.log('Next steps:');
  console.log('  1. Restart Pi / Gentle Shell to load the extension');
  console.log('  2. Run: decide-rs fetch    # Verify model download');
  console.log('  3. Run: decide-rs demo     # Test the engine');
  console.log('');
  console.log('Documentation: https://github.com/' + REPO + '#readme');
  console.log('Issues: https://github.com/' + REPO + '/issues');
}

async function main() {
  log('Starting auto-setup for pi-decide v' + VERSION + '...');
  log('Platform: ' + platformKey);
  log('Extension root: ' + EXTENSION_ROOT);
  log('Binary dir: ' + BIN_DIR);
  log('Model dir: ' + MODEL_DIR);

  try {
    const binariesOk = await setupBinaries();
    const modelOk = await setupModel();
    printNextSteps(binariesOk, modelOk);
  } catch (err) {
    error('Setup failed: ' + err.message);
    printNextSteps(false, false);
    process.exit(0); // Don't fail npm install, just warn
  }
}

main().catch((err) => {
  error('Fatal error: ' + err.message);
  process.exit(0);
});