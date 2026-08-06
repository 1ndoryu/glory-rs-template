import { readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { runDocs } from './adapters/docs.mjs';
import { runFrontend } from './adapters/frontend.mjs';
import { runRust } from './adapters/rust.mjs';
import { runSentinel } from './adapters/sentinel.mjs';
import { runVarsense } from './adapters/varsense.mjs';
import { runCustom } from './adapters/custom.mjs';
import { adapterEnvironmentAllowlist, validateAdapterManifest } from './adapter-manifest.mjs';
import { DEFAULT_ENV_ALLOWLIST } from './runner.mjs';
import { isFullExecution, PROFILE_STAGE_RULES } from './profile-contract.mjs';

const STAGE_FACTORIES = {
  sentinel: (context, scope) => ({ name: 'sentinel', run: () => runSentinel(context, scope) }),
  varsense: (context, scope) => ({ name: 'varsense', run: () => runVarsense(context, scope) }),
  rust: (context, scope) => ({ name: 'rust', run: () => runRust(context, scope) }),
  frontend: context => ({ name: 'frontend', run: () => runFrontend(context) }),
  docs: (context, _scope, taskId) => ({ name: 'docs', run: () => runDocs(context, taskId) }),
  custom: (context, scope) => ({ name: 'custom', run: () => runCustom({ ...context, scope }) }),
};

function loadAdapterManifestSync(context) {
  if (!context?.projectRoot) return null;
  const manifestPath = path.join(context.projectRoot, 'quality-adapter.json');
  try {
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    validateAdapterManifest(manifest);
    const entrypoint = path.resolve(context.projectRoot, manifest.transport.entrypoint);
    if (!statSync(entrypoint).isFile()) throw new Error('transport entrypoint no es un archivo regular');
    return manifest;
  } catch (error) {
    throw new Error(`Manifest de adapter inválido: ${error instanceof Error ? error.message : String(error)}`);
  }
}

export function stageDefinitions(context, scope, taskId, adapter = undefined) {
  const effectiveAdapter = adapter ?? loadAdapterManifestSync(context);
  if (effectiveAdapter?.adapter?.environment) {
    context.adapterEnvironmentAllowlist = adapterEnvironmentAllowlist(effectiveAdapter, DEFAULT_ENV_ALLOWLIST);
  }
  const selected = effectiveAdapter
    ? (isFullExecution(scope)
      ? Object.keys(effectiveAdapter.stages)
      : [...scope.profiles].flatMap(profile => {
        if (!Object.hasOwn(effectiveAdapter.profiles, profile)) throw new Error(`Perfil de adapter desconocido: ${profile}`);
        return effectiveAdapter.profiles[profile];
      }))
    : (isFullExecution(scope)
      ? ['varsense', 'rust', 'frontend', 'docs', 'custom']
      : [...scope.profiles].flatMap(profile => PROFILE_STAGE_RULES[profile] ?? []));
  const stageNames = [...new Set(['sentinel', ...selected])];
  return stageNames.map(name => {
    const factory = STAGE_FACTORIES[name];
    if (!factory) throw new Error(`Etapa declarada por el adapter sin implementación: ${name}`);
    return factory(context, scope, taskId);
  });
}
