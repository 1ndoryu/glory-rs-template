use std::collections::HashMap;

use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::models::{ActualizacionInmueble, FiltrosPublicos, Foto, InmuebleRow};

/* [159A-1] Acceso a `inmuebles`/`fotos` con prepared statements.
 * Listas públicas con QueryBuilder: el SQL dinámico solo concatena
 * fragmentos fijos; los valores siempre van por `push_bind`.
 * [229A-1] Todas las SELECT/RETURNING usan `{COLUMNAS}` (constante única):
 * añadir una columna al modelo es editar un solo sitio. */

const COLUMNAS: &str = "id, titulo, descripcion, ubicacion, puestos, residencia, precio, \
    tipo, operacion, habitaciones, banos, metros, metros_terreno, estado, publicado, \
    slug, copy_corta, copy_larga, copy_modelo, copy_actualizada_en, receta, extras, \
    precio_minimo, created_at, updated_at";

/* [279A-3] Columnas para la web pública: las mismas salvo `extras` y
 * `precio_minimo` (privados de la dueña). Se rellenan con valores vacíos
 * para reutilizar `InmuebleRow` sin exponer nada sensible. */
const COLUMNAS_PUBLICAS: &str = "id, titulo, descripcion, ubicacion, puestos, residencia, precio, \
    tipo, operacion, habitaciones, banos, metros, metros_terreno, estado, publicado, \
    slug, copy_corta, copy_larga, copy_modelo, copy_actualizada_en, receta, \
    '{}'::JSONB AS extras, NULL::FLOAT8 AS precio_minimo, created_at, updated_at";

/// Valores ya normalizados listos para insertar
pub struct NuevoInmueble<'a> {
    pub titulo: &'a str,
    pub descripcion: &'a str,
    pub ubicacion: &'a str,
    pub puestos: i32,
    pub residencia: &'a str,
    pub precio: f64,
    pub tipo: &'a str,
    pub operacion: &'a str,
    pub habitaciones: i32,
    pub banos: i32,
    pub metros: f64,
    pub metros_terreno: f64,
    pub estado: &'a str,
    pub slug: String,
    pub copy_corta: Option<&'a str>,
    pub copy_larga: Option<&'a str>,
    pub copy_modelo: Option<&'a str>,
    pub copy_actualizada_en: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct InmuebleRepository;

