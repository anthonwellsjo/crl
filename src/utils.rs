use std::process::Command;
use std::str;

pub fn get_user() -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg("whoami")
        .output()
        .expect("failed to execute process");

    let mut user = str::from_utf8(&output.stdout).unwrap().to_owned();

    //Removes /n from string
    user.pop().unwrap().to_string();

    user
}
