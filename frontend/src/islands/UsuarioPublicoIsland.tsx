/* [084A-7] Island de perfil público de usuario estilo Fiverr.
 * Muestra info del usuario, ratings, y reviews recibidas/dadas.
 * Accesible sin autenticación en /usuario/:username.
 * [259A-5 5b-6] Tabs y paginacion migrados a <Button>; disable html retirado por obsoleto. */

import {useParams} from 'react-router-dom';
import {Star} from 'lucide-react';
import {LayoutPagina} from '../components/layout/LayoutPagina';
import {SEOHead} from '../components/seo/SEOHead';
import {Button} from '../components/ui/Button';
import OptimizedImage from '../components/ui/OptimizedImage';
import {usePublicProfile} from '../hooks/usePublicProfile';
import type {ReviewTab} from '../hooks/usePublicProfile';
import type {PublicReviewItem, RatingDistribution} from '../api/publicUsers';
import './UsuarioPublicoIsland.css';
import './PerfilReviews.css';

function Estrellas({rating}: {rating: number}) {
    return (
        <span className="perfilEstrellas" aria-label={`${rating} de 5 estrellas`}>
            {[1, 2, 3, 4, 5].map(i => (
                <span key={`estrella-${i}`} className={i <= rating ? 'estrellaLlena' : 'estrellaVacia'}><Star size={14} /></span>
            ))}
        </span>
    );
}

function BarraRating({estrellas, cantidad, total}: {estrellas: number; cantidad: number; total: number}) {
    const porcentaje = total > 0 ? (cantidad / total) * 100 : 0;
    return (
        <div className="barraRatingFila">
            <span className="barraRatingLabel">{estrellas}<Star size={12} /></span>
            <div className="barraRatingTrack">
                {/* sentinel-disable-next-line inline-style-prohibido: dynamic width percentage */}
                <div className="barraRatingFill" style={{width: `${porcentaje}%`}} />
            </div>
            <span className="barraRatingCantidad">{cantidad}</span>
        </div>
    );
}

function ResumenRatings({ratings}: {ratings: RatingDistribution}) {
    return (
        <div className="perfilResumenRatings">
            <div className="perfilRatingPromedio">
                <span className="perfilRatingNumero">{ratings.average.toFixed(1)}</span>
                <Estrellas rating={Math.round(ratings.average)} />
                <span className="perfilRatingTotal">{ratings.total} {ratings.total === 1 ? 'reseña' : 'reseñas'}</span>
            </div>
            <div className="perfilRatingBarras">
                <BarraRating estrellas={5} cantidad={ratings.stars_5} total={ratings.total} />
                <BarraRating estrellas={4} cantidad={ratings.stars_4} total={ratings.total} />
                <BarraRating estrellas={3} cantidad={ratings.stars_3} total={ratings.total} />
                <BarraRating estrellas={2} cantidad={ratings.stars_2} total={ratings.total} />
                <BarraRating estrellas={1} cantidad={ratings.stars_1} total={ratings.total} />
            </div>
        </div>
    );
}

function CardReview({review}: {review: PublicReviewItem}) {
    const fecha = new Date(review.created_at);
    const fechaFormateada = fecha.toLocaleDateString('es-ES', {year: 'numeric', month: 'short', day: 'numeric'});
    return (
        <div className="perfilCardReview">
            <div className="perfilCardReviewHeader">
                <div className="perfilCardReviewAutor">
                    {review.author_avatar ? (
                        <OptimizedImage src={review.author_avatar} alt="" className="perfilCardReviewAvatar" loading="lazy" />
                    ) : (
                        <div className="perfilCardReviewAvatarPlaceholder">
                            {(review.author_name || '?')[0].toUpperCase()}
                        </div>
                    )}
                    <div>
                        <span className="perfilCardReviewNombre">{review.author_name || 'Usuario'}</span>
                        {review.service_title && (
                            <span className="perfilCardReviewServicio">{review.service_title}</span>
                        )}
                    </div>
                </div>
                <div className="perfilCardReviewMeta">
                    <Estrellas rating={review.rating} />
                    <span className="perfilCardReviewFecha">{fechaFormateada}</span>
                </div>
            </div>
            {review.comment && <p className="perfilCardReviewTexto">{review.comment}</p>}
            {review.employee_response && (
                <div className="perfilCardReviewRespuesta">
                    <span className="perfilCardReviewRespuestaLabel">Respuesta:</span>
                    <p>{review.employee_response}</p>
                </div>
            )}
        </div>
    );
}

function TabsReviews({
    tab, onCambiarTab, totalRecibidas, totalDadas,
}: {
    tab: ReviewTab;
    onCambiarTab: (t: ReviewTab) => void;
    totalRecibidas: number;
    totalDadas: number;
}) {
    return (
        <div className="perfilReviewTabs">
            <Button
                type="button"
                variante="texto"
                tamano="pequeno"
                className={`perfilReviewTab ${tab === 'received' ? 'perfilReviewTabActivo' : ''}`}
                onClick={() => onCambiarTab('received')}
            >
                Recibidas ({totalRecibidas})
            </Button>
            <Button
                type="button"
                variante="texto"
                tamano="pequeno"
                className={`perfilReviewTab ${tab === 'given' ? 'perfilReviewTabActivo' : ''}`}
                onClick={() => onCambiarTab('given')}
            >
                Dadas ({totalDadas})
            </Button>
        </div>
    );
}

