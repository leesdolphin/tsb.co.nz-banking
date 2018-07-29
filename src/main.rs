extern crate hyper;
extern crate url;

use hyper::header;
use hyper::rt::{self, Future, Stream};
use hyper::{Body, Client, Method, Request};
use std::fs::File;
use std::io::{self, Read, Write};
use url::form_urlencoded;

const BASE_URL: &'static str = "https://homebank.tsbbank.co.nz/online/";

fn build_request<T>(mut r: Request<T>) -> Request<T> {
    r.headers_mut().insert(
        header::USER_AGENT,
        header::HeaderValue::from_str("TSB bank automated access").unwrap(),
    );
    r
}

fn load_creds() -> std::io::Result<(String, String)> {
    let mut file = File::open("creds.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let mut lines = contents.lines();
    match (lines.next(), lines.next()) {
        (Some(user), Some(pass)) => Ok((user.to_owned(), pass.to_owned())),
        (_, _) => panic!("No!"),
    }
}

fn main() {
    let client = Client::new();
    let (user, pass) = load_creds().unwrap();

    let encoded: String = form_urlencoded::Serializer::new(String::new())
        .append_pair("card", &user)
        .append_pair("password", &pass)
        .append_pair("op", "signon")
        .append_pair("isPoli", "")
        .finish();
    let uri: hyper::Uri = BASE_URL.parse().unwrap();
    let mut req = Request::new(Body::from(encoded));
    *req.method_mut() = Method::POST;
    *req.uri_mut() = uri.clone();
    // req.headers_mut().insert("content-type", HeaderValue::from_str
}
