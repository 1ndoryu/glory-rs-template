# Plan: Galería de Imágenes SEO + Recorte + JSON-LD Editable

> **Fecha:** 2026-07-27
> **Objetivo:** Mejorar el modal de edición SEO con selector visual de imágenes (galería + recorte) y hacer el campo JSON-LD tipo editable.
> **Rama:** `glory-rust-nakomi`

---

## 1. Problema actual

| Campo | Estado actual | Problema |
|-------|--------------|----------|
| Imagen OG (URL) | Input de texto libre | El usuario no sabe qué imagen elegir. No hay preview. No hay recorte automático a 1200×630. |
| Tipo JSON-LD | Input disabled (solo lectura) | `UpdateSeoSettingBody` no incluye `json_ld_type`. El usuario no puede cambiarlo. |

---

## 2. Arquitectura de la solución

```
┌──────────────────────────────────────────────────────────┐
│ ModalSeoEdit (existente, mejorado)                       │
│                                                          │
│ ┌──────────────────────────────────────────────────────┐ │
│ │ Imagen OG                                            │ │
│ │ ┌──────────────┐  ┌────────────────────────────────┐ │ │
│ │ │ [Preview]    │  │ [🖼️ Elegir de galería]        │ │ │
│ │ │ 1200×630     │  │ [📷 Subir nueva]              │ │ │
│ │ │              │  │ [✕ Quitar]                    │ │ │
│ │ └──────────────┘  └────────────────────────────────┘ │ │
│ └──────────────────────────────────────────────────────┘ │
│                                                          │
│ ┌──────────────────────────────────────────────────────┐ │
│ │ Tipo JSON-LD                                         │ │
│ │ ┌──────────────────────────────────────────────────┐ │ │
│ │ │ [Select: Organization+WebSite ▾]                 │ │ │
│ │ └──────────────────────────────────────────────────┘ │ │
│ └──────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

### 2.1 Flujo de selección de imagen OG

```
Click "Elegir de galería"
    │
    ▼
┌─────────────────────────────────────────┐
│ Galería de Imágenes OG                  │
│ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐   │
│ │ img1 │ │ img2 │ │ img3 │ │ +    │   │
│ │      │ │      │ │      │ │Subir │   │
│ └──────┘ └──────┘ └──────┘ └──────┘   │
│                                        │
│ [Filtro: Todas / Uploads / Assets]     │
│                                        │
│ Imágenes cargadas desde /uploads/      │
│ y /assets/ (existentes en el servidor) │
└───────────────┬────────────────────────┘
                │
        Click en imagen
                │
                ▼
┌─────────────────────────────────────────┐
│ Recorte de Imagen                       │
│ ┌─────────────────────────────────────┐ │
│ │  ┌─────────────────┐               │ │
│ │  │ Zona de recorte │ (aspecto 1200:630 = 1.905:1) │
│ │  │  1200 × 630     │               │ │
│ │  └─────────────────┘               │ │
│ │                                     │ │
│ │  Imagen original                    │ │
│ └─────────────────────────────────────┘ │
│                                        │
│ Vista previa: [thumbnail 1200×630]     │
│                                        │
│ [ Cancelar ]  [ Aplicar recorte ]      │
└────────────────────────────────────────┘
                │
        Aplicar recorte
                │
                ▼
    Sube imagen recortada → /api/admin/uploads
    Retorna URL → se guarda en og_image_url
