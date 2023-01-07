use std::{fs, time::SystemTime};

use arboard::Clipboard;
use rand::Rng;

use crate::db::{get_db_path, get_latest, save_new_crl, Crl};

pub fn refresh_clipboard() {
    let mut clipboard = Clipboard::new().unwrap();
    let os_clip = clipboard.get_text().unwrap();
    let crl_clip = get_latest().unwrap();
    println!("OS clipboard: {}", os_clip);
    match crl_clip {
        Some(crl) => {
            println!("CRL clipboard: {}", crl.crl.text);
            if os_clip != crl.crl.text {
                println!("DIFF detected -> updating");
                save_new_crl(&Crl { text: os_clip }).unwrap();
            }
        }
        None => {
            println!("CRL clipboard is empty");
            println!("DIFF detected -> updating");
            save_new_crl(&Crl { text: os_clip }).unwrap();
        }
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
        db::{save_new_crl, Crl, get_latest},
    };

    use super::refresh_clipboard;

    #[test]
    fn detect_diff() {
        TestUtils::cleanup_test_database();

        let mut clipboard = Clipboard::new().unwrap();
        clipboard.set_text("prob not same as the clr clipboard text").unwrap();

        let new_crl = Crl::new("it won't be now.");
        save_new_crl(&new_crl).unwrap();

        
        thread::sleep(time::Duration::from_millis(1010));
        refresh_clipboard();

        let latest = get_latest().unwrap();

        match latest {
            Some(crl) => {
                assert_ne!(new_crl.text, crl.crl.text)
            },
            None => assert!(false),
        }
    }
}
