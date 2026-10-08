/* [08AA-6] Schemas de tools para el provider (datos puros, sin BD ni estado).
 * Salieron de `chat_tools.rs` (god-object >800): una fn por tool porque el
 * `vec!` monolítico superó el tope de líneas. `chat.rs` sigue llamando
 * `chat_tools::definiciones()` vía re-export en el módulo padre. */

use serde_json::{json, Value};

use glory_agent::tools::ToolDefinition;

/// Definiciones para el provider (schemas cortos, en español).
/// Una fn por tool: el `vec!` monolítico superó el tope de líneas.
pub(crate) fn definiciones() -> Vec<ToolDefinition> {
    vec![
        def_buscar(),
        def_detalle(),
        def_registrar_contacto(),
        def_enviar_fotos(),
        def_datos_contacto(),
        def_escalar(),
        def_consultar(),
        def_captacion(),
        def_visita(),
    ]
}

fn def_buscar() -> ToolDefinition {
    ToolDefinition::new(
        "buscar_inmuebles",
        "Busca inmuebles publicados y disponibles. Usala siempre antes de hablar de oferta concreta. \
         Pasa `habitaciones` (numero exacto) y `zona` siempre que el visitante los mencione: \
         son filtros exactos en BD, nunca filtres a ojo lo devuelto. \
         Las primeras tarjetas YA se envian solas como mensajes separados (mira `tarjetas_enviadas`): \
         no las repitas ni las listes en tu respuesta, solo intro de una linea + cierre breve. \
         Si llamas de nuevo con el mismo filtro, las tarjetas ya enviadas NO se reenvian \
         (mira `repetidas`): no las anuncies otra vez. \
         Si `total` es 0, dilo claro y pide otro filtro (no inventes oferta).",
        json!({
            "type": "object",
            "properties": {
                "texto": {"type": "string", "description": "Palabra en titulo o ubicacion"},
                "tipo": {"type": "string", "enum": ["apartamento", "casa", "local", "terreno", "townhouse"]},
                "operacion": {"type": "string", "enum": ["venta", "alquiler"]},
                "precio_max": {"type": "number"},
                "habitaciones": {"type": "integer", "description": "Numero exacto de habitaciones"},
                "zona": {"type": "string", "description": "Zona o sector (filtra por ubicacion)"},
                "limite": {"type": "integer", "default": 5}
            }
        }),
    )
}

fn def_detalle() -> ToolDefinition {
    ToolDefinition::new(
        "detalle_inmueble",
        "Ficha completa de un inmueble por su id (sale de buscar_inmuebles). \
         Trae `extras` con lo respondido en /ask (internet, agua, amoblado...) \
         y `margen_negociable` (bool, sin cifras: el minimo nunca se dice).",
        json!({
            "type": "object",
            "properties": {"id": {"type": "string", "format": "uuid"}},
            "required": ["id"]
        }),
    )
}

fn def_registrar_contacto() -> ToolDefinition {
    ToolDefinition::new(
        "registrar_contacto",
        "Guarda nombre y telefono del visitante cuando los da (ficha comercial; \
         no cambia el hilo actual). Solo di que quedo registrado si ESTA llamada \
         respondio exito en este turno: prohibido afirmarlo sin haberla llamado.",
        json!({
            "type": "object",
            "properties": {
                "nombre": {"type": "string"},
                "telefono": {"type": "string"}
            },
            "required": ["nombre", "telefono"]
        }),
    )
}

fn def_enviar_fotos() -> ToolDefinition {
    ToolDefinition::new(
        "enviar_fotos_inmueble",
        "Envia hasta 3 fotos del catalogo al visitante por WhatsApp (con el titulo como pie). \
         Usala cuando el visitante pida fotos de un inmueble o cuando ofrezcas enviarselas y acepte. \
         El id sale de buscar_inmuebles/detalle_inmueble. Tras llamarla, confirma en tu respuesta \
         que ya se las enviaste; no pegues URLs de fotos en el texto. \
         Las fotos ya enviadas en este hilo NO se reenvian (mira `repetidas`): \
         no las anuncies otra vez.",
        json!({
            "type": "object",
            "properties": {
                "id": {"type": "string", "format": "uuid"},
                "max": {"type": "integer", "minimum": 1, "maximum": 3, "default": 3}
            },
            "required": ["id"]
        }),
    )
}

fn def_datos_contacto() -> ToolDefinition {
    ToolDefinition::new(
        "datos_contacto",
        "Telefono y WhatsApp oficiales de la inmobiliaria. Llamala antes de dar un numero.",
        json!({"type": "object", "properties": {}}),
    )
}

/* Esquema compartido `motivo+resumen` de `escalar_a_humano`/`consultar_agente`. */
fn esquema_motivo() -> Value {
    json!({
        "type": "object",
        "properties": {
            "motivo": {"type": "string"},
            "resumen": {"type": "string", "description": "Resumen breve para la ficha del aviso (1..500)"}
        },
        "required": ["motivo"]
    })
}

fn def_escalar() -> ToolDefinition {
    ToolDefinition::new(
        "escalar_a_humano",
        "Deriva la conversacion a un humano: avisa por WhatsApp al admin y frena a la IA. Usala si el visitante pide un humano o das 2 respuestas sin resolver.",
        esquema_motivo(),
    )
}

fn def_consultar() -> ToolDefinition {
    ToolDefinition::new(
        "consultar_agente",
        "Duda puntual: pregunta a un humano sin delegar del todo. Congela la IA (retoma al responder) y avisa por WhatsApp con la ficha.",
        esquema_motivo(),
    )
}

fn def_captacion() -> ToolDefinition {
    ToolDefinition::new(
        "registrar_captacion",
        "El visitante quiere VENDER o ALQUILAR su propiedad: registra la captacion (queda `pendiente` para el captador) y le avisa por WhatsApp. Pide antes nombre, telefono, operacion, ubicacion y detalles; llama solo con esos datos.",
        json!({
            "type": "object",
            "properties": {
                "nombre": {"type": "string"},
                "telefono": {"type": "string"},
                "operacion": {"type": "string", "enum": ["venta", "alquiler"]},
                "ubicacion": {"type": "string"},
                "descripcion": {"type": "string", "description": "Detalles de la propiedad (1..2000)"},
                "puestos": {"type": "integer"},
                "residencia": {"type": "string"},
                "precio_estimado": {"type": "number"},
                "email": {"type": "string"}
            },
            "required": ["nombre", "telefono", "ubicacion", "descripcion"]
        }),
    )
}

fn def_visita() -> ToolDefinition {
    ToolDefinition::new(
        "agendar_visita",
        "El visitante quiere VISITAR un inmueble del catalogo: agenda la visita (queda `pendiente`: el agente confirma dia y hora) y congela la IA hasta que el humano confirme. Pide antes nombre, telefono y cuando quiere ir (texto libre, ej. 'el sabado en la manana'); el id sale de buscar_inmuebles/detalle_inmueble; llama solo con esos datos.",
        json!({
            "type": "object",
            "properties": {
                "id": {"type": "string", "format": "uuid"},
                "nombre": {"type": "string"},
                "telefono": {"type": "string"},
                "cuando": {"type": "string", "description": "Lo que dijo el visitante sobre cuando ir (1..200)"},
                "fecha": {"type": "string", "description": "Dia exacto YYYY-MM-DD, solo si lo dio el visitante"}
            },
            "required": ["id", "nombre", "telefono", "cuando"]
        }),
    )
}
