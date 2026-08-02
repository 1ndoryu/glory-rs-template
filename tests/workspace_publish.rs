//! Tests de integración del guard de coherencia de releases del workspace.
//! [028A-11] Verifican: validación estructural del árbol, refs de recursos
//! rotos (422 con detalle), cálculo del summary/diff y contrato de listado.
//! Necesitan `DATABASE_URL` apuntando a la BD local (glory_backend_wandorius).

use std::sync::OnceLock;

use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use glory_backend::services::workspace_svc::WorkspaceService;

/// Serializa los tests que publican: cada uno calcula version = max+1, y dos
/// publicaciones concurrentes colisionarían en la versión.
static PUBLISH_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
fn publish_lock() -> &'static tokio::sync::Mutex<()> {
    PUBLISH_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

struct TestContext {
    pool: sqlx::PgPool,
    admin_id: Uuid,
    releases_created: Vec<i32>,
    resources_created: Vec<Uuid>,
}

impl TestContext {
    async fn new() -> Self {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL requerido para tests");
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(&database_url)
            .await
            .expect("BD disponible");

        let admin_id = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, 'test-hash')")
            .bind(admin_id)
            .bind(format!("publish-{admin_id}@example.invalid"))
            .execute(&pool)
            .await
            .expect("admin de prueba creado");

        Self {
            pool,
            admin_id,
            releases_created: Vec::new(),
            resources_created: Vec::new(),
        }
    }

    async fn create_resource(&mut self, editorial: &str, visibility: &str) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO resources (id, kind, title, editorial, visibility, lifecycle) \
             VALUES ($1, 'article', $2, $3, $4, 'active')",
        )
        .bind(id)
        .bind("Artículo de prueba")
        .bind(editorial)
        .bind(visibility)
        .execute(&self.pool)
        .await
        .expect("recurso de prueba creado");
        self.resources_created.push(id);
        id
    }

    async fn cleanup(self) {
        for version in &self.releases_created {
            sqlx::query("DELETE FROM notifications WHERE release_version = $1")
                .bind(version)
                .execute(&self.pool)
                .await
                .expect("notificaciones limpiadas");
            sqlx::query("DELETE FROM workspace_releases WHERE version = $1")
                .bind(version)
                .execute(&self.pool)
                .await
                .expect("release limpiada");
        }
        for id in &self.resources_created {
            sqlx::query("DELETE FROM resources WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await
                .expect("recurso limpiado");
        }
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(self.admin_id)
            .execute(&self.pool)
            .await
            .expect("usuario limpiado");
    }
}

/// Árbol mínimo válido: un folder con un recurso público, una app raíz y un
/// shortcut opcional. Sin publicLocator para no acoplar al contrato de overlay.
fn valid_tree(resource_ref: Option<Uuid>) -> serde_json::Value {
    let mut nodes = serde_json::Map::new();
    nodes.insert(
        "documentos".into(),
        json!({
            "id": "documentos",
            "parentId": "desktop",
            "type": "folder",
            "label": "Documentos",
            "position": { "col": 0, "row": 0 }
        }),
    );
    nodes.insert(
        "about".into(),
        json!({
            "id": "about",
            "parentId": "desktop",
            "type": "app",
            "label": "Acerca de",
            "position": { "col": 1, "row": 0 }
        }),
    );
    if let Some(ref_id) = resource_ref {
        nodes.insert(
            "recurso".into(),
            json!({
                "id": "recurso",
                "parentId": "documentos",
                "type": "resource",
                "label": "Artículo público",
                "refId": ref_id
            }),
        );
    }
    json!({ "nodes": nodes })
}

