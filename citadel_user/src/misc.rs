use chrono::Utc;
use std::path::{Path, PathBuf};

/// Default Error type for this crate
#[derive(Debug)]
pub enum AccountError {
    /// Input/Output error. Used for possibly failed Serialization/Deserialization of underlying datatypes
    IoError(String),
    /// The client already exists
    ClientExists(u64),
    /// The client does not exist
    ClientNonExists(u64),
    /// The server exists
    ServerExists(u64),
    /// The server does not exist
    ServerNonExists(u64),
    /// Invalid username
    InvalidUsername,
    /// Invalid password
    InvalidPassword,
    /// The server is not engaged
    Disengaged(u64),
    /// Generic error
    Generic(String),
}

impl AccountError {
    pub(crate) fn msg<T: Into<String>>(msg: T) -> Self {
        Self::Generic(msg.into())
    }

    /// Consumes self and returns the underlying error message
    pub fn into_string(self) -> String {
        match self {
            AccountError::IoError(e) => e,
            AccountError::Generic(e) => e,
            AccountError::InvalidUsername => "Invalid username".to_string(),
            AccountError::InvalidPassword => "Invalid password".to_string(),
            AccountError::ClientExists(cid) => format!("Client {cid} already exists"),
            AccountError::ClientNonExists(cid) => format!("Client {cid} does not exist"),
            AccountError::ServerExists(cid) => format!("Server {cid} already exists"),
            AccountError::ServerNonExists(cid) => format!("Server {cid} does not exist"),
            AccountError::Disengaged(cid) => format!("Server {cid} is not engaged"),
        }
    }
}

impl<T: ToString> From<T> for AccountError {
    fn from(err: T) -> Self {
        AccountError::Generic(err.to_string())
    }
}

///
pub const MIN_PASSWORD_LENGTH: usize = 7;
///
pub const MAX_PASSWORD_LENGTH: usize = 17;

///
pub const MIN_USERNAME_LENGTH: usize = 3;
///
pub const MAX_USERNAME_LENGTH: usize = 37;

///
pub const MIN_NAME_LENGTH: usize = 2;
///
pub const MAX_NAME_LENGTH: usize = 77;

/// Used to determine if the desired credentials have a valid format, length, etc. This alone DOES NOT imply whether or not the
/// credentials are available
pub fn check_credential_formatting<T: AsRef<str>, R: AsRef<str>, V: AsRef<str>>(
    username: T,
    password: Option<R>,
    full_name: V,
) -> Result<(), AccountError> {
    let username = username.as_ref();
    let full_name = full_name.as_ref();

    if username.len() < MIN_USERNAME_LENGTH || username.len() > MAX_USERNAME_LENGTH {
        return Err(AccountError::Generic(format!(
            "Username must be between {MIN_USERNAME_LENGTH} and {MAX_USERNAME_LENGTH} characters",
        )));
    }

    if username.contains(' ') {
        return Err(AccountError::Generic(
            "Username cannot contain spaces. Use a period instead".to_string(),
        ));
    }

    if let Some(password) = password.as_ref() {
        let password = password.as_ref();
        if password.len() < MIN_PASSWORD_LENGTH || password.len() > MAX_PASSWORD_LENGTH {
            return Err(AccountError::Generic(format!(
                "Password must be between {MIN_PASSWORD_LENGTH} and {MAX_PASSWORD_LENGTH} characters",
            )));
        }

        if password.contains(' ') {
            return Err(AccountError::Generic(
                "Password cannot contain spaces".to_string(),
            ));
        }
    }

    if full_name.len() < MIN_NAME_LENGTH || full_name.len() > MAX_NAME_LENGTH {
        return Err(AccountError::Generic(format!(
            "Full name must be between {MIN_NAME_LENGTH} and {MAX_NAME_LENGTH} characters",
        )));
    }

    Ok(())
}

/// For passing metadata from a cnac
#[derive(Debug)]
pub struct CNACMetadata {
    /// Client ID
    pub cid: u64,
    /// Username
    pub username: String,
    /// Full name
    pub full_name: String,
    /// Whether CNAC is personal
    pub is_personal: bool,
    /// Date created
    pub creation_date: String,
}

impl PartialEq for CNACMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.cid == other.cid
            && self.username == other.username
            && self.full_name == other.full_name
            && self.is_personal == other.is_personal
    }
}

#[allow(missing_docs)]
#[cfg(all(feature = "sql", not(coverage)))]
pub mod base64_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AsRef<[u8]>,
        S: Serializer,
    {
        serializer.collect_str(&base64::encode(value))
    }

    pub fn deserialize<'de, D>(value: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        base64::decode(
            String::deserialize(value).map_err(|_| serde::de::Error::custom("Deser err"))?,
        )
        .map_err(|_| serde::de::Error::custom("Deser err"))
    }
}

/// Returns the present timestamp in ISO 8601 format
pub fn get_present_formatted_timestamp() -> String {
    Utc::now().to_rfc3339()
}

pub fn validate_virtual_path<R: AsRef<Path>>(virtual_path: R) -> Result<(), AccountError> {
    let virtual_path = virtual_path.as_ref();
    #[cfg(not(target_os = "windows"))]
    const REQUIRED_BEGINNING: &str = "/";
    #[cfg(target_os = "windows")]
    const REQUIRED_BEGINNING: &str = "\\";

    if !virtual_path.starts_with(REQUIRED_BEGINNING) {
        return Err(AccountError::IoError(format!(
            "Path {virtual_path:?} is not a valid remote encrypted virtual directory"
        )));
    }

    let buf = format!("{}", virtual_path.display());

    // we cannot use path.is_dir() since that checks for file existence, which we don't want
    if buf.ends_with(REQUIRED_BEGINNING) {
        return Err(AccountError::IoError(format!(
            "Path {virtual_path:?} is a directory, not a file"
        )));
    }

    Ok(())
}

// The goal of this function is to ensure that the provided virtual path is appropriate for
// the local operating system
pub fn prepare_virtual_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = format!("{}", path.as_ref().display());
    format_path(path).into()
}

#[cfg(not(target_os = "windows"))]
/// #
pub fn format_path(input: String) -> String {
    input.replace('\\', "/")
}

#[cfg(target_os = "windows")]
/// #
pub fn format_path(input: String) -> String {
    input.replace("/", "\\")
}

pub const VIRTUAL_FILE_METADATA_EXT: &str = ".vxe";

#[cfg(test)]
mod tests {
    use crate::misc::{prepare_virtual_path, validate_virtual_path};
    use rstest::rstest;
    use std::path::PathBuf;

    #[rstest]
    #[case("/hello/world/tmp.txt")]
    #[case("/hello/world/tmp")]
    #[case("/tmp.txt")]
    #[case("\\hello\\world\\tmp.txt")]
    #[case("\\hello\\world\\tmp")]
    #[case("\\tmp.txt")]
    fn test_virtual_dir_formatting_okay(#[case] good_path: &str) {
        let virtual_dir = PathBuf::from(good_path);
        let formatted = prepare_virtual_path(virtual_dir);
        validate_virtual_path(formatted).unwrap();
    }

    #[rstest]
    #[case("/hello/")]
    #[case("/")]
    #[case("tmp.txt")]
    #[case("\\hello\\")]
    #[case("\\")]
    fn test_virtual_dir_formatting_bad(#[case] bad_path: &str) {
        let virtual_dir = PathBuf::from(bad_path);
        let formatted = prepare_virtual_path(virtual_dir);
        assert!(validate_virtual_path(formatted).is_err());
    }
}
