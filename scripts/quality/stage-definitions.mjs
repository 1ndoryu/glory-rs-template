import { readFileSync, statSync } from 'node:fs';
import { runDocs } from './adapters/docs.mjs';
import { runFrontend } from './adapters/frontend.mjs';
import { runRust } from './adapters/rust.mjs';
import { runSentinel } from './adapters/sentinel.mjs';
import { runVarsense } from './adapters/varsense.mjs';
import { runCustom } from './adapters/custom.mjs';
import { adapterEnvironmentAllowlist, adapterStageNames, assertImplementedStages, assertStageParity, readAdapterManifest, resolveWorkspacePath, validateAdapterManifest } from './adapter-manifest.mjs';
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

function legacyStageNames(scope) {
  const selected = isFullExecution(scope)
    ? ['varsense', 'rust', 'frontend', 'docs', 'custom']
    : [...scope.profiles].flatMap(profile => PROFILE_STAGE_RULES[profile] ?? []);
  return [...new Set(['sentinel', ...selected])];
}

export function stageDefinitions(context, scope, taskId, adapter = undefined) {
  const effectiveAdapter = adapter ?? (context?.projectRoot ? loadAdapterManifestSync(context) : null);
  if (effectiveAdapter?.adapter?.environment) context.adapterEnvironmentAllowlist = adapterEnvironmentAllowlist(effectiveAdapter, DEFAULT_ENV_ALLOWLIST);
  const stageNames = effectiveAdapter ? adapterStageNames(effectiveAdapter, [...scope.profiles], isFullExecution(scope)) : legacyStageNames(scope);
  const definitions = stageNames.map(name => {
    const factory = STAGE_FACTORIES[name];
    if (!factory) throw new Error(`Etapa declarada por el adapter sin implementación: ${name}`);
    return factory(context, scope, taskId);
  });
  if (effectiveAdapter) {
    const implementedNames = definitions.map(item => item.name);
    assertImplementedStages(effectiveAdapter, stageNames, implementedNames);
    assertStageParity(stageNames, implementedNames);
  }
  return definitions;
}

function loadAdapterManifestSync(context) {
  try {
    const manifestPath = resolveWorkspacePath(context.projectRoot, 'quality-adapter.json', 'quality-adapter.json');
    if (!statSync(manifestPath).isFile()) throw new Error('quality-adapter.json no es un archivo regular');
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    validateAdapterManifest(manifest);
    const entrypoint = resolveWorkspacePath(context.projectRoot, manifest.transport.entrypoint, 'quality-adapter.json.transport.entrypoint');
    if (!statSync(entrypoint).isFile()) throw new Error('transport entrypoint no es un archivo regular');
    return manifest;
  } catch (error) {
    throw new Error(`Manifest de adapter inválido: ${error instanceof Error ? error.message : String(error)}`);
  }
}
