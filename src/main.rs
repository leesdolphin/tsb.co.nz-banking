extern crate cookie;
extern crate failure;
#[macro_use]
extern crate html5ever;
extern crate reqwest;
extern crate url;
#[macro_use]
extern crate failure_derive;

use failure::Error;

mod forms;
mod login;
use login::TsbContainer;

fn main() -> Result<(), Error> {
  let mut container = TsbContainer::load_creds()?;
  let seq = container.do_login()?;

  println!("{}, {}", seq.next_sequence_id, seq.customer_number);

  Ok(())
}
