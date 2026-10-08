# Prevención: `sqlite-carga-N-consultas` ante awaits dependientes (2026-10-08)

## Caso mínimo
`src/services/turno.rs:encolar_texto_ia`: 2 `await` directos sobre la BD
(`corte_cubre` = lectura de config, `encolar_outbox_idem` = escritura en
outbox) que la regla cuenta como N+1, pese a no haber ningún bucle: son
dos operaciones dependientes (la escritura necesita el veredicto del
corte) ejecutadas una sola vez por llamada.

## Capa responsable
Regla `sqlite-carga-N-consultas` de Sentinel: cuenta `await` directos sin
distinguir dependencia dato-a-dato (lectura→escritura) de repetición en
bucle (el verdadero N+1). Contraste: `src/services/tope_uso.rs:revisar_tope`
sí era N+1 real (4 lecturas independientes) y se unificó en un solo
`SELECT` en 08AA-13.

## Detección esperada
- Si los `await` están fuera de todo bucle y cada escritura depende del
  resultado de la lectura previa, es falso positivo: documentar en el
  comentario del código (no reestructurar para callar la regla).
- Si hay bucle (`for`/`while`/iterador) con consulta dentro, es N+1 real:
  unificar con CTE/JOIN/subselects escalares como en `revisar_tope`.

## Regla
No unir consultas dependientes lectura→escritura en una sola query solo
para callar la regla: la escritura necesita el dato leído y forzar la
unión oscurece el flujo. Falso positivo documentado aquí + comentario
`[08AA-13]` en el código.

## Referencia
`src/services/turno.rs:encolar_texto_ia`; `src/services/tope_uso.rs`
(contraejemplo real corregido en 08AA-13).
