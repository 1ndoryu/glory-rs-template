import { runProcess } from '../runner.mjs';
import { readToolReport, toolFailure, writeStageLog } from './common.mjs';

/* [018A-5] Contrato declarativo de adapters: el adapter solo describe
 * ejecutable, argumentos, schema y timeout; el runner conserva la política de
 * errores, logs y redacción en un único punto. */
export async function runStructuredTool(context, definition) {
  const execution = await runProcess(definition.executable, definition.args, {
    cwd: definition.cwd ?? context.projectRoot,
    timeoutMs: definition.timeoutMs,
  });
  const logPath = await writeStageLog(context, definition.name, `${execution.stdout}\n${execution.stderr}`);
  if (execution.code === 2 || execution.timedOut) {
    return { failure: toolFailure(definition.name, execution, logPath), logPath, execution };
  }

  try {
    const report = await readToolReport(definition.reportPath);
    if (String(report.schemaVersion) !== String(definition.expectedSchemaVersion)) {
      throw new Error(`schema ${report.schemaVersion} incompatible (esperado ${definition.expectedSchemaVersion})`);
    }
    return { report, logPath, execution };
  } catch (error) {
    return {
      failure: toolFailure(definition.name, { ...execution, code: 2, stderr: error.message }, logPath),
      logPath,
      execution,
    };
  }
}
