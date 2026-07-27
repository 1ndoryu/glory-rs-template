/* [065A-4] Dry-run de sincronizacion BDP.
 * Valida datos reales de BDP y usa CreateOrder en modo OnlyCheck para probar
 * el payload de comanda sin escribir clientes, comandas, pagos ni facturas en BDP. */

use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::models::ConfiguracionRestaurante;
use crate::repositories::BdpArticleMapRepository;
use crate::services::bdp_weblink::{BdpWeblinkClient, BdpWeblinkError};
use crate::services::bdp_weblink_catalog::{
    BdpCreateOrderRequest, BdpDepartmentsExportFromProfileRequest, BdpGetEmployeeRequest,
    BdpGetPosArticlesRequest, BdpGetPosEmployeesRequest, BdpGetPosRequest, BdpGetPosTendersRequest,
};

const BDP_DRY_RUN_MARKET_ID: i32 = 9_901;
const BDP_DRY_RUN_PAGE_SIZE: i32 = 10;

#[derive(Debug, Serialize, ToSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct BdpSyncDryRunResponse {
    pub configurado: bool,
    pub sync_habilitado: bool,
    pub escritura_real: bool,
    pub listo_para_sincronizar: bool,
    pub mensaje: String,
    pub checks: Vec<BdpSyncDryRunCheck>,
    pub payload_preview: Option<Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BdpSyncDryRunCheck {
    pub nombre: String,
    pub endpoint: String,
    pub ok: bool,
    pub mensaje: String,
    pub cantidad: Option<usize>,
    pub muestra: Option<String>,
}

struct BdpDryRunArticle {
    id: i64,
    name: String,
    price: f64,
    vat_pct: f64,
}

pub struct BdpSyncPreflightService;

impl BdpSyncPreflightService {
    pub async fn execute(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
    ) -> BdpSyncDryRunResponse {
        let mut response = BdpSyncDryRunResponse::new(config.bdp_sync_enabled);
        if !bdp_configurado(config) {
            response.mensaje = "BDP no esta configurado".to_string();
            return response;
        }
        response.configurado = true;

        /* [F2.7] Check de mapeo de artículos — verificar que hay mapeos en bdp_article_map */
        Self::check_article_mapping(pool, user_id, config, &mut response).await;

        let client = BdpWeblinkClient::new(config);
        Self::check_health(&client, &mut response).await;
        Self::check_version(&client, &mut response).await;

        let pos = Self::capture(
            &mut response,
            "Terminal POS",
            "/API/POS/Get",
            client.get_pos(&BdpGetPosRequest {
                id: config.bdp_pos_id,
            }),
            &["POS"],
        )
        .await;

        let employee = Self::capture(
            &mut response,
            "Empleado BDP",
            "/API/Employee/Get",
            client.get_employee(&BdpGetEmployeeRequest {
                id: config.bdp_employee_id,
            }),
            &["Employee"],
        )
        .await;

        let pos_employees = Self::capture(
            &mut response,
            "Empleados del POS",
            "/API/POS/Employees/Get",
            client.get_pos_employees(&BdpGetPosEmployeesRequest {
                pos_id: config.bdp_pos_id,
            }),
            &["Employees"],
        )
        .await;
        Self::check_employee_is_allowed(config, &pos_employees, &mut response);

        let tenders = Self::capture(
            &mut response,
            "Formas de pago del POS",
            "/API/Tenders/GetPOSList",
            client.get_pos_tenders(&BdpGetPosTendersRequest {
                pos_id: config.bdp_pos_id,
            }),
            &["TenderList", "Tenders"],
        )
        .await;

        /* [F3.4] Validar que los tenders configurados existen en el POS */
        Self::check_tender_mapping(config, tenders.as_ref(), &mut response);

        /* [F3.5] Validar que los order types configurados son válidos */
        Self::check_order_type_mapping(config, &mut response);

        let departments = Self::capture(
            &mut response,
            "Departamentos del perfil",
            "/API/Departments/ExportFromProfile",
            client.export_departments_from_profile(&BdpDepartmentsExportFromProfileRequest {
                profile_id: config.bdp_items_profile_id,
            }),
            &["Departamentos", "Departments"],
        )
        .await;

        let articles = Self::capture(
            &mut response,
            "Articulos del perfil",
            "/API/Articles/GetPOSList",
            client.get_pos_articles(&BdpGetPosArticlesRequest::first_page(
                config.bdp_items_profile_id,
                BDP_DRY_RUN_PAGE_SIZE,
            )),
            &[
                "ArticlesListData",
                "ArticleListData",
                "Articles",
                "ArticleList",
            ],
        )
        .await;

        if pos.is_some() && employee.is_some() && departments.is_some() {
            Self::check_order_only(config, &client, articles.as_ref(), &mut response).await;
        }

        response.listo_para_sincronizar = response.checks.iter().all(|check| check.ok);
        response.mensaje = if response.listo_para_sincronizar {
            "El simulador aceptó el contrato local; esto no demuestra el comportamiento de un BDP real"
                .to_string()
        } else {
            "BDP aun tiene checks pendientes antes de activar escrituras reales".to_string()
        };
        response
    }

