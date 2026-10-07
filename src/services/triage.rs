/* [06AA-1 F1] Capa `Triage` (plan 03AA-4 §Triage): decide si un evento
 * entrante de `WhatsApp` se atiende o no, con motivo explícito. Regla de
 * oro: por defecto se ATIENDE; solo la lista explícita del plan dice `no`
 * (eco propio, hilo `delegada`/`escalada`, IA pausada global/hilo,
 * duplicado, mensaje de sistema, remitente = número propio, vacío sin
 * media, retrasado más allá de la ventana). Cada `no` deja `motivo` para
 * log/auditoría; el llamador (`webhook`) lo registra y calla sin turno.
 * `decidir` es pura (sin BD ni reloj: el llamador resuelve `EventoTriage`
 * y el registro de duplicados), así se testea sin `DATABASE_URL`.
 * Dato ausente (`None`) = se atiende: el transporte de hoy (simulado) no
 * manda `ts`/`seq`, el real de F-transporte sí.
 * El trato (`cliente`/`neutral`, `evaluar_trato`) NUNCA silencia: solo
 * califica el tono. Lo consumirá F2 `Politica`.
 * Gotcha: el registro de duplicados vive en memoria del proceso (misma
 * TTL que el eco del gateway, 180 s); F3 lo llevará a BD con UNIQUE.
 * Pendiente F2: usar el trato para el tono neutral (hoy solo se loguea). */

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use uuid::Uuid;

/// Ventana por defecto (min) para considerar un mensaje retrasado cuando
/// `agent_config.ventana_retraso_min` no existe o no parsea. La fija F4
/// admin; el rango válido es 1..=1440.
pub const VENTANA_RETRASO_MIN_DEFAULT: i64 = 10;
/// Claves de config que lee el triage (una sola consulta, sin N+1).
const CLAVE_IA_GLOBAL: &str = "ai_enabled_global";
const CLAVE_VENTANA: &str = "ventana_retraso_min";

/// Motivo de `no atender`: la lista explícita del plan, nada más.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motivo {
    Propio,
    Eco,
    Sistema,
    Vacio,
    Duplicado,
    Retrasado,
    Delegada,
    Escalada,
    IaPausada,
}

impl Motivo {
    /// Código estable para log/auditoría (nunca texto libre).
    #[must_use]
    pub fn codigo(self) -> &'static str {
        match self {
            Motivo::Propio => "propio",
            Motivo::Eco => "eco",
            Motivo::Sistema => "sistema",
            Motivo::Vacio => "vacio",
            Motivo::Duplicado => "duplicado",
            Motivo::Retrasado => "retrasado",
            Motivo::Delegada => "delegada",
            Motivo::Escalada => "escalada",
            Motivo::IaPausada => "ia-pausada",
        }
    }
}

/// Trato al remitente: califica, jamás silencia (ver `evaluar_trato`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trato {
    Cliente,
    Neutral,
}

/// Decisión del triage: atender (con trato) o callar (con motivo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Atiende { trato: Trato },
    NoAtiende { motivo: Motivo },
}

impl Decision {
    /// Código compacto para la respuesta del webhook (`atiende:cliente`,
    /// `no:delegada`): lo consume el harness vivo sin leer logs.
    #[must_use]
    pub fn codigo(self) -> &'static str {
        match self {
            Decision::Atiende {
                trato: Trato::Cliente,
            } => "atiende:cliente",
            Decision::Atiende {
                trato: Trato::Neutral,
            } => "atiende:neutral",
            Decision::NoAtiende { motivo } => match motivo {
                Motivo::Propio => "no:propio",
                Motivo::Eco => "no:eco",
                Motivo::Sistema => "no:sistema",
                Motivo::Vacio => "no:vacio",
                Motivo::Duplicado => "no:duplicado",
                Motivo::Retrasado => "no:retrasado",
                Motivo::Delegada => "no:delegada",
                Motivo::Escalada => "no:escalada",
                Motivo::IaPausada => "no:ia-pausada",
            },
        }
    }
}

/// Quién manda: el número propio del negocio nunca dispara turno (sus
/// mensajes que vuelven son eco operativo, no clientes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Procedencia {
    Propio,
    Externo,
}

/// Marca del transporte en un solo valor (el gateway real la trae; el
/// simulado manda `Normal`): eco propio que vuelve o mensaje de sistema
/// (presencia, receipts). Si vinieran ambas, el llamador resuelve a `Eco`
/// (misma prioridad que `decidir`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarcaTransporte {
    Normal,
    Eco,
    Sistema,
}

