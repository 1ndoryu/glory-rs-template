# Guía del cliente — Integración con BDP

> **Fecha:** 26 de julio de 2026
> **Versión:** 2.0 (actualización de la guía del 19 de julio)
> **Objetivo:** explicar al cliente cómo funciona la integración entre Glory y BDP, qué puede hacer, cómo se usa día a día y qué protecciones tiene.
> **Lenguaje:** no técnico, orientado a la operación del restaurante.

---

## Resumen rápido

La integración permite que Glory y BDP compartan información. Glory puede **leer** datos de BDP (catálogo, clientes, estados) y, cuando se autorice, **escribir** (crear clientes, comandas, pagos y facturas). El estado normal es de solo lectura. Las escrituras se protegen con autorización temporal automática.

---

## 1. Qué puede hacer Glory con BDP

### Consultas (solo lectura — no modifican BDP)

Estas funciones están disponibles siempre que la integración esté activa y no cambian nada dentro de BDP:

- **Catálogo de artículos:** importar artículos con precios, impuestos, familias, códigos de barras y stock.
- **Stock:** ver el stock actual de cada artículo sincronizado desde BDP, con filtros y exportación a CSV.
- **Explorador de menús:** consultar la estructura de menús, packs y modalidades de venta (fast food, comedor, etc.).
- **Historial de pedidos:** revisar el estado de comandas ya enviadas (aceptadas, canceladas, facturadas).
- **Albaranes de compra (Compras):** importar albaranes desde BDP, marcar como borrador y conciliarlos con gastos locales.
- **Clientes:** importar clientes desde BDP con vista previa antes de copiarlos o vincularlos en Glory.
- **Salones y mesas:** ver la estructura del plano del restaurante en BDP antes de agregarla al plano local.

### Escrituras (sí modifican BDP)

Estas funciones crean o cambian información real en BDP. Cada una requiere autorización explica:

- **Crear cliente:** da de alta un cliente nuevo en BDP con un código elegido expresamente, sin reemplazar clientes existentes.
- **Crear comanda:** envía una venta con artículos, cantidades, descuentos, impuestos, cliente, canal y forma de pago.
- **Registrar pago:** marca el saldo completo de una comanda como pagado.
- **Facturar:** emite la factura de una comanda ya pagada.

> **Nota:** los pagos parciales no están incluidos en esta versión. Una venta ya enviada queda protegida contra ediciones que pudieran crear duplicados.

---

## 2. Mapa de la pantalla: dónde está cada cosa

| Qué | Dónde | Aspecto |
|---|---|---|
| 4 pantallas BDP | Menú lateral → grupo "Integración BDP" | Stock, Explorador, Historial, Compras |
| Indicador de estado | Barra superior, junto a la campana | "BDP: off" (gris), "BDP: lectura" (gris), "BDP: escritura" (ámbar) |
| Botones de pago/factura | Fila de cada venta sincronizada | Tarjeta verde = pago · Recibo violeta = factura · Lupa azul = consultar |
| Importar clientes | Página Clientes, junto a "+ Nuevo Cliente" | Botón "Importar BDP" con icono de descarga |
| Modo demo | Esquina superior derecha de cada pantalla BDP | Botón que cambia a ámbar cuando está activo |

---

## 3. Las 4 pantallas BDP en detalle

Las 4 pantallas del menú "Integración BDP" funcionan con datos reales de BDP cuando la integración está conectada, o con datos de ejemplo cuando no.

### Stock

**Dónde:** menú lateral → BDP Stock.

Muestra el catálogo de artículos sincronizados desde BDP con su stock actual. La pantalla tiene:

- **Arriba a la izquierda:** contador de artículos y fecha de última sincronización.
- **Arriba a la derecha:** botón de modo demo, botón "CSV" para exportar, y botón "Sync catálogo" para actualizar.
- **Filtros:** campo de búsqueda por código o nombre, selector de stock (Con stock / Sin stock / Todos) y selector de estado (Activos / Inactivos / Todos).
- **Tabla:** columnas de Código Glory, Código BDP, Nombre, Precio y Stock. Las columnas se pueden ordenar haciendo clic en el encabezado. El stock se muestra con un icono de paquete cuando hay unidades disponibles.
- **Paginación:** botones Anterior/Siguiente y selector de cantidad por página (10, 25 o 50).

### Explorador

**Dónde:** menú lateral → BDP Explorador.

Permite explorar la estructura de menús, packs y modalidades de venta configurados en BDP. La pantalla tiene:

- **Filtros:** selector de tipo (Menú, Fast food, Pack) y campo de búsqueda por código o nombre.
- **Resultados:** tabla con Código, Nombre, Tipo y Descripción. Cada fila tiene un botón de ojo para ver el detalle.
- **Detalle:** ventana emergente que muestra la información completa del menú/pack incluyendo la lista de artículos que lo componen con sus cantidades.

### Historial

**Dónde:** menú lateral → BDP Historial.

Muestra el registro de operaciones realizadas contra BDP. La pantalla tiene dos pestañas:

**Pestaña "Auditoría":**

- Campo de búsqueda para filtrar por operación, resultado o error.
- Tabla con columnas: Fecha, Operación (con etiquetas como "Crear comanda", "Registrar pago", etc.), Dirección (Glory → BDP o BDP → Glory), y Resultado.
- Los resultados se muestran con colores: verde para "Completada", rojo para "Falló", ámbar para "Requiere revisión".
- Cada fila tiene un botón de ojo para ver el detalle completo de la operación.

**Pestaña "Snapshots":**

- Tabla con los respaldos guardados: Tipo, Fecha, Datos (resumen), Notas.
- Cada snapshot se puede restaurar o eliminar.

### Compras

**Dónde:** menú lateral → BDP Compras.

Gestiona los albaranes de compra importados desde BDP. La pantalla tiene:

- **Arriba a la derecha:** campo "Perfil" (número del perfil de exportación de BDP), botón "Sync albaranes" para importar, y botón de modo demo.
- **Filtros:** campo de búsqueda por proveedor, y selectores de fecha desde/hasta.
- **Tabla:** columnas de Fecha, Serie, Número, Proveedor, Total y Estado.
- **Estados con colores:** "Pendiente" en gris, "Borrador" en azul, "Conciliado" como botón verde deshabilitado con icono de check.
- **Acciones según estado:**
    - Pendiente → botón "Borrador" para marcarlo como borrador.
    - Borrador → botón "Conciliar" para vincularlo con un gasto.
    - Conciliado → sin acciones (ya está procesado).
- **Al conciliar:** aparece una ventana emergente con dos opciones: "Crear gasto nuevo" o "Vincular gasto existente".

---

## 4. Cómo funciona la protección de escrituras

La integración está diseñada para que las escrituras a BDP sean seguras. Estas son las capas de protección:

### Estado normal: solo lectura

Cuando Glory arranca, la integración está en modo **Solo lectura**. Esto significa que todas las consultas funcionan (catálogo, stock, historial, etc.) pero ninguna acción puede modificar BDP.

### Auto-arming: autorización automática por operación

Cuando un usuario quiere pagar o facturar una venta desde Glory, el sistema:

1. Pide una **confirmación textual** (escribir "PAGAR" y el importe exacto).
2. Verifica automáticamente que la configuración BDP sea correcta y que exista un respaldo reciente.
3. **Autoriza esa única operación** por un máximo de 5 minutos.
4. Ejecuta la operación real contra BDP.
5. Vuelve automáticamente a **Solo lectura**.

No hay que activar nada manualmente. El sistema gestiona la autorización de forma transparente durante la operación.

### Autorización manual (para situaciones especiales)

También existe la posibilidad de autorizar una operación manualmente desde la pantalla de configuración, con un tiempo configurable de 1 a 15 minutos. Esto es útil para operaciones especiales o cuando el auto-arming no está activado.

### Protección contra duplicados

Cada operación incluye un identificador único que evita que la misma acción se ejecute dos veces, incluso si hay un error de conexión o el usuario repite el clic.

### Auditoría completa

Cada escritura que se ejecuta contra BDP queda registrada en el historial con: fecha, hora, operación, resultado y, si aplica, el motivo de la autorización.

---

## 5. Qué se guarda en el historial

El historial de BDP registra cada operación que puede modificar datos en BDP:

- fecha y hora de la operación;
- tipo de operación (crear cliente, enviar comanda, pagar, facturar);
- cliente o venta afectados;
- resultado: éxito, error o ambiguo;
- motivo de la autorización.

Las consultas de solo lectura (importar catálogo, consultar stock, ver historial de pedidos) no se registran individualmente para no llenar el historial con operaciones rutinarias.

---

## 6. Respaldos y protección de datos

### Qué respalda Glory

- **Snapshots de BDP:** antes de una escritura, Glory puede guardar una copia del estado leído de BDP (clientes, artículos, configuración). Sirve para comparar antes/después y para investigar respuestas dudosas.
- **Respaldos locales:** Glory puede guardar y restaurar su propia información (clientes locales, mapeos de artículos, configuración).