    async fn check_health(client: &BdpWeblinkClient<'_>, response: &mut BdpSyncDryRunResponse) {
        let check = match client.health().await {
            Ok(health) if health.is_alive => BdpSyncDryRunCheck::ok(
                "Health",
                "/Service/Health",
                "BDP responde IsAlive=true",
                None,
                None,
            ),
            Ok(_) => BdpSyncDryRunCheck::error(
                "Health",
                "/Service/Health",
                "BDP respondio IsAlive=false",
            ),
            Err(error) => BdpSyncDryRunCheck::error(
                "Health",
                "/Service/Health",
                format!("No se pudo contactar BDP: {error}"),
            ),
        };
        response.checks.push(check);
    }

    async fn check_version(client: &BdpWeblinkClient<'_>, response: &mut BdpSyncDryRunResponse) {
        let check = match client.get_version().await {
            Ok(version) => BdpSyncDryRunCheck::ok(
                "Sesion y version",
                "/Auth/Login + /Service/GetVersion",
                "Login y version correctos",
                None,
                Some(format!(
                    "{}.{} {}",
                    version.version, version.sub_version, version.application_description
                )),
            ),
            Err(error) => BdpSyncDryRunCheck::error(
                "Sesion y version",
                "/Auth/Login + /Service/GetVersion",
                format!("Login o GetVersion fallo: {error}"),
            ),
        };
        response.checks.push(check);
    }

    async fn capture<F>(
        response: &mut BdpSyncDryRunResponse,
        name: &str,
        endpoint: &str,
        future: F,
        array_keys: &[&str],
    ) -> Option<Value>
    where
        F: std::future::Future<Output = Result<Value, BdpWeblinkError>>,
    {
        match future.await {
            Ok(value) => {
                response.checks.push(BdpSyncDryRunCheck::ok(
                    name,
                    endpoint,
                    "Endpoint BDP respondio correctamente",
                    value_count(&value, array_keys),
                    summarize_value(&value, array_keys),
                ));
                Some(value)
            }
            Err(error) => {
                response
                    .checks
                    .push(BdpSyncDryRunCheck::error(name, endpoint, error.to_string()));
                None
            }
        }
    }

