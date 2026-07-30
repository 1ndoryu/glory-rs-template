#!/usr/bin/env node

/* Ejecuta cualquier comando de cargo con DATABASE_URL y CARGO_TARGET_DIR
 * alineados a la rama/proyecto actual. */

import { spawn } from 'node:child_process';
import { getBranchDbContext } from './branch-db.mjs';

function cargoCommand() {
  return process.platform === 'win32' ? 'cargo.exe' : 'cargo';
}

const cargoArgs = process.argv.slice(2);
if (cargoArgs.length === 0) {
  console.error('Uso: node scripts/run-with-db.mjs <subcomando cargo> [...args]');
  process.exit(1);
}

console.log('');
const { dbUrl, cargoTargetDir } = getBranchDbContext();
console.log('');

const child = spawn(cargoCommand(), cargoArgs, {
  stdio: 'inherit',
  env: { ...process.env, DATABASE_URL: dbUrl, CARGO_TARGET_DIR: cargoTargetDir },
  shell: false,
});

child.on('error', (err) => {
  console.error('[run-with-db] Error:', err.message);
  process.exit(1);
});
child.on('exit', (code, signal) => {
  if (signal || code === null) {
    console.error(`[run-with-db] Cargo terminó sin exit code (${signal ?? 'unknown signal'}).`);
    process.exit(2);
  }
  process.exit(code);
});
