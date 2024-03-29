#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket_contrib;
extern crate rocket;
extern crate rand;
extern crate serde_json;
extern crate image;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;

use image::GenericImage;

use rocket::{Request, Data};
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

#[error(404)]
fn not_found(req: &Request) -> Template {
    let mut map = std::collections::HashMap::new();
    map.insert("path", req.uri().as_str());
    Template::render("error/404", &map)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, upload, play, static_files, favicon])
        .catch(errors![not_found])
        .launch();
}
