use crate::app::{ActionResponse, ActionResponseType};

pub fn display_action_response(res: &ActionResponse) {
    if res._type == ActionResponseType::Error {
        print!("❌ ");
    }
    if res._type == ActionResponseType::Success {
        print!("👍 ");
    }
    if res.message.len() > 0 {
        println!("{}", res.message);
    }
    if res._type == ActionResponseType::Content {
        match &res.crls {
            Some(crls) => {
                for (index, crl) in crls.iter().enumerate() {
                    println!("{} {}", index, crl.crl.text);
                }
            },
            None => {}
        }
    }
}
