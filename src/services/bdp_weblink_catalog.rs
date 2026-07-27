/* [065A-3] Inventario BDP/WebLink extraido del manual.
 * Se codifican rutas y payloads minimos antes de tener acceso real al PC del
 * restaurante. Las respuestas complejas quedan como JSON hasta contrastarlas
 * contra datos reales de BDP-NET para no inventar contratos incompletos. */

use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Value;

pub const BDP_PATH_SERVICE_HEALTH: &str = "/Service/Health";
pub const BDP_PATH_SERVICE_GET_VERSION: &str = "/Service/GetVersion";
pub const BDP_PATH_AUTH_LOGIN: &str = "/Auth/Login";
pub const BDP_PATH_EXPORT_ARTICLES: &str = "/API/Articles/Export";
pub const BDP_PATH_GET_POS_ARTICLES: &str = "/API/Articles/GetPOSList";
pub const BDP_PATH_EXPORT_CUSTOMERS: &str = "/API/Customers/Export";
pub const BDP_PATH_CREATE_CUSTOMER: &str = "/API/Customers/Create";
pub const BDP_PATH_CREATE_ORDER: &str = "/API/Orders/Create";
pub const BDP_PATH_GET_ORDER: &str = "/API/Orders/Get";
pub const BDP_PATH_CANCEL_ORDER: &str = "/API/Orders/Cancel";
pub const BDP_PATH_ORDER_PAYMENT_ADD: &str = "/API/Orders/Payment/Add";
pub const BDP_PATH_INVOICE_ORDER: &str = "/API/Orders/Invoice";
pub const BDP_PATH_EXPORT_DEPARTMENTS: &str = "/API/Departments/Export";
pub const BDP_PATH_EXPORT_DEPARTMENTS_FROM_PROFILE: &str = "/API/Departments/ExportFromProfile";
pub const BDP_PATH_GET_POS: &str = "/API/POS/Get";
pub const BDP_PATH_GET_POSES: &str = "/API/POSes/Get";
pub const BDP_PATH_GET_EMPLOYEE: &str = "/API/Employee/Get";
pub const BDP_PATH_GET_EMPLOYEES: &str = "/API/Employees/Get";
pub const BDP_PATH_GET_POS_EMPLOYEES: &str = "/API/POS/Employees/Get";
pub const BDP_PATH_GET_TENDERS: &str = "/API/Tenders/GetList";
pub const BDP_PATH_GET_POS_TENDERS: &str = "/API/Tenders/GetPOSList";
/* [157A-9] F9.2: consulta individual de artículo */
pub const BDP_PATH_GET_ARTICLE: &str = "/API/Articles/Get";
/* [157A-9] F9.3: refresh de precios de artículo */
pub const BDP_PATH_GET_PRICES_ARTICLES: &str = "/API/Articles/GetPrices";
/* [157A-9] F9.4: mesas por salón / todos los salones */
pub const BDP_PATH_GET_ROOM_TABLES: &str = "/API/Room/GetTables";
pub const BDP_PATH_GET_ROOMS_TABLES: &str = "/API/Rooms/GetTables";
/* [157A-9] F9.5: definiciones de menús, fastfoods y packs */
pub const BDP_PATH_GET_MENU: &str = "/API/Menus/Get";
pub const BDP_PATH_GET_FASTFOOD: &str = "/API/FastFoods/Get";
pub const BDP_PATH_GET_PACK: &str = "/API/Packs/Get";
/* [247A-11] Fase 1 compras BDP: exportación de albaranes de compra. */
pub const BDP_PATH_EXPORT_PURCHASE_NOTES: &str = "/API/ExportProfiles/PurchaseNotes";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BdpEndpointArea {
    Servicios,
    Articulos,
    Clientes,
    Comandas,
    Departamentos,
    Terminales,
    Empleados,
    Pagos,
    Salones,
    Menus,
    Compras,
}

#[derive(Debug, Clone, Copy)]
pub struct BdpEndpointSpec {
    pub name: &'static str,
    pub area: BdpEndpointArea,
    pub path: &'static str,
    pub purpose: &'static str,
}