```

### 2.2 Flujo alternativo: subir nueva imagen

```
Click "Subir nueva" → input file → seleccionar imagen → ir directo al recorte
```

---

## 3. Componentes a crear/modificar

### 3.1 Dependencia nueva

| Paquete | Uso | Tamaño |
|---------|-----|--------|
| `react-easy-crop` | Recorte interactivo con aspecto fijo | ~15KB gzipped |

Alternativas evaluadas:
- `react-image-crop`: más antiguo, menos mantenido
- `cropperjs`: muy pesado (~60KB), jQuery legacy
- `react-advanced-cropper`: bueno pero más complejo de lo necesario

**`react-easy-crop`** es la mejor opción: ligero, moderno, soporta aspect ratio, zoom, y genera el canvas recortado.

### 3.2 Backend

| # | Archivo | Acción | Detalle |
|---|---------|--------|---------|
| B1 | `src/handlers/uploads.rs` | Modificar | Añadir endpoint `GET /api/admin/uploads` que lista imágenes existentes en `uploads/content/` |
| B2 | `src/handlers/admin_seo.rs` | Modificar | Añadir `json_ld_type: Option<String>` a `UpdateSeoSettingBody` |
| B3 | `src/repositories/seo_settings.rs` | Modificar | Añadir `json_ld_type` al `upsert()` |

### 3.3 Frontend

| # | Archivo | Acción | Detalle |
|---|---------|--------|---------|
| F1 | `frontend/package.json` | Modificar | Añadir `react-easy-crop` como dependencia |
| F2 | `frontend/src/api/uploads.ts` | Modificar | Añadir `apiListUploads()` que llama `GET /api/admin/uploads` |
| F3 | `frontend/src/components/ui/ImageCropModal.tsx` | **Nuevo** | Modal con `react-easy-crop`: aspecto 1.905:1, zoom, preview, genera blob recortado |
| F4 | `frontend/src/components/ui/ImageCropModal.css` | **Nuevo** | Estilos del modal de recorte |
| F5 | `frontend/src/components/ui/ImageGalleryPicker.tsx` | **Nuevo** | Galería modal: grid de miniaturas, botón subir, click selecciona → abre crop |
| F6 | `frontend/src/components/ui/ImageGalleryPicker.css` | **Nuevo** | Estilos de la galería |
| F7 | `frontend/src/components/panel/ModalSeoEdit.tsx` | Modificar | Reemplazar input texto OG por `ImageGalleryPicker` + preview. Input JSON-LD → Select editable |
| F8 | `frontend/src/components/panel/ModalSeoEdit.css` | Modificar | Estilos para preview OG + select JSON-LD |

---

## 4. Detalle de implementación

### B1 — Endpoint `GET /api/admin/uploads`

```rust
#[derive(Serialize)]
struct UploadEntry {
    url: String,         // "/uploads/content/abc123.webp"
    file_name: String,   // "abc123.webp"
    size_bytes: u64,
    modified_at: String, // ISO 8601
}

async fn list_uploads(auth: AuthUser) -> Result<Json<Vec<UploadEntry>>, AppError> {
    require_admin(&auth)?;
    // Leer directorio uploads/content/, filtrar imágenes
    // Retornar lista ordenada por fecha modificación DESC
}
```

Registrar en `routes()`: `.route("/admin/uploads", get(list_uploads).post(upload_image))`

### B2 — `json_ld_type` editable

```rust
#[derive(Deserialize)]
struct UpdateSeoSettingBody {
    title: String,
    description: String,
    og_image_url: Option<String>,
    json_ld_type: Option<String>,  // ← NUEVO
}
```

### B3 — upsert con json_ld_type

```rust
pub async fn upsert(
    pool: &PgPool, path: &str, title: &str, description: &str,
    og_image_url: Option<&str>, json_ld_type: Option<&str>,
) -> Result<SeoSetting, sqlx::Error> {
    // INSERT ... ON CONFLICT DO UPDATE SET
    //   json_ld_type = COALESCE(EXCLUDED.json_ld_type, seo_settings.json_ld_type)
}
```

### F3 — `ImageCropModal.tsx`

```tsx
import Cropper from 'react-easy-crop';

// Props: imagen URL, onCrop(blob), onClose
// Estado: crop {x, y}, zoom, croppedAreaPixels
// Aspecto: 1200/630 = 1.9047...
// Al confirmar: canvas.toBlob() → FormData → apiUploadImage(blob)
// Retorna URL de la imagen recortada
```

Lógica de recorte:
```typescript
// Genera canvas con las dimensiones del crop
const canvas = document.createElement('canvas');
canvas.width = 1200;
canvas.height = 630;
const ctx = canvas.getContext('2d');
ctx.drawImage(image, sx, sy, sw, sh, 0, 0, 1200, 630);
canvas.toBlob(blob => { /* upload */ }, 'image/jpeg', 0.90);
```

### F5 — `ImageGalleryPicker.tsx`

```tsx
// Props: value (URL actual), onChange(url), onClose
// Estado: images[] desde apiListUploads(), selectedImage, showCrop

