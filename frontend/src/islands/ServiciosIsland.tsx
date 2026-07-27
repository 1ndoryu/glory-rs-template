/**
 * Componente: ServiciosIsland
 * Página completa de Servicios con grid filtrable.
 * Categorías centralizadas en data/navegacion.ts (DRY).
 */
import '../styles/variables.css';
import {useTranslation} from 'react-i18next';
import {CatalogPageShell} from '../components/layout/CatalogPageShell';
import {organizationSchema} from '../components/seo/schemas';
import {BarraFiltros} from '../components/servicios/BarraFiltros';
import {GridServicios} from '../components/servicios/GridServicios';
import {useServicios} from '../hooks/useServicios';

interface ServiciosIslandProps {
    titulo?: string;
}

export const ServiciosIsland = ({titulo}: ServiciosIslandProps): JSX.Element => {
    const {t} = useTranslation();
    const {categoriaActiva, categoriasDisponibles, setCategoriaActiva, busqueda, setBusqueda, serviciosFiltrados} = useServicios();

    return (
        <CatalogPageShell
            id="paginaServicios"
            seoTitle="Nuestros Servicios de Desarrollo Web y Diseño"
            seoDescription="Descubre nuestros servicios de desarrollo web, diseño UI/UX y soluciones digitales a medida."
            path="/servicios"
            jsonLd={organizationSchema}
            title={titulo || t('services_page.title')}
            description={t('services_page.description')}
        >
            {/* [155A-19] Se elimina el menú contextual interno: el acceso rápido a servicios
                vive en el header y debe usar la misma fuente de datos que el grid público. */}
            <BarraFiltros categorias={categoriasDisponibles} categoriaActiva={categoriaActiva} busqueda={busqueda} onCategoriaChange={setCategoriaActiva} onBusquedaChange={setBusqueda} />
            <GridServicios servicios={serviciosFiltrados} />
        </CatalogPageShell>
    );
};

export default ServiciosIsland;