pub const BDP_ENDPOINTS: &[BdpEndpointSpec] = &[
    BdpEndpointSpec {
        name: "ServiceHealth",
        area: BdpEndpointArea::Servicios,
        path: BDP_PATH_SERVICE_HEALTH,
        purpose: "health remoto",
    },
    BdpEndpointSpec {
        name: "Login",
        area: BdpEndpointArea::Servicios,
        path: BDP_PATH_AUTH_LOGIN,
        purpose: "sesion autenticada",
    },
    BdpEndpointSpec {
        name: "GetVersion",
        area: BdpEndpointArea::Servicios,
        path: BDP_PATH_SERVICE_GET_VERSION,
        purpose: "version BDP-NET",
    },
    BdpEndpointSpec {
        name: "ExportArticles",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_EXPORT_ARTICLES,
        purpose: "catalogo web de articulos",
    },
    BdpEndpointSpec {
        name: "GetPOSArticlesList",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_GET_POS_ARTICLES,
        purpose: "articulos por perfil TPV",
    },
    BdpEndpointSpec {
        name: "ExportCustomers",
        area: BdpEndpointArea::Clientes,
        path: BDP_PATH_EXPORT_CUSTOMERS,
        purpose: "exportacion de clientes",
    },
    BdpEndpointSpec {
        name: "CreateCustomer",
        area: BdpEndpointArea::Clientes,
        path: BDP_PATH_CREATE_CUSTOMER,
        purpose: "alta o sobrescritura de cliente",
    },
    BdpEndpointSpec {
        name: "CreateOrder",
        area: BdpEndpointArea::Comandas,
        path: BDP_PATH_CREATE_ORDER,
        purpose: "crear comanda",
    },
    BdpEndpointSpec {
        name: "GetOrder",
        area: BdpEndpointArea::Comandas,
        path: BDP_PATH_GET_ORDER,
        purpose: "consultar comanda",
    },
    BdpEndpointSpec {
        name: "CancelOrder",
        area: BdpEndpointArea::Comandas,
        path: BDP_PATH_CANCEL_ORDER,
        purpose: "cancelar comanda",
    },
    BdpEndpointSpec {
        name: "AddOrderPayment",
        area: BdpEndpointArea::Pagos,
        path: BDP_PATH_ORDER_PAYMENT_ADD,
        purpose: "agregar pago",
    },
    BdpEndpointSpec {
        name: "InvoiceOrder",
        area: BdpEndpointArea::Pagos,
        path: BDP_PATH_INVOICE_ORDER,
        purpose: "facturar comanda",
    },
    BdpEndpointSpec {
        name: "ExportDepartments",
        area: BdpEndpointArea::Departamentos,
        path: BDP_PATH_EXPORT_DEPARTMENTS,
        purpose: "departamentos por rango",
    },
    BdpEndpointSpec {
        name: "DepartmentsExportFromProfile",
        area: BdpEndpointArea::Departamentos,
        path: BDP_PATH_EXPORT_DEPARTMENTS_FROM_PROFILE,
        purpose: "departamentos por perfil",
    },
    BdpEndpointSpec {
        name: "GetPOS",
        area: BdpEndpointArea::Terminales,
        path: BDP_PATH_GET_POS,
        purpose: "terminal concreto",
    },
    BdpEndpointSpec {
        name: "GetPOSes",
        area: BdpEndpointArea::Terminales,
        path: BDP_PATH_GET_POSES,
        purpose: "terminales disponibles",
    },
    BdpEndpointSpec {
        name: "GetEmployee",
        area: BdpEndpointArea::Empleados,
        path: BDP_PATH_GET_EMPLOYEE,
        purpose: "empleado concreto",
    },
    BdpEndpointSpec {
        name: "GetEmployees",
        area: BdpEndpointArea::Empleados,
        path: BDP_PATH_GET_EMPLOYEES,
        purpose: "empleados disponibles",
    },
    BdpEndpointSpec {
        name: "GetPOSEmployees",
        area: BdpEndpointArea::Empleados,
        path: BDP_PATH_GET_POS_EMPLOYEES,
        purpose: "empleados de un terminal",
    },
    BdpEndpointSpec {
        name: "GetTenderList",
        area: BdpEndpointArea::Pagos,
        path: BDP_PATH_GET_TENDERS,
        purpose: "formas de pago",
    },
    BdpEndpointSpec {
        name: "GetPOSTenderList",
        area: BdpEndpointArea::Pagos,
        path: BDP_PATH_GET_POS_TENDERS,
        purpose: "formas de pago por terminal",
    },
    /* [157A-9] F9.2-F9.5: nuevos endpoints catálogo/salones/menús */
    BdpEndpointSpec {
        name: "GetArticle",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_GET_ARTICLE,
        purpose: "consulta individual de articulo",
    },
    BdpEndpointSpec {
        name: "GetPricesArticles",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_GET_PRICES_ARTICLES,
        purpose: "precios de venta de un articulo",
    },
    BdpEndpointSpec {
        name: "GetRoomTables",
        area: BdpEndpointArea::Salones,
        path: BDP_PATH_GET_ROOM_TABLES,
        purpose: "mesas de un salon",
    },
    BdpEndpointSpec {
        name: "GetRoomsTables",
        area: BdpEndpointArea::Salones,
        path: BDP_PATH_GET_ROOMS_TABLES,
        purpose: "salones con mesas",
    },
    BdpEndpointSpec {
        name: "GetMenuDefinition",
        area: BdpEndpointArea::Menus,
        path: BDP_PATH_GET_MENU,
        purpose: "definicion de menu",
    },
    BdpEndpointSpec {
        name: "GetFastfoodDefinition",
        area: BdpEndpointArea::Menus,
        path: BDP_PATH_GET_FASTFOOD,
        purpose: "definicion de fastfood",
    },
    BdpEndpointSpec {
        name: "GetPackDefinition",
        area: BdpEndpointArea::Menus,
        path: BDP_PATH_GET_PACK,
        purpose: "definicion de pack",
    },
    /* [247A-11] Fase 1 compras BDP: exportación de albaranes de compra. */
    BdpEndpointSpec {
        name: "ExportPurchaseNotes",
        area: BdpEndpointArea::Compras,
        path: BDP_PATH_EXPORT_PURCHASE_NOTES,
        purpose: "albaranes de compra",
    },
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpEmptyRequest {}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportArticlesRequest {
    pub dept1: i32,
    pub dept2: i32,
    pub art1: i64,
    pub art2: i64,
    pub modified: bool,
    pub type_price: i32,
    pub disc: i32,
}

impl BdpExportArticlesRequest {
    #[must_use]
    pub const fn all_web_articles(type_price: i32) -> Self {
        Self {
            dept1: 1,
            dept2: 999,
            art1: 1,
            art2: 9_999_999_999_999,
            modified: false,
            type_price,
            disc: 0,
        }
    }
}

/* [157A-7] F9.1: Struct tipado para parsear la respuesta de ExportArticles.
 * BDP devuelve {"Articles": [{Code, Name, Family, Subfamily, Department,
 * Tax1, Tax2, Price1..Price5, Discount, BarCode, Active}]}
 * Usado por BdpSyncService::sync_catalog(). */

/// Entrada de precios donde BDP puede anidar el stock (`PricesTableDataType`).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpArticlePriceEntry {
    /// Stock dentro de una entrada de la tabla de precios.
    #[serde(default, alias = "CurrentStock", alias = "Stock")]
    pub current_stock: Option<Decimal>,
}

