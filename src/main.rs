use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::process::{Command, Stdio};

use axum::{
    middleware::self as AxumMiddleware,
    body,
    http::{HeaderMap, StatusCode},
    Json,
    response::IntoResponse, Router, routing::{get, post},
};
use serde::Deserialize;
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;

mod middleware;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        // ping pong
        .route("/", get(ping))
        // `GET /release` notifies us of a release
        .route(
            "/notify_release",
            post(notify_release).layer(ServiceBuilder::new()
                .map_request_body(body::boxed)
                .layer(AxumMiddleware::from_fn(middleware::verify_github_signature_middleware))
            ),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ping() -> &'static str {
    "Pong!"
}

async fn notify_release(
    headers: HeaderMap,
    Json(payload): Json<PackageWebhook>,
) -> impl IntoResponse {
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|header| header.to_str().ok());

    if event_type.is_none() || matches!(event_type, Some(event_type) if event_type != "package") {
        return (StatusCode::OK, "Irrelevant event. No action needed.").into_response();
    }

    let config: Config = load_config();

    let service = match config.image_to_service.get(&payload.package.name) {
        None => return StatusCode::OK.into_response(),
        Some(service) => service,
    };

    // first login to docker repo
    docker_login(
        env::var("DOCKER_REGISTRY").unwrap_or(String::from("ghcr.io")),
        env::var("DOCKER_USERNAME").unwrap_or_default(),
        env::var("DOCKER_PASSWORD").unwrap_or_default(),
    ).await;

    println!("attempting to deploy image {}", payload.package.name);

    let infra_repo_path = env::var("INFRA_REPO_PATH").unwrap_or(String::from(""));

    // then try to deploy the swarm stack
    let docker_stack_deploy_command = Command::new(get_docker_path())
        .arg("stack")
        .arg("deploy")
        .arg("--with-registry-auth")
        .arg("--compose-file")
        .arg(infra_repo_path + "/" + &service.service_name + "/" + &service.docker_stack_file) // TODO use std::path
        .arg(&service.service_name)
        .output()
        .expect("failed to execute docker stack deploy command");

    println!(": {}", docker_stack_deploy_command.status);
    io::stdout().write_all(&docker_stack_deploy_command.stdout).unwrap();
    io::stderr().write_all(&docker_stack_deploy_command.stderr).unwrap();

    if docker_stack_deploy_command.status.success() {
        StatusCode::OK.into_response()
    } else {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

fn get_docker_path() -> String {
    env::var("DOCKER_PATH").unwrap_or(String::from("/usr/bin/docker"))
}

async fn docker_login(registry: String, username: String, password: String) {
    let mut docker_repo_login_command = Command::new(get_docker_path())
        .arg("login")
        .arg(registry)
        .arg("-u")
        .arg(username)
        .arg("--password-stdin")
        .stdin(Stdio::piped())
        .spawn()
        .expect("failed to execute docker login command");
    docker_repo_login_command
        .stdin
        .take()
        .unwrap()
        .write_all(password.as_bytes())
        .unwrap();

    let result = docker_repo_login_command.wait_with_output().unwrap();

    io::stdout().write_all(&result.stdout).unwrap();
    io::stderr().write_all(&result.stderr).unwrap();

    if !result.status.success() {
        panic!("failed to log into Docker repo")
    }
}

fn load_config() -> Config {
    let file = File::open("config.json").unwrap();
    serde_json::from_reader(file).unwrap()
}

// types

#[derive(Deserialize)]
struct Config {
    image_to_service: HashMap<String, Service>,
}

#[derive(Deserialize)]
struct Service {
    service_name: String,
    docker_stack_file: String,
}

#[derive(Deserialize)]
struct PackageWebhook {
    action: String,
    package: Package,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    namespace: String,
    ecosystem: String,
    package_type: String,
    created_at: String,
    updated_at: String,
    package_version: PackageVersion,
}

#[derive(Deserialize)]
struct PackageVersion {
    id: u32,
    version: String,
    name: String,
}
