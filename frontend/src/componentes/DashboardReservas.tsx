/* [263A-16] DashboardReservas — reescrito con shadcn Card + Button + Tailwind.
 * [263A-25] Gráficos migrados a ChartContainer de shadcn — colores adaptativos dark/light.
 * [263A-27] Fusionado con Inicio: tab "General" = acciones rápidas + resumen económico + reservas hoy. */

import {useState} from 'react';
import {useDashboardReservas, useResumen, useConteoReservas, ResumenReservas, OcupacionReservas, AnalisisReservas, AgrupacionCanal} from '../api/generated';
import {Button} from '@/components/ui/button';
import {Card, CardContent, CardHeader, CardTitle} from '@/components/ui/card';
import {Table, TableBody, TableCell, TableHead, TableHeader, TableRow} from '@/components/ui/table';
import {Select, SelectContent, SelectItem, SelectTrigger, SelectValue} from '@/components/ui/select';
import {Dialog, DialogContent, DialogHeader, DialogTitle} from '@/components/ui/dialog';
import {ChartContainer, ChartTooltip, ChartTooltipContent, type ChartConfig} from '@/components/ui/chart';
import {Tabs, TabsList, TabsTrigger, TabsContent} from '@/components/ui/tabs';
import {BarChart, Bar, XAxis, YAxis, CartesianGrid, PieChart, Pie, Cell, Legend} from 'recharts';
import {DollarSign, TrendingDown, TrendingUp, CalendarDays} from 'lucide-react';
import FormularioVenta from './FormularioVenta';
import FormularioGasto from './FormularioGasto';
import FormularioReserva from './FormularioReserva';

/* CSS vars del tema — se adaptan automáticamente a dark/light mode */
const PIE_COLORS = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-3)', 'var(--chart-4)', 'var(--chart-5)'];
const CONFIG_SIMPLE: ChartConfig = {total: {label: 'Reservas', color: 'var(--chart-1)'}};
const CONFIG_TURNO: ChartConfig = {
    total: {label: 'Reservas', color: 'var(--chart-1)'},
    personas: {label: 'Personas', color: 'var(--chart-2)'}
};

/* [283A-7] Acciones rápidas extraídas al header del dashboard.
 * Muestran botones + abren modales de creación. */
function AccionesRapidas() {
    const [modalVenta, setModalVenta] = useState(false);
    const [modalGasto, setModalGasto] = useState(false);
    const [modalReserva, setModalReserva] = useState(false);

    return (
        <>
            <div className="flex flex-wrap gap-2">
                <Button onClick={() => setModalVenta(true)} className="bg-green-600 hover:bg-green-700 text-white">
                    + Venta
                </Button>
                <Button onClick={() => setModalGasto(true)} variant="default">
                    + Gasto
                </Button>
                <Button onClick={() => setModalReserva(true)} variant="secondary">
                    + Reserva
                </Button>
            </div>
            <Dialog open={modalVenta} onOpenChange={setModalVenta}>
                <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
                    <DialogHeader><DialogTitle>Nueva Venta</DialogTitle></DialogHeader>
                    <FormularioVenta onExito={() => setModalVenta(false)} />
                </DialogContent>
            </Dialog>
            <Dialog open={modalGasto} onOpenChange={setModalGasto}>
                {/* [283A-30] sm:max-w-lg = 512px (tarea 18) */}
                <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
                    <DialogHeader><DialogTitle>Nuevo Gasto</DialogTitle></DialogHeader>
                    <FormularioGasto onExito={() => setModalGasto(false)} />
                </DialogContent>
            </Dialog>
            <Dialog open={modalReserva} onOpenChange={setModalReserva}>
                <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
                    <DialogHeader><DialogTitle>Nueva Reserva</DialogTitle></DialogHeader>
                    <FormularioReserva onExito={() => setModalReserva(false)} />
                </DialogContent>
            </Dialog>
        </>
    );
}

