mod db;
mod app;
mod bash_driver;

use bash_driver::display_action_response;

fn main() {
    let action = arw_brr::get_argument_at(0).unwrap();
    let action = app::Action::from_string(&action);
    let argument = arw_brr::get_argument_at(1);

    let mut session = app::Session::new(); 
    session.run(action, argument);

    for res in session.action_responses.iter(){
        display_action_response(res);
    }
}
