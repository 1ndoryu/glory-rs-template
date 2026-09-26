use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    AuthResponse, ChangePasswordRequest, LoginRequest, QuickRegisterRequest, RegisterRequest,
    SetPasswordRequest, User, UserRole,
};
use crate::repositories::UserRepository;

/* [044A-38] Claims extendidos con role y effective_role.
 * El effective_role es el rol con el que el usuario opera: para admins
 * puede ser diferente de role si tienen active_role configurado.
 * [084A-1] impersonator: UUID del admin que inició impersonación.
 * Si Some, sub es el usuario impersonado y role/effective_role son los de ese usuario. */

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub role: UserRole,
    pub effective_role: UserRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impersonator: Option<Uuid>,
    pub exp: usize,
}

pub struct AuthService;

/* [015A-1] Helper compartido para hashear contraseñas con Argon2.
 * Reutilizado por register, quick_register y create_user admin. */
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Error al hashear contraseña: {e}")))
        .map(|h| h.to_string())
}

/* [205A-2] Helper compartido para login y cambio de contraseña.
 * Distingue entre mismatch normal y un hash almacenado corrupto. */
pub fn verify_password_hash(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|e| AppError::Internal(format!("Hash almacenado inválido: {e}")))?;

    match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(AppError::Internal(format!(
            "Error verificando contraseña: {e}"
        ))),
    }
}

/* [104A-3] Verifica si el email está en GLORY_ADMIN_EMAILS y promueve a admin.
 * Env var es comma-separated, case-insensitive. Si no existe, no promueve a nadie.
 * [20CA-1] pub porque payment.rs lo necesita para auto-promote en checkout post-pago. */
pub fn is_admin_email(email: &str) -> bool {
    std::env::var("GLORY_ADMIN_EMAILS")
        .unwrap_or_default()
        .split(',')
        .any(|e| e.trim().eq_ignore_ascii_case(email))
}

