import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { analyzeWorkspace } from '../custom-rules.mjs';

test('custom rules produce findings estructurados sin Bash', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'quality-custom-'));
  try {
    await writeFile(path.join(root, 'fixture.ts'), 'export default function bad() {\n  document.createElement("div");\n  try {} catch {}\n}\n', 'utf8');
    const findings = await analyzeWorkspace(root);
    assert.ok(findings.some(item => item.ruleId === 'dom-access-outside-platform'));
    assert.ok(findings.some(item => item.ruleId === 'catch-vacio' && item.severity === 'error'));
    assert.equal(findings.every(item => item.file.endsWith('fixture.ts') && Number.isInteger(item.line)), true);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
