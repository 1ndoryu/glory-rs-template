#!/usr/bin/env node

/* Ejecuta cualquier comando de cargo con DATABASE_URL y CARGO_TARGET_DIR
 * alineados a la rama/proyecto actual. */

import { spawn, spawnSync } from 'node:child_process';
import { getBranchDbContext } from './branch-db.mjs';

function cargoCommand() {
  return process.platform === 'win32' ? 'cargo.exe' : 'cargo';
}

function migrarBdRama(dbUrl) {
  const version = spawnSync(cargoCommand(), ['sqlx', '--version'], { encoding: 'utf8' });
  if (version.status !== 0) {
    console.warn('[db] Aviso: `cargo-sqlx` no instalado; omito migracion previa.');
    console.warn('[db] Instala con `cargo install sqlx-cli --no-default-features --features postgres`.');
    return;
  }
  const mig = spawnSync(cargoCommand(), ['sqlx', 'migrate', 'run'], {
    stdio: 'inherit',
    env: { ...process.env, DATABASE_URL: dbUrl },
    shell: false,
  });
  if (mig.status !== 0) {
    console.error('[db] La migracion de la BD de rama fallo; repara el estado de');
    console.error('[db] migraciones antes de compilar (ver `Agente/completados/tareas-2026-10-01.md`).');
    process.exit(mig.status ?? 1);
  }
}

const cargoArgs = process.argv.slice(2);
if (cargoArgs.length === 0) {
  console.error('Uso: node scripts/run-with-db.mjs <subcomando cargo> [...args]');
  process.exit(1);
}

console.log('');
const { dbUrl, cargoTargetDir } = getBranchDbContext();
console.log('');

/* [011A-3] La BD de rama se crea vacia y los macros `query_*!` validan
 * contra la BD viva: sin migraciones, `check/test` fallan con errores
 * cripticos (`no existe la relacion ...`). Migrar aqui deja la BD lista
 * antes de compilar. Si falta `cargo-sqlx` se avisa y se sigue (el backend
 * automigra al arrancar); si la migracion falla, se corta con el error
 * visible en vez de dejar que los macros fallen despues. */
migrarBdRama(dbUrl);

const child = spawn(cargoCommand(), cargoArgs, {
  stdio: 'inherit',
  /* [05AA-3] sccache útil: sin incremental + basedirs para que la caché
   * acierte tras cada purga del target/ (el usuario arranca siempre de cero).
   * RUSTC_WRAPPER ya viene del entorno de usuario. */
  env: { ...process.env, DATABASE_URL: dbUrl, CARGO_TARGET_DIR: cargoTargetDir, CARGO_INCREMENTAL: '0', SCCACHE_BASEDIRS: process.env.SCCACHE_BASEDIRS || 'C:/Users/Owner/OneDrive/Documentos/area-trabajo' },
  shell: false,
});

child.on('error', (err) => {
  console.error('[run-with-db] Error:', err.message);
  process.exit(1);
});
child.on('exit', (code) => process.exit(code ?? 0));
