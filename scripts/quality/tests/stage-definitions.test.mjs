import assert from 'node:assert/strict';
import test from 'node:test';
import { stageDefinitions } from '../stage-definitions.mjs';

const context = {};
const adapter = {
  stages: { sentinel: {}, varsense: {}, rust: {}, frontend: {}, docs: {}, custom: {} },
  profiles: { css: ['varsense'], frontend: ['varsense', 'frontend', 'custom'], rust: ['rust'], docs: ['docs'] },
};

test('perfil explícito restringe etapas aunque el alcance sea full', () => {
  const names = stageDefinitions(context, { full: true, executionFull: false, profileOverride: true, profiles: new Set(['docs']) }, '028A-6', adapter).map(stage => stage.name);
  assert.deepEqual(names, ['sentinel', 'docs']);
});

test('sin perfil explícito, full conserva todas las etapas', () => {
  const names = stageDefinitions(context, { full: true, executionFull: true, profileOverride: false, profiles: new Set() }, '028A-6', adapter).map(stage => stage.name);
  assert.deepEqual(names, ['sentinel', 'varsense', 'rust', 'frontend', 'docs', 'custom']);
});

test('un perfil frontend incluye varsense y custom, pero no rust/docs', () => {
  const names = stageDefinitions(context, { full: false, profileOverride: true, profiles: new Set(['frontend']) }, '028A-6', adapter).map(stage => stage.name);
  assert.deepEqual(names, ['sentinel', 'varsense', 'frontend', 'custom']);
});

test('el camino legacy sigue siendo compatible mientras migra', () => {
  const names = stageDefinitions(context, { full: false, profileOverride: true, profiles: new Set(['docs']) }, '028A-6').map(stage => stage.name);
  assert.deepEqual(names, ['sentinel', 'docs']);
});