function DashboardReservas() {
    const ahora = new Date();
    const [anio, setAnio] = useState(ahora.getFullYear());
    const [mes, setMes] = useState(ahora.getMonth() + 1);
    const [pestana, setPestana] = useState<string>('general');

    const {data, isLoading} = useDashboardReservas({year: anio, month: mes});
    const dashboard = data?.status === 200 ? data.data : null;

    const meses = ['Enero', 'Febrero', 'Marzo', 'Abril', 'Mayo', 'Junio', 'Julio', 'Agosto', 'Septiembre', 'Octubre', 'Noviembre', 'Diciembre'];

    /* [283A-12] Filtrar meses/años futuros — tarea 46 */
    const anioActual = ahora.getFullYear();
    const mesActual = ahora.getMonth() + 1;
    const aniosDisponibles = [anio - 1, anio, anio + 1].filter(a => a <= anioActual);

    return (
        <div className="flex flex-col gap-6">
            <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-3">
                    <Select value={String(mes)} onValueChange={v => setMes(Number(v))}>
                        <SelectTrigger className="w-36">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {meses.map((nombre, i) => {
                                const mesFuturo = anio === anioActual && (i + 1) > mesActual;
                                return (
                                    <SelectItem key={i + 1} value={String(i + 1)} disabled={mesFuturo}>
                                        {nombre}
                                    </SelectItem>
                                );
                            })}
                        </SelectContent>
                    </Select>
                    <Select value={String(anio)} onValueChange={v => {
                        const nuevoAnio = Number(v);
                        setAnio(nuevoAnio);
                        if (nuevoAnio === anioActual && mes > mesActual) setMes(mesActual);
                    }}>
                        <SelectTrigger className="w-24">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {aniosDisponibles.map(a => (
                                <SelectItem key={a} value={String(a)}>
                                    {a}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
                {/* [283A-7] Acciones rápidas reubicadas al header, alineadas a la derecha */}
                <AccionesRapidas />
            </div>

            <Tabs value={pestana} onValueChange={setPestana}>
                <TabsList variant="line">
                    <TabsTrigger value="general">General</TabsTrigger>
                    <TabsTrigger value="ocupacion">Ocupación</TabsTrigger>
                    <TabsTrigger value="analisis">Análisis</TabsTrigger>
                </TabsList>

                {/* [283A-18] General + Resumen combinados en un solo tab */}
                <TabsContent value="general" className="space-y-6">
                    <PanelGeneral anio={anio} mes={mes} />
                    {isLoading ? <p className="text-sm text-muted-foreground text-center py-8">Cargando dashboard...</p> : dashboard && <PanelResumen data={dashboard.resumen} />}
                </TabsContent>
                <TabsContent value="ocupacion" className="space-y-6">{isLoading ? <p className="text-sm text-muted-foreground text-center py-8">Cargando dashboard...</p> : dashboard && <PanelOcupacion data={dashboard.ocupacion} />}</TabsContent>
                <TabsContent value="analisis" className="space-y-6">{isLoading ? <p className="text-sm text-muted-foreground text-center py-8">Cargando dashboard...</p> : dashboard && <PanelAnalisis data={dashboard.analisis} />}</TabsContent>
            </Tabs>
        </div>
    );
}

/* [263A-27] Panel General: resumen económico + conteo reservas.
 * [283A-7] Acciones rápidas movidas al header del dashboard. */
function PanelGeneral({anio, mes}: {anio: number; mes: number}) {
    const {data: resumen, isLoading: cargandoResumen} = useResumen({year: anio, month: mes});
    const {data: conteo, isLoading: cargandoConteo} = useConteoReservas();

    const datosResumen = resumen?.status === 200 ? resumen.data : null;
    const datosConteo = conteo?.status === 200 ? conteo.data : null;

    const formatearMoneda = (valor: string) => new Intl.NumberFormat('es-ES', {style: 'currency', currency: 'EUR'}).format(parseFloat(valor));

    return (
        <>
            {/* Resumen económico */}
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className="text-sm font-medium">Ventas</CardTitle>
                        <TrendingUp className="size-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold text-green-600">{cargandoResumen ? '...' : datosResumen ? formatearMoneda(datosResumen.total_ventas) : '—'}</div>
                        <p className="text-xs text-muted-foreground">Total del mes</p>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className="text-sm font-medium">Gastos</CardTitle>
                        <TrendingDown className="size-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold text-red-600">{cargandoResumen ? '...' : datosResumen ? formatearMoneda(datosResumen.total_gastos) : '—'}</div>
                        <p className="text-xs text-muted-foreground">Total del mes</p>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className="text-sm font-medium">Margen</CardTitle>
                        <DollarSign className="size-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className={`text-2xl font-bold ${datosResumen && parseFloat(datosResumen.margen) >= 0 ? 'text-green-600' : 'text-red-600'}`}>{cargandoResumen ? '...' : datosResumen ? formatearMoneda(datosResumen.margen) : '—'}</div>
                        <p className="text-xs text-muted-foreground">Ventas − Gastos</p>
                    </CardContent>
                </Card>
            </div>

            {/* Reservas */}
            <div className="grid gap-4 sm:grid-cols-2">
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className="text-sm font-medium">Reservas hoy</CardTitle>
                        <CalendarDays className="size-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold">{cargandoConteo ? '...' : datosConteo ? datosConteo.total_hoy : 0}</div>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className="text-sm font-medium">Reservas este mes</CardTitle>
                        <CalendarDays className="size-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold">{cargandoConteo ? '...' : datosConteo ? datosConteo.total_mes : 0}</div>
                    </CardContent>
                </Card>
            </div>
        </>
    );
}

function PanelResumen({data}: {data: ResumenReservas}) {
    const signoVariacion = data.variacion_porcentaje >= 0 ? '+' : '';

    return (
        <>
            <div className="grid gap-4 sm:grid-cols-2">
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Total reservas</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.total_reservas}</span>
                        <p className={`text-xs ${data.variacion_porcentaje >= 0 ? 'text-green-600' : 'text-destructive'}`}>
                            {signoVariacion}
                            {data.variacion_porcentaje}% vs mes anterior ({data.total_mes_anterior})
                        </p>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Clientes nuevos</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.clientes_nuevos}</span>
                    </CardContent>
                </Card>
            </div>

            {/* [283A-29] Charts en grid de 2 columnas */}
            <div className="grid gap-4 lg:grid-cols-2">
                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Reservas por día</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <ChartContainer config={CONFIG_SIMPLE} className="h-[220px] w-full">
                            <BarChart data={data.por_dia.map((d: {fecha: string; total: number}) => ({...d, fecha: d.fecha.slice(5)}))} margin={{left: 0, right: 0}}>
                                <CartesianGrid vertical={false} />
                                <XAxis dataKey="fecha" tickLine={false} axisLine={false} fontSize={11} />
                                <YAxis allowDecimals={false} tickLine={false} axisLine={false} width={30} />
                                <ChartTooltip content={<ChartTooltipContent />} />
                                <Bar dataKey="total" fill="var(--color-total)" radius={4} />
                            </BarChart>
                        </ChartContainer>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Por día de semana</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <ChartContainer config={CONFIG_SIMPLE} className="h-[220px] w-full">
                            <BarChart data={data.por_dia_semana} margin={{left: 0, right: 0}}>
                                <CartesianGrid vertical={false} />
                                <XAxis dataKey="dia" tickLine={false} axisLine={false} fontSize={11} />
                                <YAxis allowDecimals={false} tickLine={false} axisLine={false} width={30} />
                                <ChartTooltip content={<ChartTooltipContent />} />
                                <Bar dataKey="total" fill="var(--color-total)" radius={4} />
                            </BarChart>
                        </ChartContainer>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Distribución por canal</CardTitle>
                    </CardHeader>
                    <CardContent>
                        {data.por_canal.length > 0 ? (
                            <ChartContainer config={CONFIG_SIMPLE} className="h-[220px] w-full">
                                <PieChart margin={{left: 0, right: 0}}>
                                    <Pie data={data.por_canal} dataKey="total" nameKey="canal" cx="50%" cy="50%" outerRadius={80}>
                                        {data.por_canal.map((_: AgrupacionCanal, i: number) => (
                                            <Cell key={i} fill={PIE_COLORS[i % PIE_COLORS.length]} />
                                        ))}
                                    </Pie>
                                    <ChartTooltip content={<ChartTooltipContent />} />
                                    <Legend />
                                </PieChart>
                            </ChartContainer>
                        ) : (
                            <p className="text-sm text-muted-foreground text-center py-8">Sin datos de canales</p>
                        )}
                    </CardContent>
                </Card>

                {/* [283A-29] Nuevo gráfico: comparación con mes anterior */}
                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Comparación mensual</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <ChartContainer config={{total: {label: 'Reservas', color: 'var(--chart-1)'}}} className="h-[220px] w-full">
                            <BarChart data={[{periodo: 'Mes anterior', total: data.total_mes_anterior}, {periodo: 'Este mes', total: data.total_reservas}]} margin={{left: 0, right: 0}}>
                                <CartesianGrid vertical={false} />
                                <XAxis dataKey="periodo" tickLine={false} axisLine={false} fontSize={11} />
                                <YAxis allowDecimals={false} tickLine={false} axisLine={false} width={30} />
                                <ChartTooltip content={<ChartTooltipContent />} />
                                <Bar dataKey="total" fill="var(--color-total)" radius={4} />
                            </BarChart>
                        </ChartContainer>
                    </CardContent>
                </Card>
            </div>
        </>
    );
}

function PanelOcupacion({data}: {data: OcupacionReservas}) {
    return (
        <>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Media personas/reserva</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.media_personas}</span>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Media reservas/día</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.media_reservas_dia}</span>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Total reservas</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.total_reservas}</span>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Antelación media</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.antelacion_media_dias} días</span>
                    </CardContent>
                </Card>
            </div>

            {/* [283A-18] Hora + Turno en grid de 2, Procedencia abajo en bloque propio */}
            <div className="grid gap-4 lg:grid-cols-2">
                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Distribución por hora</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <ChartContainer config={CONFIG_SIMPLE} className="h-[220px] w-full">
                            <BarChart data={data.por_hora} margin={{left: 0, right: 0}}>
                                <CartesianGrid vertical={false} />
                                <XAxis dataKey="hora" tickLine={false} axisLine={false} fontSize={11} />
                                <YAxis allowDecimals={false} tickLine={false} axisLine={false} width={30} />
                                <ChartTooltip content={<ChartTooltipContent />} />
                                <Bar dataKey="total" fill="var(--color-total)" radius={4} />
                            </BarChart>
                        </ChartContainer>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Por turno</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <ChartContainer config={CONFIG_TURNO} className="h-[220px] w-full">
                            <BarChart data={data.por_turno} margin={{left: 0, right: 0}}>
                                <CartesianGrid vertical={false} />
                                <XAxis dataKey="turno" tickLine={false} axisLine={false} fontSize={11} />
                                <YAxis allowDecimals={false} tickLine={false} axisLine={false} width={30} />
                                <ChartTooltip content={<ChartTooltipContent />} />
                                <Bar dataKey="total" fill="var(--color-total)" radius={4} />
                                <Bar dataKey="personas" fill="var(--color-personas)" radius={4} />
                            </BarChart>
                        </ChartContainer>
                    </CardContent>
                </Card>
            </div>

            <Card>
                <CardHeader>
                    <CardTitle className="text-sm">Procedencia (canal)</CardTitle>
                </CardHeader>
                <CardContent>
                    <div className="rounded-md border">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>Canal</TableHead>
                                    <TableHead className="text-right">Reservas</TableHead>
                                    <TableHead className="text-right">%</TableHead>
                                    <TableHead></TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {data.por_procedencia.map((c: AgrupacionCanal) => (
                                    <TableRow key={c.canal}>
                                        <TableCell>{c.canal}</TableCell>
                                        <TableCell className="text-right">{c.total}</TableCell>
                                        <TableCell className="text-right">{c.porcentaje}%</TableCell>
                                        <TableCell>
                                            <div className="h-2 w-full max-w-[100px] rounded-full bg-muted">
                                                <div className="h-2 rounded-full bg-primary" style={{width: `${Math.min(c.porcentaje, 100)}%`}} />
                                            </div>
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    </div>
                </CardContent>
            </Card>
        </>
    );
}

function PanelAnalisis({data}: {data: AnalisisReservas}) {
    const formatMoneda = (v: string | null | undefined) => {
        if (!v) return '—';
        return new Intl.NumberFormat('es-ES', {style: 'currency', currency: 'EUR'}).format(parseFloat(v));
    };

    return (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            <Card>
                <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Reservas efectivas</CardTitle>
                </CardHeader>
                <CardContent>
                    <span className="text-2xl font-bold">{data.reservas_efectivas}</span>
                    <p className="text-xs text-muted-foreground">Sin cancelaciones ni no-shows</p>
                </CardContent>
            </Card>
            <Card>
                <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Total comensales</CardTitle>
                </CardHeader>
                <CardContent>
                    <span className="text-2xl font-bold">{data.total_comensales}</span>
                </CardContent>
            </Card>
            <Card>
                <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Comensales/reserva</CardTitle>
                </CardHeader>
                <CardContent>
                    <span className="text-2xl font-bold">{data.comensales_por_reserva}</span>
                </CardContent>
            </Card>
            <Card>
                <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Ticket medio/reserva</CardTitle>
                </CardHeader>
                <CardContent>
                    <span className="text-2xl font-bold">{formatMoneda(data.ticket_medio_reserva)}</span>
                    <p className="text-xs text-muted-foreground">Ventas ÷ reservas efectivas</p>
                </CardContent>
            </Card>
            <Card>
                <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Ticket medio/persona</CardTitle>
                </CardHeader>
                <CardContent>
                    <span className="text-2xl font-bold">{formatMoneda(data.ticket_medio_persona)}</span>
                    <p className="text-xs text-muted-foreground">Ventas ÷ total comensales</p>
                </CardContent>
            </Card>
        </div>
    );
}

export default DashboardReservas;
