mod cliente;
mod inmueble;
mod note;
mod solicitud;
mod suscriptor;
mod user;
mod visita;

pub use cliente::{Cliente, ClienteRow, CreateClienteRequest};
pub use inmueble::{
    validar_extras, ActualizacionInmueble, AddFotoRequest, CopyInmueble, CreateInmuebleRequest,
    CreateUserRequest, EstadoRequest, FichaAskRequest, FichaAskResponse, FiltrosPublicos, Foto,
    FotoPublica, Inmueble, InmuebleRow, PaginatedInmuebles, PublicacionRequest, RecetaPublicidad,
    UpdateInmuebleRequest, ESTADOS, EXTENSIONES_FOTO, FORMATOS_RECETA, MAX_FOTO_BYTES, OPERACIONES,
    ORIGENES_FOTO, TIPOS,
};

pub use note::{CreateNoteRequest, Note, PaginatedNotes, PaginationParams, UpdateNoteRequest};
pub use solicitud::{
    ConteoEstado, CreateSolicitudRequest, FiltrosSolicitudes, FotoSolicitud, FotoSolicitudSubida,
    PaginatedSolicitudes, Solicitud, SolicitudRow, SolicitudesAdminParams, StatsSolicitudes,
    UpdateEstadoSolicitud, ESTADOS_SOLICITUD, MAX_FOTOS_SOLICITUD, ORIGENES_CONTACTO,
};
pub use suscriptor::{CreateSuscriptorRequest, Suscriptor, SuscriptorRow};
pub use user::{AuthResponse, LoginRequest, RegisterRequest, User, UserResponse};
pub use visita::{
    PaginatedVisitas, UpdateEstadoVisita, Visita, VisitaAdmin, VisitaRow, VisitasAdminParams,
    ESTADOS_VISITA,
};