// Layout:
// - Barra superior: filtro (Todas/Uploads/Assets), botón "Subir nueva"
// - Grid de miniaturas (4 cols desktop, 2 cols mobile)
// - Click en miniatura → abrir ImageCropModal con esa imagen
// - Botón "Subir nueva" → input file → ImageCropModal
```

### F7 — Cambios en `ModalSeoEdit.tsx`

**Campo OG Image → reemplazar input texto:**
```tsx
<div className="modalSeoCampo">
    <label className="modalSeoLabel">Imagen OG</label>
    <div className="modalSeoOgPreview">
        {ogImageUrl && <img src={ogImageUrl} alt="OG Preview" />}
        <div className="modalSeoOgActions">
            <button onClick={() => setShowGallery(true)}>🖼️ Elegir de galería</button>
            <button onClick={handleUploadNew}>📷 Subir nueva</button>
            {ogImageUrl && <button onClick={() => setOgImageUrl('')}>✕ Quitar</button>}
        </div>
    </div>
    <span className="modalSeoAyuda">Recomendado: 1200×630px. Se recorta automáticamente.</span>
</div>
```

**Campo JSON-LD → Select editable:**
```tsx
<div className="modalSeoCampo">
    <label className="modalSeoLabel">Tipo JSON-LD</label>
    <select value={jsonLdType} onChange={e => setJsonLdType(e.target.value)}>
        <option value="">— Ninguno —</option>
        <option value="Organization+WebSite">Organization + WebSite</option>
        <option value="Organization">Organization</option>
        <option value="BreadcrumbList+Person">BreadcrumbList + Person</option>
        <option value="FAQPage">FAQPage</option>
        <option value="CollectionPage">CollectionPage</option>
        <option value="ContactPage">ContactPage</option>
    </select>
</div>
```

---

## 5. Orden de ejecución

```
Paso 1: B1 — Endpoint list_uploads
Paso 2: B2 + B3 — json_ld_type editable (backend)
Paso 3: F1 — Instalar react-easy-crop
Paso 4: F2 — API listUploads frontend
Paso 5: F3 + F4 — ImageCropModal
Paso 6: F5 + F6 — ImageGalleryPicker
Paso 7: F7 + F8 — Integrar en ModalSeoEdit
Paso 8: Validación (cargo check + tsc)
Paso 9: Code review + commit
```

---

## 6. Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| `react-easy-crop` no funciona en mobile | Baja | Medio | Tiene soporte touch nativo. Test en mobile. |
| Imágenes grandes → canvas.toBlob lento | Media | Bajo | Limitar a 4096px max dimension antes de crop |
| Directorio uploads/content con muchos archivos | Baja | Bajo | Paginación o limitar a últimas 50 imágenes |
| Crop genera imagen > 500KB | Media | Bajo | JPEG quality 0.90 + max 1200×630 |

---

## 7. Estimación

| Tarea | Esfuerzo |
|-------|----------|
| B1: Endpoint list_uploads | 30min |
| B2+B3: json_ld_type editable | 20min |
| F1: Instalar dependencia | 2min |
| F2: API frontend | 5min |
| F3+F4: ImageCropModal | 1.5h |
| F5+F6: ImageGalleryPicker | 1.5h |
| F7+F8: Integrar en ModalSeoEdit | 45min |
| Validación + review | 20min |
| **Total** | **5-6h** |

---

## 8. Criterio de cierre

- [ ] Click en "Imagen OG" abre galería de imágenes del servidor
- [ ] Click en imagen abre recorte con aspecto 1200×630
- [ ] Zoom y pan funcionan en el recorte
- [ ] "Aplicar recorte" sube imagen recortada y la muestra como preview
- [ ] "Subir nueva" abre input file → va directo al recorte
- [ ] "Quitar" limpia la imagen OG
- [ ] JSON-LD type es un select editable con opciones predefinidas
- [ ] Guardar cambios persiste og_image_url y json_ld_type en DB
- [ ] Funciona en mobile (touch para crop + gallery scrollable)
