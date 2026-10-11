pub(crate) use std::process::Command;

/// Run a command line program
#[allow(dead_code)]
pub(crate) async fn run(command: &mut Command) -> Result<(), crate::error::Error> {
    let prog = command.get_program().to_string_lossy().into_owned();
    match command.output() {
        Err(error) => {
            tracing::error!(?error, ?prog, "Command failed to run");
            Err(error.into())
        }
        Ok(output) => {
            if !output.status.success() {
                tracing::error!(status = ?output.status, ?prog, "Command failed");
                Err(crate::error::Error::CommandRunError(format!(
                    "output: {}\nstderr: {}",
                    String::from_utf8_lossy(&output.stdout).into_owned(),
                    String::from_utf8_lossy(&output.stderr).into_owned(),
                )))
            } else {
                tracing::trace!(?prog, "Command succeeded");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[derive(Clone)]
    struct Capture(std::sync::Arc<std::sync::Mutex<String>>);

    impl tracing::field::Visit for Capture {
        fn record_debug(&mut self, _: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            use std::fmt::Write;
            write!(self.0.lock().unwrap(), "{value:?}").unwrap();
        }
    }

    impl tracing::Subscriber for Capture {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            event.record(&mut self.clone());
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    #[test]
    fn command_logs_omit_arguments_and_output() {
        let output = super::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "process::tests::command_logs_child",
                "--nocapture",
            ])
            .env("DATABASE_SCHEMA_TEST_LOGS", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

    #[test]
    fn command_logs_child() {
        if std::env::var_os("DATABASE_SCHEMA_TEST_LOGS").is_none() {
            return;
        }
        let capture = Capture(std::sync::Arc::default());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        tracing::subscriber::with_default(capture.clone(), || {
            let mut command = super::Command::new("sh");
            let _ = command.args(["-c", "printf '%s' \"$1\"; exit 1", "sh", "synthetic_secret"]);
            assert!(runtime.block_on(super::run(&mut command)).is_err());
        });
        let logs = capture.0.lock().unwrap();
        assert!(logs.contains("Command failed"));
        assert!(!logs.contains("synthetic_secret"));
    }

    #[tokio::test]
    async fn test_run_not_found() -> Result<(), crate::Error> {
        use super::{run, Command};
        let result = run(&mut Command::new("mysqldump-nonexistent")).await;
        assert!(
            matches!(result, Err(crate::Error::IOError(t)) if t.kind() == std::io::ErrorKind::NotFound)
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_run_found() -> Result<(), crate::Error> {
        use super::{run, Command};
        let result = run(Command::new("which").arg("which")).await;
        assert!(matches!(result, Ok(())));
        Ok(())
    }

    #[tokio::test]
    async fn test_run_invalid_arguments() -> Result<(), crate::Error> {
        use super::{run, Command};
        let result = run(Command::new("which").arg("--norberto")).await;
        assert!(matches!(result, Err(crate::Error::CommandRunError(_))));
        Ok(())
    }
}
