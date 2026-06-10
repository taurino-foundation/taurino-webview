use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use http::{
    HeaderMap, Method, Request, Response as HttpResponse, StatusCode,
    header::{
        ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
        ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE,
    },
};

use percent_encoding::percent_decode;
use reqwest::Client;

use crate::{
    async_runtime, async_runtime::Mutex, types::FrontendDist,
    utils::ManagerUriSchemeProtocol,
};

const APP_PROTOCOL: &str = "taurino";

type ProtocolResponse = HttpResponse<Cow<'static, [u8]>>;
type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
struct CachedResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

struct Context {
    frontend_dist: Option<FrontendDist>,
    window_origin: String,
    client: Client,
    response_cache: Mutex<HashMap<String, CachedResponse>>,
}

pub fn get(
    frontend_dist: Option<FrontendDist>,
    window_origin: &str,
) -> Arc<ManagerUriSchemeProtocol> {
    let context = Arc::new(Context {
        frontend_dist,
        window_origin: window_origin.to_string(),
        client: Client::new(),
        response_cache: Mutex::new(HashMap::new()),
    });

    Arc::new(ManagerUriSchemeProtocol::new(
        move |_context, request, responder| {
            let context = Arc::clone(&context);

            async_runtime::spawn(async move {
                match get_response(&context, request).await {
                    Ok(response) => responder.respond(response),
                    Err(error) => responder.respond(error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &context.window_origin,
                        &error.to_string(),
                    )),
                }
            });
        },
    ))
}

async fn get_response(
    context: &Context,
    request: Request<Vec<u8>>,
) -> Result<ProtocolResponse, BoxError> {
    if request.method() == Method::OPTIONS {
        return empty_response(StatusCode::NO_CONTENT, &context.window_origin);
    }

    let proxy_dev_server =
        matches!(context.frontend_dist, Some(FrontendDist::Url(_)));

    let path = request_path(&request, proxy_dev_server);

    let builder = HttpResponse::builder()
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, &context.window_origin)
        .header(
            ACCESS_CONTROL_ALLOW_METHODS,
            "GET, POST, PUT, PATCH, DELETE, OPTIONS",
        )
        .header(ACCESS_CONTROL_ALLOW_HEADERS, "*");

    match &context.frontend_dist {
        Some(FrontendDist::Url(url)) => {
            proxy_dev_request(
                &context.client,
                url.as_str(),
                &context.response_cache,
                path,
                builder,
                &request,
            )
            .await
        }

        Some(FrontendDist::Directory(root)) => {
            serve_directory(root, path, builder, &request).await
        }

        Some(FrontendDist::Files(files)) => {
            serve_files(files, path, builder, &request).await
        }

        None => Ok(error_response(
            StatusCode::NOT_FOUND,
            &context.window_origin,
            "No frontend_dist configured",
        )),
    }
}

async fn proxy_dev_request(
    client: &Client,
    base_url: &str,
    response_cache: &Mutex<HashMap<String, CachedResponse>>,
    path: String,
    mut builder: http::response::Builder,
    request: &Request<Vec<u8>>,
) -> Result<ProtocolResponse, BoxError> {
    let decoded_path = percent_decode(path.as_bytes())
        .decode_utf8_lossy()
        .to_string();

    let url = format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        decoded_path.trim_start_matches('/')
    );

    let mut proxy_builder = client.request(request.method().clone(), &url);

    for (name, value) in request.headers() {
        if should_forward_request_header(name.as_str()) {
            proxy_builder = proxy_builder.header(name, value);
        }
    }

    if !request.body().is_empty() {
        proxy_builder = proxy_builder.body(request.body().clone());
    }

    let response = proxy_builder.send().await.map_err(|error| {
        format!("Failed to request dev server URL `{url}`: {error}")
    })?;

    let status = response.status();

    if status == StatusCode::NOT_MODIFIED {
        let cache = response_cache.lock().await;

        if let Some(response) = cache.get(&url).cloned() {
            for (name, value) in &response.headers {
                if should_forward_response_header(name.as_str()) {
                    builder = builder.header(name, value);
                }
            }

            return Ok(builder
                .status(response.status)
                .body(Cow::Owned(response.body))?);
        }
    }

    let headers = response.headers().clone();
    let body = response.bytes().await?.to_vec();

    if request.method() == Method::GET && status.is_success() {
        let mut cache = response_cache.lock().await;

        cache.insert(
            url,
            CachedResponse {
                status,
                headers: headers.clone(),
                body: body.clone(),
            },
        );
    }

    for (name, value) in &headers {
        if should_forward_response_header(name.as_str()) {
            builder = builder.header(name, value);
        }
    }

    Ok(builder.status(status).body(Cow::Owned(body))?)
}

