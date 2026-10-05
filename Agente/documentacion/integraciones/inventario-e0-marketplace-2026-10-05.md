# E0 — Inventario del fork `marketplace-*` (03AA-3)

> Fuente: `opencode-propio/src/packages/desktop/src/main/` (otro frente, no git).
> 9 ficheros, 1420 líneas (941 fuente + 479 tests). Solo lectura; nada se modificó.
> Freeze cifrado en `%MP_PRIVADO%` (`freeze-info.txt` con ruta+hash: solo en privado, nunca en repo).

## Fuente por fichero (path:líneas)

| Fichero | Líneas | Qué contiene |
|---|---|---|
| `marketplace-assistant.ts` | 180 | `buildRespuestaMP` (25-68); plantillas `DISPONIBLE`/`PRECIO`/`VISITA`; `TEXTO_APOYO` con contacto (46-56); helpers `teleSoloDigitos`, `waLink`, `linkContacto`, `stripHtml`, `aliasDeAviso`, `contieneDisponible/precio/visita`, `normalizar`; constantes públicas `TELEFONO`, `WHATSAPP`, `ALIAS_AVISO` (número público del negocio; migran a config en E1) |
| `marketplace-catalog.ts` | 31 | `CatalogoAviso`, `leerCatalogoDesdeRespuesta`, `buscarEnCatalogo` (fuzzy por alias+teléfono) |
| `marketplace-watch.ts` | 59 | `WatchHandle`, `observarListaMarketplace` (MutationObserver), `detener`/`desconectar` |
| `marketplace-float.ts` | 368 | `FLOAT_VERSION=3`; `ensureFloatForThread` (71-103); `renderizarBorrador` (105-155); `wireFloatEvents` (157-201); API `window.__mpFloat` (207-368): `mostrar`, `accion`, `setPortada`, `estado`, `domPin`, `leerExcerptDelHilo`; ShadowDOM **open** + `data-qa` tentativa con fallback a `aria-label`; ancla por `aria-label "Conversación con el título"` (frágil: M2 lo cubre con `selectores.json`) |
| `marketplace-service.ts` | 303 | Puerto por threadId + bootstrap (15-64); `crearVentanaRespuesta` (66-115); `onSolicitudRespuesta` (117-162); `onAccionFloat` (174-210, regenerar→`__mpNucleoDeCliente`); `onNucleoDeCliente` (212-267, handshake+hilo con await); `onPuertoCerrado` (269-280); `iniciarServicioRespuestas` (282-302); exportado en `index.ts` (281-284) + tipos (40-41); depende de `browser-service.readMarketplaceThreads` |
| `marketplace-assistant.test.ts` | 161 | Tests de plantillas y helpers |
| `marketplace-float.test.ts` | 197 | Tests del float con DOM simulado |
| `marketplace-service.test.ts` | 46 | Tests del servicio |
| `marketplace-watch.test.ts` | 75 | Tests del watcher |

## Dependencias

`service` → `assistant` + `float` + `watch` + `browser-service`;
`float` → `catalog` (inyectado por service vía `setCatalogo`).
Nada más del proyecto los importa salvo `index.ts` y `tipos.ts`.

## Hallazgos para E1/E2/M2

1. **`autoCopiarSiFoco` (`service.ts:164-172`) copia al clipboard del SO automáticamente:
   PROHIBIDO por la doctrina (cero clipboard automático, 03AA-3 §Doctrina y 03AA-5).
   Debe morir en E2/M2; el nuevo diseño usa botón "Copiar" manual.**
2. `assistant.ts` contiene en un comentario un ejemplo con nombre de compradora real:
   PII dentro del freeze cifrado; nunca citar ni copiar a repo/issues.
3. Anclaje del float por `aria-label` en español: frágil ante cambios de Meta;
   M2 lo cubre con `selectores.json` versionado + test semanal.
4. ShadowDOM en modo `open`: E1 decide si se cierra o se documenta como bridge.
5. Estado del borrador vive en memoria del renderer (`estadoPorHilo`): M2 lo formaliza
   (memoria + `MutationObserver` a cambio de ruta para la SPA).

## Freeze (E0.2, detalle técnico)

- Desviación registrada: el plan pedía `7z a -p -mhe=on`, pero no hay 7-Zip,
  openssl ni age en la máquina. Sustituto equivalente: `scripts/e0-freeze.ps1`
  (ZIP en memoria → AES-256-GCM con PBKDF2-SHA256 200k; el claro nunca toca disco,
  ni siquiera `C:\tmp`; `cipher /w` no aplica porque no hubo temporal).
- Contenedor `.mpfreeze` (magia `MPF1` + sal16 + nonce12 + cifrado + tag16);
  clave aleatoria en Credential Manager genérica `mp-privado-freeze`;
  `%MP_PRIVADO%=C:\Users\Owner\.mp-privado` (variable de usuario).
- Gotcha: `icacls` rechaza `$env:USERNAME` solo ("Parámetro no válido");
  usar `whoami` con dominio. ACL final: solo el usuario, sin herencia.

## Pendiente de E0

Corpus real: 3 hilos de Messenger (precio / disponibilidad / visita) que aporta
ella, + remoto del repo + fecha de viva 30min. Sin eso, E0 no cierra (plan §E0.1).
