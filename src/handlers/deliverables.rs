/* [044A-38 Fase 6] Handlers de entregables: upload multipart, listado y descarga.
 * El upload guarda archivos en disco (uploads/) y registra en phase_deliverables.
 * Seguridad: whitelist MIME, max 10 MB por archivo, max 5 archivos por entrega,
 * detección de extensión doble. */

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::fs;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CreateNotification, DeliverPhaseResponse, OrderStatus, PhaseDeliverablesResponse, PhaseStatus,
    UserRole, ALLOWED_MIME_TYPES, MAX_FILES_PER_DELIVERY, MAX_FILE_SIZE, NOTIF_PHASE_DELIVERED,
};
use crate::repositories::{
    CreateDeliverableParams, DeliverableRepository, OrderRepository, UserRepository,
};
use crate::AppState;

/// Directorio base para uploads (relativo al CWD del servidor)
const UPLOAD_DIR: &str = "uploads/deliverables";

/* ============================================================
ENTREGAR FASE CON ARCHIVOS (multipart)
============================================================ */

/// Empleado entrega una fase con archivos adjuntos (multipart/form-data).
/// Campos: `notes` (text, opcional), `files` (file, hasta 5).
#[utoipa::path(
    post,
    path = "/api/orders/{order_id}/phases/{phase_number}/deliver",
    params(
        ("order_id" = Uuid, Path, description = "ID de la orden"),
        ("phase_number" = i32, Path, description = "Número de fase"),
    ),
    responses(
        (status = 200, description = "Fase entregada con archivos", body = DeliverPhaseResponse),
        (status = 400, description = "Datos inválidos o estado no permite entrega"),
        (status = 401, description = "No autorizado"),
        (status = 403, description = "Solo el empleado asignado"),
    ),
    security(("bearer_auth" = [])),
    tag = "deliverables"
)]
pub async fn deliver_phase_with_files(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((order_id, phase_number)): Path<(Uuid, i32)>,
    mut multipart: Multipart,
) -> Result<Json<DeliverPhaseResponse>, AppError> {
    auth.require_role(&[UserRole::Employee, UserRole::Admin])?;

    /* Validar orden y permisos */
    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    if auth.effective_role != UserRole::Admin && order.assigned_employee_id != Some(auth.user_id) {
        return Err(AppError::Forbidden(
            "Solo el empleado asignado puede entregar".into(),
        ));
    }

    /* Validar fase */
    let phase = OrderRepository::find_phase_by_number(&state.pool, order_id, phase_number)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Fase {phase_number} no encontrada")))?;

    match phase.status {
        PhaseStatus::InProgress | PhaseStatus::Paid | PhaseStatus::RevisionRequested => {}
        _ => {
            return Err(AppError::BadRequest(format!(
                "No se puede entregar una fase en estado {:?}",
                phase.status
            )));
        }
    }

    /* Calcular revision_number */
    let current_rev = DeliverableRepository::current_revision_number(&state.pool, phase.id).await?;
    let revision_number = current_rev + 1;

    /* Crear directorio de uploads (join via helper anti-traversal). */
    let upload_path = crate::util::join_write_path(
        std::path::Path::new(UPLOAD_DIR),
        &[&order_id.to_string(), &phase_number.to_string()],
    )?;
    fs::create_dir_all(&upload_path)
        .await
        .map_err(|e| AppError::Internal(format!("Error creando directorio: {e}")))?;

    /* Procesar multipart: extraer notas y archivos */
    let ctx = MultipartContext {
        pool: &state.pool,
        upload_path: &upload_path,
        phase_id: phase.id,
        uploaded_by: auth.user_id,
        order_id,
        phase_number,
        revision_number,
    };
    let (_notes, saved_files) = process_multipart_files(&mut multipart, &ctx).await?;

    /* Marcar fase como entregada */
    OrderRepository::deliver_phase(&state.pool, phase.id).await?;

    /* [104A-38] Notificar al cliente que la fase fue entregada */
    let _ = state
        .notification_hub
        .notify(CreateNotification {
            user_id: order.client_id,
            notification_type: NOTIF_PHASE_DELIVERED.to_string(),
            title: format!(
                "Entrega lista — Orden #{}, Fase {}",
                order.order_number, phase_number
            ),
            body: Some("Revisa los archivos entregados y aprueba o solicita revisión".to_string()),
            link: Some(format!("/panel?seccion=ordenes&id={}", order.id)),
            reference_type: Some("order".to_string()),
            reference_id: Some(order.id),
        })
        .await;

    /* [311A-1] Email al cliente notificando fase entregada (non-fatal) */
    if let Some(ref email_cfg) = state.email_config {
        if let Ok(Some(client_email)) =
            UserRepository::get_email(&state.pool, order.client_id).await
        {
            let cfg = email_cfg.clone();
            let pool = state.pool.clone();
            let onum = order.order_number;
            let oid = order.id;
            let cname = UserRepository::get_display_name(&state.pool, order.client_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "Cliente".to_string());
            let ptitle = phase.title.clone();
            let site_url =
                std::env::var("SITE_URL").unwrap_or_else(|_| "https://nakomi.studio".to_string());
            tokio::spawn(async move {
                crate::services::EmailService::send_phase_delivered_client(
                    &cfg,
                    &pool,
                    &client_email,
                    &cname,
                    onum,
                    &ptitle,
                    &site_url,
                    oid,
                )
                .await;
            });
        }
    }

    /* Actualizar estado de orden si estaba en InProgress */
    if order.status == OrderStatus::InProgress {
        OrderRepository::update_order_status(&state.pool, order_id, OrderStatus::UnderReview)
            .await?;
    }

    Ok(Json(DeliverPhaseResponse {
        phase_id: phase.id,
        revision_number,
        deliverables: saved_files,
    }))
}

