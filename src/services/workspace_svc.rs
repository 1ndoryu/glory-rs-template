use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::workspace::{validate_release_tree, WorkspaceRelease, WorkspaceReleasePublic};
use crate::models::workspace_overlay::validate_public_locators_in_tree;
use crate::repositories::notification_repo::NotificationRepository;
use crate::repositories::resource_repo::ResourceRepository;
use crate::repositories::workspace_repo::WorkspaceRepository;

pub struct WorkspaceService;

/// Recurso roto detectado al validar un release (para el 422 con detalle).
/// [297A-58] camelCase: el contrato del detalle usa `refId` (los DTOs del API
/// no exponen `snake_case`; sin `rename_all` el test y el frontend verían `null`).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BrokenResourceRef {
    /// id del nodo en el árbol del release
    id: String,
    /// uuid del recurso referenciado que no es publicable
    ref_id: Uuid,
    /// label del nodo (para localizar visualmente)
    label: String,
}

impl WorkspaceService {
    /// Obtener el release activo (público).
    pub async fn get_active_release(pool: &PgPool) -> Result<WorkspaceReleasePublic, AppError> {
        WorkspaceRepository::get_latest(pool)
            .await?
            .map(WorkspaceReleasePublic::from)
            .ok_or_else(|| AppError::NotFound("No hay releases publicados".into()))
    }

    /// Obtener un release por versión (público).
    pub async fn get_release_by_version(
        pool: &PgPool,
        version: i32,
    ) -> Result<WorkspaceReleasePublic, AppError> {
        WorkspaceRepository::get_by_version(pool, version)
            .await?
            .map(WorkspaceReleasePublic::from)
            .ok_or_else(|| AppError::NotFound(format!("Release v{version} no encontrado")))
    }

    /// Listar todos los releases (admin — incluye historial).
    pub async fn list_releases(pool: &PgPool) -> Result<Vec<WorkspaceRelease>, AppError> {
        Ok(WorkspaceRepository::list_releases(pool).await?)
    }

    /// Publicar un nuevo release (admin).
    /// [297A-11 §9.2] Publicación transaccional a release inmutable.
    /// [028A-11] Guard de coherencia: valida estructura del árbol (tipos,
    /// ciclos, parentId, límites), publicLocators y refs de recursos contra la
    /// BD (deben ser `active + ready + public`). El 422 devuelve la lista de
    /// refs rotos para que el panel admin las corrija antes de publicar.
    pub async fn publish(
        pool: &PgPool,
        tree: serde_json::Value,
        published_by: Uuid,
    ) -> Result<WorkspaceRelease, AppError> {
        validate_release_tree(&tree).map_err(AppError::Validation)?;

        validate_public_locators_in_tree(&tree)
            .map_err(|message| AppError::Validation(message.to_string()))?;

        let broken_refs = collect_broken_resource_refs(pool, &tree).await?;
        if !broken_refs.is_empty() {
            return Err(AppError::ValidationDetails {
                message: format!(
                    "El release referencia {} recursos no publicables",
                    broken_refs.len()
                ),
                details: serde_json::json!({ "brokenRefs": broken_refs }),
            });
        }

        /* Release anterior para el diff auditable */
        let previous = WorkspaceRepository::get_latest(pool).await?;
        let next_version = previous.as_ref().map_or(0, |r| r.version) + 1;
        let summary = compute_release_summary(&tree, previous.as_ref());
        let diff_from = previous.as_ref().map(|r| r.version);

        let mut tx = pool.begin().await?;

        let release = WorkspaceRepository::create(
            &mut tx,
            next_version,
            &tree,
            Some(published_by),
            &summary,
            diff_from,
        )
        .await?;

        NotificationRepository::create_release_notification(&mut tx, release.version, published_by)
            .await?;

        tx.commit().await?;
        Ok(release)
    }
}

/// Recorre el árbol y devuelve las refs de recursos (type resource/shortcut)
/// cuyo `refId` es UUID pero no es publicable en BD. Los refId no-UUID (p. ej.
/// ids de app del shell) no se validan aquí: no tienen registro en `resources`.
async fn collect_broken_resource_refs(
    pool: &PgPool,
    tree: &serde_json::Value,
) -> Result<Vec<BrokenResourceRef>, AppError> {
    let Some(nodes) = tree.get("nodes").and_then(serde_json::Value::as_object) else {
        return Ok(Vec::new());
    };

    let mut refs: Vec<(Uuid, BrokenResourceRef)> = Vec::new();
    for node in nodes.values() {
        let Some(node) = node.as_object() else {
            continue;
        };
        let Some(node_type) = node.get("type").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if !matches!(node_type, "resource" | "shortcut") {
            continue;
        }
        let Some(ref_id) = node.get("refId").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Ok(uuid) = Uuid::parse_str(ref_id) else {
            continue;
        };
        let id = node
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let label = node
            .get("label")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(id);
        refs.push((
            uuid,
            BrokenResourceRef {
                id: id.to_string(),
                ref_id: uuid,
                label: label.to_string(),
            },
        ));
    }

    let uuids: Vec<Uuid> = refs.iter().map(|(uuid, _)| *uuid).collect();
    let broken_uuids = ResourceRepository::find_broken_public_refs(pool, &uuids).await?;
    Ok(refs
        .into_iter()
        .filter(|(uuid, _)| broken_uuids.contains(uuid))
        .map(|(_, broken)| broken)
        .collect())
}

/// Calcula el diff auditable entre el árbol nuevo y la release anterior.
/// Shape: `{ added: [ids], removed: [ids], modified: [ids], nodeCount: n }`.
/// Para la primera release, `added` = todos los nodos. Un nodo se marca
/// `modified` si su JSON completo cambió (posición, label, refId, etc.).
fn compute_release_summary(
    tree: &serde_json::Value,
    previous: Option<&WorkspaceRelease>,
) -> serde_json::Value {
    let empty = serde_json::Map::new();
    let new_nodes = tree
        .get("nodes")
        .and_then(serde_json::Value::as_object)
        .unwrap_or(&empty);
    let node_count = new_nodes.len();

    let Some(prev_nodes) = previous
        .and_then(|r| r.tree.get("nodes"))
        .and_then(serde_json::Value::as_object)
    else {
        return serde_json::json!({
            "added": new_nodes.keys().cloned().collect::<Vec<_>>(),
            "removed": [],
            "modified": [],
            "nodeCount": node_count,
        });
    };

    let mut added: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();
    for (id, node) in new_nodes {
        match prev_nodes.get(id) {
            None => added.push(id.clone()),
            Some(prev) if prev != node => modified.push(id.clone()),
            Some(_) => {}
        }
    }
    let removed: Vec<String> = prev_nodes
        .keys()
        .filter(|id| !new_nodes.contains_key(*id))
        .cloned()
        .collect();

    serde_json::json!({
        "added": added,
        "removed": removed,
        "modified": modified,
        "nodeCount": node_count,
    })
}