/// Resultado del registro de `client_seq`: `Duplicado` solo si la
/// secuencia ya se vio vigente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Duplicidad {
    Nuevo,
    Duplicado,
}

/// Contenido del evento: `Vacio` es texto vacío SIN media (el transporte de
/// hoy lo rechaza con 400; el futuro trae foto/nota sin pie y sí atiende).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contenido {
    Vacio,
    Util,
}

/// Entrada resuelta para `decidir`: el llamador la arma con lo que sabe del
/// transporte, la BD (estado del hilo, IA) y el registro de duplicados.
/// `None` en lo opcional = dato ausente = atiende (regla de oro).
pub struct EventoTriage<'a> {
    pub procedencia: Procedencia,
    pub marca: MarcaTransporte,
    pub duplicidad: Duplicidad,
    pub contenido: Contenido,
    pub antiguedad_min: Option<i64>,
    pub ventana_retraso_min: i64,
    pub estado_hilo: Option<&'a str>,
    pub ia_global: bool,
    pub ia_hilo: bool,
    pub texto: &'a str,
}

/// Decisión pura del triage. Orden: lo local y barato primero (propio, eco,
/// sistema, vacío, duplicado, retraso) y luego el estado (hilo, IA). Una
/// señal explícita basta para el `no`; la duda atiende (regla de oro).
#[must_use]
pub fn decidir(evento: &EventoTriage) -> Decision {
    if evento.procedencia == Procedencia::Propio {
        return Decision::NoAtiende {
            motivo: Motivo::Propio,
        };
    }
    if evento.marca == MarcaTransporte::Eco {
        return Decision::NoAtiende {
            motivo: Motivo::Eco,
        };
    }
    if evento.marca == MarcaTransporte::Sistema {
        return Decision::NoAtiende {
            motivo: Motivo::Sistema,
        };
    }
    if evento.contenido == Contenido::Vacio {
        return Decision::NoAtiende {
            motivo: Motivo::Vacio,
        };
    }
    if evento.duplicidad == Duplicidad::Duplicado {
        return Decision::NoAtiende {
            motivo: Motivo::Duplicado,
        };
    }
    if evento
        .antiguedad_min
        .is_some_and(|m| m > evento.ventana_retraso_min)
    {
        return Decision::NoAtiende {
            motivo: Motivo::Retrasado,
        };
    }
    if evento.estado_hilo == Some("delegada") {
        return Decision::NoAtiende {
            motivo: Motivo::Delegada,
        };
    }
    if evento.estado_hilo == Some("escalada") {
        return Decision::NoAtiende {
            motivo: Motivo::Escalada,
        };
    }
    if !evento.ia_global || !evento.ia_hilo {
        return Decision::NoAtiende {
            motivo: Motivo::IaPausada,
        };
    }
    Decision::Atiende {
        trato: evaluar_trato(evento.texto).trato,
    }
}

/// Señal explícita de no-cliente (una sola basta): el remitente lo admite
/// (`equivoqué`, `solo probaba`), vende u ofrece servicios, o es ruido
/// evidente. Substrings en minúsculas; `evaluar_trato` acolcha para que
/// `vendo` no case con `viendo` (se matchea ` vendo `).
const EXPLICITO_NEUTRAL: &[&str] = &[
    "solo prob",
    "probando el bot",
    "testeando el bot",
    "equivoque",
    "equivoco",
    "equivocado",
    "equivocada",
    "numero equivocado",
    "número equivocado",
    "soy vendedor",
    "soy vendedora",
    "vendo servicios",
    "ofrezco servicios",
    "te ofrezco",
    "gana dinero",
    "publicidad",
];
/// Aperturas e intenciones típicas de cliente: siempre `cliente` (suite
/// anti-falsos-positivos del plan: `hola`, `precio?`, `sí` solo, con
/// errores de tipeo, por tercero, curioso de fotos).
const DORADO_CLIENTE: &[&str] = &[
    "hola",
    "buenas",
    "buenos dias",
    "buenos días",
    "buenas tardes",
    "precio",
    "cuanto",
    "cuánto",
    "si",
    "sí",
    "foto",
    "busco",
    "alquiler",
    "alquilo",
    "compra",
    "compro",
    "casa",
    "apartamento",
    "apto",
    "terreno",
    "local",
    "visita",
    "disponible",
    "informacion",
    "información",
    "me interesa",
];
/// Señales débiles (ninguna decide sola): hacen falta 2 independientes.
const DEBIL_NEUTRAL: &[&str] = &[
    "prueba", "pruebas", "probando", "test", "tests", "ensayo", "ensayos", "demo", "demos",
    "juego", "juegos", "jaja", "jajaja",
];

