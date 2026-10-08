/* Reglas de diseño de la página pública, fijadas tal cual está la página.
 * Para escalar sin romper lo construido: ningún componente de `publica/`
 * usa colores, medidas o tamaños literales; sale de aquí.
 *
 * Receta: fondo #e8e7e3, tinta #050200, sin redondeados, sin sombras,
 * texto único 16px Söhne 400 (`--texto-publica` en index.css). */

export const COLOR_FONDO = '#e8e7e3';
export const COLOR_TINTA = '#050200';
export const COLOR_RELLENO_SUAVE = '#dddbd5';
export const COLOR_ACENTO = '#F59820';

/* Fragmentos de clase compartidos (literales completos para Tailwind). */
export const CLASE_FONDO = 'bg-[#e8e7e3]';
export const CLASE_BORDE = 'border-[#050200]';
export const CLASE_TINTA = 'text-[#050200]';
export const CLASE_ACENTO = 'text-[#F59820]';
export const CLASE_DIVISOR = 'divide-[#050200]';
export const CLASE_TEXTO = 'texto-publica';
export const CLASE_ACTIVO = 'bg-[#F59820] text-[#050200]';
export const CLASE_REPOSO = 'bg-transparent text-black';
export const CLASE_RELLENO_SUAVE = 'bg-[#dddbd5]';
export const CLASE_HOVER_CAJA = 'hover:bg-[#dddbd5]';
export const CLASE_MINIATURA_ACTIVA = 'opacity-100';
export const CLASE_MINIATURA_REPOSO = 'opacity-60 hover:opacity-100';

/* Laterales del sitio: 10px en móvil y escritorio, 20px en tableta
 * (md a lg); lo usan los contenedores de página y la presentación. */
export const RELLENO_LATERAL_SITIO = 'px-[10px] md:px-[20px] lg:px-[10px]';

/* Visibilidad por tramo: solo móvil (<md) o solo escritorio (md+). */
export const SOLO_MOVIL = 'md:hidden';
export const SOLO_ESCRITORIO_ENLINEA = 'hidden md:inline';
export const SOLO_ESCRITORIO_BLOQUE = 'hidden md:block';
export const SOLO_ESCRITORIO_FLEX = 'hidden md:flex';
/* Solo móvil y tableta (<lg): la fila de tipos se oculta en esos tramos
 * y el select de tipo del modal solo aparece en ellos. */
export const SOLO_MOVIL_TABLETA = 'lg:hidden';
export const SOLO_ESCRITORIO_ANCHO = 'hidden lg:block';

/* Panel del menú móvil de la cabecera: mismo cuadro con bordes que la
 * cabecera, anclado a su borde inferior; opciones a ancho completo. */
export const PANEL_MENU_MOVIL =
  'absolute inset-x-0 top-full mt-2 divide-y divide-[#050200] border border-[#050200] bg-[#e8e7e3]';
export const OPCION_MENU_MOVIL =
  'flex w-full cursor-pointer items-center gap-2 bg-transparent px-4 py-3 text-left text-black hover:bg-[#dddbd5]';

/* Medidas de la retícula. */
export const ANCHO_PAGINA = 'max-w-[1120px]';
export const ALTO_CAJA = 'h-[100px]';
export const TAMANO_IMAGEN_CAJA = 'h-[64px] w-[64px] md:h-[80px] md:w-[80px]';
export const MARGEN_IMAGEN_CAJA = 'm-[10px]';
export const ANCHO_SPEC = 'w-16';
export const ANCHO_SPEC_LARGO = 'w-24';
export const ANCHO_PRECIO = 'w-28';

/* Elementos por página de la lista pública. */
export const POR_PAGINA = 10;

/* Cuadro de presentación: cabe junto con la cabecera en la primera pantalla.
 * Descuenta 107px = 91px del bloque cabecera (pt-4 + cabecera + mb-2,
 * medido en desktop) + 16px de margen inferior, igual que los 16px
 * superiores; queda fijo (`sticky`) y la hoja con la lista lo cubre. */
export const ALTURA_PRESENTACION = 'h-[calc(100dvh-107px)]';
export const MARGEN_INFERIOR_PRESENTACION = 'mb-4';
/* Cabecera fija con 16px de aire al borde superior (67px medidos de alto).
 * La hoja baja con el scroll único de la página, en flujo normal y sin
 * scroll propio: el cuadro desaparece al bajar y la lista completa sube
 * hasta la cabecera. */
