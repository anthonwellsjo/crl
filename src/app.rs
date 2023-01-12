use std::{
    fs::{self, File},
    process::Command,
    time::Duration,
};

use arboard::Clipboard;
use arw_brr::{get_processes, get_user};
use daemonize::Daemonize;
use rand::Rng;
use tokio::time::interval;

use crate::db::{self, get_db_path, get_latest, get_many, save_new_crl, Crl, SavedCrl};

#[derive(Debug, PartialEq)]
pub enum Action {
    Start,
    Health,
    Kill,
    List,
    Clean,
    Set,
    Get,
    Help,
}
impl Action {
    pub fn from_string(s: &str) -> Option<Action> {
        match s {
            "start" => Some(Action::Start),
            "s" | "set" => Some(Action::Set),
            "health" => Some(Action::Health),
            "k" | "kill" => Some(Action::Kill),
            "l" | "list" => Some(Action::List),
            "c" | "clean" => Some(Action::Clean),
            "g" | "get" => Some(Action::Get),
            "h" | "help" => Some(Action::Help),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ActionResponse {
    pub message: String,
    pub _type: ActionResponseType,
    pub crls: Option<Vec<SavedCrl>>,
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
                self.run_daemon(false);
            }
            Some(Action::Health) => {
                self.check_health();
            }
            Some(Action::Kill) => {
                self.kill_daemon();
            }
            Some(Action::List) => {
                self.list_crls(argument);
            }
            Some(Action::Set) => {
                self.set_crl_to_clipboard(argument);
            }
            Some(Action::Clean) => {
                self.clean_database();
            }
            Some(Action::Get) => {
                todo!("a fn for getting crls from a query");
            }
            Some(Action::Help) => {
                self.show_help();
            }
            None => {
                self.action_responses.push(ActionResponse {
                    message: "no action?".to_string(),
                    _type: ActionResponseType::Success,
                    crls: None,
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

    pub fn run_daemon(&mut self, block_polling: bool) {
        self.action_responses.push(ActionResponse {
            message: "Starting crl...".to_string(),
            _type: ActionResponseType::Content,
            crls: None,
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
                        if !block_polling {
                            Session::refresh_clipboard();
                        }
                        interval.tick().await;
                    }
                }),
            Err(e) => {
                self.action_responses.push(ActionResponse {
                    message: e.to_string(),
                    _type: ActionResponseType::Content,
                    crls: None,
                });
                return;
            }
        }
        self.action_responses.push(ActionResponse {
            message: "Ok!".to_string(),
            _type: ActionResponseType::Success,
            crls: None,
        });
        self.action_responses.push(ActionResponse {
            message: "But check that process is alive with arg 'crl health'".to_string(),
            _type: ActionResponseType::Content,
            crls: None,
        });
    }

    fn check_health(&mut self) {
        let procs = get_processes("crl", true);

        if procs.pid.len() < 1 {
            self.action_responses.push(ActionResponse {
                message: "clr daemon is not running. run 'sudo crl start' to restart...".to_owned(),
                _type: ActionResponseType::Error,
                crls: None,
            })
        }

        if procs.pid.len() == 1 {
            self.action_responses.push(ActionResponse {
                message: "crl daemon is running on pid: ".to_owned()
                    + &procs.pid.first().unwrap().to_string(),
                _type: ActionResponseType::Success,
                crls: None,
            })
        }

        if procs.pid.len() > 1 {
            self.action_responses.push(ActionResponse {
                message: "more than one daemon running... not good.".to_owned(),
                _type: ActionResponseType::Error,
                crls: None,
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
            crls: None,
        })
    }

    fn list_crls(&mut self, arg: Option<String>) {
        let crls;
        match arg {
            Some(arg) => {
                if arg.parse::<u32>().is_ok() {
                    crls = get_many(arg.parse::<u32>().unwrap())
                } else {
                    self.action_responses.push(ActionResponse {
                        message: "limit argument needs to fit in a u32 integer.".to_string(),
                        _type: ActionResponseType::Error,
                        crls: None,
                    });
                    return;
                }
            }
            None => {
                crls = get_many(25);
            }
        }
        self.action_responses.push(ActionResponse {
            message: "".to_string(),
            _type: ActionResponseType::Content,
            crls: Some(crls.unwrap()),
        });
    }

    fn set_crl_to_clipboard(&mut self, id: Option<String>) {
        match id {
            Some(id) => {
                let mut clipboard = Clipboard::new().unwrap();
                let res = db::get_one(&id);

                match res {
                    Ok(res) => {
                        if let Some(crl) = res {
                            clipboard.set_text(&crl.crl.text).unwrap();
                            self.action_responses.push(ActionResponse {
                                message: "success!".to_string(),
                                _type: ActionResponseType::Success,
                                crls: Some(vec![crl]),
                            })
                        } else {
                            self.action_responses.push(ActionResponse {
                                message: "no crl with that id!".to_string(),
                                _type: ActionResponseType::Error,
                                crls: None,
                            })
                        }
                    }
                    Err(err) => self.action_responses.push(ActionResponse {
                        message: err.to_string(),
                        _type: ActionResponseType::Error,
                        crls: None,
                    }),
                }
            }
            None => self.action_responses.push(ActionResponse {
                message: "please provide a valid id.".to_string(),
                _type: ActionResponseType::Error,
                crls: None,
            }),
        }
    }

    fn clean_database(&mut self) {
        match db::reset() {
            Ok(num) => self.action_responses.push(ActionResponse {
                message: num.to_string() + &" crls deleted".to_string(),
                _type: ActionResponseType::Success,
                crls: None,
            }),
            Err(err) => self.action_responses.push(ActionResponse {
                message: err.to_string(),
                _type: ActionResponseType::Error,
                crls: None,
            }),
        }
    }

    fn show_help(&mut self) {
        self.action_responses.push(ActionResponse {
            message: "
command:        argument:

start           -                   start crl daemon
s, set          crl id              sets crl to os clipboard
health          -                   check daemon health
k, kill         -                   kill crl daemon
l, list         -, limit            lists crls 
c, clean        -                   deletes all crl
g, get          query               queries crls and lists them
h, help         -                   what you are doing now
            ".to_string(),
            _type: ActionResponseType::Content,
            crls: None,
        });
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

    use super::{Session};

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
