mod cliente;
mod inmueble;
mod note;
mod solicitud;
mod suscriptor;
mod user;

pub use cliente::ClienteRepository;
pub use inmueble::{InmuebleRepository, NuevoInmueble};
pub use note::NoteRepository;
pub use solicitud::{NuevaSolicitud, SolicitudRepository};
pub use suscriptor::{NuevoSuscriptor, SuscriptorRepository};
pub use user::UserRepository;
