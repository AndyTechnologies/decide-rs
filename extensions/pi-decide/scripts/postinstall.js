#!/usr/bin/env node
/**
 * Post-install script for gentle-agent-extensions
 * Downloads decide-rs binaries and Julia-1 model automatically
 */

import { existsSync, mkdirSync, chmodSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import https from 'https';
import { createWriteStream } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const EXTENSION_ROOT = join(__dirname, '..');
const BIN_DIR = join(__dirname, '..', 'bin');
const HOME_DIR = process.env.HOME || '/home/' + (process.env.USER || 'user');
const MODEL_DIR = join(process.env.HOME || '/home/' + (process.env.USER || 'user'), '.local', 'share', 'decide-rs', 'julia-checkpoint');

const BINARIES = [
  { name: 'decide-rs', url: 'https://github.com/AndyTechnologies/decide-rs/releases/download/v0.1.0/decide-rs-x86_64-unknown-linux-gnu' },
  { name: 'decide-rs-mcp', url: 'https://github.com/AndyTechnologies/decide-rs/releases/download/v0.1.0/decide-rs-mcp-x86_64-unknown-linux-gnu' },
  { name: 'decide-rs-download', url: 'https://github.com/AndyTechnologies/decide-rs/releases/download/v0.1.0/decide-rs-download-x86_64-unknown-linux-gnu' },
];

const MODEL_FILES = [
  'julia_config.json',
  'encoder/config.json',
  'tokenizer/tokenizer.json',
  'tokenizer/tokenizer_config.json',
  'model.safetensors',
];

const MODEL_BASE_URL = 'https://huggingface.co/SupersonicLabs/Julia-1/resolve/a85b127321d580d65176c89ced8273f305745d85';

function log(message) {
  console.log('[gentle-agent-extensions] ' + message);
}

function error(message) {
  console.error('[gentle-agent-extensions] ERROR: ' + message);
}

function downloadFile(url, destPath) {
  return new Promise((resolve, reject) => {
    log('Downloading ' + url + '...');
    const file = createWriteStream(destPath);
    
    const req = https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        const redirectUrl = response.headers.location;
        if (redirectUrl) {
          log('Redirecting to ' + redirectUrl);
          downloadFile(redirectUrl, destPath).then(resolve).catch(reject);
          return;
        }
      }
      if (response.statusCode !== 200) {
        reject(new Error('HTTP ' + response.statusCode + ': ' + response.statusMessage));
        return;
      }
      const total = parseInt(response.headers['content-length'] || '0', 10);
      let downloaded = 0;
      response.on('data', (chunk) => {
        downloaded += chunk.length;
        if (total > 0) {
          const percent = ((downloaded / total) * 100).toFixed(1);
          process.stdout.write('\r  Progress: ' + percent + '% (' + (downloaded / 1024 / 1024).toFixed(1) + ' MB / ' + (total / 1024 / 1024).toFixed(1) + ' MB)');
        }
      });
      response.pipe(file);
      file.on('finish', () => {
        file.close();
        process.stdout.write('\n');
        log('Downloaded to ' + destPath);
        resolve();
      });
    });
    
    req.on('error', (err) => {
      file.destroy();
      reject(err);
    });
  });
}

function downloadModelFile(file, destDir) {
  return new Promise((resolve, reject) => {
    const url = 'https://huggingface.co/SupersonicLabs/Julia-1/resolve/a85b127321d580d65176c89ced8273f305745d85/' + file;
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
          log('Redirecting to ' + redirectUrl);
          downloadFile(redirectUrl, destPath).then(resolve).catch(reject);
          return;
        }
      }
      if (response.statusCode !== 200) {
        reject(new Error('HTTP ' + response.statusCode + ' for ' + file));
        return;
      }
      const total = parseInt(response.headers['content-length'] || '0', 10);
      let downloaded = 0;
      response.on('data', (chunk) => {
        downloaded += chunk.length;
        if (total > 0) {
          const percent = ((downloaded / total) * 100).toFixed(1);
          process.stdout.write('\r  Progress: ' + percent + '% (' + (downloaded / 1024 / 1024).toFixed(1) + ' MB / ' + (total / 1024 / 1024).toFixed(1) + ' MB)');
        }
      });
      response.pipe(fileStream);
      fileStream.on('finish', () => {
        fileStream.close();
        log('  ' + file + ': downloaded');
        resolve();
      });
    });
    
    req.on('error', (err) => {
      fileStream.destroy();
      reject(err);
    });
  });
}

async function setupBinaries() {
  log('Setting up binaries...');
  
  if (!existsSync(BIN_DIR)) {
    mkdirSync(BIN_DIR, { recursive: true });
  }
  
  for (const binary of BINARIES) {
    const destPath = join(BIN_DIR, binary.name);
    if (existsSync(destPath)) {
      log('Binary ' + binary.name + ' already exists, skipping');
      continue;
    }
    try {
      await downloadFile(binary.url, destPath);
      chmodSync(destPath, 0o755);
    } catch (err) {
      error('Failed to download ' + binary.name + ': ' + err.message);
      throw err;
    }
  }
  
  log('Binaries installed to ' + BIN_DIR);
  log('Add to PATH: export PATH="' + BIN_DIR + ':$PATH"');
}

async function setupModel() {
  log('Setting up Julia-1 model...');
  
  if (!existsSync(MODEL_DIR)) {
    mkdirSync(MODEL_DIR, { recursive: true });
  }
  
  const modelPath = join(MODEL_DIR, 'model.safetensors');
  if (existsSync(modelPath)) {
    log('Model already exists, skipping download');
    return;
  }
  
  log('Downloading Julia-1 model files...');
  for (const file of MODEL_FILES) {
    try {
      await downloadModelFile(file, MODEL_DIR);
    } catch (err) {
      error('Failed to download ' + file + ': ' + err.message);
      throw err;
    }
  }
  
  log('Model downloaded successfully');
}

async function main() {
  log('Starting auto-setup for gentle-agent-extensions...');
  log('Extension root: ' + EXTENSION_ROOT);
  log('Binary dir: ' + BIN_DIR);
  log('Model dir: ' + MODEL_DIR);
  
  try {
    await setupBinaries();
    await setupModel();
    log('Auto-setup completed successfully!');
    console.log('');
    console.log('Next steps:');
    console.log('  1. Add binaries to PATH: export PATH="' + BIN_DIR + ':$PATH"');
    console.log('  2. Run: decide-rs fetch (to verify model download)');
    console.log('  3. Restart Pi to load the extension');
  } catch (err) {
    error('Setup failed: ' + err.message);
    process.exit(1);
  }
}

main().catch((err) => {
  error('Fatal error: ' + err.message);
  process.exit(1);
});