/* ============================================================
LISTAR ENTREGABLES DE UNA FASE
============================================================ */

/// Listar entregables de una fase con estado de aprobación
#[utoipa::path(
    get,
    path = "/api/orders/{order_id}/phases/{phase_number}/deliverables",
    params(
        ("order_id" = Uuid, Path, description = "ID de la orden"),
        ("phase_number" = i32, Path, description = "Número de fase"),
    ),
    responses(
        (status = 200, description = "Lista de entregables", body = PhaseDeliverablesResponse),
        (status = 401, description = "No autorizado"),
        (status = 404, description = "Fase no encontrada"),
    ),
    security(("bearer_auth" = [])),
    tag = "deliverables"
)]
pub async fn list_deliverables(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((order_id, phase_number)): Path<(Uuid, i32)>,
) -> Result<Json<PhaseDeliverablesResponse>, AppError> {
    /* Verificar acceso a la orden */
    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    verify_order_access(&order, &auth)?;

    let phase = OrderRepository::find_phase_by_number(&state.pool, order_id, phase_number)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Fase {phase_number} no encontrada")))?;

    let deliverables = DeliverableRepository::list_by_phase(&state.pool, phase.id).await?;

    Ok(Json(PhaseDeliverablesResponse {
        deliverables,
        approval_status: format!("{:?}", phase.status),
        revisions_used: phase.revisions_used,
        max_revisions: phase.max_revisions,
    }))
}

/* ============================================================
DESCARGAR ENTREGABLE
============================================================ */

/// Descargar un archivo entregable
#[utoipa::path(
    get,
    path = "/api/deliverables/{deliverable_id}/download",
    params(("deliverable_id" = Uuid, Path, description = "ID del entregable")),
    responses(
        (status = 200, description = "Archivo descargado"),
        (status = 401, description = "No autorizado"),
        (status = 404, description = "Archivo no encontrado"),
    ),
    security(("bearer_auth" = [])),
    tag = "deliverables"
)]
pub async fn download_deliverable(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(deliverable_id): Path<Uuid>,
) -> Result<Response, AppError> {
    let deliverable = DeliverableRepository::find_by_id(&state.pool, deliverable_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Entregable no encontrado".into()))?;

    /* Verificar acceso: obtener la fase → orden → verificar */
    let order_id: Option<Uuid> = OrderRepository::phase_order_id(&state.pool, deliverable.phase_id)
        .await
        .map_err(AppError::Database)?;

    let order_id =
        order_id.ok_or_else(|| AppError::NotFound("Fase asociada no encontrada".into()))?;

    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    verify_order_access(&order, &auth)?;

    /* [064A-73] Validar path traversal: el archivo debe estar dentro de uploads/ */
    let file_path = format!(".{}", deliverable.file_url);
    let canonical = std::path::Path::new(&file_path)
        .canonicalize()
        .map_err(|_| AppError::NotFound("Archivo no encontrado en disco".into()))?;
    let base_dir = std::path::Path::new("uploads")
        .canonicalize()
        .map_err(|_| AppError::Internal("Directorio uploads no existe".into()))?;
    if !canonical.starts_with(&base_dir) {
        return Err(AppError::BadRequest("Ruta de archivo inválida".into()));
    }

    let data = fs::read(&file_path)
        .await
        .map_err(|_| AppError::NotFound("Archivo no encontrado en disco".into()))?;

    let content_type = deliverable
        .mime_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    let file_name = &deliverable.file_name;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{file_name}\""),
        )
        .body(Body::from(data))
        .map_err(|e| AppError::Internal(format!("Error construyendo respuesta: {e}")))?;

    Ok(response)
}