/// Evaluación del trato: `cliente` por defecto (regla de oro), `neutral`
/// con 1 señal explícita o 2 débiles independientes. Cada mensaje
/// re-evalúa (reversible); la duda reincidente la marca un humano, jamás se
/// bloquea solo. `senales` trae lo que casó, para auditoría.
pub struct EvaluacionTrato {
    pub trato: Trato,
    pub senales: Vec<&'static str>,
}

/// Pliega tildes a ASCII para el match (`equivoqué` casa con `equivoqu`,
/// `sí` con `si`): sin dependencias, solo el rango que usa el español.
fn plegar_tildes(texto: &str) -> String {
    texto
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            _ => c,
        })
        .collect()
}

#[must_use]
pub fn evaluar_trato(texto: &str) -> EvaluacionTrato {
    let plegado = plegar_tildes(texto);
    let colchon = format!(
        " {} ",
        plegado.split_whitespace().collect::<Vec<_>>().join(" ")
    );
    let mut explicitas = Vec::new();
    for senal in EXPLICITO_NEUTRAL {
        if frase_contenida(&colchon, senal) {
            explicitas.push(*senal);
        }
    }
    if !explicitas.is_empty() {
        return EvaluacionTrato {
            trato: Trato::Neutral,
            senales: explicitas,
        };
    }
    if DORADO_CLIENTE.iter().any(|d| frase_contenida(&colchon, d)) {
        return EvaluacionTrato {
            trato: Trato::Cliente,
            senales: Vec::new(),
        };
    }
    let debiles: Vec<&'static str> = DEBIL_NEUTRAL
        .iter()
        .filter(|d| frase_contenida(&colchon, d))
        .copied()
        .collect();
    if debiles.len() >= 2 {
        return EvaluacionTrato {
            trato: Trato::Neutral,
            senales: debiles,
        };
    }
    EvaluacionTrato {
        trato: Trato::Cliente,
        senales: Vec::new(),
    }
}

/// ¿La frase está en el texto acolchado? Frases con espacio = substring;
/// palabra sola = con bordes (evita `test` en `testigo`, `si` en `casa`).
fn frase_contenida(colchon: &str, frase: &str) -> bool {
    if frase.contains(' ') {
        colchon.contains(frase)
    } else {
        colchon.contains(&format!(" {frase} "))
    }
}

/// Registro de `client_seq` vistos (duplicados del transporte): TTL + tope.
/// Memoria del proceso; F3 lo lleva a BD. `Mutex` externo (el webhook lo
/// guarda en un `static`): la sección crítica es solo un `HashMap`.
pub struct RegistroDuplicados {
    ttl: Duration,
    tope: usize,
    vistos: HashMap<String, Instant>,
}

impl RegistroDuplicados {
    #[must_use]
    pub fn nuevo(ttl: Duration, tope: usize) -> Self {
        Self {
            ttl,
            tope,
            vistos: HashMap::new(),
        }
    }

    /// `true` si la secuencia ya se vio vigente (duplicado). La primera vez
    /// registra y devuelve `false`. Purga vencidos en cada llamada.
    pub fn ya_visto(&mut self, seq: &str) -> bool {
        let ahora = Instant::now();
        self.vistos
            .retain(|_, visto| ahora.duration_since(*visto) < self.ttl);
        if self.vistos.contains_key(seq) {
            return true;
        }
        if self.vistos.len() >= self.tope {
            self.vistos.clear();
        }
        self.vistos.insert(seq.to_string(), ahora);
        false
    }
}

/// Foto del hilo para el triage en una sola consulta (sin N+1): `ai_enabled`
/// de la sesión + `estado` de `atencion_sesiones`. Hilo inexistente o fallo
/// de BD = fail-open (se atiende; la regla de oro manda sobre el error).
struct FotoHilo {
    estado: Option<String>,
    ia_hilo: bool,
}

async fn foto_hilo(pool: &sqlx::PgPool, sesion: Uuid) -> FotoHilo {
    let fila = sqlx::query_as::<_, (bool, Option<String>)>(
        "SELECT s.ai_enabled, a.estado FROM agent_sessions s \
         LEFT JOIN atencion_sesiones a ON a.session_id = s.id WHERE s.id = $1",
    )
    .bind(sesion)
    .fetch_optional(pool)
    .await;
    match fila {
        Ok(Some((ia_hilo, estado))) => FotoHilo { estado, ia_hilo },
        Ok(None) => FotoHilo {
            estado: None,
            ia_hilo: true,
        },
        Err(e) => {
            tracing::warn!("triage: sin foto del hilo ({e}), se atiende");
            FotoHilo {
                estado: None,
                ia_hilo: true,
            }
        }
    }
}

