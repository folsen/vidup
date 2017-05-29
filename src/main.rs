#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate regex;
extern crate rocket_contrib;
extern crate rocket;
extern crate rand;
extern crate serde_json;
extern crate image;

use regex::Regex;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;
use std::fs::File;

use image::GenericImage;

use rocket::{Request, Route, Data};
use rocket::handler::Outcome;
use rocket::http::Method::*;
use rocket::response::{Response};
use rocket_contrib::Template;

use rocket::response::{NamedFile, Failure};
use rocket::http::{Status};
use rand::{Rng};

#[get("/assets/<file..>")]
fn static_files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}

#[get("/favicon.ico")]
fn favicon() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/favicon.ico")).ok()
}

#[get("/")]
fn index() -> Template {
    let context: HashMap<String,String> = HashMap::new();
    Template::render("index", &context)
}

#[post("/upload", data = "<data>")]
fn upload(data: Data) -> Result<String, Failure> {
    let s = rand::thread_rng().gen_ascii_chars().take(10).collect::<String>();
    data.stream_to_file(format!("tmp/{}", s)).unwrap();
    let output = Command::new("ffmpeg")
                         .arg("-i")
                         .arg(format!("tmp/{}", s))
                         .arg("-vcodec")
                         .arg("libx264")
                         .arg("-vprofile")
                         .arg("high")
                         .arg("-vf")
                         .arg("scale=-1:720")
                         .arg("-threads")
                         .arg("0")
                         .arg("-strict")
                         .arg("-2")
                         .arg("-b:a")
                         .arg("128k")
                         .arg(format!("files/{}.mp4", s))
                         .output();
    Command::new("ffmpeg")
        .arg("-i")
        .arg(format!("files/{}.mp4", s))
        .arg("-vframes")
        .arg("1")
        .arg("-q:v")
        .arg("2")
        .arg(format!("files/{}.jpg", s))
        .output()
        .unwrap();
    match output {
        Ok(out) =>  {
            if out.status.success() {
                Ok(s)
            } else {
                Err(Failure(Status::NotAcceptable))
            }
        },
        Err(_) => Err(Failure(Status::InternalServerError))
    }
}

#[get("/<id>")]
fn play(id: String) -> Template {
    let mut context = HashMap::new();
    let img = image::open(&Path::new(&format!("files/{}.jpg", id))).unwrap();
    let dim = img.dimensions();
    context.insert("video", id);
    context.insert("width", dim.0.to_string());
    context.insert("height", dim.1.to_string());
    Template::render("video", &context)
}

fn video<'a>(req: &'a Request, _: Data) -> Outcome<'a> {
    let filename = format!("files/{}", req.get_param(0).unwrap_or("unnamed"));
    let mut f = File::open(&filename).unwrap();
    let meta = f.metadata().unwrap();
    let headers = req.headers();
    let mut resp = Response::new();
    resp.set_raw_header("Accept-Ranges", "bytes");
    resp.set_raw_header("Content-Range", format!("bytes 0-{}/{}", meta.len() - 1, meta.len()));
    if Regex::new(r"^.*\.mp4$").unwrap().is_match(&filename) {
        resp.set_raw_header("Content-Type", "video/mp4");
    } else if Regex::new(r"^.*\.jpg$").unwrap().is_match(&filename) {
        resp.set_raw_header("Content-Type", "image/jpg");
    }
    if let Some(val) = headers.get("Range").next() {
        if let Some(range) = parse_range(val) {
            f.seek(SeekFrom::Start(range.from)).unwrap();
            if let Some(end) = range.to {
                resp.set_streamed_body(f.take(end));
                resp.set_raw_header("Content-Range", format!("bytes {}-{}/{}", range.from, end, meta.len()));
            } else {
                resp.set_streamed_body(f);
                resp.set_raw_header("Content-Range", format!("bytes {}-{}/{}", range.from, meta.len() - 1, meta.len()));
            }
        }
    } else {
        resp.set_streamed_body(f);
    }
    resp.set_status(Status::PartialContent);

    rocket::Outcome::Success(resp)
}

#[derive(Debug, PartialEq, Eq)]
struct Range {
    from: u64,
    to: Option<u64>
}

fn parse_range(arg: &str) -> Option<Range> {
    let re = Regex::new(r"bytes=(\d+)-(\d*)").unwrap();
    if re.is_match(arg) {
        let caps = re.captures(arg).unwrap();
        Some(Range{
            from: caps.get(1).unwrap().as_str().parse().unwrap(),
            to: caps.get(2).unwrap().as_str().parse().ok()
        })
    } else {
        None
    }
}

#[test]
fn test_parse_range() {
    assert_eq!(parse_range(""), None);
    assert_eq!(parse_range("bytes=0-").unwrap(), Range{from: 0, to: None});
    assert_eq!(parse_range("bytes=1048576-").unwrap(), Range{from: 1048576, to: None});
    assert_eq!(parse_range("bytes=1048576-7048576").unwrap(), Range{from: 1048576, to: Some(7048576)});
    assert_eq!(parse_range("bytes=ab-7048576"), None);
    assert_eq!(parse_range("bytes=-7048576"), None);
}

#[error(404)]
fn not_found(req: &Request) -> Template {
    let mut map = std::collections::HashMap::new();
    map.insert("path", req.uri().as_str());
    Template::render("error/404", &map)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, upload, play, static_files, favicon])
        .mount("/files", vec![Route::new(Get, "/<filename>", video)])
        .catch(errors![not_found])
        .launch();
}
