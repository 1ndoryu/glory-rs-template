# Pruebas de conversaciones con el agente — resumen en simple

> Fecha: 2026-10-07. Qué se probó hablando con el agente, qué salió bien y
> qué falta. Sin tecnicismos.

## 1. Cuestionario de ficha (/ask) — 27-sep

- **Qué es:** el agente hace preguntas de a una por vez para llenar la
  ficha de un inmueble (como un censo por chat).
- **Resultado:** funciona en local. **Falta:** que la usuaria lo pruebe
  con respuestas reales.

## 2. WhatsApp un solo número (octubre)

El agente atiende por el número principal (0412 0825234). Esto fue lo que
le hablamos y cómo respondió:

| Día | Prueba | Resultado |
|-----|--------|-----------|
| 6-oct | Hablarle normal ("hola, busco apartamento") | ✅ Atiende y abre conversación |
| 6-oct | Mandarle su propio eco (mensaje repetido por la red) | ✅ Lo ignora y dice por qué (`no:eco`) |
| 6-oct | Mandarle el mismo mensaje dos veces | ✅ El segundo lo ignora (`no:duplicado`) |
| 7-oct | Gente normal vs gente autorizada | ✅ Al autorizado lo trata distinto (rol `autorizado`) |
| 7-oct | Seguir hablando en el mismo chat | ✅ Reutiliza la conversación y avanza el turno |
| 6-oct | Cambiarle la configuración desde el panel admin | ✅ 16/16: todo se controla sin tocar código |
| 7-oct | Escribirle al número viejo (B) | ✅ Contesta bien pero no hace nada (`canal-jubilado`), sin guardar basura |
| 7-oct | Escribirle a un número que no existe | ✅ Rechaza con error 400 |
| 7-oct | Todo junto, 8 casos finales | ✅ 13/13 verde |

- **Detalle:** cada "no" del agente siempre trae su motivo escrito. Nada
  se queda callado sin explicar por qué.
- **Limpieza:** después de cada prueba se borraron los mensajes de
  prueba de la base de datos. No quedó basura.

## 3. Asistente de Marketplace (respuestas de avisos) — 6-oct

| Prueba | Resultado |
|--------|-----------|
| Pedir un borrador de respuesta para un aviso | ✅ Responde con texto, sin inventar precio |
| Pedir lo mismo dos veces | ✅ La 2ª sale de memoria (caché), sin gastar IA |
| Regenerar y corregir un borrador | ✅ Funciona; corregir marca el texto como corregido |
| Pedir sin permiso o demasiadas veces | ✅ Rechaza (401/422/429 según el caso) |
| Panel de uso (cuántas respuestas, copias, etc.) | ✅ Muestra conteos sin guardar datos privados |

- **Falta:** la prueba en vivo de 30 minutos contigo + decidir dónde
  vive el código del panel (el remoto) + la firma de otro frente para
  borrar la interfaz vieja (M1).

## Dónde está la evidencia

- Pruebas automáticas: `cargo test` (116/116) y panel (51/51).
- Guiones de prueba hablada: `C:\tmp\probar-f4.mjs`, `probar-f5.mjs`,
  `probar-f6.mjs` (viven fuera del repo, por convención).
- Actas por día: `Agente/completados/tareas-2026-10-06.md` y
  `tareas-2026-10-07.md`.