### Límite importante

Un snapshot de Glory **no es una copia restaurable de BDP**. La integración no puede usarlo para:

- eliminar un cliente creado en BDP;
- borrar o anular una comanda;
- devolver un pago;
- anular una factura o recuperar su numeración.

Esos cambios requieren el procedimiento manual que utilice el restaurante dentro de BDP.

---

## 7. Feature flags: interruptores de funcionalidad

La integración incluye 6 interruptores que permiten activar o desactivar funciones específicas. Están en **Configuración → BDP** y se pueden cambiar en cualquier momento:

| Interruptor               | Qué controla                                          | Valor inicial |
| ------------------------- | ----------------------------------------------------- | ------------- |
| Auto-arming BDP           | Activa la autorización automática al pagar o facturar | Desactivado   |
| Pagos parciales           | Permite registrar pagos parciales (no solo el total)  | Desactivado   |
| Cancelar comandas         | Permite cancelar comandas ya enviadas a BDP           | Desactivado   |
| Lectura de albaranes      | Permite importar albaranes de compra desde BDP        | Desactivado   |
| Borrador de albaranes     | Permite marcar albaranes como borrador                | Desactivado   |
| Conciliación de albaranes | Permite conciliar albaranes con gastos                | Desactivado   |

> **Recomendación para producción:** activar los interruptores progresivamente según se vayan verificando las funciones en el restaurante.

---

## 8. Indicador de estado BDP en la barra superior

La barra superior de Glory muestra un indicador del estado de la integración BDP:

| Indicador          | Significado                                                                                                                            |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| **BDP: off**       | La integración está desactivada. Se puede activar directamente desde ahí si las credenciales están configuradas, o ir a Configuración. |
| **BDP: lectura**   | La integración está activa en modo solo lectura. Se puede cambiar a escritura temporal o ir al historial.                              |
| **BDP: escritura** | Hay una autorización temporal activa. Se puede desactivar manualmente.                                                                 |

Al hacer clic en el indicador aparece un menú con opciones rápidas: activar escritura temporal, desactivar escritura, ver historial BDP o ir a la configuración.

---

## 9. Cómo entender la pantalla de configuración BDP

**Dónde:** menú lateral → Configuración → pestaña BDP.

La configuración se organiza en 4 bloques (tarjetas) apilados verticalmente:

### Bloque 1: "Conexión BDP"

Es el bloque principal. Contiene:

- **URL pública BDP:** dirección del servidor BDP del restaurante.
- **Datos de acceso:** login, contraseña, código integrador, terminal POS, empleado y perfil de artículos. Estos valores los configura el responsable técnico. **No deben modificarse sin consultar.**
- **Interruptor "Integración BDP activa":** interruptor general. Si está apagado, Glory no procesa la integración. Si está encendido, permite las consultas configuradas pero **no concede por sí solo permiso para escribir en BDP**.
- **Catálogo de artículos BDP:** tabla con los artículos sincronizados (código Glory, código BDP, nombre, precio, stock). Incluye botones para sincronizar el catálogo completo o solo los precios, y un formulario para crear mapeos manuales.
- **Actualización de estados:** interruptor para activar la consulta automática de estados de comandas, con campo para indicar cada cuántos segundos se consulta.
- **Sección técnica colapsable:** "Correspondencias Glory ↔ BDP (solo soporte)" — solo visible al expandir, contiene los mapeos técnicos entre Glory y BDP.
- **Botones de diagnóstico:** "Probar conexión" (verifica que Glory puede comunicarse con BDP) y "Validar con simulador local" (comprueba cada endpoint sin modificar datos).
- **Botón "Guardar conexión BDP"** al final para aplicar los cambios.

### Bloque 2: "Modo de operaciones BDP"

Muestra visualmente el modo actual con dos recuadros:

- **"Solo lectura (BDP → Glory)"** — borde resaltado cuando está activo.
- **"Autorización manual (Glory → BDP)"** — borde ámbar cuando está activo.

### Bloque 3: "Funcionalidades BDP"

Contiene los 6 interruptores de funcionalidad (ver sección 6). Cada uno tiene una descripción que explica qué hace:

- Auto-arming BDP.
- Pagos parciales.
- Cancelar comandas.
- Lectura de albaranes.
- Borradores de albaranes.
- Conciliación de albaranes.

### Bloque 4: Explorador de menús

Muestra un explorador de solo lectura para consultar menús, packs y fast food directamente desde la configuración.

### Configuración técnica