/* ============================================================
RUTAS
============================================================ */

/* [204A-15] DefaultBodyLimit para que axum no rechace antes del handler.
 * MAX_FILE_SIZE = 10 MB × MAX_FILES_PER_DELIVERY = 5 → hasta 50 MB por request. */
#[allow(clippy::cast_possible_truncation)] /* 50 MB: safe on 64-bit systems */
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/orders/:order_id/phases/:phase_number/deliver",
            post(deliver_phase_with_files),
        )
        .route(
            "/orders/:order_id/phases/:phase_number/deliverables",
            get(list_deliverables),
        )
        .route(
            "/deliverables/:deliverable_id/download",
            get(download_deliverable),
        )
        .layer(DefaultBodyLimit::max(
            (MAX_FILE_SIZE * MAX_FILES_PER_DELIVERY as u64) as usize,
        ))
}

/* ============================================================
UTILIDADES
============================================================ */

/// Verifica que el usuario tiene acceso a una orden
/// [074A-50] Admin real siempre tiene acceso, `effective_role` solo afecta UI.
fn verify_order_access(order: &crate::models::Order, auth: &AuthUser) -> Result<(), AppError> {
    if auth.role == UserRole::Admin {
        return Ok(());
    }
    match auth.effective_role {
        UserRole::Admin => {}
        UserRole::Client => {
            if order.client_id != auth.user_id {
                return Err(AppError::Forbidden("No tienes acceso a esta orden".into()));
            }
        }
        UserRole::Employee => {
            if order.assigned_employee_id != Some(auth.user_id) {
                return Err(AppError::Forbidden("No tienes acceso a esta orden".into()));
            }
        }
    }
    Ok(())
}

/* [174A-2] Contexto para procesamiento multipart — agrupa parámetros de la fase/orden */
struct MultipartContext<'a> {
    pool: &'a sqlx::PgPool,
    upload_path: &'a std::path::Path,
    phase_id: Uuid,
    uploaded_by: Uuid,
    order_id: Uuid,
    phase_number: i32,
    revision_number: i32,
}

