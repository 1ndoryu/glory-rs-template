const PROFILE_REMINDERS = {
  css: 'UI: revisa navegador y viewports; reutiliza tokens y componentes.',
  frontend: 'Async/UI: confirma teardown, estados vacíos y feedback visible.',
  auth: 'Auth: prueba visitante/usuario; ocultar UI no autoriza endpoints.',
  commerce: 'Comercio: precio, webhook y acceso a archivos son server-side.',
  workspace: 'Workspace: prueba release, overlay, papelera y conflictos 409.',
  mobile: 'Móvil: sin ventanas/barras; tablet conserva escritorio.',
  docs: 'Docs: actualiza checklist, dependencias y enlaces canónicos.',
  rust: 'Rust: evita unwrap externo y confirma errores/logs accionables.',
};

export function selectReminders(scope, stages, limit = 4) {
  const reminders = [];
  const failed = stages.some(stage => stage.status === 'fail' || stage.status === 'error');
  if (failed) reminders.push('Corrige los primeros hallazgos y repite exactamente el mismo comando.');
  for (const profile of scope.profiles) {
    if (PROFILE_REMINDERS[profile] && !reminders.includes(PROFILE_REMINDERS[profile])) {
      reminders.push(PROFILE_REMINDERS[profile]);
    }
  }
  if (!failed) {
    reminders.push('Cierre: staging explícito, commit/push y relectura completa del roadmap.');
  }
  return reminders.slice(0, limit);
}