#[tokio::test]
async fn publish_accepts_valid_tree_and_computes_summary() {
    let _guard = publish_lock().lock().await;
    let mut ctx = TestContext::new().await;

    let resource_id = ctx.create_resource("ready", "public").await;
    let tree = valid_tree(Some(resource_id));

    let result = WorkspaceService::publish(&ctx.pool, tree.clone(), ctx.admin_id).await;
    let release = result.expect("release válida publicada");
    ctx.releases_created.push(release.version);

    /* diff contra la release v3 existente: nada de v3 está en nuestro árbol
     * mínimo, así que todo aparece en removed y lo nuestro en added. */
    assert!(release.version > 3, "version debe continuar tras v3");
    assert_eq!(release.diff_from, Some(3));
    let summary = release.summary.as_object().expect("summary objeto");
    assert!(summary["added"]
        .as_array()
        .unwrap()
        .contains(&json!("documentos")));
    assert!(summary["added"]
        .as_array()
        .unwrap()
        .contains(&json!("about")));
    assert!(summary["added"]
        .as_array()
        .unwrap()
        .contains(&json!("recurso")));
    assert!(summary["removed"]
        .as_array()
        .unwrap()
        .contains(&json!("documentos-imagenes")));
    assert_eq!(summary["nodeCount"], json!(3));

    /* Contrato de lectura: activo = máximo, listado desc, get por versión. */
    let active = WorkspaceService::get_active_release(&ctx.pool)
        .await
        .expect("release activa");
    assert_eq!(active.version, release.version);

    let by_version = WorkspaceService::get_release_by_version(&ctx.pool, release.version)
        .await
        .expect("release por versión");
    assert_eq!(by_version.version, release.version);

    let list = WorkspaceService::list_releases(&ctx.pool)
        .await
        .expect("listado de releases");
    let versions: Vec<i32> = list.iter().map(|r| r.version).collect();
    assert_eq!(versions.first(), Some(&release.version));
    assert!(
        versions.windows(2).all(|w| w[0] > w[1]),
        "orden descendente"
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn publish_rejects_unknown_resource_ref_with_details() {
    let _guard = publish_lock().lock().await;
    let ctx = TestContext::new().await;

    /* UUID inexistente: no hay recurso en BD. */
    let ghost = Uuid::new_v4();
    let tree = valid_tree(Some(ghost));

    let err = WorkspaceService::publish(&ctx.pool, tree, ctx.admin_id)
        .await
        .expect_err("debe rechazar ref inexistente");
    match err {
        glory_backend::errors::AppError::ValidationDetails { message, details } => {
            assert!(message.contains("recursos no publicables"));
            let broken = details["brokenRefs"].as_array().expect("brokenRefs lista");
            assert_eq!(broken.len(), 1);
            assert_eq!(broken[0]["refId"], json!(ghost.to_string()));
            assert_eq!(broken[0]["id"], json!("recurso"));
        }
        other => panic!("esperaba ValidationDetails, obtuve: {other:?}"),
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn publish_rejects_draft_or_private_resource_ref() {
    let _guard = publish_lock().lock().await;
    let mut ctx = TestContext::new().await;

    /* Existe pero no es publicable (draft). */
    let draft_id = ctx.create_resource("draft", "public").await;
    let tree = valid_tree(Some(draft_id));
    let err = WorkspaceService::publish(&ctx.pool, tree, ctx.admin_id)
        .await
        .expect_err("debe rechazar recurso draft");
    assert!(matches!(
        err,
        glory_backend::errors::AppError::ValidationDetails { .. }
    ));

    /* Existe pero privado. */
    let private_id = ctx.create_resource("ready", "private").await;
    let tree = valid_tree(Some(private_id));
    let err = WorkspaceService::publish(&ctx.pool, tree, ctx.admin_id)
        .await
        .expect_err("debe rechazar recurso privado");
    assert!(matches!(
        err,
        glory_backend::errors::AppError::ValidationDetails { .. }
    ));

    ctx.cleanup().await;
}

#[tokio::test]
async fn publish_rejects_structural_issues() {
    let _guard = publish_lock().lock().await;
    let ctx = TestContext::new().await;

    /* Ciclo: b cuelga de a y a cuelga de b. */
    let cycle = json!({
        "nodes": {
            "a": { "id": "a", "parentId": "b", "type": "folder", "label": "A" },
            "b": { "id": "b", "parentId": "a", "type": "folder", "label": "B" }
        }
    });
    let err = WorkspaceService::publish(&ctx.pool, cycle, ctx.admin_id)
        .await
        .expect_err("debe rechazar ciclo");
    assert!(matches!(err, glory_backend::errors::AppError::Validation(m) if m.contains("ciclo")));

    /* parentId inexistente. */
    let orphan = json!({
        "nodes": {
            "a": { "id": "a", "parentId": "no-existe", "type": "folder", "label": "A" }
        }
    });
    let err = WorkspaceService::publish(&ctx.pool, orphan, ctx.admin_id)
        .await
        .expect_err("debe rechazar parentId inexistente");
    assert!(matches!(
        err,
        glory_backend::errors::AppError::Validation(m) if m.contains("parentId inexistente")
    ));

    /* Tipo no válido. */
    let bad_type = json!({
        "nodes": {
            "a": { "id": "a", "parentId": "desktop", "type": "widget", "label": "A" }
        }
    });
    let err = WorkspaceService::publish(&ctx.pool, bad_type, ctx.admin_id)
        .await
        .expect_err("debe rechazar tipo no válido");
    assert!(matches!(
        err,
        glory_backend::errors::AppError::Validation(_)
    ));

    /* Más de 500 nodos. */
    let mut nodes = serde_json::Map::new();
    for i in 0..501 {
        nodes.insert(
            format!("n{i}"),
            json!({ "id": format!("n{i}"), "parentId": "desktop", "type": "folder", "label": format!("N{i}") }),
        );
    }
    let too_many = json!({ "nodes": nodes });
    let err = WorkspaceService::publish(&ctx.pool, too_many, ctx.admin_id)
        .await
        .expect_err("debe rechazar árbol con más de 500 nodos");
    assert!(matches!(
        err,
        glory_backend::errors::AppError::Validation(m) if m.contains("límite de 500")
    ));

    ctx.cleanup().await;
}