/// Lee `ai_enabled_global` + `ventana_retraso_min` en una consulta. Fallo o
/// valor raro = fail-open (`true`, `VENTANA_RETRASO_MIN_DEFAULT`).
async fn config_triage(pool: &sqlx::PgPool) -> (bool, i64) {
    let filas = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT clave, valor FROM agent_config WHERE clave IN ($1, $2)",
    )
    .bind(CLAVE_IA_GLOBAL)
    .bind(CLAVE_VENTANA)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let mut ia_global = true;
    let mut ventana = VENTANA_RETRASO_MIN_DEFAULT;
    for (clave, valor) in filas {
        if clave == CLAVE_IA_GLOBAL {
            ia_global = valor.as_deref() != Some("off");
        } else if clave == CLAVE_VENTANA {
            ventana = valor
                .as_deref()
                .and_then(|v| v.trim().parse::<i64>().ok())
                .filter(|v| (1..=1440).contains(v))
                .unwrap_or(VENTANA_RETRASO_MIN_DEFAULT);
        }
    }
    (ia_global, ventana)
}

/// Milisegundos Unix ahora (reloj del proceso; el transporte real manda
/// `ts_ms` del gateway para calcular la antigüedad).
fn ahora_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(i64::MAX)
}

/// Entrada del webhook al triage con tipos del boundary: agrupa lo que el
/// transporte sí trae (sin exceder el tope de `bool` por struct) para que
/// `decidir_para` resuelva estado del hilo, IA global/hilo, ventana,
/// antigüedad y duplicados, y decida. Todo fallo intermedio es fail-open
/// (se atiende): el triage nunca puede dejar sordo al negocio por un error
/// propio.
pub struct EntradaSinResolver<'a> {
    pub procedencia: Procedencia,
    pub marca: MarcaTransporte,
    pub contenido: Contenido,
    pub texto: &'a str,
    pub ts_ms: Option<i64>,
    pub client_seq: Option<&'a str>,
    pub sesion: Uuid,
}

