use arboard::Clipboard;
use daemonize::Daemonize;
use std::fs::File;
use tokio::time;

use crate::utils::get_user;
mod utils;
mod db;
mod app;

fn main() {

    get_user();

    let stdout = File::create("./daemon.out").unwrap();
    let stderr = File::create("./daemon.err").unwrap();

    let daemonize = Daemonize::new()
        .pid_file("/tmp/test.pid") // Every method except `new` and `start`
        .chown_pid_file(true) // is optional, see `Daemonize` documentation
        .working_directory("/tmp") // for default behaviour.
        .user(&*get_user())
        .group("daemon") // Group name
        .group(2) // or group id.
        .umask(0o777) // Set umask, `0o027` by default.
        .stdout(stdout) // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr) // Redirect stderr to `/tmp/daemon.err`.
        .exit_action(|| println!("Executed before master process exits"))
        .privileged_action(|| "Executed before drop privileges");

    match daemonize.start() {
        Ok(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut interval = time::interval(time::Duration::from_millis(500));
                loop {
                    refresh_clipboard();
                    interval.tick().await;
                }
            }),
        Err(e) => eprintln!("Error, {}", e),
    }
}