    #[allow(clippy::ref_option)]
    fn check_employee_is_allowed(
        config: &ConfiguracionRestaurante,
        employees: &Option<Value>, // TODO: refactor a Option<&Value> en limpieza futura
        response: &mut BdpSyncDryRunResponse,
    ) {
        let Some(employees) = employees else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "Empleado permitido en POS",
                "/API/POS/Employees/Get",
                "No se pudo leer la lista de empleados del POS",
            ));
            return;
        };
        let ok = value_array(employees, &["Employees"]).is_some_and(|items| {
            items
                .iter()
                .any(|item| number_i64(item, &["Id"]) == Some(i64::from(config.bdp_employee_id)))
        });
        let check = if ok {
            BdpSyncDryRunCheck::ok(
                "Empleado permitido en POS",
                "/API/POS/Employees/Get",
                "El empleado configurado aparece asociado al terminal",
                None,
                Some(config.bdp_employee_id.to_string()),
            )
        } else {
            BdpSyncDryRunCheck::error(
                "Empleado permitido en POS",
                "/API/POS/Employees/Get",
                format!(
                    "El empleado {} no aparece en el terminal {}",
                    config.bdp_employee_id, config.bdp_pos_id
                ),
            )
        };
        response.checks.push(check);
    }

    /// [F2.7] Verificar que hay mapeos de artículos en `bdp_article_map`.
    /// Si no hay ninguno, la sync usará el artículo default para todas las líneas.
    async fn check_article_mapping(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
        response: &mut BdpSyncDryRunResponse,
    ) {
        match BdpArticleMapRepository::listar(pool, user_id).await {
            Ok(mapas) if !mapas.is_empty() => {
                response.checks.push(BdpSyncDryRunCheck::ok(
                    "Mapeo de articulos",
                    "bdp_article_map",
                    format!("{} articulos mapeados Glory→BDP", mapas.len()),
                    Some(mapas.len()),
                    mapas.first().map(|m| {
                        format!(
                            "{} → {} ({})",
                            m.articulo_glory_codigo, m.articulo_bdp_codigo, m.articulo_bdp_nombre
                        )
                    }),
                ));
            }
            Ok(_) => {
                /* Sin mapeos — no es error pero conviene avisar */
                let msg = if config.bdp_default_article_code.is_empty() {
                    "Sin mapeos y sin articulo default configurado. Las lineas no tendran resolucion.".to_string()
                } else {
                    format!(
                        "Sin mapeos de articulos. Todas las lineas usaran el default: '{}' ({})",
                        config.bdp_default_article_code, config.bdp_default_article_name
                    )
                };
                response.checks.push(BdpSyncDryRunCheck {
                    nombre: "Mapeo de articulos".into(),
                    endpoint: "bdp_article_map".into(),
                    ok: true, /* Warning, no blocker */
                    mensaje: msg,
                    cantidad: Some(0),
                    muestra: None,
                });
            }
            Err(e) => {
                response.checks.push(BdpSyncDryRunCheck::error(
                    "Mapeo de articulos",
                    "bdp_article_map",
                    format!("Error consultando mapeos: {e}"),
                ));
            }
        }
    }

    /// [F3.4] Validar que los `TenderIds` en `bdp_tender_map` existen en el POS.
    /// Compara las claves/valores del JSONB contra la lista de tenders devuelta por BDP.
    fn check_tender_mapping(
        config: &ConfiguracionRestaurante,
        tenders: Option<&Value>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        let map = match config.bdp_tender_map.as_object() {
            Some(m) if !m.is_empty() => m,
            _ => {
                response.checks.push(BdpSyncDryRunCheck {
                    nombre: "Mapeo de tenders".into(),
                    endpoint: "bdp_tender_map".into(),
                    ok: true,
                    mensaje: "Sin mapeo de tenders configurado. No se enviara TenderId.".into(),
                    cantidad: Some(0),
                    muestra: None,
                });
                return;
            }
        };

        let Some(tenders_value) = tenders else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "Mapeo de tenders",
                "bdp_tender_map",
                "No se pudo obtener la lista de tenders del POS para validar",
            ));
            return;
        };

        /* Extraer IDs válidos del POS */
        let valid_ids: Vec<i64> = value_array(tenders_value, &["TenderList", "Tenders"])
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| number_i64(item, &["Id"]))
                    .collect()
            })
            .unwrap_or_default();

        let mut issues = Vec::new();
        let mut mapped = 0;
        for (key, value) in map {
            let Some(tender_str) = value.as_str() else {
                issues.push(format!("{key}: valor no es string"));
                continue;
            };
            let Ok(tender_id) = tender_str.parse::<i64>() else {
                issues.push(format!("{key}: '{tender_str}' no es numérico"));
                continue;
            };
            if valid_ids.is_empty() || valid_ids.contains(&tender_id) {
                mapped += 1;
            } else {
                issues.push(format!(
                    "{key}→{tender_id}: no existe en POS (disponibles: {valid_ids:?})"
                ));
            }
        }

        if issues.is_empty() {
            response.checks.push(BdpSyncDryRunCheck::ok(
                "Mapeo de tenders",
                "bdp_tender_map",
                format!("{mapped} tenders mapeados correctamente"),
                Some(mapped),
                None,
            ));
        } else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "Mapeo de tenders",
                "bdp_tender_map",
                format!("{} problema(s): {}", issues.len(), issues.join("; ")),
            ));
        }
    }

    /// [F3.5] Validar que los order types en `bdp_order_type_map` son válidos (>= 0).
    /// BDP Type: 0=Barra, 1=Mesa, 2=Delivery, etc. Valores negativos son inválidos.
    fn check_order_type_mapping(
        config: &ConfiguracionRestaurante,
        response: &mut BdpSyncDryRunResponse,
    ) {
        let map = match config.bdp_order_type_map.as_object() {
            Some(m) if !m.is_empty() => m,
            _ => {
                response.checks.push(BdpSyncDryRunCheck {
                    nombre: "Mapeo de order types".into(),
                    endpoint: "bdp_order_type_map".into(),
                    ok: true,
                    mensaje: "Sin mapeo de order types. Todas las ventas usaran Type=0 (Barra)."
                        .into(),
                    cantidad: Some(0),
                    muestra: None,
                });
                return;
            }
        };

        let mut issues = Vec::new();
        for (key, value) in map {
            match value.as_str().and_then(|s| s.parse::<i32>().ok()) {
                Some(t) if t >= 0 => {}
                Some(t) => issues.push(format!("{key}: Type={t} es negativo")),
                None => issues.push(format!("{key}: valor '{value}' no es numérico válido")),
            }
        }

        if issues.is_empty() {
            response.checks.push(BdpSyncDryRunCheck::ok(
                "Mapeo de order types",
                "bdp_order_type_map",
                format!("{} canales mapeados", map.len()),
                Some(map.len()),
                None,
            ));
        } else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "Mapeo de order types",
                "bdp_order_type_map",
                format!("{} problema(s): {}", issues.len(), issues.join("; ")),
            ));
        }
    }

    async fn check_order_only(
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        articles: Option<&Value>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        /* [R12] Pasar iva_por_defecto de config como fallback */
        let default_iva = {
            use rust_decimal::prelude::ToPrimitive;
            config.iva_por_defecto.to_f64().unwrap_or(10.0)
        };
        let Some(article) = articles.and_then(|a| first_article(a, default_iva)) else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "CreateOrder OnlyCheck",
                "/API/Orders/Create",
                "No hay un articulo valido para construir la comanda de prueba",
            ));
            return;
        };

        let request = build_only_check_order(config, &article);
        response.payload_preview = serde_json::to_value(&request).ok();
        let check = match client.check_order(&request).await {
            Ok(value) => BdpSyncDryRunCheck::ok(
                "CreateOrder OnlyCheck",
                "/API/Orders/Create",
                "El destino aceptó OnlyCheck; no se asume que una instalación real sea no persistente",
                None,
                summarize_value(&value, &["OrderId", "InvoiceNumber"]),
            ),
            Err(error) => BdpSyncDryRunCheck::error(
                "CreateOrder OnlyCheck",
                "/API/Orders/Create",
                format!("BDP rechazo el payload de comanda: {error}"),
            ),
        };
        response.checks.push(check);
    }
}

