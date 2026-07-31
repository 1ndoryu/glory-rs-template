# Plan especializado: experiencia móvil tipo launcher

> **Tarea:** 297A-12  
> **Fecha:** 2026-07-29  
> **Estado:** prototipo visual implementado; pendiente de aprobación explícita
> **Alcance:** teléfonos `<768px`; tablet conserva experiencia desktop

## 1. Resultado

En teléfono, wandori.us se percibe como un móvil minimalista con el mismo lenguaje visual del OS:

- Launcher con aplicaciones/carpetas ordenables como una pantalla de inicio.
- Sin ventanas flotantes, titlebars, barra superior desktop ni taskbar.
- Abrir una app la muestra a pantalla completa.
- Navegar atrás vuelve al estado anterior o al launcher.
- Mismas apps, datos, permisos, rutas, comandos y analytics del desktop.
- Tablet (`>=768px`) conserva escritorio, ventanas y barras.

## 2. Invariantes para no reinventar

- [ ] AppRegistry es único para desktop/tablet/móvil.
- [ ] Una app devuelve el mismo `MountedView`; no existe `MobileReader`, `MobileStore`, etc.
- [ ] MobileAppStack es una proyección del mismo estado/comandos, no otro store de negocio.
- [ ] CommandRegistry resuelve acciones; clic derecho desktop se adapta a long press/menú móvil.
- [ ] RouteAppAdapter conserva deep links y Back.
- [ ] Workspace overlay usa los mismos nodos; solo cambia `mobileOrder` y presentación.
- [ ] Seguridad, estados, papelera, pagos y analytics no dependen del viewport.

## 3. Contrato de presentación

```ts
type OsPresentationMode = 'mobile' | 'desktop';

interface MobileNavigationState {
  stack: Array<{
    appId: string;
    resourceId?: string;
    instanceId: string;
  }>;
}
```

- `mobile` aplica por capacidad/layout `<768px`; `desktop` aplica desde tablet.
- El breakpoint se decide en un adaptador de presentación, no dentro de cada app.
- Cambiar orientación/breakpoint preserva app y recurso activo.
- Bounds/z-order/minimized no se usan en móvil; se conservan para volver a desktop.
- Mobile stack no se persiste como historial infinito; solo estado recuperable definido.

## 4. Bloque 1 — Prototipo visual sin lógica final

- [x] Crear boceto real del launcher con grid, carpetas y Papelera; badges quedan para el estado real.
- [x] Mostrar una app pública a pantalla completa.
- [x] Mostrar Finder/carpeta móvil.
- [x] Mostrar Reader móvil con artículo/media.
- [x] Mostrar Tienda/Compra móvil sin integrar pago.
- [x] Definir navegación Back/Home sin barras desktop.
- [x] Revisar 390×844, 360×800 y 320px; confirmar escritorio en tablet 768px.
- [ ] Obtener aprobación explícita del usuario antes de implementar runtime móvil.

**Implementación de revisión:** `frontend/src/features/mobile/mobile-prototype.ts` y
`frontend/src/styles/mobile/mobile-prototype.css`. Los datos son demostrativos y el
módulo no implementa persistencia, pagos, drag, long press ni el stack definitivo.

**Gate:** aspecto y navegación base aprobados.

## 5. Bloque 2 — Shell móvil compartido

- [ ] Añadir `OsPresentationMode` central.
- [ ] Crear MobileLauncher que consume nodos/registry existentes.
- [ ] Crear MobileAppStack que monta `MountedView` existente.
- [ ] Ocultar/no montar DesktopWindow, top menu y taskbar en móvil.
- [ ] Integrar safe areas, viewport dinámico y teclado virtual.
- [ ] Mantener navegación exterior accesible mediante comando acordado en prototipo.
- [ ] Evitar listeners/media queries duplicados por app.

**Gate:** Perfil/Finder/Reader abren full-screen sin chrome desktop ni duplicación de contenido.

## 6. Bloque 3 — Launcher, carpetas y organización

