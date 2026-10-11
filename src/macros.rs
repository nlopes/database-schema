//! Generate a schema with a runtime managed by the macro.
//!
//! It provides the following macros:
//!
//! * [`generate_without_runtime!`](crate::generate_without_runtime) - Generate a `destination_path` file using migrations from the provided
//!   `migrations_path` folder.
//!
//! * [`generate_without_runtime_using_defaults!`](crate::generate_without_runtime_using_defaults) - Generate a `./structure.sql` file using migrations
//!   from the `./migrations` folder.
//!
//! Enable `macros` and a runtime feature. If both runtimes are enabled, the macros
//! use Tokio. Both macros return `()` and panic if schema generation fails.

/// Generate a `destination_path` file using migrations from the provided
/// `migrations_path` folder.
///
/// # Panics
///
/// Panics if schema generation or runtime creation fails. With `runtime-tokio`,
/// a call inside an active Tokio runtime also panics.
#[macro_export]
macro_rules! generate_without_runtime {
    ($migrations_path:expr, $structure_path:expr) => {{
        let migrations_path: ::std::path::PathBuf = $migrations_path.into();
        let destination_path: ::std::path::PathBuf = $structure_path.into();
        $crate::macros::__generate_within_runtime(migrations_path, destination_path)
    }};
}

/// Generate a `./structure.sql` file using migrations from the `./migrations` folder.
///
/// # Panics
///
/// Panics under the same conditions as [`generate_without_runtime!`].
#[macro_export]
macro_rules! generate_without_runtime_using_defaults {
    () => {{
        $crate::macros::__generate_within_runtime(
            ::std::path::PathBuf::from("./migrations"),
            ::std::path::PathBuf::from("./structure.sql"),
        );
    }};
}

/// Generate a structure.sql file using migrations from the `migrations_path` folder.
/// Use [`generate_without_runtime!`](crate::generate_without_runtime) instead.
#[doc(hidden)]
pub fn __generate_within_runtime(
    migrations_path: std::path::PathBuf,
    destination_path: std::path::PathBuf,
) {
    run_runtime(migrations_path, destination_path);
}

fn run_runtime(migrations_path: std::path::PathBuf, destination_path: std::path::PathBuf) {
    #[cfg(feature = "runtime-tokio")]
    {
        let rt = tokio::runtime::Runtime::new().expect("could not create tokio runtime");
        let local = tokio::task::LocalSet::new();
        local
            .block_on(&rt, async {
                crate::generate(None, migrations_path, destination_path).await
            })
            .expect("could not run tokio runtime");
    }

    #[cfg(all(feature = "runtime-async-std", not(feature = "runtime-tokio")))]
    async_std::task::block_on(async {
        crate::generate(None, migrations_path, destination_path).await
    })
    .expect("could not run async-std runtime");
}
