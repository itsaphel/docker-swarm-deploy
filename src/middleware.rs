use std::env;
use axum::{
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    middleware::{self, Next},
    body::{self, BoxBody, Bytes, Full}
};
use axum::http::HeaderMap;
use sha2::Sha256;
use hmac::{Hmac, Mac};
use crate::middleware::VerificationResult::{Failed, Successful};

enum VerificationResult {
    Successful,
    Failed
}

pub async fn verify_github_signature_middleware(
    request: Request<BoxBody>,
    next: Next<BoxBody>,
) -> Result<impl IntoResponse, Response> {
    let request = buffer_request_body(request).await?;

    Ok(next.run(request).await)
}

async fn buffer_request_body(request: Request<BoxBody>) -> Result<Request<BoxBody>, Response> {
    let (parts, body) = request.into_parts();

    // this wont work if the body is an long running stream
    let bytes = hyper::body::to_bytes(body)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    let verification_result = verify_github_signature(&parts.headers, bytes.clone()).await;

    match verification_result {
        Successful => Ok(Request::from_parts(parts, body::boxed(Full::from(bytes)))),
        Failed => Err((StatusCode::UNAUTHORIZED, "invalid GitHub signature in request").into_response()),
    }
}

async fn verify_github_signature(headers: &HeaderMap, payload: Bytes) -> VerificationResult {
    let our_signature = env::var("GITHUB_WEBHOOK_SECRET").unwrap();
    let request_signature = headers
        .get("x-hub-signature-256")
        .and_then(|header| header.to_str().ok());

    match request_signature {
        Some(request_signature) if verify_signed_payload(payload, &our_signature, request_signature).await => {
            Successful
        }
        _ => Failed
    }
}

async fn verify_signed_payload(payload: Bytes, our_secret: &str, their_signature: &str) -> bool {
    let mut digest = Hmac::<Sha256>::new_from_slice(our_secret.as_bytes()).expect("HMAC can take key of any size");
    digest.update(&payload);

    let signature = digest.finalize();
    let signature = "sha256=".to_string() + hex::encode(signature.into_bytes()).as_str();

    signature == their_signature
}