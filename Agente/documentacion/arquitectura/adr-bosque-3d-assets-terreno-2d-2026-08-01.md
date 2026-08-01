# ADR — Bosque 3D, assets externos y terreno editable en 2D

> **Fecha:** 2026-08-01
> **Estado:** aceptado; parámetros de relieve, cámara y presupuestos pendientes de medir/aprobar.
> **Epic:** GAME-01
> **Decisión del usuario:** se aprueba la dirección visual Three.js isométrica del segundo boceto.

## Contexto

El boceto 3D conserva la estética de tinta y permite una cámara espacial real. El proyecto no debe convertirse en un modelador 3D ni en un editor de mundos infinito. El usuario ya dispone de programas externos para crear modelos, pero necesita administrar esos assets y construir el terreno dentro del OS sin complicar el flujo.

## Decisión

### 1. Renderer y mundo lógico

- El cliente usa Three.js, siempre lazy y detrás del lifecycle de la app.
- La presentación es 3D; la simulación autoritativa conserva coordenadas `x/z` y obtiene `y` del terreno determinista.
- El servidor no confía en matrices, alturas o colisiones calculadas únicamente por el navegador.
- El mapa es finito y tiene bounds explícitos. No existe generación ni streaming de mundo infinito en el MVP.

### 2. El OS no modela geometría

- Blender u otra herramienta externa crea mallas, UV, rig y animaciones.
- El formato runtime canónico es GLB/glTF 2.0. Los archivos fuente editables (`.blend`, etc.) permanecen fuera del runtime y bajo control del autor.
- El programa `Assets 3D` importa, valida, previsualiza, clasifica, versiona y publica; no mueve vértices, esculpe ni edita rigs.
- Una versión publicada es inmutable. Reexportar un modelo crea una versión nueva y los mapas existentes conservan la versión fijada.

### 3. Terreno finito editable como datos 2D

- `Editor de mapa` presenta una vista cenital 2D con cuadrícula y herramientas de brocha.
- Cada celda guarda altura cuantizada, tipo de superficie y flags de navegación; agua, camino y terreno son datos, no mallas dibujadas manualmente.
- Los assets se colocan como instancias con `assetVersionId`, `x/z`, rotación Y, escala acotada y anclaje al terreno.
- Three.js genera mallas por chunks a partir del documento. Las costuras comparten borde y la vista 3D reutiliza exactamente el mismo contrato.
- El MVP excluye cuevas, voladizos, túneles y escultura libre. Puentes u objetos elevados serán assets con proxies de colisión explícitos cuando exista un caso real.

### 4. Persistencia y ubicación

- PostgreSQL guarda metadatos, estados, versiones, relaciones y documentos de mapa.
- El almacenamiento de objetos guarda GLB/texturas/miniaturas por hash; el repositorio solo contiene fixtures pequeños y originales.
- Finder puede mostrar referencias virtuales en una carpeta `Assets 3D`, pero moverlas no duplica el binario.
- Todo asset y mapa nace `draft + private + active`; publicar exige capacidad admin y validación server-side.
- Una sala fija `mapVersionId` y las versiones exactas de sus assets hasta cerrarse; publicar no muta partidas activas.

### 5. Optimización obligatoria

- El mapa se divide en chunks finitos; solo se montan chunks próximos/visibles.
- Árboles, rocas y props repetidos usan instancing por versión/material.
- Frustum culling, caché con referencias y teardown GPU son obligatorios antes de añadir LOD o compresión compleja.
- LOD, Meshopt/Draco y KTX2 solo se incorporan con medición, tooling reproducible y fallback; no son requisitos del primer importador.
- El importador aplica presupuestos por perfil de asset y el runtime aplica presupuesto visible por dispositivo. Exceder un límite bloquea publicación o produce advertencia admin explícita, nunca degradación silenciosa.

## Contratos principales

```text
GameAsset
  └─ GameAssetVersion -> GLB + metadata + collisionProxy + optional LODs

MapDraft
  └─ TerrainDocument -> bounds + cellSize + chunks + heights + surfaces
  └─ AssetInstance[] -> assetVersionId + transform + terrainAnchor
  └─ SpawnPoint[] / zones / navigation flags

MapVersion (inmutable)
  └─ manifiesto exacto de chunks y asset versions
```

El mapa nunca incrusta GLB ni permite código, shaders arbitrarios, URIs externas o metadata ejecutable.

## Validación de un GLB

- Magic bytes, MIME, tamaño total y estructura glTF válidos.
- Límites de nodos, primitivas, triángulos, materiales, texturas, dimensiones, skins, huesos y animaciones.
- Extensiones allowlisted; URIs externas y referencias fuera del paquete prohibidas.
- Escala/unidades, eje Y-up, bounds, pivot y nombres normalizados.
- Colisión elegida entre proxies simples allowlisted; una malla visual no se convierte automáticamente en autoridad física.
- Procedencia/licencia, hash y resultado del análisis guardados con la versión.

## Consecuencias

### Positivas

- Se obtiene una vista 3D expresiva sin construir un editor 3D completo.
- El terreno se puede editar, versionar, validar y probar como datos compactos.
- El servidor puede validar movimiento y alturas sin ejecutar Three.js.
- Repetición de árboles/rocas escala mediante instancing y chunks, no copiando mallas.

### Costes

- Hace falta un exportador GLB consistente y disciplina de unidades/pivots.
- La vista 2D debe comunicar bien altura y superficies para que editar no sea confuso.
- Las animaciones/personajes requieren un perfil de asset distinto al de props estáticos.
- WebGL móvil obliga a fijar presupuestos reales antes de poblar el mapa final.

## Rechazos explícitos

- Editor de vértices, materiales o rigs dentro del OS.
- Guardar modelos de producción directamente en Git.
- Mapa infinito, procedural o sin bounds en el MVP.
- Colisión basada ciegamente en cada triángulo visual.
- Un único JSON gigante con GLB embebidos o carga completa del mapa al abrir.
- Sustituir validación server-side por una previsualización exitosa en el navegador.

## Decisiones todavía abiertas

1. Relieve: plano, alturas discretas o colinas suaves. Recomendación inicial: alturas discretas con herramienta `elevar/bajar/alisar`, sin cuevas.
2. Cámara jugable: libre como el boceto o isométrica limitada siguiendo al personaje. Recomendación: órbita limitada + recenter.
3. Materiales: monocromo estricto o paleta muy restringida dentro del mundo 3D.
4. Primera animación: personaje rígido/provisional o GLB con rig y clips `idle/walk`.
