export async function runBoundedStages(definitions, runStage, options = {}) {
  const concurrency = Math.max(1, Math.min(options.maxConcurrency ?? 1, definitions.length || 1));
  const results = new Array(definitions.length);
  let cursor = 0;

  async function worker() {
    while (true) {
      const index = cursor;
      cursor += 1;
      if (index >= definitions.length) return;
      if (options.isCancelled?.()) throw new Error('quality gate cancelado durante las etapas');
      results[index] = await runStage(definitions[index]);
    }
  }

  await Promise.all(Array.from({ length: concurrency }, () => worker()));
  return results;
}
