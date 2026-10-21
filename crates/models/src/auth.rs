//! Authentication types shared by the connector, the Tauri boundary, and the
//! login screen. The password hash is **not** serialized to the frontend.

use std::fmt;

use serde::{Deserialize, Serialize};

/// One `officer` row as read from HOSxP. Internal to the backend crates; the
/// hash never crosses the IPC boundary.
#[derive(Clone, PartialEq, Eq)]
pub struct OfficerAccount {
    /// `officer.officer_id`.
    pub officer_id: i64,
    /// `officer.officer_login_name` - the username.
    pub login_name: String,
    /// `officer.officer_login_password_md5` - 32 hex characters, lower/upper.
    pub password_md5: String,
    /// `officer.officer_active`; only `"Y"` is accepted.
    pub active: String,
    /// `officer.officer_authentication_type_id`; `NULL` was treated as 1
    /// (local database) in the source query.
    pub auth_type_id: i64,
    /// `officer.officer_doctor_code`, when present.
    pub doctor_code: Option<String>,
    /// Display name assembled from the officer name fields.
    pub display_name: String,
}

impl fmt::Debug for OfficerAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OfficerAccount")
            .field("officer_id", &self.officer_id)
            .field("login_name", &self.login_name)
            .field("password_md5", &"<redacted>")
            .field("active", &self.active)
            .field("auth_type_id", &self.auth_type_id)
            .field("doctor_code", &self.doctor_code)
            .field("display_name", &self.display_name)
            .finish()
    }
}

/// The sanitized user returned to the frontend after a successful login.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionUser {
    /// `officer.officer_id`.
    pub officer_id: i64,
    /// HOSxP login name.
    pub login_name: String,
    /// Display name for the top bar; falls back to the login name.
    pub display_name: String,
    /// `officer.officer_doctor_code`, when present.
    pub doctor_code: Option<String>,
    /// App role. HOSxP group permissions are empty, so the first version
    /// assigns every signed-in officer the same role.
    pub role: String,
}

/// Non-secret connection fields shown on the login screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSummary {
    /// HOSxP server host.
    pub host: String,
    /// HOSxP server port.
    pub port: u16,
    /// HOSxP database name.
    pub database: String,
    /// Read-only MySQL account name.
    pub user: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn officer_debug_redacts_the_password_hash() {
        let account = OfficerAccount {
            officer_id: 1,
            login_name: "pharm".into(),
            password_md5: "5f4dcc3b5aa765d61d8327deb882cf99".into(),
            active: "Y".into(),
            auth_type_id: 1,
            doctor_code: None,
            display_name: "Pharmacist".into(),
        };
        let debug = format!("{account:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("5f4dcc3b5aa765d61d8327deb882cf99"));
    }
}