impl BdpSyncDryRunResponse {
    fn new(sync_habilitado: bool) -> Self {
        Self {
            configurado: false,
            sync_habilitado,
            escritura_real: false,
            listo_para_sincronizar: false,
            mensaje: String::new(),
            checks: Vec::new(),
            payload_preview: None,
        }
    }
}

impl BdpSyncDryRunCheck {
    fn ok(
        name: impl Into<String>,
        endpoint: impl Into<String>,
        message: impl Into<String>,
        count: Option<usize>,
        sample: Option<String>,
    ) -> Self {
        Self {
            nombre: name.into(),
            endpoint: endpoint.into(),
            ok: true,
            mensaje: message.into(),
            cantidad: count,
            muestra: sample,
        }
    }

    fn error(
        name: impl Into<String>,
        endpoint: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            nombre: name.into(),
            endpoint: endpoint.into(),
            ok: false,
            mensaje: message.into(),
            cantidad: None,
            muestra: None,
        }
    }
}

fn build_only_check_order(
    config: &ConfiguracionRestaurante,
    article: &BdpDryRunArticle,
) -> BdpCreateOrderRequest {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let marketplace_order_id = format!("GDRY{:010}", Utc::now().timestamp() % 10_000_000_000);
    BdpCreateOrderRequest {
        employee_id: config.bdp_employee_id,
        items_profile_id: config.bdp_items_profile_id,
        order_end_type: 0,
        order_operation_type: 1,
        invoice: Some(false),
        order: json!({
            "MarketplaceOrderId": marketplace_order_id,
            "MarketId": BDP_DRY_RUN_MARKET_ID,
            "MarketName": "Glory Dry Run",
            "PreparationTime": now,
            "OrderId": 0,
            "PosId": config.bdp_pos_id,
            "Type": 0, /* Barra/Ticket aparcado — el único tipo que pasa validación en POS 31.
                           Type=1 (Mesa) falla 300008 (salón incorrecto) y Type=2 (Delivery) falla 300009.
                           POS 31 usa serie 00031TI (IVA incluido) desde 2026-06-07.
                           Ver bdp-300035-resumen-completo-2026-06-01.md. */
            "RoomNumber": 0,
            "TableNumber": 0,
            "Items": [{
                "Lin": 1,
                "Id": article.id,
                "Name": article.name,
                "Units": 1.0,
                "Price": article.price,
                "Supplement": 0.0,
                "Discount": 0.0,
                "DiscountPct": false,
                "Total": article.price,
                "VatPct": article.vat_pct,
                "Comments": [],
                "Supplements": [],
                "OrderItemType": 0,
                "OrderItemTypeMetaInfo": "",
                "TyC_D1": 0,
                "TyC_D2": 0,
                "TyC_D3": 0,
                "OnSale": false
            }],
            "Discount": 0.0,
            "DiscountPct": false,
            "Total": article.price,
            "ExecutionTime": now,
            "Status": 0,
            "AlreadyInvoiced": false,
            "Comments": "GLORY DRY RUN - NO CREAR"
        }),
    }
}