async fn serve_directory(
    root: &Path,
    path: String,
    mut builder: http::response::Builder,
    request: &Request<Vec<u8>>,
) -> Result<ProtocolResponse, BoxError> {
    if request.method() != Method::GET {
        return Ok(builder
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header(CONTENT_TYPE, mime::TEXT_PLAIN.essence_str())
            .body(Cow::Owned(
                b"Only GET is supported for static assets".to_vec(),
            ))?);
    }

    let Some(asset_path) = safe_asset_path(root, &path) else {
        return Ok(builder
            .status(StatusCode::BAD_REQUEST)
            .header(CONTENT_TYPE, mime::TEXT_PLAIN.essence_str())
            .body(Cow::Owned(b"Invalid asset path".to_vec()))?);
    };

    let asset_path = if asset_path.is_dir() {
        asset_path.join("index.html")
    } else {
        asset_path
    };

    let asset_path = if asset_path.exists() {
        asset_path
    } else {
        root.join("index.html")
    };

    let body = tokio::fs::read(&asset_path).await?;

    let mime_type = mime_guess::from_path(&asset_path)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    builder = builder.header(CONTENT_TYPE, mime_type);

    Ok(builder.status(StatusCode::OK).body(Cow::Owned(body))?)
}

async fn serve_files(
    files: &[PathBuf],
    path: String,
    mut builder: http::response::Builder,
    request: &Request<Vec<u8>>,
) -> Result<ProtocolResponse, BoxError> {
    if request.method() != Method::GET {
        return Ok(builder
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header(CONTENT_TYPE, mime::TEXT_PLAIN.essence_str())
            .body(Cow::Owned(
                b"Only GET is supported for static assets".to_vec(),
            ))?);
    }

    let requested = path
        .trim_start_matches('/')
        .split(['?', '#'])
        .next()
        .unwrap_or_default();

    let requested = if requested.is_empty() {
        "index.html"
    } else {
        requested
    };

    let Some(asset_path) = files.iter().find(|file| {
        file.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == requested)
            .unwrap_or(false)
    }) else {
        return Ok(builder
            .status(StatusCode::NOT_FOUND)
            .header(CONTENT_TYPE, mime::TEXT_PLAIN.essence_str())
            .body(Cow::Owned(b"Static file not found".to_vec()))?);
    };

    let body = tokio::fs::read(asset_path).await?;

    let mime_type = mime_guess::from_path(asset_path)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    builder = builder.header(CONTENT_TYPE, mime_type);

    Ok(builder.status(StatusCode::OK).body(Cow::Owned(body))?)
}

fn request_path(request: &Request<Vec<u8>>, proxy_dev_server: bool) -> String {
    let uri = request.uri().to_string();

    let path = if proxy_dev_server {
        uri
    } else {
        uri.split(['?', '#']).next().unwrap_or_default().to_string()
    };

    strip_app_origin(&path)
}

fn strip_app_origin(value: &str) -> String {
    let custom_origin = format!("{APP_PROTOCOL}://localhost");
    let http_origin = format!("http://{APP_PROTOCOL}.localhost");
    let https_origin = format!("https://{APP_PROTOCOL}.localhost");

    value
        .strip_prefix(&custom_origin)
        .or_else(|| value.strip_prefix(&http_origin))
        .or_else(|| value.strip_prefix(&https_origin))
        .unwrap_or(value)
        .to_string()
}

fn safe_asset_path(root: &Path, uri_path: &str) -> Option<PathBuf> {
    let clean_path = uri_path
        .trim_start_matches('/')
        .split(['?', '#'])
        .next()
        .unwrap_or_default();

    let mut output = root.to_path_buf();

    if clean_path.is_empty() {
        output.push("index.html");
        return Some(output);
    }

    for component in Path::new(clean_path).components() {
        match component {
            Component::Normal(part) => output.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }

    Some(output)
}

fn empty_response(
    status: StatusCode,
    window_origin: &str,
) -> Result<ProtocolResponse, BoxError> {
    Ok(HttpResponse::builder()
        .status(status)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, window_origin)
        .header(
            ACCESS_CONTROL_ALLOW_METHODS,
            "GET, POST, PUT, PATCH, DELETE, OPTIONS",
        )
        .header(ACCESS_CONTROL_ALLOW_HEADERS, "*")
        .body(Cow::Owned(Vec::new()))?)
}

fn error_response(
    status: StatusCode,
    window_origin: &str,
    message: &str,
) -> ProtocolResponse {
    HttpResponse::builder()
        .status(status)
        .header(CONTENT_TYPE, mime::TEXT_PLAIN.essence_str())
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, window_origin)
        .body(Cow::Owned(message.as_bytes().to_vec()))
        .unwrap()
}

fn should_forward_request_header(name: &str) -> bool {
    !matches!(
        name.to_ascii_lowercase().as_str(),
        "host"
            | "connection"
            | "content-length"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn should_forward_response_header(name: &str) -> bool {
    !matches!(
        name.to_ascii_lowercase().as_str(),
        "connection" | "content-length" | "transfer-encoding" | "upgrade"
    )
}
