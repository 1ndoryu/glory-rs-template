import { cleanupTargets, shouldRunMaintenance, markMaintenanceRun, DEFAULT_MAINTENANCE_BUDGET_MS } from './target-maintenance.mjs';

/* [028A-6] La supervisión de targets es mantenimiento auxiliar, igual que la
 * retención de reportes: un fallo de filesystem o un pase truncado nunca
 * debe convertir un gate válido en error ni ocultar sus etapas. El gate
 * ejecuta el pase con throttle (una vez por ventana) y presupuesto de
 * tiempo; el comando manual `npm run quality:cleanup` fuerza el pase
 * completo. */
export async function runTargetMaintenanceBestEffort({
  projectRoot,
  targetRoot,
  now = Date.now(),
  intervalMs,
  /* [028A-6] El budget tiene default AQUÍ: sin él, el primer pase del gate
   * caminaría los GB de targets sin límite y colgaría el gate. */
  budgetMs = DEFAULT_MAINTENANCE_BUDGET_MS,
  cleanup = cleanupTargets,
  shouldRun = shouldRunMaintenance,
  mark = markMaintenanceRun,
} = {}) {
  try {
    const due = await shouldRun({ targetRoot, now, intervalMs });
    if (!due) {
      return { status: 'pass', skipped: 'cooldown', targetRoot };
    }
    const result = await cleanup({ projectRoot, targetRoot, now, dryRun: false, budgetMs });
    await mark(targetRoot, now);
    return { status: 'pass', ...result };
  } catch (error) {
    return { status: 'error', message: error instanceof Error ? error.message : String(error) };
  }
}
