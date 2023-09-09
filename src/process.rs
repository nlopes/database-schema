pub(crate) use std::process::Command;

/// Run a command line program
#[allow(dead_code)]
pub(crate) async fn run(command: &mut Command) -> Result<(), crate::error::Error> {
    let prog = command.get_program().to_string_lossy().into_owned();
    let args = command
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect::<Vec<String>>();

    match command.output() {
        Err(error) => {
            tracing::error!(?error, ?prog, ?args, "Command failed to run");
            Err(error.into())
        }
        Ok(output) => {
            if !output.status.success() {
                tracing::error!(?output, ?prog, ?args, "Command failed");
                Err(crate::error::Error::CommandRunError(format!(
                    "output: {}\nstderr: {}",
                    String::from_utf8_lossy(&output.stdout).into_owned(),
                    String::from_utf8_lossy(&output.stderr).into_owned(),
                )))
            } else {
                tracing::trace!(?prog, ?args, "Command succeeded");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