fn first_article(value: &Value, default_iva_pct: f64) -> Option<BdpDryRunArticle> {
    value_array(
        value,
        &[
            "ArticlesListData",
            "ArticleListData",
            "Articles",
            "ArticleList",
        ],
    )?
    .iter()
    .find_map(|item| {
        let id = number_i64(item, &["ArtCode", "Id", "Code"])?;
        let name = text_field(item, &["ArtDescription", "Description", "Name"])?;
        let price = number_f64(item, &["Price1", "Price", "Total"])?;
        /* [R12] Usar iva_por_defecto de config como fallback en vez de 10.0 hardcodeado */
        let vat_pct = number_f64(item, &["TAVPer", "VatPct"]).unwrap_or(default_iva_pct);
        (id > 0 && price > 0.0).then_some(BdpDryRunArticle {
            id,
            name,
            price,
            vat_pct,
        })
    })
}

fn value_array<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Vec<Value>> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_array))
}

fn summarize_value(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(items) = value.get(*key).and_then(Value::as_array) {
            return Some(format!("{} elementos", items.len()));
        }
        if let Some(text) = value.get(*key).and_then(Value::as_str) {
            return Some(text.to_string());
        }
        if let Some(number) = value.get(*key).and_then(Value::as_i64) {
            return Some(number.to_string());
        }
    }
    None
}

fn value_count(value: &Value, keys: &[&str]) -> Option<usize> {
    for key in keys {
        if let Some(items) = value.get(*key).and_then(Value::as_array) {
            return Some(items.len());
        }
        if value.get(*key).and_then(Value::as_object).is_some() {
            return Some(1);
        }
    }
    None
}

fn number_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|field| {
            field.as_i64().or_else(|| {
                #[allow(clippy::cast_possible_truncation)]
                field.as_f64().map(|number| number as i64)
            })
        })
    })
}

fn number_f64(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|field| {
            field.as_f64().or_else(|| {
                #[allow(clippy::cast_precision_loss)]
                field.as_i64().map(|number| number as f64)
            })
        })
    })
}

fn text_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
    })
}