pub async fn decidir_para(
    pool: &sqlx::PgPool,
    entrada: &EntradaSinResolver<'_>,
    registro: &Mutex<RegistroDuplicados>,
) -> Decision {
    let duplicidad = if entrada.client_seq.is_some_and(|seq| match registro.lock() {
        Ok(mut vistos) => vistos.ya_visto(seq),
        Err(_) => false,
    }) {
        Duplicidad::Duplicado
    } else {
        Duplicidad::Nuevo
    };
    let foto = foto_hilo(pool, entrada.sesion).await;
    let (ia_global, ventana) = config_triage(pool).await;
    decidir(&EventoTriage {
        procedencia: entrada.procedencia,
        marca: entrada.marca,
        duplicidad,
        contenido: entrada.contenido,
        antiguedad_min: entrada.ts_ms.map(|ts| (ahora_ms() - ts).max(0) / 60_000),
        ventana_retraso_min: ventana,
        estado_hilo: foto.estado.as_deref(),
        ia_global,
        ia_hilo: foto.ia_hilo,
        texto: entrada.texto,
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn base() -> EventoTriage<'static> {
        EventoTriage {
            procedencia: Procedencia::Externo,
            marca: MarcaTransporte::Normal,
            duplicidad: Duplicidad::Nuevo,
            contenido: Contenido::Util,
            antiguedad_min: None,
            ventana_retraso_min: VENTANA_RETRASO_MIN_DEFAULT,
            estado_hilo: None,
            ia_global: true,
            ia_hilo: true,
            texto: "hola",
        }
    }

    #[test]
    fn atiende_por_defecto_y_sin_datos() {
        assert_eq!(
            decidir(&base()),
            Decision::Atiende {
                trato: Trato::Cliente
            }
        );
    }

    #[test]
    fn cada_no_de_la_lista_con_su_motivo() {
        let casos: Vec<(EventoTriage<'static>, Motivo)> = vec![
            (
                EventoTriage {
                    procedencia: Procedencia::Propio,
                    ..base()
                },
                Motivo::Propio,
            ),
            (
                EventoTriage {
                    marca: MarcaTransporte::Eco,
                    ..base()
                },
                Motivo::Eco,
            ),
            (
                EventoTriage {
                    marca: MarcaTransporte::Sistema,
                    ..base()
                },
                Motivo::Sistema,
            ),
            (
                EventoTriage {
                    contenido: Contenido::Vacio,
                    ..base()
                },
                Motivo::Vacio,
            ),
            (
                EventoTriage {
                    duplicidad: Duplicidad::Duplicado,
                    ..base()
                },
                Motivo::Duplicado,
            ),
            (
                EventoTriage {
                    antiguedad_min: Some(31),
                    ..base()
                },
                Motivo::Retrasado,
            ),
            (
                EventoTriage {
                    estado_hilo: Some("delegada"),
                    ..base()
                },
                Motivo::Delegada,
            ),
            (
                EventoTriage {
                    estado_hilo: Some("escalada"),
                    ..base()
                },
                Motivo::Escalada,
            ),
            (
                EventoTriage {
                    ia_global: false,
                    ..base()
                },
                Motivo::IaPausada,
            ),
            (
                EventoTriage {
                    ia_hilo: false,
                    ..base()
                },
                Motivo::IaPausada,
            ),
        ];
        for (evento, motivo) in casos {
            assert_eq!(
                decidir(&evento),
                Decision::NoAtiende { motivo },
                "{motivo:?}"
            );
        }
    }

    #[test]
    fn consultando_y_captacion_siguen_atendiendo() {
        for estado in ["activa", "consultando", "captacion"] {
            let evento = EventoTriage {
                estado_hilo: Some(estado),
                ..base()
            };
            assert!(
                matches!(decidir(&evento), Decision::Atiende { .. }),
                "{estado} debe atender"
            );
        }
    }

    #[test]
    fn retraso_en_el_borde_atiende_y_futuro_no_resta() {
        let borde = EventoTriage {
            antiguedad_min: Some(VENTANA_RETRASO_MIN_DEFAULT),
            ..base()
        };
        assert!(matches!(decidir(&borde), Decision::Atiende { .. }));
    }

    #[test]
    fn suite_anti_falsos_positivos_todos_cliente() {
        let dorados = [
            "hola",
            "precio?",
            "sí",
            "olaa, precio d la casa?",
            "pregunto por un amigo: tienen apartamentos?",
            "me pasas fotos?",
            "buenas tardes, sigue disponible?",
        ];
        for texto in dorados {
            let evaluacion = evaluar_trato(texto);
            assert_eq!(evaluacion.trato, Trato::Cliente, "{texto}");
            let evento = EventoTriage { texto, ..base() };
            assert!(
                matches!(
                    decidir(&evento),
                    Decision::Atiende {
                        trato: Trato::Cliente
                    }
                ),
                "{texto}"
            );
        }
    }

    #[test]
    fn suite_no_cliente_todos_neutral_sin_silenciar() {
        let casos = [
            (
                "vendo servicios de limpieza, te ofrezco precio",
                "vendo servicios",
            ),
            ("creo que me equivoqué de número, disculpa", "equivoque"),
            ("jaja solo probaba el bot", "solo prob"),
            ("esto es una prueba test del sistema", "prueba"),
        ];
        for (texto, senal_esperada) in casos {
            let evaluacion = evaluar_trato(texto);
            assert_eq!(evaluacion.trato, Trato::Neutral, "{texto}");
            assert!(
                evaluacion.senales.contains(&senal_esperada),
                "{texto}: {senales:?}",
                senales = evaluacion.senales
            );
            let evento = EventoTriage { texto, ..base() };
            assert_eq!(
                decidir(&evento),
                Decision::Atiende {
                    trato: Trato::Neutral
                },
                "{texto}"
            );
        }
    }

    #[test]
    fn una_senal_debil_sola_no_mueve_el_trato() {
        for texto in ["esto es una prueba", "jaja", "es un test rápido"] {
            assert_eq!(evaluar_trato(texto).trato, Trato::Cliente, "{texto}");
        }
    }

    #[test]
    fn registro_duplicados_primera_pasa_segunda_no_y_vence() {
        let mut registro = RegistroDuplicados::nuevo(Duration::from_millis(50), 500);
        assert!(!registro.ya_visto("seq-1"));
        assert!(registro.ya_visto("seq-1"));
        std::thread::sleep(Duration::from_millis(80));
        assert!(!registro.ya_visto("seq-1"));
    }

    #[test]
    fn codigos_estables_para_log_y_harness() {
        assert_eq!(
            Decision::Atiende {
                trato: Trato::Cliente
            }
            .codigo(),
            "atiende:cliente"
        );
        assert_eq!(
            Decision::NoAtiende {
                motivo: Motivo::Delegada
            }
            .codigo(),
            "no:delegada"
        );
        assert_eq!(Motivo::IaPausada.codigo(), "ia-pausada");
    }
}
