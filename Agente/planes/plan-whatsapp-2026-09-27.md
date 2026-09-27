# Plan: Agente MN pulido — dos modos, delegación, clientes, observabilidad (279A-2) — 2026-09-27

> Revisión mayor 2026-09-27 (~18:30): ya no es "solo WhatsApp". Cubre
> personalidad, 2 números × 2 modos, delegación con congelamiento, gestión
> de clientes, ventana de contexto, logs de tokens y consola del dueño.
> Decisiones de infra previas (Baileys, PC usuaria, `opencode serve`) se
> mantienen. `[DECIDIR]` = pregunta abierta a la usuaria al final del plan.

## 0. Visión y piezas

- **Parte agentes**: la IA atiende (web + 2 números WhatsApp), los agentes
  humanos toman hilos, la IA delega/consulta. Todo queda registrado.
- **Parte dueña**: la usuaria (o yo en su nombre) ve conversaciones, envía
  mensajes manuales, audita qué hacen agentes humanos e IA, y ve uso
  (mensajes, tokens). Endpoints documentados para operarlos.
- Dos piezas separadas por rol, misma BD. Nada de lógica de dueña dentro
  del loop de la IA y viceversa (SRP).

## 1. Personalidad y reglas de conversación (prompt)

- Cordial, amable, **breve** (WhatsApp: 1 idea por mensaje, sin muros).
- Siempre se declara: «Soy el asistente IA de MN Inmobiliaria» (decidido
  2026-09-27: sin nombre de persona, igual en ambos números y en web).
- Nunca inventa: precio/dirección/foto solo de `buscar/detalle`; lo que
  no sabe → `consultar_agente` (§5), no improvisación.
- `No lo sé` de /ask también informa: «aún no tenemos ese dato» en vez
  de callar o inventar.

## 2. Dos números × dos modos (dos instancias, mismo binario)

- **Completo (número A)**: atiende todo; **nunca delega dentro de la
  conversación**. Si hay que escalar, avisa al número del agente humano
  (el del modo inicial, decidido 2026-09-27) con la **ficha comercial**
  del cliente y sigue disponible.
- **Inicial (número B, el que usa un agente humano)**: solo preguntas
  iniciales; **casi siempre delega** (el humano sigue en el mismo chat).
- Implementación: dos `AgentState` (dos `PromptConfig`, tools y
  `prompt_extra` distintos) anidados como `Router<()>` bajo prefijos
  distintos (`/agente/completo`, `/agente/inicial`) — el núcleo solo deja
  UN `PromptConfig` por instancia y `process_incoming`/`rest_send` son
  privados, así que el webhook reparte por HTTP loopback interno, no por
  llamada directa. Comparten pool; hub compartido (fanout por sesión).
- Sesión = cliente × canal. Web + WhatsApp del mismo cliente se enlazan
  por `clientes` (§4), no mezclando hilos.

## 3. Delegación con congelamiento (máquina de estados)

Estados por sesión (tabla propia `atencion_sesiones`, §4; el núcleo no
tiene campo): `activa` → `consultando` → (`activa` | `delegada`).

- **Consultar** (duda puntual): tool `consultar_agente` → outbox al número
  humano con contexto + `ai_enabled=false` (el gate del núcleo calla) +
  ciclo `waiting`. Al responder el humano: acción staff «devolver a IA
  con nota» → `ai_enabled=true`, nota como mensaje `staff`, ciclo
  `answered`, la IA retoma con contexto.
- **Delegar**: `escalar_a_humano` (existe) → `status=escalated` + ciclo
  `escalated` + `ai_enabled=false`: triple freno verificado del núcleo.
  La IA **no vuelve sola**; solo un humano la re-activa.
- Regla de oro: `waiting`=la IA sigue pudiendo hablar (así es el núcleo:
  solo `escalated` calla); el silencio en `consultando` lo da
  `ai_enabled=false` + la fila propia. No confundir con toma humana:
  el modo queda en `atencion_sesiones.modo`.
- Completo avisa al otro número con ficha del cliente (nombre, teléfono,
  resumen, último interés) vía outbox `kind='whatsapp'` con `destino`
  explícito (hoy el worker solo sabe `whatsapp_admin` fijo: ampliar).

## 4. Datos (todo en este repo; el núcleo no se toca — regla 17)

- `clientes(id, nombre, telefono UNIQUE, origen, interes, presupuesto,
  zona, notas, created_at)` (ficha comercial decidida 2026-09-27: la de
  delegación lleva nombre+teléfono+resumen+interés+presupuesto+zona).
- `canal_sesiones(session_id PK→agent_sessions, cliente_id→clientes,
  canal: web|wa_a|wa_b, telefono, modo: completo|inicial)`.
- `atencion_sesiones(session_id PK, estado: activa|consultando|delegada,
  modo, asignado_a, updated_at)`.
- `uso_mensajes(id, session_id, remitente, modelo, tokens_est,
  tokens_in, tokens_out, created_at)`: conteo sale de `agent_messages`;
  tokens fase 1 estimados (`len/4`, fórmula del núcleo), fase 2 exactos
  con pista núcleo (§7).
- `registrar_contacto` además crea/actualiza `clientes` (hoy solo toca
  la sesión: no perder clientes = entidad propia).

## 5. Conversación natural (capacidades)

- Enviar inmuebles: texto + **fotos con pie** (outbox payload con
  `media_url`; gateway Baileys las manda; pie = título+precio+slug).
- Descripción y preguntas: con `detalle_inmueble` (ya trae `extras` y
  margen — 279A-8) + `buscar_inmuebles`.
