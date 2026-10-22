//! The command error contract: a machine-readable kind plus a Thai message
//! shown verbatim by the UI (AGENTS.md section 14).
//!
//! This is the **only** place crate errors gain user-facing Thai text;
//! everything else in the workspace is English (AGENTS-RUST.md section 14
//! override).

use er_refill_core::RepositoryError;
use er_refill_hosxp_connector::Error as ConnectorError;
use er_refill_hosxp_connector::config::ConfigError;
use serde::Serialize;

use crate::settings::SettingsError;

/// Failure class of a command - the frontend switches presentation on this,
/// never on message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandErrorKind {
    /// No connection settings stored yet.
    NotConfigured,
    /// The database is unreachable.
    Connection,
    /// The read-only guard rejected a statement.
    Guard,
    /// A query failed (or an unexpected internal failure occurred).
    Query,
    /// The operator's input was rejected before any I/O.
    Validation,
    /// The operator is not signed in, or the credentials were rejected.
    Auth,
    /// Reading/writing local files (settings, encrypted config) failed.
    Storage,
}

/// A command failure: kind + Thai message. Serializable across the IPC.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// Machine-readable failure class.
    pub kind: CommandErrorKind,
    /// Thai message to display verbatim.
    pub message: String,
}

impl CommandError {
    /// Builds an error from its parts.
    pub fn new(kind: CommandErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP".
    pub fn not_configured() -> Self {
        Self::new(CommandErrorKind::NotConfigured, "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP")
    }

    /// "ข้อมูลไม่ถูกต้อง: ...".
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(
            CommandErrorKind::Validation,
            format!("ข้อมูลไม่ถูกต้อง: {}", message.into()),
        )
    }

    /// "กรุณาเข้าสู่ระบบก่อนใช้งาน".
    pub fn unauthorized() -> Self {
        Self::new(CommandErrorKind::Auth, "กรุณาเข้าสู่ระบบก่อนใช้งาน")
    }

    /// An authentication failure with its Thai message.
    pub fn auth(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::Auth, message)
    }

    /// "บันทึกหรืออ่านการตั้งค่าไม่สำเร็จ".
    pub fn storage(message: impl Into<String>) -> Self {
        dev_log(&format!("storage failure: {}", message.into()));
        Self::new(CommandErrorKind::Storage, "บันทึกหรืออ่านการตั้งค่าไม่สำเร็จ")
    }
}

impl From<RepositoryError> for CommandError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::NotConfigured => Self::not_configured(),
            RepositoryError::Connection => Self::new(
                CommandErrorKind::Connection,
                "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ กรุณาตรวจสอบการตั้งค่าการเชื่อมต่อและเครือข่าย",
            ),
            RepositoryError::Query(detail) => {
                dev_log(&format!("query failed: {detail}"));
                Self::new(
                    CommandErrorKind::Query,
                    "ดึงข้อมูลจาก HOSxP ไม่สำเร็จ กรุณาลองใหม่อีกครั้ง",
                )
            }
            RepositoryError::Guard => {
                dev_log("read-only guard rejected a statement");
                Self::new(
                    CommandErrorKind::Guard,
                    "คำสั่งถูกปฏิเสธโดยระบบป้องกันการเขียนข้อมูล",
                )
            }
        }
    }
}

impl From<ConnectorError> for CommandError {
    fn from(err: ConnectorError) -> Self {
        match err {
            ConnectorError::Config(inner) => inner.into(),
            ConnectorError::Connect(inner) => {
                dev_log(&format!("connect failed: {inner}"));
                Self::new(
                    CommandErrorKind::Connection,
                    "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ กรุณาตรวจสอบการตั้งค่าการเชื่อมต่อและเครือข่าย",
                )
            }
            ConnectorError::Database(inner) => {
                dev_log(&format!("database error: {inner}"));
                Self::new(
                    CommandErrorKind::Query,
                    "ดึงข้อมูลจาก HOSxP ไม่สำเร็จ กรุณาลองใหม่อีกครั้ง",
                )
            }
            ConnectorError::Guard => {
                dev_log("read-only guard rejected a statement");
                Self::new(
                    CommandErrorKind::Guard,
                    "คำสั่งถูกปฏิเสธโดยระบบป้องกันการเขียนข้อมูล",
                )
            }
        }
    }
}

impl From<ConfigError> for CommandError {
    fn from(err: ConfigError) -> Self {
        Self::storage(err.to_string())
    }
}

impl From<SettingsError> for CommandError {
    fn from(err: SettingsError) -> Self {
        Self::storage(err.to_string())
    }
}

/// Dev-only logging of error causes. Never logs secrets, credentials, or
/// parameter values (AGENTS.md section 2) - callers pass driver/summary text only.
pub(crate) fn dev_log(message: &str) {
    if cfg!(debug_assertions) {
        eprintln!("[er-refill] {message}");
    } else {
        let _ = message;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_connection_maps_to_thai_connection_message() {
        let mapped = CommandError::from(RepositoryError::Connection);
        assert_eq!(mapped.kind, CommandErrorKind::Connection);
        assert!(mapped.message.contains("เชื่อมต่อ"));
    }

    #[test]
    fn repository_query_maps_to_thai_query_message() {
        let mapped = CommandError::from(RepositoryError::Query("boom".into()));
        assert_eq!(mapped.kind, CommandErrorKind::Query);
        assert!(mapped.message.contains("ดึงข้อมูล"));
    }

    #[test]
    fn not_configured_maps_without_detail_leak() {
        let mapped = CommandError::from(RepositoryError::NotConfigured);
        assert_eq!(mapped.kind, CommandErrorKind::NotConfigured);
        assert!(mapped.message.contains("ยังไม่ได้ตั้งค่า"));
    }

    #[test]
    fn validation_prefixes_input_errors() {
        let mapped = CommandError::validation("ช่วงเวลาไม่ถูกต้อง");
        assert_eq!(mapped.kind, CommandErrorKind::Validation);
        assert!(mapped.message.starts_with("ข้อมูลไม่ถูกต้อง:"));
    }

    #[test]
    fn unauthenticated_has_its_own_kind() {
        let mapped = CommandError::unauthorized();
        assert_eq!(mapped.kind, CommandErrorKind::Auth);
        assert!(mapped.message.contains("เข้าสู่ระบบ"));
    }
}
