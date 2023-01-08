use std::{
    fs::{self, File},
    process::{self, Command},
    time::Duration,
};

use arboard::Clipboard;
use arw_brr::{get_processes, get_user};
use daemonize::Daemonize;
use rand::Rng;
use tokio::time::interval;

use crate::db::{get_crls, get_db_path, get_latest, save_new_crl, Crl, SavedCrl};

#[derive(Debug, PartialEq)]
pub enum Action {
    Start,
    Health,
    Kill,
    List,
}
impl Action {
    pub fn from_string(s: &str) -> Option<Action> {
        match s {
            "s" | "start" => Some(Action::Start),
            "h" | "health" => Some(Action::Health),
            "k" | "kill" => Some(Action::Kill),
            "l" | "list" => Some(Action::List),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ActionResponse {
    pub message: String,
    pub _type: ActionResponseType,
    pub crl: Option<SavedCrl>,
}

#[derive(Debug, PartialEq)]
pub enum ActionResponseType {
    Error,
    Success,
    Content,
}
pub struct Session {
    pub action_responses: Vec<ActionResponse>,
}

impl Session {
    pub fn new() -> Self {
        Session {
            action_responses: vec![],
        }
    }

    pub fn run(&mut self, action: Option<Action>, argument: Option<String>) {
        match action {
            Some(Action::Start) => {
                self.run_daemon();
            }
            Some(Action::Health) => {
                self.check_health();
            }
            Some(Action::Kill) => {
                self.kill_daemon();
            }
            Some(Action::List) => {
                match argument {
                    Some(arg) => {
                        if arg.parse::<u32>().is_ok() {
                            self.list_crls(arg.parse::<u32>().unwrap());
                        }
                    }
                    None => {
                        self.action_responses.push(ActionResponse {
                            message: "this action requires an argument.".to_string(),
                            _type: ActionResponseType::Error,
                            crl: None,
                        });
                    }
                };
            }
            None => {
                self.action_responses.push(ActionResponse {
                    message: "no action?".to_string(),
                    _type: ActionResponseType::Success,
                    crl: None,
                });
            }
        }
    }

    pub fn refresh_clipboard() {
        let mut clipboard = Clipboard::new().unwrap();
        let os_clip = clipboard.get_text().unwrap();
        let crl_clip = get_latest().unwrap();
        // println!("OS clipboard: {}", os_clip);
        match crl_clip {
            Some(crl) => {
                // println!("CRL clipboard: {}", crl.crl.text);
                if os_clip != crl.crl.text {
                    // println!("DIFF detected -> updating");
                    save_new_crl(&Crl { text: os_clip }).unwrap();
                }
            }
            None => {
                // println!("CRL clipboard is empty");
                // println!("DIFF detected -> updating");
                save_new_crl(&Crl { text: os_clip }).unwrap();
            }
        }
    }

    pub fn run_daemon(&mut self) {
        self.action_responses.push(ActionResponse {
            message: "Starting crl...".to_string(),
            _type: ActionResponseType::Content,
            crl: None,
        });

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
            .exit_action(|| println!("crl process started. check health with 'crl health'"))
            .privileged_action(|| "Executed before drop privileges");

        match daemonize.start() {
            Ok(_) => tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let mut interval = interval(Duration::from_millis(500));
                    loop {
                        Session::refresh_clipboard();
                        interval.tick().await;
                    }
                }),
            Err(e) => {
                self.action_responses.push(ActionResponse {
                    message: e.to_string(),
                    _type: ActionResponseType::Content,
                    crl: None,
                });
                return;
            }
        }
        self.action_responses.push(ActionResponse {
            message: "Ok!".to_string(),
            _type: ActionResponseType::Success,
            crl: None,
        });
        self.action_responses.push(ActionResponse {
            message: "But check that process is alive with arg 'crl health'".to_string(),
            _type: ActionResponseType::Content,
            crl: None,
        });
    }

    fn check_health(&mut self) {
        let procs = get_processes("crl", true);

        if procs.pid.len() < 1 {
            self.action_responses.push(ActionResponse {
                message: "clr daemon is not running. run 'sudo crl start' to restart...".to_owned(),
                _type: ActionResponseType::Error,
                crl: None,
            })
        }

        if procs.pid.len() == 1 {
            self.action_responses.push(ActionResponse {
                message: "crl daemon is running on pid: ".to_owned()
                    + &procs.pid.first().unwrap().to_string(),
                _type: ActionResponseType::Success,
                crl: None,
            })
        }

        if procs.pid.len() > 1 {
            self.action_responses.push(ActionResponse {
                message: "more than one daemon running... not good.".to_owned(),
                _type: ActionResponseType::Error,
                crl: None,
            })
        }
    }

    fn kill_daemon(&mut self) {
        let procs = get_processes("crl", true);
        for p in procs.pid {
            Command::new("sh")
                .arg("-c")
                .arg("kill -9 ".to_owned() + &p.to_string())
                .output()
                .expect("failed to execute process");
        }

        self.action_responses.push(ActionResponse {
            message: "daemon killed. check health with 'crl health'".to_string(),
            _type: ActionResponseType::Success,
            crl: None,
        })
    }

    fn list_crls(&mut self, arg: u32) {
        let num = neutralize_num(arg, 0, 50);
        let crls = get_crls(arg).unwrap();
        println!("{:?}", crls);
    }
}

pub fn neutralize_num(input: u32, min: u32, max: u32) -> u32 {
    if input > max {
        max
    } else if input < min {
        min
    } else {
        input
    }
}

// /// Contains common util functions and properties for testing
pub struct TestUtils {}

impl TestUtils {
    pub fn create_rnd_string() -> String {
        let mut rng = rand::thread_rng();
        let rand_num: u16 = rng.gen();
        rand_num.to_string()
    }
    pub fn cleanup_test_database() {
        fn remove_test_db() {
            if std::path::Path::new(&get_db_path()).exists() {
                fs::remove_file(get_db_path()).unwrap_or_else(|err| {
                    panic!("Panicking while deleting test database: {}", err)
                });
            }
        }
        remove_test_db();
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time};

    use arboard::Clipboard;

    use crate::{
        app::TestUtils,
        db::{get_latest, save_new_crl, Crl},
    };

    use super::Session;

    #[test]
    fn detect_diff() {
        TestUtils::cleanup_test_database();

        let mut clipboard = Clipboard::new().unwrap();
        clipboard
            .set_text("prob not same as the clr clipboard text")
            .unwrap();

        let new_crl = Crl::new("it won't be now.");
        save_new_crl(&new_crl).unwrap();

        thread::sleep(time::Duration::from_millis(1010));
        Session::refresh_clipboard();

        let latest = get_latest().unwrap();

        match latest {
            Some(crl) => {
                assert_ne!(new_crl.text, crl.crl.text)
            }
            None => assert!(false),
        }
    }
}