- Recibir fotos del cliente: se guardan (storage `[DECIDIR]`: disco PC
  vs volumen) pero **no se describen** (decidido 2026-09-27: WhatsApp es
  solo-enviar; las fotos quedan para la web).
- Audios → Whisper local → texto (plan original F4, se mantiene).
- Desconocimiento → `consultar_agente` primero (§3). Jamás rellenar.

## 6. Ventana de contexto

- Hoy el núcleo arma el LLM con **solo el mensaje actual** + tope 30k
  (`transport` + `MAX_CONTEXT_TOKENS` fijos): no hay memoria multi-turno
  real. «Subir la ventana» = pista núcleo (§7): historial real con tope
  configurable + resumen. Sin eso, «fluida» es solo apariencia.
- Mientras tanto: nada de parches (inyectar historial por `prompt_extra`
  global es inseguro y mezcla hilos).

## 7. Pista núcleo (repo `glory-agent`, otro release)

Requiere versión nueva (publicar + bump de `rev`, flujo regla 17):

1. Historial real al LLM (últimos N mensajes + resumen, tope
   configurable; hoy solo el actual).
2. `usage` del provider por turno (`input/output_tokens`; hoy se
   descarta) → alimenta `uso_mensajes` exacto.
3. Ventana configurable (`MAX_CONTEXT_TOKENS` hoy const 30k).
4. Opcional tarde: passthrough de media/canal en mensajes y outbox
   (hoy todo es `body` texto; lo suplimos con payload propio).

Sin 1+2 el plan llega hasta: estima de tokens + memoria de un turno.

## 8. Consola de la dueña (admin + operable por mí)

- Existe: bandeja, hilo, responder (toma el hilo), tomar/soltar IA,
  cerrar, config (`VistaMensajes`, rutas staff).
- Nuevo: **clientes** (CRUD + ver sus sesiones), **iniciar/enviar
  mensaje** (nueva sesión o existente → outbox; «dime y lo envío»),
  **auditoría** (quién tomó cada hilo, tiempos, mensajes IA vs humano),
  **uso** (mensajes y tokens por día/sesión/cliente).
- Todo vía endpoints admin documentados en el propio plan al
  implementar, para que yo pueda operarlos por terminal/HTTP.

## 9. Fases (cada una usable sola, con su verificación)

- **F0 Pista núcleo**: release glory-agent (historial+usage+ventana) y
  bump aquí. Sin esto, F4 es parcial. Verificar: turno con memoria de
  3 mensajes + `usage` persistido.
- **F1 Tablas propias + `registrar_contacto`→clientes**: `clientes`,
  `canal_sesiones`, `atencion_sesiones`, `uso_mensajes` (estima).
  Verificar: contacto guarda cliente sin duplicar por teléfono.
- **F2 Webhook + reparto por número**: Baileys 2 sesiones (QR ×2),
  webhook con `numero_destino` → modo → instancia; worker con
  `destino`+`media`. Verificar: mensaje a cada número llega a su modo.
- **F3 Delegación real**: `consultar_agente`, aviso con ficha al otro
  número, congelar/retomar, toma en mismo chat (inicial). Verificar:
  matriz (consulta→retoma, delega→calla, completo→avisa+ficha).
- **F4 Memoria + exactitud**: historial del núcleo + tokens exactos +
  fotos entrantes/salientes + audios. Verificar: conversación de 10
  turnos coherente, fotos con pie, audio transcrito.
- **F5 Consola dueña**: clientes, envío manual, auditoría, uso.
  Verificar: enviar desde panel, ver tokens del día, auditar toma.

## 10. SOLID / escala / huecos (revisión 2026-09-27)

- Hoy bien: handlers por dominio, structs `Tarjeta`/`Ficha` en vez de
  tuplas, una query por bandeja (sin N+1), errores con contexto.
- Riesgos al crecer: `chat_tools.rs` será god-file → un módulo por tool
  al añadir `consultar_agente`/`enviar_ficha`; webhook, workers y rutas
  en módulos propios (`canal/`, `delegacion/`, `clientes/`,
  `observabilidad/`), nunca todo en `chat.rs`.
- Escala: hub realtime en memoria (al reiniciar se re-suscribe; las
  sesiones persisten: degradado aceptado); outbox con un worker por
  `kind`; gateway en PC = cuello documentado (requiere PC encendida);
  coste LLM con tope por sesión/día (config) + alerta.
- Huecos que este plan cierra: memoria multi-turno (F0), tokens (F1/F4),
  clientes (F1), media (F2/F4), dos modos (F2), auditoría (F5).

## Estado

- **Número A (completo, pruebas): 0412 0825234** → `584120825234`
  (registrado 2026-09-27).
- **Falta**: número B (inicial, el del agente humano) + QR de ambos
  (2 min por número) + storage de fotos (disco PC vs volumen).
- Nada implementado de este plan; sin código hasta F0/F1.

## Gate / DoD por fase

- Rust: `cargo fmt --check && cargo check && cargo clippy -- -D
  warnings && cargo test` (en rama `inmobiliaria`, BD de rama).
- Front: `npx tsc --noEmit` (+ 2 resoluciones si hay UI).
- Funcional real en cada fase (conversación/mensaje de verdad).
- Deploy vía coolify-manager-rs si toca prod.

## Riesgos

- Baileys no oficial: solo 1:1, nada masivo (restricción).
- Gateway en PC: requiere PC encendida; documentar arranque.
- Núcleo externo: F0 depende de otro repo (publicar+probar+bump).
- Coste LLM: medir desde F1 (estima), tope desde F4 (exacto).
