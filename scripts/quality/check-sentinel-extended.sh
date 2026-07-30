#!/bin/bash
# Sentinel Extended Checks — Reglas P0/P1 implementadas como scripts standalone
# [Plan mejora quality tool] Reglas que el CLI v0.4.0 aún no soporta
# Compatible con Windows Git Bash (sin -P, sin lookaheads)

echo "=== Sentinel Extended: P0/P1 Architecture Checks ==="
echo ""

FRONTEND_SRC="${1:-frontend/src}"
EXIT_CODE=0

# === P0: archivo-max-lineas (300 líneas) ===
echo "--- P0: Archivos >300 líneas ---"
OVERSIZED=$(find "$FRONTEND_SRC" -name '*.ts' ! -name '*.d.ts' ! -name '*.test.ts' \
  ! -path '*/node_modules/*' ! -path '*/api/generated/*' \
  -exec wc -l {} + 2>/dev/null | sort -rn | awk '$1 > 300 && !/total/ {print}')

if [ -n "$OVERSIZED" ]; then
  echo "$OVERSIZED"
  echo ""
  echo "⚠️  Archivos exceden límite de 300 líneas. Dividir por responsabilidad."
  EXIT_CODE=1
else
  echo "✅ Todos los archivos están bajo 300 líneas"
fi
echo ""

# === P1: any-type-prohibido ===
echo "--- P1: Uso de 'any' prohibido ---"
ANY_USAGE=$(grep -rn 'as any\b\|@ts-ignore\|@ts-expect-error\|: any\b' "$FRONTEND_SRC" \
  --include="*.ts" --exclude="*.test.ts" --exclude="*.d.ts" \
  | grep -v 'node_modules' | grep -v 'Promise\.any\|Array\.any\|\.any(' \
  | head -20)

if [ -n "$ANY_USAGE" ]; then
  echo "$ANY_USAGE"
  echo ""
  echo "⚠️  Uso de 'any' detectado. Tipar correctamente."
  EXIT_CODE=1
else
  echo "✅ Sin uso de 'any' ni @ts-ignore"
fi
echo ""

# === P1: export-default-prohibido ===
echo "--- P1: Default exports prohibidos ---"
DEFAULT_EXPORTS=$(grep -rn '^export default ' "$FRONTEND_SRC" \
  --include="*.ts" --exclude="*.test.ts" --exclude="*.d.ts" \
  | grep -v 'node_modules' | head -10)

if [ -n "$DEFAULT_EXPORTS" ]; then
  echo "$DEFAULT_EXPORTS"
  echo ""
  echo "⚠️  Default exports detectados. Usar named exports."
  EXIT_CODE=1
else
  echo "✅ Sin default exports"
fi
echo ""

# === P1: console-log-produccion ===
echo "--- P1: Console en producción ---"
CONSOLE_USAGE=$(grep -rn 'console\.\(log\|error\|warn\|debug\)(' "$FRONTEND_SRC" \
  --include="*.ts" --exclude="*.test.ts" --exclude="*.d.ts" \
  | grep -v 'node_modules' | grep -v 'scripts/' | grep -v '\.mjs' \
  | grep -v 'vite\.config' | grep -v 'safe-async\.ts' \
  | grep -v 'command-registry\.ts' \
  | head -15)

if [ -n "$CONSOLE_USAGE" ]; then
  echo "$CONSOLE_USAGE"
  echo ""
  echo "⚠️  Console en código de producción. Usar showToast o servicio de logging."
else
  echo "✅ Sin console en producción"
fi
echo ""

# === P1: subscribe-sin-cleanup ===
echo "--- P1: Subscribe sin cleanup ---"
# Buscar .subscribe( que NO esté precedido por asignación (const x = ... o let x = ...)
# Excluir shell-level (viven toda la sesión) y stores globales
SUBSCRIBE_NO_CLEANUP=$(grep -rn '\.subscribe(' "$FRONTEND_SRC" \
  --include="*.ts" --exclude="*.test.ts" --exclude="*.d.ts" \
  | grep -v 'node_modules' \
  | grep -v 'stores\.ts\|store\.ts\|main\.ts\|desktop-shell\.ts\|reactive-taskbar\.ts\|workspace-icon-grid\.ts' \
  | grep -v 'const \|let \|const unsubscribe\|this\.\w\+ =' \
  | head -15)

