import { runDocs } from './adapters/docs.mjs';
import { runFrontend } from './adapters/frontend.mjs';
import { runRust } from './adapters/rust.mjs';
import { runSentinel } from './adapters/sentinel.mjs';
import { runVarsense } from './adapters/varsense.mjs';
import { runCustom } from './adapters/custom.mjs';
import { isFullExecution } from './profile-contract.mjs';

export function stageDefinitions(context, scope, taskId) {
  const definitions = [{ name: 'sentinel', run: () => runSentinel(context, scope) }];
  /* [028A-6] --full/--ci amplía el fingerprint, pero un perfil explícito
   * sigue siendo un filtro estricto de etapas. */
  const runAllStages = isFullExecution(scope);
  if (runAllStages || scope.profiles.has('css') || scope.profiles.has('frontend')) {
    definitions.push({ name: 'varsense', run: () => runVarsense(context) });
  }
  if (runAllStages || scope.profiles.has('rust')) definitions.push({ name: 'rust', run: () => runRust(context) });
  if (runAllStages || scope.profiles.has('frontend')) definitions.push({ name: 'frontend', run: () => runFrontend(context) });
  if (runAllStages || scope.profiles.has('docs')) definitions.push({ name: 'docs', run: () => runDocs(context, taskId) });
  /* [Auditoría v4] Custom checks: DOM abstraction, singleton state, window refs */
  if (runAllStages || scope.profiles.has('frontend')) definitions.push({ name: 'custom', run: () => runCustom({ ...context, scope }) });
  return definitions;
}
