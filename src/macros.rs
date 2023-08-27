//! This module contains a few useful macros in order to not need the builder.
//!
//! It provides the following macros:
//!
//! * `generate!` - Generate a `destination_path` file using migrations from the provided
//! `migrations_path` folder.
//!
//! * `generate_using_defaults!` - Generate a `./structure.sql` file using migrations
//! from the `./migrations` folder.

/// Generate a `destination_path` file using migrations from the provided
/// `migrations_path` folder.
#[macro_export]
macro_rules! generate_without_runtime {
    ($migrations_path:expr, $structure_path:expr) => {
        let migrations_path: ::std::path::PathBuf = $migrations_path.into();
        let destination_path: ::std::path::PathBuf = $structure_path.into();
        $crate::macros::__generate_within_runtime(migrations_path, destination_path)
    };
}

/// Generate a `./structure.sql` file using migrations from the `./migrations` folder.
#[macro_export]
macro_rules! generate_without_runtime_using_defaults {
    () => {
        $crate::macros::__generate_within_runtime(
            ::std::path::PathBuf::from("./migrations"),
            ::std::path::PathBuf::from("./structure.sql"),
        );
    };
}

/// Generate a structure.sql file using migrations from the `migrations_path` folder.
/// DO NOT USE THIS DIRECTLY. Use the `generate!` macro instead.
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

    #[cfg(feature = "runtime-async-std")]
    async_std::task::block_on(async {
        crate::generate(None, migrations_path, destination_path).await
    })
    .expect("could not run async-std runtime");
}