if [ -n "$SUBSCRIBE_NO_CLEANUP" ]; then
  echo "$SUBSCRIBE_NO_CLEANUP"
  echo ""
  echo "ℹ️  Subscribe sin cleanup en componentes. Shell-level (desktop-shell, reactive-taskbar, workspace-icon-grid) ya excluidos — viven toda la sesión."
else
  echo "✅ Todos los subscribes tienen cleanup asignado"
fi
echo ""

# === P1: api-call-en-logica ===
echo "--- P1: API calls fuera de services ---"
# Excluir comentarios JSDoc y líneas de ejemplo
API_CALLS=$(grep -rn 'api\.\(get\|post\|put\|delete\)(' "$FRONTEND_SRC" \
  --include="*.ts" --exclude="*.test.ts" --exclude="*.d.ts" \
  | grep -v 'node_modules' \
  | grep -v 'api/client\.ts\|services/' \
  | grep -v '^\s*[*]' \
  | grep -v '\/\/' \
  | grep -v 'safe-async\.ts' \
  | head -15)

if [ -n "$API_CALLS" ]; then
  echo "$API_CALLS"
  echo ""
  echo "⚠️  API calls directas fuera de services. Usar service layer."
  EXIT_CODE=1
else
  echo "✅ API calls correctamente delegadas a services"
fi
echo ""

# === P1: import-store-directo ===
echo "--- P1: Imports directos de stores desde lógica ---"
STORE_IMPORTS=$(grep -rn "import.*from.*['\"].*stores\?\.ts['\"]" "$FRONTEND_SRC" \
  --include="*.ts" --exclude="*.test.ts" --exclude="*.d.ts" \
  | grep -v 'node_modules' \
  | grep -v 'stores\.ts\|store\.ts\|index\.ts\|workspace-store\.ts' \
  | head -15)

if [ -n "$STORE_IMPORTS" ]; then
  echo "$STORE_IMPORTS"
  echo ""
  echo "⚠️  Imports directos de stores desde módulos de lógica."
else
  echo "✅ Stores correctamente encapsulados"
fi
echo ""

# === INFO: store-mutation-in-view ===
echo "--- INFO: Store mutations en vistas ---"
# En vanilla TS sin framework, showProfile/authStore.set() en pages es el patrón esperado.
# No hay lifecycle de componente para delegar. Registrar como INFO, no error.
STORE_MUTATIONS=$(grep -rn '\w\+Store\.\(set\|update\)(' "$FRONTEND_SRC/pages/" \
  --include="*.ts" --exclude="*.test.ts" 2>/dev/null \
  | grep -v 'node_modules' | head -10)

STORE_MUTATIONS2=$(grep -rn '\w\+\.\(set\|update\)(' "$FRONTEND_SRC/pages/" \
  --include="*.ts" --exclude="*.test.ts" 2>/dev/null \
  | grep -v 'node_modules' | grep -E '(showProfile|authStore|fontStore|siteConfig|showSidebar)\.' \
  | head -10)

if [ -n "$STORE_MUTATIONS" ] || [ -n "$STORE_MUTATIONS2" ]; then
  echo "${STORE_MUTATIONS}${STORE_MUTATIONS2}"
  echo ""
  echo "ℹ️  Store mutations en vistas (vanilla TS — patrón legítimo sin framework)."
else
  echo "✅ Sin store mutations directas en vistas"
fi
echo ""

if [ $EXIT_CODE -eq 0 ]; then
  echo "✅ Todas las verificaciones P0/P1 pasaron"
else
  echo "⚠️  Se encontraron violaciones. Revisar hallazgos arriba."
fi

exit $EXIT_CODE
