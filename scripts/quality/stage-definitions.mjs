import { runDocs } from './adapters/docs.mjs';
import { runFrontend } from './adapters/frontend.mjs';
import { runRust } from './adapters/rust.mjs';
import { runSentinel } from './adapters/sentinel.mjs';
import { runVarsense } from './adapters/varsense.mjs';
import { runCustom } from './adapters/custom.mjs';
import { isFullExecution, PROFILE_STAGE_RULES } from './profile-contract.mjs';

const STAGE_FACTORIES = {
  varsense: context => ({ name: 'varsense', run: () => runVarsense(context) }),
  rust: context => ({ name: 'rust', run: () => runRust(context) }),
  frontend: context => ({ name: 'frontend', run: () => runFrontend(context) }),
  docs: (context, _scope, taskId) => ({ name: 'docs', run: () => runDocs(context, taskId) }),
  custom: (context, scope) => ({ name: 'custom', run: () => runCustom({ ...context, scope }) }),
};

export function stageDefinitions(context, scope, taskId) {
  const definitions = [{ name: 'sentinel', run: () => runSentinel(context, scope) }];
  /* [028A-6] --full/--ci amplía el fingerprint, pero un perfil explícito
   * sigue siendo un filtro estricto de etapas. */
  const stageNames = isFullExecution(scope)
    ? ['varsense', 'rust', 'frontend', 'docs', 'custom']
    : [...new Set([...scope.profiles].flatMap(profile => PROFILE_STAGE_RULES[profile] ?? []))];
  for (const name of stageNames) {
    const factory = STAGE_FACTORIES[name];
    if (factory) definitions.push(factory(context, scope, taskId));
  }
  return definitions;
}