/// Un artículo individual del array `Articles` en la respuesta de `ExportArticles`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportArticleItem {
    /// Código del artículo (puede venir como string o número en BDP)
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub code: Option<String>,
    /// Fallback: algunos endpoints usan `ItemCode` en vez de `Code`
    #[serde(
        default,
        alias = "ItemCode",
        deserialize_with = "deserialize_optional_string"
    )]
    pub item_code: Option<String>,
    /// Descripción / nombre del artículo
    #[serde(default, alias = "Description")]
    pub name: Option<String>,
    /// Código de familia
    #[serde(default)]
    pub family: Option<i32>,
    /// Código de subfamilia
    #[serde(default)]
    pub subfamily: Option<i32>,
    /// Código de departamento
    #[serde(default)]
    pub department: Option<i32>,
    /// Porcentaje IVA 1 (venta)
    #[serde(default)]
    pub tax1: Option<Decimal>,
    /// Porcentaje IVA 2
    #[serde(default)]
    pub tax2: Option<Decimal>,
    /// Precio tarifa 1
    #[serde(default)]
    pub price1: Option<Decimal>,
    /// Precio tarifa 2
    #[serde(default)]
    pub price2: Option<Decimal>,
    /// Precio tarifa 3
    #[serde(default)]
    pub price3: Option<Decimal>,
    /// Precio tarifa 4
    #[serde(default)]
    pub price4: Option<Decimal>,
    /// Precio tarifa 5
    #[serde(default)]
    pub price5: Option<Decimal>,
    /// Porcentaje de descuento
    #[serde(default)]
    pub discount: Option<Decimal>,
    /// Código de barras
    #[serde(default)]
    pub bar_code: Option<String>,
    /// Artículo activo
    #[serde(default = "default_true")]
    pub active: bool,
    /* [237A-4] Stock actual del artículo. BDP puede devolverlo como CurrentStock
     * o Stock en la respuesta de ExportArticles (PricesTableDataType).
     * Si no viene (módulo de almacén no activo o perfil sin stock), será None
     * y el campo quedará en 0 al hacer sync-catalog.
     * ⚠️ Si el stock siempre aparece como 0 en la tabla, verificar que el
     * perfil de exportación de BDP incluya el campo CurrentStock. */
    #[serde(default, alias = "CurrentStock", alias = "Stock")]
    pub current_stock: Option<Decimal>,
    /// Tabla de precios/stock alternativa. BDP puede devolver `CurrentStock`
    /// anidado dentro de `PricesTableData` o `Prices` en lugar de a nivel raíz.
    #[serde(default, alias = "PricesTableData", alias = "Prices")]
    pub prices_table_data: Vec<BdpArticlePriceEntry>,
}