_(Dentro del bloque 1, sección colapsable.)_

Relaciona los códigos de Glory con los códigos propios de BDP:

- **Formas de pago:** qué código BDP corresponde a efectivo, tarjeta u otros métodos.
- **Canales:** relación entre comedor, barra o domicilio con el tipo de pedido BDP.
- **Artículo sin equivalencia:** artículo BDP genérico cuando una línea de Glory no tiene relación específica.
- **Cliente por defecto:** código numérico del cliente genérico de BDP.

Estos valores dependen de cada instalación y se configuran durante la puesta en marcha.

---

## 10. Flujo de trabajo diario

### Consultar stock o catálogo

1. Ir al menú lateral → **BDP Stock** (icono de paquete).
2. Usar los filtros de búsqueda, stock y estado para encontrar artículos.
3. Si se necesita exportar, hacer clic en el botón **"CSV"** (junto a los filtros, arriba a la derecha).
4. Para actualizar los datos desde BDP, hacer clic en **"Sync catálogo"**.

### Explorar menús y packs

1. Ir al menú lateral → **BDP Explorador** (icono de lupa).
2. Seleccionar el tipo (Menú, Fast food o Pack) y buscar por código o nombre.
3. Hacer clic en el icono de ojo de cada fila para ver el detalle completo.

### Revisar el historial de operaciones

1. Ir al menú lateral → **BDP Historial** (icono de base de datos).
2. La pestaña "Auditoría" muestra todas las escrituras realizadas con su resultado (verde = éxito, rojo = error, ámbar = requiere revisión).
3. La pestaña "Snapshots" muestra los respaldos guardados.
4. Hacer clic en el icono de ojo de cada fila para ver el detalle completo.

### Importar clientes desde BDP

1. Ir al menú lateral → **Clientes**.
2. Hacer clic en el botón **"Importar BDP"** (icono de descarga, junto a "+ Nuevo Cliente").
3. En el diálogo que aparece, hacer clic en **"Previsualizar sin cambios"** para ver qué clientes se encontraron.
4. Revisar el resumen: Nuevos, Vínculos, Sin cambios, Conflictos.
5. Escribir la confirmación ("IMPORTAR CLIENTES BDP") y hacer clic en **"Aplicar en Glory"**.

### Sincronizar un cliente individual

1. En la tabla de clientes, la columna "BDP" muestra el estado de cada cliente:
    - Etiqueta gris con código = ya sincronizado.
    - Etiqueta roja = error en la sincronización.
    - Etiqueta gris "Sin vincular" = no tiene código BDP.
2. Para vincular un cliente, hacer clic en el botón **"BDP"** de su fila y seguir las instrucciones.

### Enviar una venta a BDP (pagar o facturar)

1. Crear la venta normalmente en Glory.
2. En la lista de ventas, buscar la fila de la venta. Si está sincronizada con BDP, aparecen pequeños botones con iconos:
    - **Verde (tarjeta de crédito):** para registrar el pago.
    - **Violeta (recibo):** para facturar.
    - **Azul (lupa):** para consultar el estado en BDP.
3. Al hacer clic en el botón de pago, aparece un diálogo con:
    - Resumen de total, pagado y pendiente.
    - Campo para el importe a pagar.
    - Campo para el identificador de forma de pago (Tender ID).
    - Confirmación textual: escribir exactamente "PAGAR" seguido del ID de venta y el importe.
4. Al hacer clic en el botón de factura, aparece un diálogo con:
    - Resumen de total y pendiente.
    - Confirmación textual: escribir exactamente "FACTURAR" seguido del ID de venta.
5. El sistema gestiona la autorización automáticamente (ver sección 3).
6. Verificar el resultado en **BDP → Historial**, pestaña "Auditoría".

### Conciliar un albarán de compra

1. Ir al menú lateral → **BDP Compras** (icono de recibo).
2. Indicar el número de perfil en el campo "Perfil" (arriba a la derecha) y hacer clic en **"Sync albaranes"**.
3. Usar los filtros de proveedor y fechas para encontrar el albarán.
4. Hacer clic en **"Borrador"** en la fila del albarán pendiente.
5. Revisar los datos del albarán.
6. Hacer clic en **"Conciliar"** en la fila del albarán en borrador.
7. En la ventana emergente, elegir entre crear un gasto nuevo o vincular a uno existente.
8. Confirmar la conciliación.

---

## 10. Qué queda fuera de esta integración

No se incluyeron:

