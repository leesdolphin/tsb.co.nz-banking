use forms::find_forms;
use forms::get_attr;
use std::fs::File;
use std::io::{Error as IoError, Read, Write};

use cookie::{Cookie, CookieJar};

use reqwest::{header, Client, Error as ReqwestError, RequestBuilder, Response};

use html5ever::rcdom::Handle;

use forms::{find_inputs, get_node_name, parse_dom, FormElement, IterNodes};

const BASE_URL: &'static str = "https://homebank.tsbbank.co.nz/online/";
const COOKIE_BASE: &'static str = "tsbbank.co.nz";
static mut debug_count: u8 = 0;

#[derive(Fail, Debug)]
pub enum MissingUsernamePassword {
  #[fail(display = "Cannot get username/password. {}", _0)]
  FileIoError(IoError),
  #[fail(display = "Cannot get username/password. File's content invalid")]
  FileFormatError,
}

#[derive(Fail, Debug)]
pub enum UnableToLogin {
  #[fail(display = "TSB rejected the credentials: {}", _0)]
  BadCredentials(String),
  #[fail(display = "Error contacting TSB: {}", _0)]
  ReqwestError(ReqwestError),
  #[fail(display = "TSB returned an invalid Document: {}", _0)]
  InvalidContent(IoError),
  #[fail(display = "TSB returned an unsuported DOM")]
  InvalidDom(),
  #[fail(display = "Missing a sequence id needed to continue.")]
  MissingSequenceID(),
  #[fail(display = "Missing a customer number needed to continue.")]
  MissingCustomerNumber(),
}

pub struct TsbContainer {
  user: String,
  pass: String,
  client: Client,
  jar: CookieJar,
}

pub struct TsbLoggedInUser<'a> {
  c: &'a mut TsbContainer,
  next_sequence_id: String,
  customer_number: String,
}

fn debug_request(text: &String, name: &str) {
  // let mut file = File::create(format!("text-{}-{}.txt", name, debug_count)).unwrap();
  // debug_count += 1;
  // write!(file, "{}", text).unwrap();
}

impl TsbContainer {
  pub fn load_creds() -> Result<Self, MissingUsernamePassword> {
    let mut file = File::open("creds.txt").map_err(|e| MissingUsernamePassword::FileIoError(e))?;
    let mut contents = String::new();
    file
      .read_to_string(&mut contents)
      .map_err(|e| MissingUsernamePassword::FileIoError(e))?;
    let mut lines = contents.lines();
    match (lines.next(), lines.next()) {
      (Some(user), Some(pass)) => Ok(TsbContainer {
        user: user.to_owned(),
        pass: pass.to_owned(),
        client: Client::new(),
        jar: CookieJar::new(),
      }),
      (_, _) => Err(MissingUsernamePassword::FileFormatError),
    }
  }

  pub fn update_cookies(&mut self, response: &Response) {
    (&response)
      .headers()
      .get::<header::SetCookie>()
      .iter()
      .flat_map(|x| x.iter())
      .filter_map(|cookie| Cookie::parse(cookie.clone()).ok())
      .for_each(|c| self.jar.add(c));
  }
  pub fn get_cookies(&self) -> Vec<(String, String)> {
    self
      .jar
      .iter()
      .filter(|cookie| cookie.secure().unwrap_or(true))
      .filter(|cookie| {
        cookie
          .domain()
          .unwrap_or(COOKIE_BASE)
          .ends_with(COOKIE_BASE)
      }).map(|cookie| (cookie.name().to_owned(), cookie.value().to_owned()))
      .collect()
  }

  pub fn get_document<
    F: FnOnce(&Client) -> RequestBuilder,
    G: FnOnce(&mut RequestBuilder) -> &mut RequestBuilder,
  >(
    &mut self,
    req_b: F,
    req_c: G,
  ) -> Result<String, ReqwestError> {
    let mut cookie_header = header::Cookie::new();
    let cookies = self.get_cookies();
    cookies
      .iter()
      .for_each(|(n, v)| cookie_header.set(n.clone(), v.clone()));

    let mut rb = req_b(&self.client);
    rb.header(cookie_header);
    req_c(&mut rb);
    let mut res = rb.send()?.error_for_status()?;
    self.update_cookies(&res);
    res.text()
  }

  pub fn get_home(&mut self) -> Result<Handle, UnableToLogin> {
    let text = self
      .get_document(|c| c.get(BASE_URL), |r| r)
      .map_err(|e| UnableToLogin::ReqwestError(e))?;
    debug_request(&text, "home");
    Ok(parse_dom(text).map_err(|e| UnableToLogin::InvalidContent(e))?)
  }

  pub fn do_login(&mut self) -> Result<TsbLoggedInUser, UnableToLogin> {
    let dom = self.get_home()?;

    let params = find_forms(&dom)
      .iter()
      .filter(|form| match get_attr(form, "id") {
        Some(id) => match id.as_str() {
          "signonForm" => true,
          _ => false,
        },
        _ => false,
      }).map(|form| {
        find_inputs(form)
          .iter()
          .filter_map(|elm| match elm {
            FormElement::Input {
              name: Some(name),
              value,
              ..
            } => Some((
              name.to_string(),
              match value {
                Some(v) => v.to_owned(),
                None => "".to_owned(),
              },
            )),
            _ => None,
          }).map(|(n, v)| match n.as_str() {
            "card" => (n, self.user.to_owned()),
            "password" => (n, self.pass.to_owned()),
            _ => (n, v),
          }).collect::<Vec<(String, String)>>()
      }).next()
      .ok_or_else(|| UnableToLogin::InvalidDom())?;
    let text = self
      .get_document(|c| c.post(BASE_URL), |r| r.form(&params))
      .map_err(|e| UnableToLogin::ReqwestError(e))?;

    let dom = self.get_home()?;
    let next_sequence_id = find_next_sequence_id(&dom)?;
    let customer_number = find_customer_number(&dom)?;

    Ok(TsbLoggedInUser {
      c: self,
      next_sequence_id: next_sequence_id,
      customer_number: customer_number,
    })
  }
}

fn find_next_sequence_id(doc: &Handle) -> Result<String, UnableToLogin> {
  find_inputs(doc)
    .iter()
    .filter_map(|elm| {
      match elm {
        FormElement::Input { name, id, value } => {
          if name == &Some("nextSequenceID".to_owned()) || id == &Some("nextSequenceID".to_owned())
          {
            return value.clone();
          }
        }
      }
      None
    }).next()
    .ok_or(UnableToLogin::MissingSequenceID())
    .into()
}
fn find_customer_number(doc: &Handle) -> Result<String, UnableToLogin> {
  IterNodes::from(doc)
    .filter(|elm| match get_node_name(elm) {
      Some(name) => name == "dashboard",
      _ => false,
    }).filter_map(|elm| get_attr(elm, "customer-number"))
    .next()
    .ok_or(UnableToLogin::MissingCustomerNumber())
    .into()
}
