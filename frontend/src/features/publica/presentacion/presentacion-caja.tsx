import { Button } from '@/components/ui/button';
import {
  BOTON_PRIMARIO_PRESENTACION,
  BOTON_SECUNDARIO_PRESENTACION,
  CENTRADO_PRESENTACION,
  CLASE_BORDE,
  CLASE_TEXTO,
  ESCENARIO_PRESENTACION,
  FILA_BOTONES_PRESENTACION,
  FONDO_PRESENTACION,
  PARRAFO_PRESENTACION,
  PISTA_PRESENTACION,
  TARJETA_PRESENTACION,
  TEXTO_SOBRE_IMAGEN,
  TITULO_PRESENTACION,
  VELO_PRESENTACION,
} from '../disenno';

type PropiedadesCaja = {
  titulo?: string;
  texto?: string;
  primario?: string;
  secundario?: string;
};

/* Concepto 1 (conservado): presentación en 3 fases ligadas al scroll
 * (sin JS): arriba cubre la pantalla sin márgenes; al bajar
 * encoge a la caja; luego se funde mientras la hoja con la lista pasa
 * por encima. Pista alta con escenario fijo: solo se animan posición y
 * opacidad (sin bucles). Contenido provisional (lorem ipsum); se define
 * el definitivo si vuelve a usarse. */
export function PresentacionCaja({
  titulo = 'Lorem ipsum dolor sit amet',
  texto = 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.',
  primario = 'Lorem ipsum',
  secundario = 'Dolor sit amet',
}: PropiedadesCaja) {
  return (
    <section aria-label="Presentación" className={PISTA_PRESENTACION}>
      <div className={ESCENARIO_PRESENTACION}>
        <div className={`${TARJETA_PRESENTACION} ${CLASE_BORDE} ${FONDO_PRESENTACION}`}>
          <div aria-hidden="true" className={VELO_PRESENTACION} />
          <div className={CENTRADO_PRESENTACION}>
            <h1 className={`${TITULO_PRESENTACION} ${TEXTO_SOBRE_IMAGEN}`}>{titulo}</h1>
            <p className={`${CLASE_TEXTO} ${PARRAFO_PRESENTACION} ${TEXTO_SOBRE_IMAGEN}`}>{texto}</p>
            <div className={FILA_BOTONES_PRESENTACION}>
              <Button type="button" variant="ghost" className={`${CLASE_TEXTO} ${BOTON_PRIMARIO_PRESENTACION}`}>
                {primario}
              </Button>
              <Button type="button" variant="ghost" className={`${CLASE_TEXTO} ${BOTON_SECUNDARIO_PRESENTACION} ${TEXTO_SOBRE_IMAGEN}`}>
                {secundario}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
