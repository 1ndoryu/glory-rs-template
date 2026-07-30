#!/usr/bin/env node
/* Script que parsea roadmap.md y reporta tareas pendientes.
 * Disenado para correr como VS Code task con problemMatcher,
 * generando warnings estilo compilador que aparecen en Problemas.
 * Formato: archivo:linea:columna: warning: mensaje */

import { readFileSync, existsSync, watchFile } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, '..');

function findRoadmaps() {
  const paths = [
    resolve(projectRoot, 'roadmap.md'),
    resolve(projectRoot, 'App', 'roadmap.md'),
  ];
  return paths.filter((value) => existsSync(value));
}

function parsePendingTasks(filePath) {
  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split(/\r?\n/);
  const tasks = [];
  let inPending = false;

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const lower = line.toLowerCase();

    if (lower.includes('pendientes ordenados') && /^##\s/.test(line)) {
      inPending = true;
      continue;
    }

    if (inPending && /^##\s/.test(line) && !/^###\s/.test(line)) {
      inPending = false;
      continue;
    }

    if (!inPending) continue;

    const trimmed = line.trim();
    if (/^- \[ \]/.test(trimmed)) tasks.push({ line: index + 1, text: trimmed });
  }

  return tasks;
}

function report() {
  const roadmaps = findRoadmaps();
  let totalTasks = 0;

  for (const roadmap of roadmaps) {
    const tasks = parsePendingTasks(roadmap);
    totalTasks += tasks.length;

    for (const task of tasks) {
      console.log(`${roadmap}:${task.line}:1: warning: TAREA PENDIENTE: ${task.text}`);
    }
  }

  if (totalTasks === 0) {
    console.log('[roadmap-watcher] Sin tareas pendientes.');
  } else {
    console.log(`\n[roadmap-watcher] ${totalTasks} tarea(s) pendiente(s) detectada(s).`);
  }

  return totalTasks;
}

const watchMode = process.argv.includes('--watch');

if (watchMode) {
  console.log('[roadmap-watcher] Vigilando roadmap.md...');
  report();

  const roadmaps = findRoadmaps();
  for (const roadmap of roadmaps) {
    watchFile(roadmap, { interval: 2000 }, () => {
      console.log(`\n[roadmap-watcher] Cambio detectado en ${roadmap}`);
      report();
    });
  }
} else {
  const count = report();
  process.exit(count > 0 ? 1 : 0);
}