function Paginacion({page, total, perPage, onCambiar}: {
    page: number; total: number; perPage: number; onCambiar: (p: number) => void;
}) {
    const totalPages = Math.ceil(total / perPage);
    if (totalPages <= 1) return null;
    return (
        <div className="perfilPaginacion">
            <Button type="button" variante="outline" tamano="pequeno" className="perfilPaginacionControl" disabled={page <= 1} onClick={() => onCambiar(page - 1)}>← Anterior</Button>
            <span>{page} / {totalPages}</span>
            <Button type="button" variante="outline" tamano="pequeno" className="perfilPaginacionControl" disabled={page >= totalPages} onClick={() => onCambiar(page + 1)}>Siguiente →</Button>
        </div>
    );
}

export function UsuarioPublicoIsland() {
    const {username} = useParams<{username: string}>();
    const {
        profile, isLoadingProfile, profileError,
        ratings, receivedReviews, givenReviews,
        isLoadingReviews, reviewTab, setReviewTab,
        receivedPage, setReceivedPage, givenPage, setGivenPage,
    } = usePublicProfile(username);

    if (isLoadingProfile) {
        return (
            <LayoutPagina className="perfilPublicoMain" id="paginaPerfilPublico">
                <div className="perfilPublicoCargando">Cargando perfil...</div>
            </LayoutPagina>
        );
    }

    if (profileError || !profile) {
        return (
            <LayoutPagina className="perfilPublicoMain" id="paginaPerfilPublico">
                <div className="perfilPublicoError">Usuario no encontrado</div>
            </LayoutPagina>
        );
    }

    const nombre = profile.display_name || profile.username;
    const miembroDesde = new Date(profile.member_since).toLocaleDateString('es-ES', {
        year: 'numeric', month: 'long',
    });
    const currentReviews = reviewTab === 'received' ? receivedReviews : givenReviews;

    return (
        <LayoutPagina className="perfilPublicoMain" id="paginaPerfilPublico">
            <SEOHead
                title={`${nombre} — Perfil`}
                description={profile.bio || `Perfil público de ${nombre}`}
                path={`/usuario/${profile.username}`}
            />

            <section className="perfilPublicoHero">
                <div className="perfilPublicoContenedor">
                    <div className="perfilPublicoHeader">
                        <div className="perfilPublicoAvatarZona">
                            {profile.avatar_url ? (
                                <OptimizedImage src={profile.avatar_url} alt={nombre} className="perfilPublicoAvatar" loading="lazy" />
                            ) : (
                                <div className="perfilPublicoAvatarPlaceholder">
                                    {nombre[0].toUpperCase()}
                                </div>
                            )}
                        </div>
                        <div className="perfilPublicoInfo">
                            <h1 className="perfilPublicoNombre">{nombre}</h1>
                            {profile.specialties && profile.specialties.length > 0 && (
                                <div className="perfilPublicoEspecialidades">
                                    {profile.specialties.map((s) => (
                                        <span key={s} className="perfilPublicoTag">{s}</span>
                                    ))}
                                </div>
                            )}
                            <div className="perfilPublicoStats">
                                {profile.average_rating != null && profile.average_rating > 0 && (
                                    <span className="perfilPublicoStat">
                                        <Estrellas rating={Math.round(profile.average_rating)} />
                                        <strong>{profile.average_rating.toFixed(1)}</strong>
                                    </span>
                                )}
                                {profile.total_completed_orders != null && profile.total_completed_orders > 0 && (
                                    <span className="perfilPublicoStat">
                                        {profile.total_completed_orders} {profile.total_completed_orders === 1 ? 'orden completada' : 'órdenes completadas'}
                                    </span>
                                )}
                                <span className="perfilPublicoStat">Miembro desde {miembroDesde}</span>
                            </div>
                            {profile.bio && <p className="perfilPublicoBio">{profile.bio}</p>}
                            {(profile.linkedin || profile.twitter || profile.website) && (
                                <div className="perfilPublicoSocial">
                                    {profile.linkedin && (
                                        <a href={profile.linkedin} target="_blank" rel="noopener noreferrer" className="perfilPublicoSocialLink">LinkedIn</a>
                                    )}
                                    {profile.twitter && (
                                        <a href={profile.twitter} target="_blank" rel="noopener noreferrer" className="perfilPublicoSocialLink">Twitter</a>
                                    )}
                                    {profile.website && (
                                        <a href={profile.website} target="_blank" rel="noopener noreferrer" className="perfilPublicoSocialLink">Web</a>
                                    )}
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            </section>

            <section className="perfilPublicoReviewsSeccion">
                <div className="perfilPublicoContenedor">
                    {ratings && ratings.total > 0 && <ResumenRatings ratings={ratings} />}

                    <TabsReviews
                        tab={reviewTab}
                        onCambiarTab={setReviewTab}
                        totalRecibidas={receivedReviews?.total ?? 0}
                        totalDadas={givenReviews?.total ?? 0}
                    />

                    {isLoadingReviews ? (
                        <div className="perfilPublicoCargando">Cargando reseñas...</div>
                    ) : currentReviews && currentReviews.reviews.length > 0 ? (
                        <>
                            <div className="perfilReviewsLista">
                                {currentReviews.reviews.map(r => (
                                    <CardReview key={r.id} review={r} />
                                ))}
                            </div>
                            <Paginacion
                                page={reviewTab === 'received' ? receivedPage : givenPage}
                                total={currentReviews.total}
                                perPage={currentReviews.per_page}
                                onCambiar={reviewTab === 'received' ? setReceivedPage : setGivenPage}
                            />
                        </>
                    ) : (
                        <p className="perfilSinReviews">Sin reseñas por el momento.</p>
                    )}
                </div>
            </section>
        </LayoutPagina>
    );
}