impl InmuebleRepository {
    pub async fn create(
        pool: &PgPool,
        nuevo: &NuevoInmueble<'_>,
    ) -> Result<InmuebleRow, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, InmuebleRow>(&format!(
            "INSERT INTO inmuebles (id, titulo, descripcion, ubicacion, puestos, residencia, \
              precio, tipo, operacion, habitaciones, banos, metros, metros_terreno, estado, \
              slug, copy_corta, copy_larga, copy_modelo, copy_actualizada_en) \
              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, \
               $16, $17, $18, $19) \
              RETURNING {COLUMNAS}",
        ))
        .bind(id)
        .bind(nuevo.titulo)
        .bind(nuevo.descripcion)
        .bind(nuevo.ubicacion)
        .bind(nuevo.puestos)
        .bind(nuevo.residencia)
        .bind(nuevo.precio)
        .bind(nuevo.tipo)
        .bind(nuevo.operacion)
        .bind(nuevo.habitaciones)
        .bind(nuevo.banos)
        .bind(nuevo.metros)
        .bind(nuevo.metros_terreno)
        .bind(nuevo.estado)
        .bind(&nuevo.slug)
        .bind(nuevo.copy_corta)
        .bind(nuevo.copy_larga)
        .bind(nuevo.copy_modelo)
        .bind(nuevo.copy_actualizada_en)
        .fetch_one(pool)
        .await
    }

    /// El slug ya existe (violación de UNIQUE 23505)
    #[must_use]
    pub fn es_conflicto_slug(err: &sqlx::Error) -> bool {
        matches!(err, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505"))
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<InmuebleRow>, sqlx::Error> {
        sqlx::query_as::<_, InmuebleRow>(&format!("SELECT {COLUMNAS} FROM inmuebles WHERE id = $1"))
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Detalle público: solo visible si está publicado
    pub async fn find_public_by_slug(
        pool: &PgPool,
        slug: &str,
    ) -> Result<Option<InmuebleRow>, sqlx::Error> {
        sqlx::query_as::<_, InmuebleRow>(&format!(
            "SELECT {COLUMNAS_PUBLICAS} FROM inmuebles WHERE slug = $1 AND publicado = TRUE"
        ))
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    /* [08AA-10] Títulos publicados (id + título) para emparejar el aviso de
     * Facebook del hilo cuando no hay `avisoId` (piloto: siempre). Solo
     * publicados: un borrador jamás cita el precio de un aviso oculto. */
    pub async fn titulos_publicados(pool: &PgPool) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
        sqlx::query_as("SELECT id, titulo FROM inmuebles WHERE publicado = TRUE")
            .fetch_all(pool)
            .await
    }

    pub async fn list_admin(
        pool: &PgPool,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<InmuebleRow>, i64), sqlx::Error> {
        let offset = (page - 1) * per_page;
        let rows = sqlx::query_as::<_, InmuebleRow>(&format!(
            "SELECT {COLUMNAS} FROM inmuebles ORDER BY updated_at DESC LIMIT $1 OFFSET $2",
        ))
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM inmuebles")
            .fetch_one(pool)
            .await?;

        Ok((rows, total))
    }

    /// Aplica los filtros públicos sobre un `QueryBuilder` ya iniciado con el WHERE base
    fn aplicar_filtros(qb: &mut QueryBuilder<'_, Postgres>, f: &FiltrosPublicos) {
        if let Some(tipo) = &f.tipo {
            qb.push(" AND tipo = ");
            qb.push_bind(tipo.clone());
        }
        if let Some(operacion) = &f.operacion {
            qb.push(" AND operacion = ");
            qb.push_bind(operacion.clone());
        }
        if let Some(min) = f.precio_min {
            qb.push(" AND precio >= ");
            qb.push_bind(min);
        }
        if let Some(max) = f.precio_max {
            qb.push(" AND precio <= ");
            qb.push_bind(max);
        }
    }

    pub async fn list_public(
        pool: &PgPool,
        f: &FiltrosPublicos,
    ) -> Result<(Vec<InmuebleRow>, i64), sqlx::Error> {
        let offset = (f.page - 1) * f.per_page;

        let mut qb = QueryBuilder::new(format!(
            "SELECT {COLUMNAS_PUBLICAS} FROM inmuebles WHERE publicado = TRUE"
        ));
        Self::aplicar_filtros(&mut qb, f);
        qb.push(" ORDER BY created_at DESC LIMIT ");
        qb.push_bind(f.per_page);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows = qb.build_query_as::<InmuebleRow>().fetch_all(pool).await?;

        let mut qb_total =
            QueryBuilder::new("SELECT COUNT(*) FROM inmuebles WHERE publicado = TRUE");
        Self::aplicar_filtros(&mut qb_total, f);
        let (total,): (i64,) = qb_total.build_query_as().fetch_one(pool).await?;

        Ok((rows, total))
    }

    /* [08AA-3] B5: 21 params -> struct `ActualizacionInmueble` (ver modelo).
     * El orden de los `.bind` sigue al de las columnas ($1..$18, $19 = id). */
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        cambios: &ActualizacionInmueble<'_>,
    ) -> Result<Option<InmuebleRow>, sqlx::Error> {
        sqlx::query_as::<_, InmuebleRow>(&format!(
            "UPDATE inmuebles \
             SET titulo = COALESCE($1, titulo), \
                 descripcion = COALESCE($2, descripcion), \
                 ubicacion = COALESCE($3, ubicacion), \
                 puestos = COALESCE($4, puestos), \
                 residencia = COALESCE($5, residencia), \
                 precio = COALESCE($6, precio), \
                 tipo = COALESCE($7, tipo), \
                 operacion = COALESCE($8, operacion), \
                 habitaciones = COALESCE($9, habitaciones), \
                 banos = COALESCE($10, banos), \
                 metros = COALESCE($11, metros), \
                 metros_terreno = COALESCE($12, metros_terreno), \
                 estado = COALESCE($13, estado), \
                 copy_corta = COALESCE($14, copy_corta), \
                 copy_larga = COALESCE($15, copy_larga), \
                  copy_modelo = COALESCE($16, copy_modelo), \
                  copy_actualizada_en = COALESCE($17, copy_actualizada_en), \
                  receta = COALESCE($18, receta), \
                  /* [279A-7] Al llegar el dato real se borra su marca «no sé»
                   * de `extras` (el front la borra en local al mismo tiempo).
                   * Quitar una clave ausente es no-op, por eso el ELSE ''. */
                  extras = extras \
                    - CASE WHEN $3 IS NOT NULL AND btrim($3) <> '' THEN 'ubicacion_nose' ELSE '' END \
                    - CASE WHEN $5 IS NOT NULL AND btrim($5) <> '' THEN 'ubicacion_nose' ELSE '' END \
                    - CASE WHEN $4 IS NOT NULL AND $4 > 0 THEN 'puestos_nose' ELSE '' END \
                    - CASE WHEN $11 IS NOT NULL AND $11 > 0 THEN 'metros_nose' ELSE '' END \
                    - CASE WHEN $12 IS NOT NULL AND $12 > 0 THEN 'metros_terreno_nose' ELSE '' END, \
                  updated_at = NOW() \
              WHERE id = $19 \
              RETURNING {COLUMNAS}",
        ))
        .bind(cambios.titulo)
        .bind(cambios.descripcion)
        .bind(cambios.ubicacion)
        .bind(cambios.puestos)
        .bind(cambios.residencia)
        .bind(cambios.precio)
        .bind(cambios.tipo)
        .bind(cambios.operacion)
        .bind(cambios.habitaciones)
        .bind(cambios.banos)
        .bind(cambios.metros)
        .bind(cambios.metros_terreno)
        .bind(cambios.estado)
        .bind(cambios.copy_corta)
        .bind(cambios.copy_larga)
        .bind(cambios.copy_modelo)
        .bind(cambios.copy_actualizada_en)
        .bind(cambios.receta.clone())
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn set_publicado(
        pool: &PgPool,
        id: Uuid,
        publicado: bool,
    ) -> Result<Option<InmuebleRow>, sqlx::Error> {
        sqlx::query_as::<_, InmuebleRow>(&format!(
            "UPDATE inmuebles SET publicado = $1, updated_at = NOW() WHERE id = $2 \
             RETURNING {COLUMNAS}",
        ))
        .bind(publicado)
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM inmuebles WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Fotos de varios inmuebles en una sola query (evita N+1)
    pub async fn fotos_por_inmuebles(
        pool: &PgPool,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<Foto>>, sqlx::Error> {
        let mut mapa: HashMap<Uuid, Vec<Foto>> = HashMap::new();
        if ids.is_empty() {
            return Ok(mapa);
        }

        let fotos = sqlx::query_as::<_, Foto>(
            "SELECT id, inmueble_id, storage_key, orden, origen, created_at \
             FROM fotos WHERE inmueble_id = ANY($1) ORDER BY orden ASC",
        )
        .bind(ids)
        .fetch_all(pool)
        .await?;

        for foto in fotos {
            mapa.entry(foto.inmueble_id).or_default().push(foto);
        }
        Ok(mapa)
    }

    pub async fn add_foto(
        pool: &PgPool,
        inmueble_id: Uuid,
        storage_key: &str,
        orden: Option<i32>,
        origen: &str,
    ) -> Result<Foto, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Foto>(
            "INSERT INTO fotos (id, inmueble_id, storage_key, orden, origen) \
             VALUES ($1, $2, $3, \
              COALESCE($4, (SELECT COALESCE(MAX(orden), -1) + 1 FROM fotos WHERE inmueble_id = $2)), \
              $5) \
             RETURNING id, inmueble_id, storage_key, orden, origen, created_at",
        )
        .bind(id)
        .bind(inmueble_id)
        .bind(storage_key)
        .bind(orden)
        .bind(origen)
        .fetch_one(pool)
        .await
    }

    /* [08AA-4] Borrado de foto sin desfase: si cae una `original`, su
     * `mejorada` hermana (mismo `inmueble_id` + `orden`) cae en la misma
     * transacción y el resto se renumera para mantener `orden` denso. Sin
     * esto el pareo original↔mejorada por `orden` se cruza (Altos del
     * Caroní: 15 mejoradas para 12 originales + 3 duplicadas). Borrar solo
     * la mejorada no toca el original (caso "regenerar"). Devuelve las
     * `storage_key` eliminadas para limpiar disco fuera de la transacción. */
    pub async fn delete_foto_en_cascada(
        pool: &PgPool,
        foto_id: Uuid,
    ) -> Result<Option<Vec<String>>, sqlx::Error> {
        let Some(foto) = Self::find_foto(pool, foto_id).await? else {
            return Ok(None);
        };
        let mut tx = pool.begin().await?;
        let claves: Vec<String> = if foto.origen == "original" {
            let filas: Vec<(String,)> = sqlx::query_as(
                "DELETE FROM fotos WHERE inmueble_id = $1 AND orden = $2 \
                 RETURNING storage_key",
            )
            .bind(foto.inmueble_id)
            .bind(foto.orden)
            .fetch_all(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE fotos SET orden = orden - 1 \
                 WHERE inmueble_id = $1 AND orden > $2",
            )
            .bind(foto.inmueble_id)
            .bind(foto.orden)
            .execute(&mut *tx)
            .await?;
            filas.into_iter().map(|fila| fila.0).collect()
        } else {
            sqlx::query("DELETE FROM fotos WHERE id = $1")
                .bind(foto_id)
                .execute(&mut *tx)
                .await?;
            vec![foto.storage_key]
        };
        /* [249A-1] Invalida la caché de fotos (`?v=<updated_at>`). */
        Self::tocar_inmueble_tx(&mut tx, foto.inmueble_id).await?;
        tx.commit().await?;
        Ok(Some(claves))
    }

    /* [249A-1] Las fotos versionan su URL con `?v=<updated_at>` (front
     * `fotosVisiblesDe`): al cambiar fotos hay que tocar el padre para que
     * la caché del navegador/CDN se invalide sin renombrar ficheros. */
    pub async fn tocar_inmueble(pool: &PgPool, inmueble_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE inmuebles SET updated_at = NOW() WHERE id = $1")
            .bind(inmueble_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Variante de [`Self::tocar_inmueble`] dentro de una transacción abierta.
    /* [08AA-3] Gotcha sqlx 0.8: `&mut Transaction` NO implementa `Executor`
     * (solo `&mut PgConnection`); hay que bajar a la conexión con `as_mut()`. */
    async fn tocar_inmueble_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        inmueble_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE inmuebles SET updated_at = NOW() WHERE id = $1")
            .bind(inmueble_id)
            .execute(tx.as_mut())
            .await?;
        Ok(())
    }

    /* [249A-1] Sitemap: última modificación del conjunto publicado (el
     * detalle es solo-modal, sin URLs con slug: el sitemap declara `/`). */
    pub async fn ultima_modificacion_publica(
        pool: &PgPool,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
        let fila: (Option<chrono::DateTime<chrono::Utc>>,) =
            sqlx::query_as("SELECT MAX(updated_at) FROM inmuebles WHERE publicado = TRUE")
                .fetch_one(pool)
                .await?;
        Ok(fila.0)
    }

    /// Una foto por id (para localizar su archivo en disco al borrar)
    pub async fn find_foto(pool: &PgPool, foto_id: Uuid) -> Result<Option<Foto>, sqlx::Error> {
        sqlx::query_as::<_, Foto>(
            "SELECT id, inmueble_id, storage_key, orden, origen, created_at \
             FROM fotos WHERE id = $1",
        )
        .bind(foto_id)
        .fetch_optional(pool)
        .await
    }

    /* [279A-3] Ficha /ask: leer y guardar `extras` + `precio_minimo`.
     * Lo privado solo sale por aquí (rutas admin con JWT); las vistas
     * públicas usan `Inmueble`, que ni declara el campo. */

    /// Lee la ficha /ask de un inmueble (incluye lo privado)
    pub async fn get_ficha(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<Option<(sqlx::types::Json<serde_json::Value>, Option<f64>)>, sqlx::Error> {
        sqlx::query_as::<_, (sqlx::types::Json<serde_json::Value>, Option<f64>)>(
            "SELECT extras, precio_minimo FROM inmuebles WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Guarda la ficha /ask y devuelve la fila completa
    pub async fn set_ficha(
        pool: &PgPool,
        id: Uuid,
        extras: sqlx::types::Json<serde_json::Value>,
        precio_minimo: Option<f64>,
    ) -> Result<Option<InmuebleRow>, sqlx::Error> {
        sqlx::query_as::<_, InmuebleRow>(&format!(
            /* [279A-7] Con mínimo real se borra su marca «no sé» (con `None`
             * se conserva: es el estado pendiente o la marca recién guardada). */
            "UPDATE inmuebles SET extras = $1 - CASE WHEN $2 IS NOT NULL AND $2 > 0 \
              THEN 'precio_minimo_nose' ELSE '' END, precio_minimo = $2, updated_at = NOW() \
              WHERE id = $3 RETURNING {COLUMNAS}",
        ))
        .bind(extras)
        .bind(precio_minimo)
        .bind(id)
        .fetch_optional(pool)
        .await
    }
}
