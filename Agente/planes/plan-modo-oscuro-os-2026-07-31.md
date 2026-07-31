# Plan — modo claro/oscuro del OS

> **Fecha:** 2026-07-31  
> **Estado:** fases 1-3 implementadas y aprobadas visualmente en navegador; fase 4 (remoto) bloqueada por 297A-13; fase 5 (matriz completa) pendiente; fase 6 en curso.  
> **Alcance:** chrome y superficies del OS; la navegación exterior queda fuera del tema.

## Objetivo y límites

Añadir un modo claro/oscuro coherente con la identidad Macintosh minimalista, usando los mismos componentes, iconos Lucide de 1px y tipografía JetBrains Mono. El contenido multimedia puede conservar color; ventanas, menús, taskbar, launcher y estados del OS consumen tokens semánticos. No usar `filter: invert()`, colores por componente, sombras nuevas ni un `MobileFooApp`.

## Fases y checklist

### 1. Contrato de tema

- [x] Definir estados `light`, `dark` y resolución inicial `system`, con override explícito del usuario. *(ThemeMode system/claro/oscuro)*
- [x] Definir qué pertenece al OS y qué permanece sin cambios fuera del shell (incluida la navegación exterior). *(override scoped en .desktop-window/.movilApp/.movilLauncher; navegación legacy queda clara)*
- [x] Mapear roles semánticos: fondo, texto, borde, foco, selección, ventana activa/inactiva, menú, taskbar, estados y feedback. *(tokens --sistema-* en variables.css)*
- [ ] Registrar decisión y compatibilidad con el manual visual; no actualizarlo como aprobado hasta completar la revisión visual.

**Gate:** contrato revisado; no quedan colores hexadecimales ni reglas de tema implícitas en componentes.

### 2. Prototipo visual sin lógica

- [x] Crear una muestra clara/oscura con el escritorio, una ventana, menú contextual, taskbar, launcher móvil y un programa de texto. *(implementado y verificado en navegador)*
- [x] Mantener el lenguaje 1-bit: inversión de roles y tramas permitidas, sin grises decorativos, gradientes ni sombras.
- [x] Mostrar el botón único claro/oscuro con etiqueta, foco y estado activo; no crear toggles por aplicación.
- [ ] Presentar capturas comparables en desktop, tablet y móvil para aprobación explícita del usuario.

**Gate:** aprobación visual registrada; si se rechaza, corregir tokens/prototipo antes de implementar persistencia. *(aprobación informal en navegador; capturas formales pendientes)*

### 3. Tokens e integración

- [x] Centralizar tokens en `variables.css` y hacer que componentes base/Lucide consuman `currentColor`.
- [x] Aplicar el tema mediante atributo/clase raíz del shell; ventanas y apps reciben contexto, no estilos duplicados. *(data-tema en documentElement)*
- [ ] Añadir feature flag y fallback claro para rollback; el tema no debe romper hydration ni primera pintura. *(anti-flash sí; feature flag no implementado)*
- [x] Cubrir CSS con VarSense y Sentinel; corregir referencias huérfanas y especificaciones visuales locales. *(gate PASS: 0 errores ambas herramientas)*

**Gate:** type-check, VarSense, Sentinel y pruebas de render pasan para ambas variantes.

### 4. Preferencias y sincronización

- [ ] Guardar la elección anónima en el overlay local sin modificar el release público ni `mobileOrder`. *(hoy en localStorage; migrar a overlay en 297A-13)*
- [ ] Sincronizar la preferencia de cuenta con el overlay remoto usando revisión, merge y 409; nunca sobrescribir silenciosamente. *(bloqueado por 297A-13)*
- [ ] Resolver login/logout, otro dispositivo y reset a `system`; evitar flash de tema en primera pintura. *(anti-flash resuelto; resto pendiente)*
- [x] Emitir `theme_changed` con modo y ámbito, sin contenido ni identificadores sensibles. *(ThemeEvent; presentationMode pendiente)*

**Gate:** pruebas local/remoto, dos pestañas/dispositivos, pérdida de red y logout pasan con rollback visible.

### 5. Accesibilidad y validación

- [ ] Verificar contraste AA, foco de teclado, forced-colors, reduced motion, zoom 200% y lectura de estado del botón. *(legibilidad dark de Perfil verificada en navegador; matriz completa pendiente)*
- [ ] Probar 1440x900, 1024x768, 768px, 390px, 360px y 320px; incluir orientación, safe area y teclado móvil.
- [ ] Ejecutar E2E de cambio de tema en escritorio/tablet/móvil y confirmar que la URL/app/recurso se conserva.
- [ ] Medir rendimiento de primera pintura y cambio de tema; documentar cualquier presupuesto excedido.

**Gate:** evidencia visual/E2E y quality gate completo; no se marca como terminado con warnings bloqueantes.

### 6. Cierre documental

- [ ] Actualizar manual visual, arquitectura e índice solo después de la aprobación y la implementación. *(pendiente de matriz de validación)*
- [x] Registrar una entrada en completados con archivos, gotchas, Sentinel/VarSense y GLORY. *(tareas-2026-07-31.md)*
- [ ] Confirmar revisión SOLID/escalabilidad: un tercer tema de prueba no exige reescribir componentes ni comandos.

## Definition of Done

- [ ] Un único botón global cambia el tema y es accesible.
- [ ] Desktop, tablet y móvil comparten tokens, comandos, permisos y analítica.
- [ ] Preferencia local/remota tiene ámbito, merge, reset y rollback explícitos.
- [ ] Manual visual, roadmap, pruebas y quality gate están sincronizados.
