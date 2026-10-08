/* [08AA-7] Singleflight del asistente Marketplace en su dominio: N
 * peticiones concurrentes con la misma clave (doble clic en Regenerar,
 * reintento + original en vuelo) comparten UNA generación de IA en vez de
 * gastar N. El líder computa y difunde; los seguidores esperan el mismo
 * `Arc`. Ventana residual de microsegundos entre `send` y `remove` (el que
 * llegue ahí recomputa): best-effort honesto para doble clic humano, no
 * barrera distribuida; el `UNIQUE` + `DO NOTHING` de la tabla respalda
 * duplicados. Solo vive en memoria del proceso.
 * [08AA-6] Fan-out con `mpsc::unbounded_channel` por seguidor en vez de
 * `broadcast`: `broadcast::Sender::send` toma un `std::Mutex` interno que
 * bloquea workers tokio bajo contención; el `send` unbounded nunca espera
 * (un mensaje por seguidor, acotado por definición) y el `drop` de la lista
 * si el líder cae degrada a fallback igual que antes (fail-open).
 * Se re-exporta desde `marketplace.rs` para no mover sus 4 usos externos. */

use super::marketplace::{asegurar_contacto, FALLBACK_BORRADOR};

/// Generación compartible en vuelo: texto + fuente (`ia`, nunca `reserva` —
/// el fallback no entra al vuelo: cada miss reintenta la IA).
#[derive(Debug, Clone)]
pub struct Generado {
    pub texto: String,
    pub fuente: String,
}

#[derive(Debug, Default)]
pub struct Singleflight {
    vuelo: tokio::sync::Mutex<
        std::collections::HashMap<
            String,
            Vec<tokio::sync::mpsc::UnboundedSender<std::sync::Arc<Generado>>>,
        >,
    >,
}

impl Singleflight {
    /// Ejecuta `f` si nadie vuela con `clave`; si no, espera el resultado
    /// ajeno. Sin `Send` en `f`: se sondea inline, sin `spawn`.
    pub async fn ejecutar<F, Fut>(&self, clave: &str, f: F) -> std::sync::Arc<Generado>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Generado>,
    {
        let seguidor = {
            let mut mapa = self.vuelo.lock().await;
            if let Some(lista) = mapa.get_mut(clave) {
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                lista.push(tx);
                Some(rx)
            } else {
                mapa.insert(clave.to_string(), Vec::new());
                None
            }
        };
        /* Seguidor: el líder difunde; si el líder cayó (su lista se dropeó
         * con los `Sender`), se degrada a fallback en vez de colgar
         * (fail-open). */
        if let Some(mut rx) = seguidor {
            rx.recv().await.unwrap_or_else(|| {
                std::sync::Arc::new(Generado {
                    texto: asegurar_contacto(FALLBACK_BORRADOR),
                    fuente: "reserva".to_string(),
                })
            })
        } else {
            let gen = std::sync::Arc::new(f().await);
            let mut mapa = self.vuelo.lock().await;
            if let Some(lista) = mapa.remove(clave) {
                for tx in lista {
                    let _ = tx.send(std::sync::Arc::clone(&gen));
                }
            }
            gen
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /* Singleflight (assert del plan): 10 concurrentes con la misma clave =
     * UNA sola ejecución y el mismo `Arc` para todos. */
    #[tokio::test]
    async fn vuelo_comparte_una_generacion() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let vuelo = Singleflight::default();
        let vuelo = Arc::new(vuelo);
        let contador = Arc::new(AtomicUsize::new(0));
        let mut tareas = Vec::new();
        for _ in 0..10 {
            let v = Arc::clone(&vuelo);
            let c = Arc::clone(&contador);
            tareas.push(tokio::spawn(async move {
                v.ejecutar("clave-x", || async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    Generado {
                        texto: "hola".to_string(),
                        fuente: "ia".to_string(),
                    }
                })
                .await
            }));
        }
        let mut resultados = Vec::new();
        for t in tareas {
            resultados.push(t.await.expect("tarea"));
        }
        assert_eq!(contador.load(Ordering::SeqCst), 1);
        for r in &resultados {
            assert_eq!(r.texto, "hola");
            assert!(Arc::ptr_eq(&resultados[0], r), "mismo Arc para todos");
        }
    }
}
