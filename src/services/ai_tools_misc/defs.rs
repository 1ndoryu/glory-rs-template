/* [01AA-4-f3q] Definiciones JSON de tools misceláneas (extraido de ai_tools_misc.rs).
 * [084A-51] show_service y list_services removidos — las service cards son
 * antinaturales en un chat conversacional. Solo factura, escalación y captura. */

use serde_json::{json, Value};

/* [084A-51] exec_show_service y exec_list_services eliminados — herramientas
 * desactivadas. Si se reactivan, recuperar del git history (anterior a 084A-51). */

pub(crate) fn service_tool_defs() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "create_invoice",
                "description": "Genera una factura de Stripe con link de pago para el cliente. Solo úsalo cuando el cliente confirme que quiere pagar y tenga claro el servicio/monto.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "amount_cents": { "type": "integer", "description": "Monto en centavos USD (ej: 10000 = $100.00)" },
                        "description": { "type": "string", "description": "Descripción del concepto de la factura" },
                        "client_email": { "type": "string", "description": "Email del cliente para la factura de Stripe" }
                    },
                    "required": ["amount_cents", "description", "client_email"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "request_human_assistance",
                "description": "Solicita intervención humana. Úsalo cuando no puedas resolver la solicitud, el cliente esté frustrado, pida hablar con una persona, o el tema sea legal/contractual.",
                "parameters": {
                    "type": "object",
                    "properties": { "reason": { "type": "string", "description": "Motivo breve de la escalación" } },
                    "required": ["reason"]
                }
            }
        }
    ])
}

/* Tools de captura de email e info del cliente (T-3) */
pub(crate) fn visitor_tool_defs() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "capture_email",
                "description": "Guarda el email del cliente. Úsalo cuando el cliente comparta su correo electrónico durante la conversación. No lo pidas de forma forzada — espera a que surja naturalmente o cuando sea necesario para una factura.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "email": { "type": "string", "description": "Email del cliente" },
                        "display_name": { "type": "string", "description": "Nombre del cliente (si lo mencionó)" }
                    },
                    "required": ["email"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "save_client_info",
                "description": "Guarda información relevante del cliente para futuras conversaciones (industria, presupuesto, intereses, tipo de proyecto, nombre). Úsalo cuando el cliente mencione datos útiles sobre su negocio, necesidades, o cuando dé su nombre.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Nombre del visitante. Úsalo cuando el cliente diga su nombre" },
                        "industry": { "type": "string", "description": "Sector o industria del cliente" },
                        "budget_range": { "type": "string", "description": "Rango de presupuesto mencionado" },
                        "interests": { "type": "array", "items": { "type": "string" }, "description": "Servicios o temas de interés" },
                        "project_description": { "type": "string", "description": "Descripción breve del proyecto que necesita" },
                        "notes": { "type": "string", "description": "Cualquier otra info relevante" }
                    }
                }
            }
        }
    ])
}

/* [T-9][095A-20] Tools de cuenta registradas y protegidas por rol efectivo. */
pub(crate) fn registered_client_tool_defs() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "list_my_orders",
                "description": "Lista pedidos visibles para la cuenta autenticada. Cliente: sus propios pedidos. Empleado: pedidos asignados. Admin efectivo: pedidos recientes globales. Úsalo antes de responder estados, fases, entregables o empleado asignado.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_my_payments",
                "description": "Lista pagos/facturas de pedidos visibles para la cuenta autenticada. No revela secretos de Stripe; informa estado, monto, fase y si puede reintentarse desde el panel.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_my_reports",
                "description": "Lista reportes/problemas abiertos o recientes visibles según rol: cliente por sus pedidos, empleado por pedidos asignados, admin efectivo global.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_order_report",
                "description": "Crea un reporte de problema sobre un pedido visible para la cuenta autenticada. Requiere order_id u order_number y una descripción clara. Valida permisos en backend.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "order_id": { "type": "string", "description": "UUID del pedido si está disponible" },
                        "order_number": { "type": "integer", "description": "Número legible del pedido si el usuario lo menciona" },
                        "reason": { "type": "string", "description": "Problema reportado, entre 10 y 2000 caracteres" }
                    },
                    "required": ["reason"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "admin_operational_summary",
                "description": "Resumen operativo global para administradores efectivos: pedidos por estado, reportes abiertos y hosting. Prohibida para clientes, empleados e impersonaciones sin rol admin efectivo.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_support_ticket",
                "description": "Crea un ticket de soporte para el cliente. Úsalo cuando el cliente reporte un problema con su hosting, pedido, facturación o necesite asistencia técnica. Esto crea una nota interna que el equipo revisará.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "enum": ["hosting_issue", "order_issue", "billing_issue", "general"],
                            "description": "Categoría del problema"
                        },
                        "description": {
                            "type": "string",
                            "description": "Descripción detallada del problema reportado"
                        },
                        "priority": {
                            "type": "string",
                            "enum": ["low", "medium", "high"],
                            "description": "Prioridad del ticket"
                        }
                    },
                    "required": ["category", "description"]
                }
            }
        }
    ])
}