/// Procesa campos multipart: extrae `notes` (texto) y `files` (binarios).
/// Valida MIME, tamaño, extensiones dobles. Guarda en disco y BD.
async fn process_multipart_files(
    multipart: &mut Multipart,
    ctx: &MultipartContext<'_>,
) -> Result<(Option<String>, Vec<crate::models::PhaseDeliverable>), AppError> {
    let mut notes: Option<String> = None;
    let mut saved_files: Vec<crate::models::PhaseDeliverable> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Error leyendo multipart: {e}")))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "notes" {
            let text = field
                .text()
                .await
                .map_err(|e| AppError::BadRequest(format!("Error leyendo notas: {e}")))?;
            if !text.is_empty() {
                notes = Some(text);
            }
            continue;
        }

        if field_name == "files" {
            if saved_files.len() >= MAX_FILES_PER_DELIVERY {
                return Err(AppError::BadRequest(format!(
                    "Máximo {MAX_FILES_PER_DELIVERY} archivos por entrega"
                )));
            }

            let original_name = field.file_name().unwrap_or("archivo").to_string();
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            if !ALLOWED_MIME_TYPES.contains(&content_type.as_str()) {
                return Err(AppError::BadRequest(format!(
                    "Tipo de archivo no permitido: {content_type}"
                )));
            }

            if has_suspicious_extension(&original_name) {
                return Err(AppError::BadRequest(
                    "Nombre de archivo sospechoso: extensión doble detectada".into(),
                ));
            }

            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Error leyendo archivo: {e}")))?;

            /* [064A-73] Validar magic bytes: el contenido real debe coincidir con el
             * MIME declarado por el cliente. Previene upload de ejecutables disfrazados. */
            if !validate_magic_bytes(&data, &content_type) {
                return Err(AppError::BadRequest(format!(
                    "El contenido del archivo no coincide con el tipo declarado: {content_type}"
                )));
            }

            #[allow(clippy::cast_possible_truncation)]
            let file_size = data.len() as u64;
            if file_size > MAX_FILE_SIZE {
                return Err(AppError::BadRequest(format!(
                    "Archivo demasiado grande: {file_size} bytes (máx {} MB)",
                    MAX_FILE_SIZE / 1024 / 1024
                )));
            }

            let safe_name = sanitize_filename(&original_name);
            let unique_name = format!("{}-{}-{safe_name}", ctx.phase_id, ctx.revision_number);
            /* [259A-2] Join via helper anti-traversal (doble capa sobre sanitize). */
            let file_path = crate::util::join_write_path(ctx.upload_path, &[&unique_name])?;

            fs::write(&file_path, &data)
                .await
                .map_err(|e| AppError::Internal(format!("Error guardando archivo: {e}")))?;

            /* [064A-73] Log de upload para auditoría */
            tracing::info!(
                user_id = %ctx.uploaded_by,
                file = %original_name,
                mime = %content_type,
                size = file_size,
                "Archivo subido"
            );

            let file_url = format!(
                "/uploads/deliverables/{}/{}/{unique_name}",
                ctx.order_id, ctx.phase_number
            );

            #[allow(clippy::cast_possible_wrap)]
            let deliverable = DeliverableRepository::create(
                ctx.pool,
                CreateDeliverableParams {
                    phase_id: ctx.phase_id,
                    uploaded_by: ctx.uploaded_by,
                    file_name: &original_name,
                    file_url: &file_url,
                    file_size_bytes: Some(file_size as i64),
                    mime_type: Some(&content_type),
                    revision_number: ctx.revision_number,
                    notes: notes.as_deref(),
                },
            )
            .await?;

            saved_files.push(deliverable);
        }
    }

    Ok((notes, saved_files))
}

/// Sanitiza nombre de archivo: quita path traversal y caracteres peligrosos
fn sanitize_filename(name: &str) -> String {
    let name = name
        .replace(['/', '\\'], "")
        .replace("..", "")
        .replace(char::is_control, "");

    if name.is_empty() {
        "archivo".to_string()
    } else {
        name
    }
}

/// Detecta extensiones dobles sospechosas (ej: script.exe.pdf, virus.bat.jpg)
fn has_suspicious_extension(name: &str) -> bool {
    let dangerous = [
        ".exe", ".bat", ".cmd", ".com", ".msi", ".scr", ".pif", ".vbs", ".js", ".ws", ".wsf",
        ".ps1", ".sh",
    ];
    let lower = name.to_lowercase();
    for ext in &dangerous {
        /* Buscar extensión peligrosa que no esté al final (hidden extension) */
        if let Some(pos) = lower.find(ext) {
            let after = pos + ext.len();
            if after < lower.len() && lower.as_bytes().get(after) == Some(&b'.') {
                return true;
            }
        }
    }
    false
}

/* [064A-73] Valida que los primeros bytes del archivo coincidan con el MIME declarado.
 * Evita que un atacante suba un .exe renombrado a .pdf.
 * Para application/octet-stream no se puede validar — se acepta siempre.
 * Para SVG se busca la etiqueta <svg en los primeros 512 bytes (es XML). */
fn validate_magic_bytes(data: &[u8], claimed_mime: &str) -> bool {
    match claimed_mime {
        "application/pdf" => data.starts_with(b"%PDF"),
        "application/msword" => data.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            data.starts_with(&[0x50, 0x4B, 0x03, 0x04])
        }
        "image/png" => data.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "image/jpeg" => data.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/gif" => data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a"),
        "image/svg+xml" => {
            let check_len = data.len().min(512);
            let prefix = String::from_utf8_lossy(&data[..check_len]);
            prefix.contains("<svg")
        }
        "image/webp" => data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP",
        "video/mp4" => data.len() >= 8 && &data[4..8] == b"ftyp",
        "application/zip" => data.starts_with(&[0x50, 0x4B]),
        "application/x-rar-compressed" => data.starts_with(b"Rar!"),
        "application/x-7z-compressed" => data.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]),
        /* octet-stream es genérico, no se puede validar por contenido */
        "application/octet-stream" => true,
        /* MIME desconocido que pasó la whitelist: rechazar por precaución */
        _ => false,
    }
}
