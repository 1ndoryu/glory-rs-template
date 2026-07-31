# Plan — modo claro/oscuro del OS

> **Fecha:** 2026-07-31  
> **Estado:** pendiente de concepto visual y autorización  
> **Alcance:** chrome y superficies del OS; la navegación exterior queda fuera del tema.

## Objetivo y límites

Añadir un modo claro/oscuro coherente con la identidad Macintosh minimalista, usando los mismos componentes, iconos Lucide de 1px y tipografía JetBrains Mono. El contenido multimedia puede conservar color; ventanas, menús, taskbar, launcher y estados del OS consumen tokens semánticos. No usar `filter: invert()`, colores por componente, sombras nuevas ni un `MobileFooApp`.

## Fases y checklist

### 1. Contrato de tema

- [ ] Definir estados `light`, `dark` y resolución inicial `system`, con override explícito del usuario.
- [ ] Definir qué pertenece al OS y qué permanece sin cambios fuera del shell (incluida la navegación exterior).
- [ ] Mapear roles semánticos: fondo, texto, borde, foco, selección, ventana activa/inactiva, menú, taskbar, estados y feedback.
- [ ] Registrar decisión y compatibilidad con el manual visual; no actualizarlo como aprobado hasta completar la revisión visual.

**Gate:** contrato revisado; no quedan colores hexadecimales ni reglas de tema implícitas en componentes.

### 2. Prototipo visual sin lógica

- [ ] Crear una muestra clara/oscura con el escritorio, una ventana, menú contextual, taskbar, launcher móvil y un programa de texto.
- [ ] Mantener el lenguaje 1-bit: inversión de roles y tramas permitidas, sin grises decorativos, gradientes ni sombras.
- [ ] Mostrar el botón único claro/oscuro con etiqueta, foco y estado activo; no crear toggles por aplicación.
- [ ] Presentar capturas comparables en desktop, tablet y móvil para aprobación explícita del usuario.

**Gate:** aprobación visual registrada; si se rechaza, corregir tokens/prototipo antes de implementar persistencia.

### 3. Tokens e integración

- [ ] Centralizar tokens en `variables.css` y hacer que componentes base/Lucide consuman `currentColor`.
- [ ] Aplicar el tema mediante atributo/clase raíz del shell; ventanas y apps reciben contexto, no estilos duplicados.
- [ ] Añadir feature flag y fallback claro para rollback; el tema no debe romper hydration ni primera pintura.
- [ ] Cubrir CSS con VarSense y Sentinel; corregir referencias huérfanas y especificaciones visuales locales.

**Gate:** type-check, VarSense, Sentinel y pruebas de render pasan para ambas variantes.

### 4. Preferencias y sincronización

- [ ] Guardar la elección anónima en el overlay local sin modificar el release público ni `mobileOrder`.
- [ ] Sincronizar la preferencia de cuenta con el overlay remoto usando revisión, merge y 409; nunca sobrescribir silenciosamente.
- [ ] Resolver login/logout, otro dispositivo y reset a `system`; evitar flash de tema en primera pintura.
- [ ] Emitir `theme_changed` con modo, ámbito y `presentationMode`, sin contenido ni identificadores sensibles.

**Gate:** pruebas local/remoto, dos pestañas/dispositivos, pérdida de red y logout pasan con rollback visible.

### 5. Accesibilidad y validación

- [ ] Verificar contraste AA, foco de teclado, forced-colors, reduced motion, zoom 200% y lectura de estado del botón.
- [ ] Probar 1440x900, 1024x768, 768px, 390px, 360px y 320px; incluir orientación, safe area y teclado móvil.
- [ ] Ejecutar E2E de cambio de tema en escritorio/tablet/móvil y confirmar que la URL/app/recurso se conserva.
- [ ] Medir rendimiento de primera pintura y cambio de tema; documentar cualquier presupuesto excedido.

**Gate:** evidencia visual/E2E y quality gate completo; no se marca como terminado con warnings bloqueantes.

### 6. Cierre documental

- [ ] Actualizar manual visual, arquitectura e índice solo después de la aprobación y la implementación.
- [ ] Registrar una entrada en completados con archivos, gotchas, Sentinel/VarSense y GLORY.
- [ ] Confirmar revisión SOLID/escalabilidad: un tercer tema de prueba no exige reescribir componentes ni comandos.

## Definition of Done

- [ ] Un único botón global cambia el tema y es accesible.
- [ ] Desktop, tablet y móvil comparten tokens, comandos, permisos y analítica.
- [ ] Preferencia local/remota tiene ámbito, merge, reset y rollback explícitos.
- [ ] Manual visual, roadmap, pruebas y quality gate están sincronizados.
