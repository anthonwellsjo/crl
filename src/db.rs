use std::fs;

use dirs;
use rusqlite::{Connection, Result};

pub struct SavedCrl {
    pub crl: Crl,
    pub id: i64,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct Crl {
    pub text: String,
}

impl Crl {
    pub fn new(text: &str) -> Crl {
        Crl {
            text: text.to_owned(),
        }
    }
}

///  Gets connection to DB. This function will create a new DB if
///  not already present
pub fn get_db_connection() -> Result<Connection> {
    let conn = Connection::open(get_db_path())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS crls (
             id INTEGER PRIMARY KEY,
             text TEXT NOT NULL,
             timestamp INTEGER DEFAULT CURRENT_TIMESTAMP,
         )",
        [],
    )?;
    Ok(conn)
}

/// Gets all dros from the database
/// # Examples
/// ```
/// use core::db::get_dros;
/// let res = get_dros();
/// ```
pub fn get_crls() -> Result<Vec<SavedCrl>> {
    let conn = get_db_connection()?;

    let mut stmt = conn.prepare(
        "SELECT text, timestamp, id
         FROM crls",
    )?;

    let crls = stmt.query_map([], |row| {
        Ok(SavedCrl {
            crl: Crl {
                text: row.get(0)?,
            },
            timestamp: row.get(1)?,
            id: row.get(2)?,
        })
    })?;

    let mut saved_crls: Vec<SavedCrl> = Vec::new();

    for crl in crls {
        let file = match crl {
            Ok(file) => file,
            Err(error) => panic!("Problem opening the file: {:?}", error),
        };
        saved_crls.push(file);
    }

    Ok(saved_crls)
}

/// Saves a dro to the database
/// # Arguments
/// * `to_do` - In instance of the dro struct that will be saved.
/// # Examples
/// ```
/// use core::db::{Dro, save_dro_to_db};
/// let to_do = Dro::new("Fix the bike wheel");
/// let res = save_dro_to_db(to_do);
/// assert_eq!(res, Ok(()));
/// ```
pub fn save_new_crl(crl: &Crl) -> Result<()> {
    let conn = get_db_connection()?;

    conn.execute(
        "INSERT INTO crls (text) values (?1)",
        &[&crl.text.to_string()],
    )?;

    conn.close()
        .unwrap_or_else(|_| panic!("Panicking while closing conection."));

    Ok(())
}

/// Gets db-path depending on environment and os. Creates path if not yet there.
fn get_db_path() -> String {
    if cfg!(test) {
        String::from("./test-db.sql")
    } else {
        match dirs::home_dir() {
            Some(dir) => {
                let path = dir.to_str().unwrap().to_owned() + "/dro/";
                fs::create_dir_all(&path).unwrap();
                path + "db.sql"
            }
            None => panic!("Could not find a home directory"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db::{save_new_crl, Crl};


    #[test]
    fn get_crls() {
        cleanup_test_database();
        let descs = vec!["one", "two", "three"];
        for desc in descs.iter() {
            let to_do = Crl::new("very important text");
            save_new_crl(&to_do).unwrap();
        }
        let crls_from_db = super::get_crls().unwrap();
        let mut descs_from_db = crls_from_db.iter().map(|crl| -> &str { &crl.description });
        assert!(descs_from_db.all(|item| descs.contains(&item)));
    }

    #[test]
    fn save_a_dro() {
        let description = "Test description";
        let to_do = Dro::new(description);
        save_dro_to_db(&to_do).unwrap();
        let to_dos = get_dros().unwrap();
        assert_eq!(to_dos.iter().any(|i| i.description == description), true);
    }

    #[test]
    fn save_and_load_dros_from_db() {
        let description = TestUtils::create_rnd_string();
        let description_two = TestUtils::create_rnd_string();
        let to_do = Dro::new(&description);
        let to_do2 = Dro::new(&description_two);
        save_dro_to_db(&to_do).unwrap();
        save_dro_to_db(&to_do2).unwrap();

        let dros = get_dros().unwrap();
        assert!(&dros.iter().any(|x| x.description == description_two));
    }

    #[test]
    fn mark_as_done() {
        cleanup_test_database();
        let description = TestUtils::create_rnd_string();
        let to_do = Dro::new(&description);
        save_dro_to_db(&to_do).unwrap();
        mark_dro_as_done(&description).unwrap();
        let dros = get_dros().unwrap();
        let dro: &Dro = dros.iter().nth(0).unwrap();
        assert_eq!(dro.done, true);
    }

    #[test]
    fn mark_as_undone() {
        cleanup_test_database();
        let description = TestUtils::create_rnd_string();
        let to_do = Dro::new(&description);
        save_dro_to_db(&to_do).unwrap();
        mark_dro_as_done(&description).unwrap();
        let dros_done = get_dros().unwrap();
        let dro_done: &Dro = dros_done.iter().nth(0).unwrap();
        assert_eq!(dro_done.done, true);
        mark_dro_as_undone(&description).unwrap();
        let dros_undone = get_dros().unwrap();
        let dro_undone: &Dro = dros_undone.iter().nth(0).unwrap();
        assert_eq!(dro_undone.done, false);
    }

    #[test]
    #[ignore]
    fn cleanup_test_database() {
        fn remove_test_db() {
            fs::remove_file(get_db_path())
                .unwrap_or_else(|err| panic!("Panicking while deleting test database: {}", err));
        }
        remove_test_db();
    }

    /// Contains common util functions and properties for testing
    struct TestUtils {}

    impl TestUtils {
        fn create_rnd_string() -> String {
            let mut rng = rand::thread_rng();
            let rand_num: u16 = rng.gen();
            rand_num.to_string()
        }
    }
}
