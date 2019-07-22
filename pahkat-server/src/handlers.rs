use std::fs::{self, File};
use std::io::prelude::*;
use std::io::BufReader;

use actix_multipart::Multipart;
use actix_web::{http::header::AUTHORIZATION, web, HttpRequest, HttpResponse, Responder};
use chrono::offset::Utc;
use chrono::Duration;
use form_data::{handle_multipart, Error};
use futures::future::{ok, Future};
use log::{error, info};
use serde::Deserialize;
use serde_json::json;

use pahkat_common::database::models::NewDownload;
use pahkat_common::open_package;
use pahkat_types::{Downloadable, Installer};

use crate::ServerState;

fn read_file(path: &str) -> std::io::Result<String> {
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;

    Ok(contents)
}

pub fn repo_index(state: web::Data<ServerState>) -> impl Responder {
    let mut repo_index_path = state.path.clone();

    repo_index_path.push("index.json");

    match read_file(
        repo_index_path
            .to_str()
            .expect("Cannot convert path to string"),
    ) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .body(body),
        Err(e) => {
            error!("Error while reading repo index file: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub fn packages_index(state: web::Data<ServerState>) -> impl Responder {
    let mut packages_index_path = state.path.clone();

    packages_index_path.push("packages");
    packages_index_path.push("index.json");

    match read_file(
        packages_index_path
            .to_str()
            .expect("Cannot convert path to string"),
    ) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .body(body),
        Err(e) => {
            error!("Error while reading packages index file: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub fn packages_package_index(
    state: web::Data<ServerState>,
    path: web::Path<String>,
) -> impl Responder {
    let package_id = path.clone();

    let mut packages_package_index_path = state.path.clone();

    packages_package_index_path.push("packages");
    packages_package_index_path.push(package_id);
    packages_package_index_path.push("index.json");
    let index_path_str = packages_package_index_path
        .to_str()
        .expect("Cannot convert path to string");

    match read_file(index_path_str) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .body(body),
        Err(e) => {
            error!(
                "Error while reading packages package index {}: {:?}",
                index_path_str, e
            );
            HttpResponse::NotFound().finish()
        }
    }
}

#[derive(Deserialize)]
struct UploadParams {
    pub channel: String,
    pub version: String,
    pub installer: Installer,
}

pub fn upload_package(
    request: HttpRequest,
    state: web::Data<ServerState>,
    multipart: Multipart,
) -> Box<dyn Future<Item = HttpResponse, Error = Error>> {
    let auth_header = request.headers().get(AUTHORIZATION);
    if auth_header == None {
        // TODO: Potentially use Forbidden, otherwise need to add auth header response
        return Box::new(ok(HttpResponse::Unauthorized().finish()));
    }

    let auth_header = auth_header.unwrap();

    let database = &state.database;

    let str_header = auth_header.to_str().unwrap();

    info!("Str header: {}", str_header);

    let split_vec: Vec<_> = str_header.split(' ').collect();
    if split_vec.len() != 2 || split_vec.get(0).unwrap() != &"Bearer" {
        info!("split vec: {:?}", split_vec);
        return Box::new(ok(HttpResponse::Unauthorized().finish()));
    }

    let result = database.validate_token(split_vec.get(1).unwrap());
    info!("db result: {:?}", &result);
    if !result.is_ok() && result.unwrap() == false {
        return Box::new(ok(HttpResponse::Unauthorized().finish()));
    }

    let ref_state = state.get_ref().clone();
    let mut destination_dir = ref_state.config.artifacts_dir.clone();
    let form = ref_state.upload_form;

    info!("HttpRequest: {:?}", request);

    Box::new(handle_multipart(multipart, form).map(|uploaded_content| {
        println!("execute");
        let mut map = uploaded_content.map().unwrap();
        let params = map.remove("params").unwrap().text().unwrap();

        let upload_params: Result<UploadParams, _> = serde_json::from_str(&params);
        match upload_params {
            Err(e) => {
                return HttpResponse::BadRequest().body(format!("Error processing params: {}", e));
            }
            Ok(upload_params) => {
                let (filename, path) = map.remove("payload").unwrap().file().unwrap();

                info!("text: {}", params);
                info!(
                    "filename: {}, path: {:?}",
                    filename,
                    path.as_path().display()
                );

                let copy_error = format!(
                    "failed to copy temp file {:?} to artifacts dir {:?}",
                    &path, &destination_dir
                );

                destination_dir.push(filename);
                fs::copy(&path, destination_dir).expect(&copy_error);

                // Tarball is not supported
                match upload_params.installer {
                    Installer::Tarball(_) => return HttpResponse::BadRequest().finish(),
                    installer => {
                        let url = installer.url();

                        info!("installer url: {}", url);
                    }
                };

                //info!("text: {:?}, file: {:?}", &text, &file);
                HttpResponse::Created().finish()
            }
        }
    }))
}

pub fn download_package(state: web::Data<ServerState>, path: web::Path<String>) -> impl Responder {
    let package_id = path.clone();

    let mut package_index_path = state.path.clone();
    package_index_path.push("packages");
    package_index_path.push(package_id);

    let package = match open_package(package_index_path.as_path(), None) {
        Ok(package) => package,
        Err(_) => {
            return HttpResponse::NotFound()
                .content_type("application/json")
                .json(json!({ "message": "Package not found." }))
        }
    };

    let installer = match package.installer {
        Some(installer) => installer,
        _ => {
            return HttpResponse::NotFound()
                .content_type("application/json")
                .json(json!({ "message": "No installer found for this package." }))
        }
    };

    let url = installer.url();

    let _count = state.database.create_download(NewDownload {
        package_id: package.id,
        package_version: package.version,
        timestamp: Utc::now().naive_utc(),
    });

    HttpResponse::Found().header("Location", url).finish()
}

pub fn package_stats(
    state: web::Data<ServerState>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::error::Error> {
    let database = &state.database;

    let package_id = path.clone();

    let mut package_index_path = state.path.clone();
    package_index_path.push("packages");
    package_index_path.push(&package_id);

    let package = match open_package(package_index_path.as_path(), None) {
        Ok(package) => package,
        Err(_) => {
            return Ok(HttpResponse::NotFound()
                .content_type("application/json")
                .json(json!({ "message": "Package not found." })))
        }
    };

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(json!({
            "today": database.query_package_download_count_since(&package, Duration::days(1))?,
            "lastWeek": database.query_package_download_count_since(&package, Duration::days(7))?,
            "last30Days": database.query_package_download_count_since(&package, Duration::days(30))?,
            "thisVersion": database.query_package_version_download_count(&package)?,
            "allTime": database.query_package_download_count(&package)? })))
}

pub fn repo_stats(state: web::Data<ServerState>) -> Result<HttpResponse, actix_web::error::Error> {
    let limit = 5;
    let days = 30;

    let downloads_since: Vec<serde_json::Value> = (&state
        .database
        .query_top_downloads_since(limit, Duration::days(days))?)
        .iter()
        .map(|download| json!({download.package_id.clone(): &download.count}))
        .collect();

    let downloads_all: Vec<serde_json::Value> = (&state.database.query_top_downloads(limit)?)
        .iter()
        .map(|download| json!({download.package_id.clone(): &download.count}))
        .collect();

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(json!({
            "packagesToday": &state.database.query_distinct_downloads_since(Duration::days(1))?,
            format!("top{}Packages{}Days", limit, days): downloads_since,
            format!("top{}PackagesAllTime", limit): downloads_all,
        })))
}

pub fn virtuals_index(state: web::Data<ServerState>) -> impl Responder {
    let mut virtuals_index_path = state.path.clone();

    virtuals_index_path.push("virtuals");
    virtuals_index_path.push("index.json");

    match read_file(
        virtuals_index_path
            .to_str()
            .expect("Cannot convert path to string"),
    ) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .body(body),
        Err(e) => {
            error!("Error while reading virtuals index file: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub fn virtuals_package_index(
    state: web::Data<ServerState>,
    path: web::Path<String>,
) -> impl Responder {
    let package_id = path.clone();

    let mut virtuals_package_index_path = state.path.clone();

    virtuals_package_index_path.push("virtuals");
    virtuals_package_index_path.push(package_id);
    virtuals_package_index_path.push("index.json");
    let index_path_str = virtuals_package_index_path
        .to_str()
        .expect("Cannot convert path to string");

    match read_file(index_path_str) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .body(body),
        Err(e) => {
            error!(
                "Error while reading virtuals package index {}: {:?}",
                index_path_str, e
            );
            HttpResponse::NotFound().finish()
        }
    }
}
