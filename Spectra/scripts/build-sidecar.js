import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Get the project root directory (workspace root)
const rootDir = path.join(__dirname, '..');

try {
  console.log('Retrieving rustc target triple...');
  // Get the target triple from rustc
  const rustcOutput = execSync('rustc -vV').toString();
  const hostLine = rustcOutput.split('\n').find(line => line.startsWith('host: '));
  
  if (!hostLine) {
    throw new Error('Could not determine rustc host triple');
  }
  
  const targetTriple = hostLine.split(' ')[1].trim();
  console.log(`Detected target triple: ${targetTriple}`);

  // Build the MCP server
  console.log('Building spectra-mcp release binary...');
  execSync('cargo build -p spectra-mcp --release', { 
    cwd: rootDir,
    stdio: 'inherit' 
  });

  // Source binary path
  const ext = process.platform === 'win32' ? '.exe' : '';
  const sourceBin = path.join(rootDir, 'target', 'release', `spectra-mcp${ext}`);
  
  // Destination path (Tauri requires it in a specific folder, e.g. crates/spectra-tauri/bin/spectra-mcp-<triple>)
  const binDir = path.join(__dirname, '..', 'crates', 'spectra-tauri', 'bin');
  
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  const targetBin = path.join(binDir, `spectra-mcp-${targetTriple}${ext}`);

  console.log(`Copying binary from ${sourceBin} to ${targetBin}`);
  fs.copyFileSync(sourceBin, targetBin);
  
  // Ensure it's executable
  if (process.platform !== 'win32') {
    fs.chmodSync(targetBin, '0755');
  }

  console.log('Sidecar build completed successfully!');
} catch (error) {
  console.error('Error building sidecar:', error.message);
  process.exit(1);
}
