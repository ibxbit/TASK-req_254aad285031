use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::State;
use shared::Role;
use sqlx::MySqlPool;
use uuid::Uuid;

use super::session;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
}

impl AuthUser {
    pub fn require_role(&self, role: Role) -> Result<(), Status> {
        if self.role == role {
            Ok(())
        } else {
            Err(Status::Forbidden)
        }
    }

    pub fn require_any(&self, roles: &[Role]) -> Result<(), Status> {
        if roles.contains(&self.role) {
            Ok(())
        } else {
            Err(Status::Forbidden)
        }
    }
}

pub struct BearerToken(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for BearerToken {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req
            .headers()
            .get_one("Authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
        {
            Some(t) => Outcome::Success(BearerToken(t.to_string())),
            None => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let Some(header) = req.headers().get_one("Authorization") else {
            return Outcome::Error((Status::Unauthorized, ()));
        };
        let Some(token) = header.strip_prefix("Bearer ") else {
            return Outcome::Error((Status::Unauthorized, ()));
        };

        let pool = match req.guard::<&State<MySqlPool>>().await {
            Outcome::Success(p) => p,
            _ => return Outcome::Error((Status::InternalServerError, ())),
        };

        let uid = match session::lookup(pool.inner(), token).await {
            Ok(Some(uid)) => uid,
            _ => return Outcome::Error((Status::Unauthorized, ())),
        };

        // Read the `is_active` flag alongside identity so a deactivated
        // user cannot continue to use an outstanding bearer token.
        // Admin deactivation also actively DELETEs the session rows (see
        // admin::users::update_status) — this check is the second line of
        // defence in case a session sneaks through e.g. a race.
        let row: Option<(String, String, i8)> =
            match sqlx::query_as("SELECT username, role, is_active FROM users WHERE id = ?")
                .bind(&uid.as_bytes()[..])
                .fetch_optional(pool.inner())
                .await
            {
                Ok(r) => r,
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            };

        let Some((username, role_str, is_active)) = row else {
            return Outcome::Error((Status::Unauthorized, ()));
        };

        if is_active == 0 {
            // Best-effort cleanup: revoke any sessions that might still
            // reference this user. Failures are ignored because the 401
            // response is correct regardless.
            let _ = super::session::delete_all_for_user(pool.inner(), uid).await;
            return Outcome::Error((Status::Unauthorized, ()));
        }

        let Some(role) = Role::from_str(&role_str) else {
            return Outcome::Error((Status::InternalServerError, ()));
        };

        Outcome::Success(AuthUser {
            id: uid,
            username,
            role,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::Role;
    use uuid::Uuid;

    fn make_user(role: Role) -> AuthUser {
        AuthUser {
            id: Uuid::new_v4(),
            username: "test".into(),
            role,
        }
    }

    #[test]
    fn require_role_succeeds_for_matching_role() {
        let u = make_user(Role::Administrator);
        assert!(u.require_role(Role::Administrator).is_ok());
    }

    #[test]
    fn require_role_fails_with_forbidden_for_wrong_role() {
        let u = make_user(Role::Administrator);
        let err = u.require_role(Role::Requester).unwrap_err();
        assert_eq!(err, Status::Forbidden);
    }

    #[test]
    fn require_any_succeeds_when_role_in_slice() {
        let u = make_user(Role::Requester);
        assert!(u
            .require_any(&[Role::Requester, Role::ServiceManager])
            .is_ok());
    }

    #[test]
    fn require_any_succeeds_for_single_element_slice_matching() {
        let u = make_user(Role::Moderator);
        assert!(u.require_any(&[Role::Moderator]).is_ok());
    }

    #[test]
    fn require_any_fails_when_role_absent_from_slice() {
        let u = make_user(Role::Intern);
        let err = u
            .require_any(&[Role::Requester, Role::Administrator])
            .unwrap_err();
        assert_eq!(err, Status::Forbidden);
    }

    #[test]
    fn require_any_fails_for_empty_slice() {
        let u = make_user(Role::Administrator);
        assert_eq!(u.require_any(&[]).unwrap_err(), Status::Forbidden);
    }

    #[test]
    fn require_role_is_exact_match_not_partial() {
        let u = make_user(Role::ServiceManager);
        assert!(u.require_role(Role::ServiceManager).is_ok());
        assert!(u.require_role(Role::Administrator).is_err());
        assert!(u.require_role(Role::Requester).is_err());
    }
}
