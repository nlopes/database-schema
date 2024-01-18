// All of the code in this module was taken from sqlx's mysql implementation.
// See https://github.com/launchbadge/sqlx/tree/main/sqlx-mysql/src/options.
//
// I did this so that I didn't need to bring `sqlx` as a dependency when using `diesel` -
// this is because `diesel` doesn't expose the MysqlConnectOptions struct.
//
// If they ever do, we can remove this module and use their implementation instead.

use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Options for controlling the desired security state of the connection to the MySQL server.
///
/// It is used by the [`ssl_mode`](super::MySqlConnectOptions::ssl_mode) method.
#[derive(Default, Debug, Clone, Copy)]
pub enum MySqlSslMode {
    /// Establish an unencrypted connection.
    Disabled,

    /// Establish an encrypted connection if the server supports encrypted connections, falling
    /// back to an unencrypted connection if an encrypted connection cannot be established.
    ///
    /// This is the default if `ssl_mode` is not specified.
    #[default]
    Preferred,

    /// Establish an encrypted connection if the server supports encrypted connections.
    /// The connection attempt fails if an encrypted connection cannot be established.
    Required,

    /// Like `Required`, but additionally verify the server Certificate Authority (CA)
    /// certificate against the configured CA certificates. The connection attempt fails
    /// if no valid matching CA certificates are found.
    VerifyCa,

    /// Like `VerifyCa`, but additionally perform host name identity verification by
    /// checking the host name the client uses for connecting to the server against the
    /// identity in the certificate that the server sends to the client.
    VerifyIdentity,
}

impl std::fmt::Display for MySqlSslMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MySqlSslMode::Disabled => write!(f, "disabled"),
            MySqlSslMode::Preferred => write!(f, "preferred"),
            MySqlSslMode::Required => write!(f, "required"),
            MySqlSslMode::VerifyCa => write!(f, "verify_ca"),
            MySqlSslMode::VerifyIdentity => write!(f, "verify_identity"),
        }
    }
}

impl FromStr for MySqlSslMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        Ok(match &*s.to_ascii_lowercase() {
            "disabled" => MySqlSslMode::Disabled,
            "preferred" => MySqlSslMode::Preferred,
            "required" => MySqlSslMode::Required,
            "verify_ca" => MySqlSslMode::VerifyCa,
            "verify_identity" => MySqlSslMode::VerifyIdentity,

            _ => {
                return Err(format!("unknown value {s:?} for `ssl_mode`"));
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct MySqlConnectOptions {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) socket: Option<PathBuf>,
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) database: Option<String>,
    pub(crate) ssl_mode: MySqlSslMode,
    pub(crate) ssl_ca: Option<PathBuf>,
    pub(crate) ssl_client_cert: Option<PathBuf>,
    pub(crate) ssl_client_key: Option<PathBuf>,
}

impl Default for MySqlConnectOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl MySqlConnectOptions {
    /// Creates a new, default set of options ready for configuration
    pub fn new() -> Self {
        Self {
            port: 3306,
            host: String::from("localhost"),
            socket: None,
            username: String::from("root"),
            password: None,
            database: None,
            ssl_mode: MySqlSslMode::Preferred,
            ssl_ca: None,
            ssl_client_cert: None,
            ssl_client_key: None,
        }
    }

    /// Sets the name of the host to connect to.
    ///
    /// The default behavior when the host is not specified,
    /// is to connect to localhost.
    pub fn host(mut self, host: &str) -> Self {
        self.host = host.to_owned();
        self
    }

    /// Sets the port to connect to at the server host.
    ///
    /// The default port for MySQL is `3306`.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Pass a path to a Unix socket. This changes the connection stream from
    /// TCP to UDS.
    ///
    /// By default set to `None`.
    pub fn socket(mut self, path: impl AsRef<Path>) -> Self {
        self.socket = Some(path.as_ref().to_path_buf());
        self
    }

    /// Sets the username to connect as.
    pub fn username(mut self, username: &str) -> Self {
        self.username = username.to_owned();
        self
    }

    /// Sets the password to connect with.
    pub fn password(mut self, password: &str) -> Self {
        self.password = Some(password.to_owned());
        self
    }

    /// Sets the database name.
    pub fn database(mut self, database: &str) -> Self {
        self.database = Some(database.to_owned());
        self
    }

    /// Sets whether or with what priority a secure SSL TCP/IP connection will be negotiated
    /// with the server.
    ///
    /// By default, the SSL mode is [`Preferred`](MySqlSslMode::Preferred), and the client will
    /// first attempt an SSL connection but fallback to a non-SSL connection on failure.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use sqlx_core::mysql::{MySqlSslMode, MySqlConnectOptions};
    /// let options = MySqlConnectOptions::new()
    ///     .ssl_mode(MySqlSslMode::Required);
    /// ```
    pub fn ssl_mode(mut self, mode: MySqlSslMode) -> Self {
        self.ssl_mode = mode;
        self
    }

    /// Sets the name of a file containing a list of trusted SSL Certificate Authorities.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use sqlx_core::mysql::{MySqlSslMode, MySqlConnectOptions};
    /// let options = MySqlConnectOptions::new()
    ///     .ssl_mode(MySqlSslMode::VerifyCa)
    ///     .ssl_ca("path/to/ca.crt");
    /// ```
    pub fn ssl_ca(mut self, file_name: impl AsRef<Path>) -> Self {
        self.ssl_ca = Some(file_name.as_ref().to_owned());
        self
    }

    /// Sets the name of a file containing SSL client certificate.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use sqlx_core::mysql::{MySqlSslMode, MySqlConnectOptions};
    /// let options = MySqlConnectOptions::new()
    ///     .ssl_mode(MySqlSslMode::VerifyCa)
    ///     .ssl_client_cert("path/to/client.crt");
    /// ```
    pub fn ssl_client_cert(mut self, cert: impl AsRef<Path>) -> Self {
        self.ssl_client_cert = Some(cert.as_ref().to_path_buf());
        self
    }

    /// Sets the name of a file containing SSL client key.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use sqlx_core::mysql::{MySqlSslMode, MySqlConnectOptions};
    /// let options = MySqlConnectOptions::new()
    ///     .ssl_mode(MySqlSslMode::VerifyCa)
    ///     .ssl_client_key("path/to/client.key");
    /// ```
    pub fn ssl_client_key(mut self, key: impl AsRef<Path>) -> Self {
        self.ssl_client_key = Some(key.as_ref().to_path_buf());
        self
    }
}