impl AuthService {
    /// Registra un nuevo usuario: valida unicidad, hashea contraseña, genera JWT.
    /// [154A-5] Si el email ya existe pero `password_set` = false (`quick_register`),
    /// actualiza la contraseña en lugar de retornar Conflict.
    pub async fn register(
        pool: &PgPool,
        req: RegisterRequest,
        jwt_secret: &str,
    ) -> Result<AuthResponse, AppError> {
        let existing = UserRepository::find_by_email(pool, &req.email).await?;

        let password_hash = hash_password(&req.password)?;

        let user = if let Some(existing_user) = existing {
            if existing_user.password_set {
                return Err(AppError::Conflict("Email ya registrado".into()));
            }
            /* Usuario de quick_register sin contraseña propia: actualizar hash */
            UserRepository::set_password(pool, existing_user.id, &password_hash).await?
        } else {
            UserRepository::create(pool, &req.email, &password_hash, true).await?
        };

        /* [104A-3] Auto-promote admin emails on registration */
        let user = if is_admin_email(&req.email) {
            UserRepository::update_role(pool, user.id, UserRole::Admin).await?
        } else {
            user
        };

        let effective = user.effective_role();
        let token = Self::generate_token(user.id, user.role, effective, None, jwt_secret)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
            email: user.email.clone(),
            role: user.role,
            effective_role: effective,
            impersonating: false,
            needs_password: false,
        })
    }

    /* [064A-3] Registro rapido solo con email (flujo de compra).
     * Genera password aleatorio; el usuario puede cambiarlo desde el panel.
     * [154A-5] Marca password_set = false para que el frontend muestre aviso.
     * [20CA-1] Ahora es idempotente para usuarios sin contraseña: si el email
     * ya existe con password_set=false, retorna el usuario existente en vez de 409.
     * Esto permite que el flujo de checkout llame quick_register antes Y después
     * del pago sin crear cuentas duplicadas. Solo retorna 409 si el usuario YA
     * tiene contraseña (debe hacer login). */
    pub async fn quick_register(
        pool: &PgPool,
        req: QuickRegisterRequest,
        jwt_secret: &str,
    ) -> Result<AuthResponse, AppError> {
        /* [20CA-1] Si el email ya existe: */
        if let Some(existing) = UserRepository::find_by_email(pool, &req.email).await? {
            if existing.password_set {
                /* Usuario con contraseña → debe hacer login */
                return Err(AppError::Conflict("Email ya registrado".into()));
            }
            /* [20CA-1] Usuario sin contraseña (creado por checkout o quick_register previo).
             * Retornar JWT sin crear duplicado. */
            let effective = existing.effective_role();
            let token =
                Self::generate_token(existing.id, existing.role, effective, None, jwt_secret)?;
            return Ok(AuthResponse {
                token,
                user_id: existing.id,
                email: existing.email.clone(),
                role: existing.role,
                effective_role: effective,
                impersonating: false,
                needs_password: true,
            });
        }

        let random_password: String = {
            use argon2::password_hash::rand_core::RngCore;
            let mut buf = [0u8; 32];
            OsRng.fill_bytes(&mut buf);
            hex::encode(buf)
        };

        let password_hash = hash_password(&random_password)?;

        let user = UserRepository::create(pool, &req.email, &password_hash, false).await?;

        /* [104A-3] Auto-promote admin emails on quick registration */
        let user = if is_admin_email(&req.email) {
            UserRepository::update_role(pool, user.id, UserRole::Admin).await?
        } else {
            user
        };

        let effective = user.effective_role();
        let token = Self::generate_token(user.id, user.role, effective, None, jwt_secret)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
            email: user.email.clone(),
            role: user.role,
            effective_role: effective,
            impersonating: false,
            needs_password: !user.password_set,
        })
    }

    /// Inicia sesión: verifica credenciales y genera JWT
    pub async fn login(
        pool: &PgPool,
        req: LoginRequest,
        jwt_secret: &str,
    ) -> Result<AuthResponse, AppError> {
        let user = UserRepository::find_by_email(pool, &req.email)
            .await?
            .ok_or(AppError::Unauthorized)?;

        if !verify_password_hash(&req.password, &user.password_hash)? {
            return Err(AppError::Unauthorized);
        }

        let effective = user.effective_role();
        let token = Self::generate_token(user.id, user.role, effective, None, jwt_secret)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
            email: user.email.clone(),
            role: user.role,
            effective_role: effective,
            impersonating: false,
            needs_password: !user.password_set,
        })
    }

    /* [154A-5] Permite a un usuario autenticado establecer su contraseña.
     * Solo para usuarios con password_set = false (quick_register). */
    pub async fn set_password(
        pool: &PgPool,
        user_id: Uuid,
        req: SetPasswordRequest,
    ) -> Result<(), AppError> {
        let user = UserRepository::find_by_id(pool, user_id)
            .await?
            .ok_or(AppError::NotFound("Usuario no encontrado".into()))?;

        if user.password_set {
            return Err(AppError::BadRequest(
                "Ya tienes una contraseña establecida. Usa cambiar contraseña.".into(),
            ));
        }

        let password_hash = hash_password(&req.password)?;

        UserRepository::set_password(pool, user_id, &password_hash).await?;
        Ok(())
    }

    /* [205A-2] Cambio de contraseña desde perfil.
     * El flujo de quick_register sigue usando set_password porque no conoce la contraseña aleatoria. */
    pub async fn change_password(
        pool: &PgPool,
        user_id: Uuid,
        req: ChangePasswordRequest,
    ) -> Result<User, AppError> {
        let user = UserRepository::find_by_id(pool, user_id)
            .await?
            .ok_or(AppError::NotFound("Usuario no encontrado".into()))?;

        if !user.password_set {
            return Err(AppError::BadRequest(
                "Tu cuenta todavía no tiene una contraseña propia. Usa primero Crear contraseña."
                    .into(),
            ));
        }

        if req.current_password == req.new_password {
            return Err(AppError::BadRequest(
                "La nueva contraseña debe ser distinta a la actual".into(),
            ));
        }

        if !verify_password_hash(&req.current_password, &user.password_hash)? {
            return Err(AppError::BadRequest(
                "La contraseña actual es incorrecta".into(),
            ));
        }

        let password_hash = hash_password(&req.new_password)?;

        UserRepository::set_password(pool, user_id, &password_hash)
            .await
            .map_err(|e| AppError::Internal(format!("Error actualizando contraseña: {e}")))
    }

    /// Genera un JWT con expiración de 10 años (efectivamente permanente para CMS).
    /// Si `impersonator` es Some, el token representa una sesión impersonada por un admin.
    pub fn generate_token(
        user_id: Uuid,
        role: UserRole,
        effective_role: UserRole,
        impersonator: Option<Uuid>,
        secret: &str,
    ) -> Result<String, AppError> {
        let timestamp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::days(3650))
            .ok_or_else(|| AppError::Internal("Error calculando expiración del token".into()))?
            .timestamp();
        let exp = usize::try_from(timestamp)
            .map_err(|_| AppError::Internal("Timestamp fuera de rango".into()))?;

        let claims = Claims {
            sub: user_id,
            role,
            effective_role,
            impersonator,
            exp,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Error generando token: {e}")))
    }

    /// Verifica un JWT y retorna los claims
    pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized)
    }
}