- administración de stock desde Glory (solo lectura);
- transferencias entre almacenes;
- tallas, colores ni fidelización;
- administración completa de menús y packs (solo consulta);
- sincronización general en ambas direcciones;
- pagos parciales (disponible como feature flag, desactivado por defecto);
- cancelación de comandas (disponible como feature flag, desactivado por defecto).

---

## 11. Pruebas realizadas

### Pruebas del equipo técnico (sin BDP real)

- **11 tests automáticos** de los servicios de escritura contra un simulador local que replica la API de BDP.
- **57 tests unitarios** de validación, lógica de negocio y protecciones de seguridad.
- **92 tests del simulador** que verifican cada endpoint de BDP.
- Verificación de protecciones: duplicados, escrituras sin autorización, respuestas dudosas, timeouts.

### Pruebas de lectura contra BDP real

Las 4 pantallas de lectura (Stock, Explorador, Historial, Compras) fueron verificadas contra BDP conectado para confirmar que los datos se leen y muestran correctamente.

### Pruebas de escritura contra BDP real

Las 4 operaciones de escritura (crear cliente, crear comanda, registrar pago, facturar) requieren una sesión de pruebas con el BDP del restaurante. Ver sección siguiente.

---

## 12. Pruebas de escritura pendientes (requieren BDP del restaurante)

Estas 4 pruebas producen cambios reales en BDP y solo pueden hacerse con acceso al BDP del restaurante:

| Prueba         | Cambio que producirá en BDP                                              |
| -------------- | ------------------------------------------------------------------------ |
| Crear cliente  | Dará de alta un cliente con un código nuevo                              |
| Crear comanda  | Creará una comanda que puede aparecer en TPV, cocina e informes          |
| Registrar pago | Marcará el saldo completo de la comanda como pagado y puede afectar caja |
| Facturar       | Emitirá una factura y puede afectar numeración e información fiscal      |

### Antes de probar

Confirmar que:

- la versión actualizada está instalada en el entorno del restaurante;
- Glory inicia en **Solo lectura**;
- hay un respaldo reciente comprobado;
- el destino configurado corresponde al BDP del restaurante;
- el responsable conoce cómo corregir manualmente cada efecto en BDP.

### Condiciones

- una hora de poca actividad;
- quién observará BDP, TPV y cocina;
- un cliente de prueba y un código confirmado como libre;
- una venta pequeña, identificada como prueba;
- cómo anular o corregir manualmente cada registro;
- que cualquier pago o factura de prueba sea aceptable para caja y administración.

### Reglas durante las pruebas

1. Realizar una sola acción cada vez.
2. Comprobar el resultado directamente en BDP antes de continuar.
3. No repetir una acción si la aplicación queda esperando o muestra un resultado dudoso.
4. Guardar una captura y anotar la hora, el importe y los identificadores.
5. Detener toda la revisión ante el primer resultado inesperado.

---

## 13. Preguntas frecuentes

**¿Tengo que activar manualmente la escritura cada vez que quiero pagar una venta?**
No. Si el interruptor de auto-arming está activado, el sistema autoriza automáticamente cada operación de pago o factura. Solo hay que confirmar con el texto indicado.

**¿Qué pasa si se corta la conexión durante una escritura?**
El sistema registra el resultado como "ambiguo" y bloquea nuevos intentos hasta verificar qué ocurrió en BDP. Esto evita duplicados.

**¿Puedo ver el stock de BDP en Glory?**
Sí. La pantalla de Stock muestra el stock sincronizado desde BDP. Se actualiza cada vez que se sincroniza el catálogo.

**¿Los albaranes de compra se concilian solos?**
No. El flujo es: importar → revisar como borrador → conciliar manualmente con un gasto. Esto permite verificar que los importes coincidan antes de registrar el gasto.

**¿Qué pasa si cambio la URL o las credenciales de BDP?**
Cualquier autorización temporal activa se anula automáticamente. Esto es una protección para evitar que escrituras se envíen al destino equivocado.

**¿Puedo deshacer una factura emitida desde Glory?**
No desde Glory. Las facturas emitidas en BDP deben anularse desde el propio BDP. Glory registra el resultado pero no puede revertirlo.

---

## Referencias

- `Agente/documentacion/bdp/runbook-operativo-bdp-2026-07-26.md` — procedimientos ante incidentes.
- `Agente/documentacion/bdp/feature-flags-bdp-2026-07-26.md` — documentación detallada de los 6 feature flags.
- `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` — riesgos conocidos y mitigaciones.
- `Agente/usuario/mapeo-visual-integracion-bdp-2026-07-23.md` — dónde se ve cada funcionalidad en el frontend.
