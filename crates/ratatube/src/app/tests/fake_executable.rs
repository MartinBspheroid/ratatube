//! Race-free creation of the script-based test doubles.
//!
//! `execve` fails with `ETXTBSY` while *any* process holds the target file open
//! for writing. These suites run in parallel threads of one test binary, so
//! while thread A still has its script's write fd open, thread B's fork for its
//! own child inherits that fd and drops it only at the child's `exec` (Rust
//! opens files `O_CLOEXEC`). During that window thread A's `exec` of its own
//! script loses to the inherited fd and fails with "Text file busy".
//!
//! The window is narrow and self-closing: once the writer's handle is gone and
//! the transient children have exec'd, nobody can hold a write fd on the script
//! again. So the helper closes the window itself before handing the script to a
//! test — write it, drop the handle, then `exec` it repeatedly until the `exec`
//! stops failing with `ETXTBSY`. After one probe succeeds the test's real
//! invocation cannot hit the race.
//!
//! Every script therefore starts with a guard line that exits before the body
//! runs when the first argument is [`PROBE_ARGUMENT`], so a probe leaves no
//! captured arguments, pid file, or clipboard record behind for the test to
//! trip over.
//!
//! A deliberate twin of `ratatube-services/tests/support/fake_executable.rs`:
//! test-only code cannot cross a crate boundary, and this crate's browser test
//! doubles need the same guarantee. Keep the two copies in step.

use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

/// First argument that makes a fake executable exit before its body runs.
const PROBE_ARGUMENT: &str = "--ratatube-exec-probe";

/// How long a script may keep reporting `ETXTBSY` before we call it a real bug
/// rather than the fork/exec window this helper exists to wait out.
const PROBE_BUDGET: Duration = Duration::from_secs(10);

/// Write `body` as an executable `interpreter` script at `path`, returning only
/// once the script can actually be exec'd.
pub(crate) fn write_fake_executable(path: &Path, interpreter: &str, body: &str) {
    let script =
        format!("#!{interpreter}\nif [ \"$1\" = \"{PROBE_ARGUMENT}\" ]; then exit 0; fi\n{body}\n");
    // `fs::write` closes its handle before returning; the probe below waits out
    // the copies sibling threads' forks may still be holding.
    std::fs::write(path, script).expect("write fake executable");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("make fake executable");
    wait_until_executable(path);
}

/// Exec `path` until the kernel stops answering `ETXTBSY`.
fn wait_until_executable(path: &Path) {
    let deadline = Instant::now() + PROBE_BUDGET;
    let mut attempts = 0_u32;
    loop {
        attempts += 1;
        match std::process::Command::new(path)
            .arg(PROBE_ARGUMENT)
            .output()
        {
            Ok(output) => {
                assert!(
                    output.status.success(),
                    "exec probe of {} exited with {} and stderr {:?}",
                    path.display(),
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
                return;
            }
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                assert!(
                    Instant::now() < deadline,
                    "{} still reported ETXTBSY after {attempts} probes over {PROBE_BUDGET:?}; \
                     something holds it open for writing",
                    path.display()
                );
                std::thread::yield_now();
            }
            Err(error) => panic!("exec probe of {} failed: {error}", path.display()),
        }
    }
}
