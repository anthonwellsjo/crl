use std::fs;

use dirs;
use rusqlite::{Connection, Result};

#[derive(Debug)]
pub struct SavedCrl {
    pub crl: Crl,
    pub id: i64,
    pub created_at: String,
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
    let conn = Connection::open(get_app_path())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS crls (
             id INTEGER PRIMARY KEY,
             text TEXT NOT NULL,
             created_at INTEGER DEFAULT CURRENT_TIMESTAMP)",
        [],
    )?;

    Ok(conn)
}

pub fn reset() -> Result<usize> {
    let conn = Connection::open(get_app_path())?;
    let rows = conn.execute(
        "DELETE FROM crls",
        [],
    )?;

    Ok(rows)
}

pub fn get_many(limit: u32) -> Result<Vec<SavedCrl>> {
    let limit = neutralize_num(limit, 0, 50);
    let conn = get_db_connection()?;

    let mut stmt = conn.prepare(
        &("SELECT id, text, created_at
         FROM crls
         ORDER BY created_at DESC
         LIMIT ".to_owned() + &limit.to_string())
    )?;

    let crls = stmt.query_map([], |row| {
        let crl = SavedCrl {
            id: row.get(0)?,
            crl: Crl { text: row.get(1)? },
            created_at: row.get(2)?,
        };
        Ok(crl)
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

pub fn get_latest() -> Result<Option<SavedCrl>> {
    let conn = get_db_connection()?;

    let mut stmt = conn.prepare(
        "SELECT id, text, MAX(created_at)
         FROM crls",
    )?;

    let crls = stmt.query_map([], |row| {
        let crl = SavedCrl {
            id: row.get(0)?,
            crl: Crl { text: row.get(1)? },
            created_at: row.get(2)?,
        };
        Ok(crl)
    })?;

    let mut saved_crls: Vec<Option<SavedCrl>> = Vec::new();

    for crl in crls {
        let record = match crl {
            Ok(_crl) => Some(_crl),
            Err(_) => None,
        };
        saved_crls.push(record);
    }

    match saved_crls.pop() {
        Some(crl) => {
            match crl {
                Some(_) => Ok(crl),
                None => Ok(None),
            }
        }
        None => Ok(None),
    }
}

pub fn get_one(id: &str) -> Result<Option<SavedCrl>> {
    let conn = get_db_connection()?;

    let mut stmt = conn.prepare(
        &("SELECT id, text, created_at
         FROM crls
         WHERE id=".to_owned() + id),
    )?;

    let crls = stmt.query_map([], |row| {
        let crl = SavedCrl {
            id: row.get(0)?,
            crl: Crl { text: row.get(1)? },
            created_at: row.get(2)?,
        };
        Ok(crl)
    })?;

    let mut saved_crls: Vec<Option<SavedCrl>> = Vec::new();

    for crl in crls {
        let record = match crl {
            Ok(_crl) => Some(_crl),
            Err(_) => None,
        };
        saved_crls.push(record);
    }

    match saved_crls.pop() {
        Some(crl) => {
            match crl {
                Some(_) => Ok(crl),
                None => Ok(None),
            }
        }
        None => Ok(None),
    }
}

pub fn save_new_crl(crl: &Crl) -> Result<()> {
    let conn = get_db_connection()?;

    conn.execute(
        "INSERT INTO crls (text) values (?1)",
        &[&crl.text],
    )?;

    conn.close()
        .unwrap_or_else(|_| panic!("Panicking while closing conection."));

    Ok(())
}

/// Gets db-path depending on environment and os. Creates path if not yet there.
pub fn get_app_path() -> String {
    if cfg!(test) {
        String::from("./test-db.sql")
    } else {
        match dirs::home_dir() {
            Some(dir) => {
                let path = dir.to_str().unwrap().to_owned() + "/Library/Application Support/crl/";
                fs::create_dir_all(&path).unwrap();
                path + "db.sql"
            }
            None => panic!("Could not find a home directory"),
        }
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

#[cfg(test)]
mod tests {
    use crate::{db::{save_new_crl, Crl}, app::TestUtils};
    use core::time;
    use std::thread;

    #[test]
    fn get_crls() {
        TestUtils::cleanup_test_database();
        let texts = vec!["one", "two", "three"];
        for text in texts.iter() {
            let clr = Crl::new(text);
            save_new_crl(&clr).unwrap();
        }
        let crls_from_db = super::get_many(10).unwrap();
        let mut texts_from_db = crls_from_db.iter().map(|crl| -> &str { &crl.crl.text });
        assert!(texts_from_db.all(|item| texts.contains(&item)));
    }

    #[test]
    fn save_a_crl() {
        let text = "Test description";
        let crl = Crl::new(text);
        save_new_crl(&crl).unwrap();
        let crls = super::get_many(10).unwrap();
        assert_eq!(crls.iter().any(|i| i.crl.text == text), true);
    }

    #[test]
    fn save_and_load_crls_from_db() {
        let text = TestUtils::create_rnd_string();
        let text_two = TestUtils::create_rnd_string();
        let crl = Crl::new(&text);
        let crl2 = Crl::new(&text_two);
        save_new_crl(&crl).unwrap();
        save_new_crl(&crl2).unwrap();

        let crls = super::get_many(10).unwrap();
        assert!(&crls.iter().any(|x| x.crl.text == text_two));
    }

    #[test]
    fn get_latest_crl() {
        let text = TestUtils::create_rnd_string();
        let text_two = TestUtils::create_rnd_string();
        let crl = Crl::new(&text);
        let crl2 = Crl::new(&text_two);
        save_new_crl(&crl).unwrap();

        //Wait just about one sec for SQL filter to work properly
        thread::sleep(time::Duration::from_millis(1010));
        save_new_crl(&crl2).unwrap();

        let crl = super::get_latest().unwrap();
        match crl{
            Some(crl) => assert_eq!(crl.crl.text, text_two),
            None => assert!(false)
        }
    }

}
