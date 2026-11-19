
use std::error::Error;

fn keyring_entry() -> Result<keyring::Entry, Box<dyn Error>> {
    let username = whoami::username();
    Ok(keyring::Entry::new("otp-app", &username)?)
}


pub fn retrieve_password() -> Result<String, Box<dyn Error>> {
    Ok(keyring_entry()?.get_password()?)
}

pub fn store_password(password: &str) -> Result<(), Box<dyn Error>> {
    keyring_entry()?.set_password(password)?;
    // println!("Password saved to the otp-app keyring entry for the current user.");
    Ok(())
}