impl BdpExportArticleItem {
    /// Devuelve el stock efectivo del artículo: primero el campo raíz, luego
    /// cualquier entrada anidada en la tabla de precios.
    #[must_use]
    pub fn effective_stock(&self) -> Option<Decimal> {
        self.current_stock
            .or_else(|| self.prices_table_data.iter().find_map(|p| p.current_stock))
    }
}

fn default_true() -> bool {
    true
}

impl BdpExportArticleItem {
    /// Devuelve el código del artículo (Code o `ItemCode`)
    #[must_use]
    pub fn art_code(&self) -> Option<&str> {
        self.code
            .as_deref()
            .or(self.item_code.as_deref())
            .filter(|s| !s.is_empty())
    }

    /// Devuelve la descripción del artículo
    #[must_use]
    pub fn description(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
}

/// Respuesta tipada de `POST /API/Articles/Export`.
/// El array viene dentro de la clave `Articles`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportArticlesResponse {
    #[serde(default)]
    pub articles: Vec<BdpExportArticleItem>,
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = serde::Deserialize::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) => Ok(Some(value)),
        Some(serde_json::Value::Number(value)) => Ok(Some(value.to_string())),
        Some(other) => Err(serde::de::Error::custom(format!(
            "código BDP debe ser string o número, recibido {other}"
        ))),
    }
}

/// Resultado del sync de catálogo BDP → Glory (F9.1).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BdpCatalogSyncResult {
    pub creados: u32,
    pub actualizados: u32,
    pub sin_cambios: u32,
    pub errores: u32,
    pub total_bdp: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPosArticlesRequest {
    pub art1: i64,
    pub art2: i64,
    pub dept1: i32,
    pub dept2: i32,
    pub description: String,
    pub description_query_type: i32,
    pub items_per_page: i32,
    pub actual_page: i32,
    #[serde(rename = "nField")]
    pub n_field: i32,
    #[serde(rename = "nOrder")]
    pub n_order: i32,
    pub profile_code: i32,
}