/* [065A-5] pub para uso por BdpSyncService en bdp_sync.rs */
pub fn bdp_configurado(config: &ConfiguracionRestaurante) -> bool {
    !config.bdp_base_url.trim().is_empty()
        && !config.bdp_login.trim().is_empty()
        && !config.bdp_password.trim().is_empty()
        && !config.bdp_integrator_code.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;
    use rust_decimal::Decimal;
    use uuid::Uuid;

    fn config() -> ConfiguracionRestaurante {
        ConfiguracionRestaurante {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            reserva_email_obligatorio: false,
            reserva_telefono_obligatorio: true,
            reserva_nombre_obligatorio: true,
            reserva_apellidos_obligatorio: false,
            iva_por_defecto: Decimal::new(10, 0),
            nombre_restaurante: "Nakomi".to_string(),
            groq_api_key: None,
            auto_venta_reserva: true,
            hora_desayuno_inicio: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            hora_desayuno_fin: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            hora_comida_inicio: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            hora_comida_fin: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            hora_cena_inicio: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            hora_cena_fin: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
            url_haddock: String::new(),
            haddock_api_token: String::new(),
            haddock_sync_enabled: false,
            bdp_base_url: "http://bdp.test".to_string(),
            bdp_login: "usuario".to_string(),
            bdp_password: "secreto".to_string(),
            bdp_integrator_code: "INTEGRADOR".to_string(),
            bdp_sync_enabled: true,
            bdp_pos_id: 31,
            bdp_employee_id: 1,
            bdp_items_profile_id: 1,
            bdp_default_article_code: "GLORY".to_string(),
            bdp_default_article_name: "Servicio Glory".to_string(),
            bdp_tender_map: serde_json::json!({"efectivo": "1", "tarjeta": "2"}),
            bdp_order_type_map: serde_json::json!({"comedor": "0", "barra": "0"}),
            bdp_default_customer_code: "DEFAULT".to_string(),
            bdp_poll_interval_secs: 60,
            bdp_poll_enabled: false,
            google_review_url: String::new(),
            telefono_restaurante: String::new(),
            url_reservas: String::new(),
            bdp_auto_sync_customers: false,
            bdp_sync_mode: "read_only".to_string(),
            bdp_backup_retention_days: 30,
            bdp_auto_backup_before_write: true,
            bdp_env_bootstrap_applied_at: None,
            /* [XT2-1] Feature flags BDP desactivados por defecto en tests */
            ff_bdp_auto_arm: false,
            ff_bdp_partial_payments: false,
            ff_bdp_cancel_order: false,
            ff_bdp_purchase_notes_read: false,
            ff_bdp_purchase_notes_draft: false,
            ff_bdp_purchase_notes_receive: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn first_article_uses_real_bdp_fields() {
        let value = json!({
            "ArticleListData": [{
                "ArtCode": 1001,
                "ArtDescription": "COCA-COLA",
                "Price1": 1.05,
                "TAVPer": 10.0
            }]
        });

        let article = first_article(&value, 10.0).unwrap();

        assert_eq!(article.id, 1001);
        assert_eq!(article.name, "COCA-COLA");
        assert_eq!(article.price, 1.05);
        assert_eq!(article.vat_pct, 10.0);
    }

    #[test]
    fn first_article_accepts_integer_prices() {
        let value = json!({
            "ArticleListData": [{
                "ArtCode": 1001,
                "ArtDescription": "MENU",
                "Price1": 12,
                "TAVPer": 10
            }]
        });

        let article = first_article(&value, 10.0).unwrap();

        assert_eq!(article.price, 12.0);
        assert_eq!(article.vat_pct, 10.0);
    }

    #[test]
    fn first_article_accepts_real_bdp_articles_list_key() {
        let value = json!({
            "ArticlesListData": [{
                "ArtCode": 1001,
                "ArtDescription": "CAFE BOMBON",
                "Price1": 1.4,
                "TAVPer": 10.0
            }]
        });

        let article = first_article(&value, 10.0).unwrap();

        assert_eq!(article.id, 1001);
        assert_eq!(article.name, "CAFE BOMBON");
    }

    /* [R12] Test: first_article usa el IVA por defecto del config cuando BDP no lo devuelve */
    #[test]
    fn first_article_uses_default_iva_when_missing() {
        let value = json!({
            "ArticleListData": [{
                "ArtCode": 1001,
                "ArtDescription": "SIN IVA",
                "Price1": 5.0
            }]
        });

        let article = first_article(&value, 21.0).unwrap();
        assert_eq!(article.vat_pct, 21.0, "Debe usar el default_iva_pct pasado como parámetro");
    }

    #[test]
    fn build_order_never_uses_create_operation() {
        let config = config();
        let article = BdpDryRunArticle {
            id: 1001,
            name: "COCA-COLA".to_string(),
            price: 1.05,
            vat_pct: 10.0,
        };
        let order = build_only_check_order(&config, &article);

        assert_eq!(order.order_operation_type, 1);
        assert_eq!(order.invoice, Some(false));
        assert_eq!(order.order["Items"][0]["Id"], 1001);
        assert_eq!(order.order["AlreadyInvoiced"], false);
        assert!(order.order.get("Payments").is_none());
        assert!(order.order["MarketplaceOrderId"].as_str().unwrap().len() <= 15);
    }
}