export const TOPE_CABECERA_FIJA = 'top-4';
/* Cabecera fija al viewport (fixed, no sticky): el contenedor ya no le da
 * recorrido al quedar el cuadro fuera; ancho completo menos los laterales
 * del sitio (10px móvil/escritorio, 20px tableta) y centrada (el
 * max-w-[1120px] lo pone ANCHO_PAGINA en el header). */
export const CABECERA_FIJA =
  'fixed left-1/2 top-4 z-20 w-[calc(100%-20px)] -translate-x-1/2 md:w-[calc(100%-40px)] lg:w-[calc(100%-20px)]';
/* Cuadro fijo justo bajo la cabecera fija: 16 de aire + 67 de cabecera. */
export const TOPE_PRESENTACION_FIJA = 'top-[83px]';
/* Descanso de la hoja del concepto 1: su tope se clava 200px bajo la
 * cabecera (83+200). Reservado para la caja animada; el concepto 2 va
 * sin snap. */
export const MARGEN_ANCLA_LISTA = 'scroll-mt-[283px]';

/* Presentación en 3 fases: pista alta a pantalla completa con escenario
 * fijo; la tarjeta va de borde a borde a caja y luego se funde. */
export const PISTA_PRESENTACION = 'relative -mt-4 h-[130dvh] w-full';
export const ESCENARIO_PRESENTACION = 'sticky top-0 h-dvh w-full';
export const TARJETA_PRESENTACION = 'absolute inset-0 fases-presentacion border';
export const CENTRADO_PRESENTACION = 'relative flex h-full flex-col items-center justify-center gap-6 px-6 text-center';
export const TITULO_PRESENTACION = 'text-5xl font-bold tracking-tight md:text-6xl';
export const PARRAFO_PRESENTACION = 'max-w-[560px]';
export const FILA_BOTONES_PRESENTACION = 'flex flex-wrap items-center justify-center gap-4';
export const BOTON_PRIMARIO_PRESENTACION = 'bg-[#e8e7e3] px-6 py-3 text-[#050200] hover:bg-[#F59820]';
export const BOTON_SECUNDARIO_PRESENTACION = 'border border-[#ffffff] px-6 py-3 hover:bg-[#dddbd5] hover:text-[#050200]';
/* Foto de fondo con velo oscuro y texto claro encima. */
export const FONDO_PRESENTACION = "bg-[url('/img/presentacion.jpg')] bg-cover bg-center";
export const VELO_PRESENTACION = 'absolute inset-0 bg-[#050200]/40';
export const TEXTO_SOBRE_IMAGEN = 'text-[#ffffff]';

/* Presentación sencilla (concepto 2): titular a la izquierda en flujo
 * normal, con unos 200px de margen superior (libra de sobra la cabecera
 * fija: 16 de aire + 67 de cabecera); el pb la separa de la hoja. Solo
 * lo usa el concepto 2: la caja (concepto 1) no se ve afectada. */
export const AIRE_PRESENTACION_SENCILLA = 'mt-[200px] pb-8 text-center md:text-left';
/* Ancho máximo del titular sencillo (concepto 2): bloque estrecho y
 * centrado en móvil, 60% de la columna a la izquierda en escritorio. */
export const ANCHO_TITULO_SENCILLO = 'mx-auto max-w-[320px] md:mx-0 md:max-w-[60%]';
/* Tamaño del titular sencillo: 34px en móvil, 56px en md, con
 * interlineado apretado. Solo concepto 2. */
export const TITULO_SENCILLO = 'text-[34px] font-bold leading-[1.05] tracking-tight md:text-[56px]';
/* Cursor de la máquina de escribir del titular (concepto 2): barra de
 * acento tras la palabra rotativa, con parpadeo por pasos. */
export const CURSOR_MAQUINA = 'ml-2 inline-block h-[0.9em] w-[4px] translate-y-[0.08em] bg-[#F59820] parpadeo-cursor';

/* Aire del mensaje de estado (carga, error, vacío): el contenedor de la
 * lista va en py-0 porque las cajas gestionan su propia altura. */
export const RELLENO_VACIO = 'px-4 py-12';

/* Pie público: solo la línea de derechos, centrada y sin borde. */
export const RELLENO_PIE = 'py-6';
/* Separación del pie respecto al contenido: 20px por encima. */
export const SEPARACION_PIE = 'mt-[20px]';