impl BdpGetPosArticlesRequest {
    #[must_use]
    pub fn first_page(profile_code: i32, items_per_page: i32) -> Self {
        Self {
            art1: 1,
            art2: 9_999_999_999_999,
            dept1: 1,
            dept2: 999,
            description: String::new(),
            description_query_type: 0,
            items_per_page,
            actual_page: 1,
            n_field: 1,
            n_order: 0,
            profile_code,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportCustomersRequest {
    pub customer1: i32,
    pub customer2: i32,
}

impl Default for BdpExportCustomersRequest {
    fn default() -> Self {
        Self {
            customer1: 1,
            customer2: 999_999,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateCustomerRequest {
    pub code: i32,
    pub fiscal_name: String,
    pub commercial_name: String,
    pub mobile_phone: String,
    #[serde(rename = "EMail")]
    pub email: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateOrderRequest {
    pub employee_id: i32,
    pub items_profile_id: i32,
    pub order_end_type: i32,
    pub order_operation_type: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<bool>,
    pub order: Value,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpOrderIdentifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketplace_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_number: Option<i32>,
}

impl BdpOrderIdentifier {
    #[must_use]
    pub const fn by_order_id(order_id: i64) -> Self {
        Self {
            order_id: Some(order_id),
            market_id: None,
            marketplace_order_id: None,
            room_number: None,
            table_number: None,
        }
    }

    #[must_use]
    pub fn by_market(market_id: i32, marketplace_order_id: impl Into<String>) -> Self {
        Self {
            order_id: None,
            market_id: Some(market_id),
            marketplace_order_id: Some(marketplace_order_id.into()),
            room_number: None,
            table_number: None,
        }
    }

    #[must_use]
    pub const fn by_table(room_number: i32, table_number: i32) -> Self {
        Self {
            order_id: None,
            market_id: None,
            marketplace_order_id: None,
            room_number: Some(room_number),
            table_number: Some(table_number),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetOrderRequest {
    pub order_identifier: BdpOrderIdentifier,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCancelOrderRequest {
    pub pos_id: i32,
    pub order_identifier: BdpOrderIdentifier,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpOrderPayment {
    pub tender_id: i32,
    pub amount: Decimal,
    pub payment_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpAddOrderPaymentRequest {
    pub order_identifier: BdpOrderIdentifier,
    pub payment: BdpOrderPayment,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub employee_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpInvoiceOrderRequest {
    pub pos_id: i32,
    pub employee_id: i32,
    pub order_identifier: BdpOrderIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportDepartmentsRequest {
    pub dept1: i32,
    pub dept2: i32,
    pub description: String,
    pub description_query_type: i32,
    pub items_per_page: i32,
    pub actual_page: i32,
    #[serde(rename = "nField")]
    pub n_field: i32,
    #[serde(rename = "nOrder")]
    pub n_order: i32,
}

impl Default for BdpExportDepartmentsRequest {
    fn default() -> Self {
        Self {
            dept1: 1,
            dept2: 999,
            description: String::new(),
            description_query_type: 0,
            items_per_page: 0,
            actual_page: 1,
            n_field: 1,
            n_order: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpDepartmentsExportFromProfileRequest {
    pub profile_id: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPosRequest {
    pub id: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct BdpGetPosEmployeesRequest {
    #[serde(rename = "POSId")]
    pub pos_id: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetEmployeeRequest {
    pub id: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetEmployeesRequest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_salespeople: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct BdpGetPosTendersRequest {
    #[serde(rename = "POSId")]
    pub pos_id: i32,
}

/* [157A-9] F9.2: GetArticle — consulta individual de artículo por código.
 * Path: POST /API/Articles/Get
 * Input: { "ArtCode": 1001 }
 * Output: ArticleData con campos extensos (DeptCode, TAVPer, Price1..5, etc.) */
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetArticleRequest {
    pub art_code: i64,
}

/* [157A-9] F9.3: GetPricesArticles — precios de venta de un artículo.
 * Path: POST /API/Articles/GetPrices
 * Input: { "ArtCode": 1001 }
 * Output: { "Prices": [1.05,...], "Discounts": [25.0,...], "ErrorMessage": "" } */
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPricesArticlesRequest {
    pub art_code: i64,
}

/// Respuesta tipada de `POST /API/Articles/GetPrices`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPricesArticlesResponse {
    #[serde(default)]
    pub prices: Vec<Decimal>,
    #[serde(default, alias = "Disconts")]
    pub discounts: Vec<Decimal>,
    #[serde(default)]
    pub error_message: String,
}

/* [157A-9] F9.4: GetRoomTables — mesas de un salón.
 * Path: POST /API/Room/GetTables
 * Input: { "Id": 1 }
 * Output: { "Tables": [1,3,5,...], "ErrorMessage": "" } */
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetRoomTablesRequest {
    pub id: i32,
}

/// Respuesta tipada de `POST /API/Room/GetTables`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetRoomTablesResponse {
    #[serde(default)]
    pub tables: Vec<i32>,
    #[serde(default)]
    pub error_message: String,
}

/* [157A-9] F9.4: GetRoomsTables — todos los salones con mesas.
 * Path: POST /API/Rooms/GetTables
 * Input: { "Ids": [1,2] } o {}
 * Output: { "Rooms": [{ "Id": 1, "Name": "Comedor", "Tables": [...] }], "ErrorMessage": "" } */
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetRoomsTablesRequest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<i32>,
}

/// Un salón con sus mesas en la respuesta de `GetRoomsTables`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpRoomData {
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tables: Vec<i32>,
}

/// Respuesta tipada de `POST /API/Rooms/GetTables`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetRoomsTablesResponse {
    #[serde(default)]
    pub rooms: Vec<BdpRoomData>,
    #[serde(default)]
    pub error_message: String,
}

/* [157A-9] F9.5: GetMenuDefinition, GetFastfoodDefinition, GetPackDefinition.
 * Endpoints informativos que exponen JSON raw de BDP. */

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetMenuRequest {
    pub menu_id: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetFastfoodRequest {
    pub fastfood_id: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPackRequest {
    pub pack_id: i32,
}

/* [247A-11] Fase 1 compras BDP: petición de ExportPurchaseNotes.
 * Los rangos de fecha/proveedor/serie son opcionales; BDP usa defaults. */
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportPurchaseNotesRequest {
    pub export_profile_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_supplier: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_supplier: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_serial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_serial: Option<String>,
}

/// Albarán de compra individual devuelto por `ExportPurchaseNotes`.
/// Solo se mapean los campos de cabecera confirmados; el resto del JSON se
/// conserva en `extra` y se persiste en `datos_bdp`. La estructura exacta de
/// las líneas de albarán aún no está verificada con datos reales de BDP.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpPurchaseNoteData {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub serie_albaran: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub num_albaran: Option<String>,
    #[serde(default)]
    pub fecha_albaran: Option<String>,
    #[serde(default)]
    pub cod_proveedor: Option<serde_json::Value>,
    #[serde(default)]
    pub nom_proveedor: Option<String>,
    #[serde(default)]
    pub total_albaran: Option<Decimal>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Respuesta tipada de `POST /API/ExportProfiles/PurchaseNotes`.
/// BDP devuelve `DocumentsLists` como array de albaranes.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportPurchaseNotesResponse {
    #[serde(default, alias = "DocumentsLists")]
    pub documents_lists: Vec<BdpPurchaseNoteData>,
    #[serde(default)]
    pub error_message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn endpoint_inventory_covers_pending_bdp_domains() {
        for area in [
            BdpEndpointArea::Articulos,
            BdpEndpointArea::Servicios,
            BdpEndpointArea::Clientes,
            BdpEndpointArea::Comandas,
            BdpEndpointArea::Departamentos,
            BdpEndpointArea::Terminales,
            BdpEndpointArea::Empleados,
            BdpEndpointArea::Pagos,
            BdpEndpointArea::Salones,
            BdpEndpointArea::Menus,
            BdpEndpointArea::Compras,
        ] {
            assert!(BDP_ENDPOINTS.iter().any(|endpoint| endpoint.area == area));
        }
    }

    #[test]
    fn requests_match_bdp_pascal_case_examples() {
        let articles = serde_json::to_value(BdpExportArticlesRequest::all_web_articles(1)).unwrap();
        assert_eq!(articles["Dept1"], 1);
        assert_eq!(articles["Art2"], 9_999_999_999_999_i64);
        assert!(articles.get("type_price").is_none());

        let tenders = serde_json::to_value(BdpGetPosTendersRequest { pos_id: 1 }).unwrap();
        assert_eq!(tenders["POSId"], 1);
    }

    #[test]
    fn export_articles_parses_pascal_case_and_numeric_codes() {
        let parsed: BdpExportArticlesResponse = serde_json::from_value(serde_json::json!({
            "Articles": [{"Code": 1001, "Name": "Café", "Price1": 2.5}]
        }))
        .unwrap();
        assert_eq!(parsed.articles.len(), 1);
        assert_eq!(parsed.articles[0].art_code(), Some("1001"));
    }

    /* [247A-10/S1] Tests defensivos de parsing de stock. */
    #[test]
    fn effective_stock_reads_root_current_stock() {
        let parsed: BdpExportArticlesResponse = serde_json::from_value(serde_json::json!({
            "Articles": [{"Code": "1001", "CurrentStock": 12.34}]
        }))
        .unwrap();
        assert_eq!(
            parsed.articles[0].effective_stock(),
            Some(rust_decimal::Decimal::from_str("12.34").unwrap())
        );
    }

    #[test]
    fn effective_stock_falls_back_to_prices_table_data() {
        let parsed: BdpExportArticlesResponse = serde_json::from_value(serde_json::json!({
            "Articles": [{"Code": "1002", "PricesTableData": [{"CurrentStock": 5.0}]}]
        }))
        .unwrap();
        assert_eq!(
            parsed.articles[0].effective_stock(),
            Some(rust_decimal::Decimal::from_f64_retain(5.0).unwrap())
        );
    }

    #[test]
    fn effective_stock_returns_none_when_stock_module_inactive() {
        let parsed: BdpExportArticlesResponse = serde_json::from_value(serde_json::json!({
            "Articles": [{"Code": "1003", "Price1": 2.5}]
        }))
        .unwrap();
        assert!(parsed.articles[0].effective_stock().is_none());
    }
}