- [ ] Grid ordenable con `mobileOrder`.
- [ ] Carpetas abren vista full-screen y permiten navegación jerárquica.
- [ ] Drag/reorder con alternativa accesible por comandos.
- [ ] Long press abre CommandRegistry móvil.
- [ ] Crear carpeta, copiar, cortar, pegar y mover usan comandos existentes.
- [ ] Papelera muestra solo capas autorizadas.
- [ ] Badges/estados siguen el manual visual.
- [ ] Overlay local/remoto sincroniza orden y additions/tombstones.

**Gate:** reorganización móvil sobrevive reload/sync sin alterar layout desktop ni release público.

## 7. Bloque 4 — Navegación y cambio de modo

- [ ] Back cierra menú, vuelve dentro de app o desapila app en orden correcto.
- [ ] Home vuelve al launcher sin destruir estado permitido.
- [ ] Deep link abre directamente app/recurso full-screen.
- [ ] Refresh reconstruye estado seguro desde URL/bootstrap.
- [ ] Pasar móvil→tablet transforma app activa en ventana recuperable.
- [ ] Pasar tablet→móvil selecciona la ventana activa como app full-screen.
- [ ] Ventanas secundarias se conservan sin mostrarse o siguen política explícita probada.
- [ ] Foco se restaura al icono/elemento correcto.

**Gate:** resize/orientación/deep links no pierden app, recurso ni navegación.

## 8. Bloque 5 — Apps críticas

- [ ] Cuenta/login/registro full-screen y teclado usable.
- [ ] Editor admin full-screen con toolbar adaptada.
- [ ] Store/Product/Checkout respeta viewport y proveedor.
- [ ] Recibo/descarga accesibles sin popups ocultos.
- [ ] Reader conserva legibilidad, media y progreso.
- [ ] Navegador retro/proyectos respeta sandbox y Back.
- [ ] Configuración y Estadísticas usan componentes compartidos responsive.

**Gate:** cada app crítica pasa prueba funcional móvil sin versión paralela.

## 9. Accesibilidad y visual

- [ ] JetBrains Mono y Lucide 1px permanecen.
- [ ] Chrome móvil monocromo, sin sombras/radios/colores.
- [ ] Área táctil mínima 44×44 cuando sea viable; icono óptico no se engrosa.
- [ ] Foco visible para teclado/switch y orden lógico.
- [ ] Zoom 200% y texto grande no bloquean navegación.
- [ ] No depender solo de long press; comandos tienen alternativa visible/accesible.
- [ ] Reduced motion y sin gestos obligatorios sin alternativa.
- [ ] Contenido no queda bajo notch/safe area/teclado.

## 10. Analytics y rendimiento

- [ ] Eventos usan mismos nombres/appId/resourceId y agregan `presentationMode=mobile` permitido.
- [ ] No medir touchmove/drag por pixel.
- [ ] Lazy-load solo apps pesadas medido, no otro bundle móvil.
- [ ] Launcher inicia con bootstrap único y sin N+1 de iconos/estados.
- [ ] Memoria se libera al desapilar/destruir apps según lifecycle.
- [ ] Definir budgets de arranque, interacción y media móvil.

## 11. Pruebas obligatorias

- [ ] 320px, 360×800, 390×844 y orientación horizontal.
- [ ] Tablet 768×1024 conserva escritorio/ventanas.
- [ ] Abrir/cerrar/Back/Home y deep link.
- [ ] Cambio móvil↔tablet con app activa.
- [ ] Reordenar, carpeta, long press, papelera y reset.
- [ ] Cuenta, Reader, Finder, Editor, Compra y descarga.
- [ ] Teclado virtual, safe area, zoom y foco.
- [ ] Visitante no accede a comandos/recursos admin.
- [ ] No se crean componentes o stores móviles duplicados.

## 12. Criterio final de cierre

- [ ] Teléfono usa launcher y apps full-screen sin barras/ventanas desktop.
- [ ] Tablet conserva comportamiento desktop.
- [ ] Apps y lógica son idénticas entre presentaciones.
- [ ] Organización móvil persiste sin contaminar bounds desktop.
- [ ] Navegación, accesibilidad y rutas funcionan en los tamaños mínimos.
- [ ] Manual visual, arquitectura, roadmap y Sentinel reflejan la variante móvil.
