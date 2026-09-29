# Fase 3 — Batería completa F1–F10 con E-fluido (2026-09-29, ~17:00–18:10 VET)

Batería sintética por `POST /api/agent/whatsapp/webhook` (vía `wa_b`, remitentes
`18149575461–69`, destino `18149575416`). Una sesión virgen por escenario; segunda
vuelta desde el número re-claveado cuando `registrar_contacto` movía el hilo
(hallazgo H1). Sin cambios de código: backend `b2876bd5` (E-fluido).
65 filas outbox + 29 mensajes + 11 sesiones + 11 atenciones + 11 clientes
sintéticos (más 9 clientes huérfanos "Test F*" de Fase 1) borrados al cierre;
`solicitudes`/`visitas` en 0 (la batería no creó negocio real).

## Resultado por escenario

- **F1 búsqueda simple** (apartamento 2 hab, venta; sesión `6d648f45`): PARCIAL.
  4 tarjetas + acuse (turno >10 s) + intro/cierre, estado `activa`, cero `**`.
  Pero solo 1/4 es 2 hab (Alta Vista): Calas Suites, Caroní Plaza y Vista Hermosa
  son 3 hab. El filtro de habitaciones no se respetó.
- **F2 filtros** (casa 3 hab norte ≤90k; sesión `3a0242a5`): PARCIAL. Acuse + 2
  partes, `activa`, sin tarjetas (dijo no tener). Pero Guayana Country Club
  (3 hab, $85k, venta, publicada) encaja y no se ofreció; además afirmó "ya
  quedaste registrada" con la ficha vacía (ver H2). Pregunta de seguimiento
  (venta/alquiler) bien.
- **F3 detalle 2º turno** (puestos Alta Vista; sesión `1e0b2ba2`): OK. Con historial
  resolvió "el de Alta Vista", llamó a detalle y respondió "1 puesto, 87 m²"
  (exacto en BD) + re-envió la tarjeta como contexto + cierre. Intro/cierre
  partidas, `activa`.
- **F4 ficha cliente** (Carlos Méndez, 2 hab, $50k; sesión `c5cc8136`): PARCIAL.
  5 tarjetas + intro/cierre, `activa`. Ignoró "2 habitaciones" (3/5 son 3 hab,
  1 es alquiler $300 sin pedirse) y NO llamó a `registrar_contacto` (ficha vacía,
  ver H2). Igual que Fase 1.
- **F5 fuera de catálogo** (castillo con foso y dragón; sesión `8cad90b8`): NO
  según criterio (no llamó a `consultar_agente`, quedó `activa`), pero con matiz
  positivo: no congeló la charla por un chiste, respondió con humor y pidió
  venta/alquiler/zona/presupuesto para reconducir. Afirmó "ya guardé tu contacto"
  con ficha vacía (H2). Esta vez SÍ llamó a `registrar_contacto` (solo teléfono,
  sin nombre) → re-claveó el hilo (H1).
- **F6 staff→retome** (nota staff simulada por SQL —endpoints ya verificados en
  Fase 1— + `answered`, sesión `8cad90b8`): OK. La IA retomó y usó la nota ("¿te
  cuento más de la del Country Club?") + 3 tarjetas de casas. Sesión extra
  `e923da4b` (turno lanzado con SQL fallido por ñ en psql): respondió bien igual.
- **F7 escalar a humano** (queja + "persona real ya"; sesión `5bd82797`): OK.
  `escalar_a_humano` → `delegada` + `ai_enabled=false`, avisó a persona real y dio
  el +584249208855. IA calla.
- **F8 datos contacto** (teléfono + dirección; sesión `4034f02d`): PARCIAL (igual
  que Fase 1). Teléfono +584249208855 OK vía `datos_contacto`; "la dirección
  exacta no la tengo a mano" — sigue sin estar en el conocimiento. Ofreció que un
  agente la confirme. Acuse cubrió el turno lento.
- **F9 cambio de tema** (apartamento alquiler → casa 3 hab venta; sesión
  `2aef3562`): OK en pivote (misma sesión, sin mezclar). Mismo matiz de filtro:
  "3 casas" pero Riberas es 2 hab e Icabarú 4 hab; solo Orquídea es 3 hab.
- **F10 saludo "hola"** (sesión `8fa703e2`, ~18:05 VET): OK. "Hola, buenas
  tardes" (período correcto E17), texto plano, intro + cierre partidas. Acuse
  previo por latencia del modelo.

## Transversales (abren trabajo nuevo)

- **H1 — `registrar_contacto` parte el hilo.** Al guardar, `canal_sesiones.telefono`
  se re-clavea al número dado en el texto (ej. `18149575463` → `34633333333`):
  el siguiente mensaje desde el número original abre sesión nueva sin historial
  (F3: `66384d4b`; F5/F6). En producción el remitente YA es el número real, así
  que el caso normal es idempotente; el filo real es dar un número distinto
  (el del cónyuge): la charla se bifurca. Propuesta: no re-clavear el hilo al
  registrar; guardar el número solo en `clientes`.
- **H2 — la IA afirma registrar sin registrar.** Patrón en F2/F4/F5: "ya quedaste
  registrada / ya guardé tu contacto" con `nombre/interes/presupuesto` vacíos.
  El prompt ordena llamar a `registrar_contacto` EN ESTE MISMO TURNO pero el
  modelo a veces lo omite y declara éxito. Propuesta: regla de prompt ("si dices
  que guardaste, la tool tuvo que responder éxito en este turno") o chequeo
  post-turno que corrija ("veo que no quedó guardado, ¿me confirmas...?").
- **H3 — filtro `habitaciones` decorativo.** F1/F4/F9: `buscar` trae de más y el
  modelo presenta sin filtrar (3 hab cuando piden 2). Propuesta: `buscar` con
  `habitaciones` exacto por defecto en SQL (no confiar al modelo) o post-filtro
  en `encolar_tarjetas`.
- **H4 — geografía "norte" sin filtro.** `buscar` no tiene filtro de zona; F2
  falló por no poder resolver "norte" (Country Club encajaba y dijo que no).
  Propuesta: filtro de texto en `ubicacion` o pedir zona antes de buscar.
- **H5 — dirección de oficina ausente.** F8 repite Fase 1: el conocimiento no
  trae dirección exacta. Propuesta: agregarla al prompt o a `datos_contacto`.
- **H6 — orden acuse tras tarjetas.** Si las tools son rápidas pero la redacción
  lenta, el acuse (10 s) llega DESPUÉS de las tarjetas (F1: 4 tarjetas → acuse →
  intro). Aceptable, pero el orden ideal sería acuse primero. Propuesta: encolar
  acuse también si hay tarjetas sin cierre a los ~6 s, o aceptar el orden actual.
- **H7 — ` figure psql + ñ.** `psql` por PowerShell falla con UTF8 si el SQL lleva
  eñes (`dueña` → `secuencia de bytes no válida`); escribir SQL ASCII (F6: nota
  con "Orquidea/mas/mansion"). Solo entorno de pruebas, no código.
- **H8 — Fase 1 dejó 9 clientes "Test F*" huérfanos** (`18149575451–59`): la
  limpieza de Fase 1 borró sesiones pero no `clientes`. Borrados ahora.

## Formato E-fluido (veredicto global)

En los 10 escenarios: cero `**`/tablas/encabezados, tarjetas ≤110 chars con
precio en miles, intro + cierre partidas siempre, acuse solo en turnos >10 s
(F1,F2,F5,F6,F8,F10), ningún silencio, ningún turno colgado. El envoltorio
funciona; lo que falla es criterio del modelo (filtros, ficha, zona), no el
formato.
