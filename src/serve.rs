use crate::block_list::build_deny_ips;
use crate::models::{Dump, DumpDetails, File};
use crate::util::calculate_expires;
use crate::{models::Url, opts::ServeArgs};
use chrono::TimeDelta;
use humantime::parse_duration;
use poem::error::{Forbidden, InsufficientStorage, NotFoundError};
use poem::http::header;
use poem::middleware::TowerLayerCompatExt;
use poem::{
    error::{BadRequest, InternalServerError, PayloadTooLarge},
    get, handler,
    listener::TcpListener,
    middleware::AddData,
    post,
    web::{Data, Multipart, Path},
    Body, EndpointExt, Response, Result, Route, Server,
};
use std::{error::Error, fmt::Display, sync::Arc};
use tower::limit::RateLimitLayer;
#[derive(Debug, Clone)]
pub struct DumpError {
    message: String,
}

impl DumpError {
    pub fn new(message: String) -> DumpError {
        DumpError {
            message: message + "\n",
        }
    }
    fn too_large(max_size: usize, my_size: usize) -> DumpError {
        DumpError::new(format!(
            "File of size {} larger than the maximum of {}",
            my_size, max_size
        ))
    }
}

impl Display for DumpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for DumpError {
    fn description(&self) -> &str {
        &self.message
    }
}

async fn dump_parse_multipart(mut multipart: Multipart, state: Arc<ServeArgs>) -> Result<Dump> {
    let mut file_name: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut secret: Option<String> = None;
    let mut passed_expires: Option<TimeDelta> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field
            .name()
            .map(ToString::to_string)
            .ok_or(BadRequest(DumpError::new(
                "Could not read field name".to_string(),
            )))?;
        if name == "file" {
            file_name = Some(field.file_name().map(ToString::to_string).ok_or(
                poem::error::BadRequest(DumpError::new("Could not read file name".to_string())),
            )?);
            let bytes = match field.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => return Err(e.into()),
            };
            if bytes.len() > state.max_size {
                return Err(PayloadTooLarge(DumpError::too_large(
                    state.max_size,
                    bytes.len(),
                )));
            }
            file_bytes = Some(bytes);
        } else if name == "secret" {
            let secret_bytes = match field.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => return Err(e.into()),
            };
            secret = Some(
                std::str::from_utf8(&secret_bytes)
                    .map_err(|_e| BadRequest(DumpError::new("Could not parse secret".to_string())))?
                    .to_string(),
            );
        } else if name == "expires" {
            let expires_bytes = match field.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => return Err(e.into()),
            };
            let expires_string = std::str::from_utf8(&expires_bytes)
                .map_err(|_e| BadRequest(DumpError::new("Could not parse expires".to_string())))?
                .to_string();
            let expires_duration = parse_duration(&expires_string)
                .map_err(|_e| BadRequest(DumpError::new("Could not parse expires".to_string())))?;
            passed_expires =
                Some(TimeDelta::from_std(expires_duration).map_err(|_e| {
                    BadRequest(DumpError::new("Could not parse expires".to_string()))
                })?);
        }
    }
    if file_name.is_none() || file_bytes.is_none() {
        return Err(BadRequest(DumpError::new(
            "Missing file and file name".to_string(),
        )));
    }
    let file_name = file_name.unwrap();
    let file_bytes = file_bytes.unwrap();
    let mut expires = calculate_expires(
        file_bytes.len(),
        state.min_expires,
        state.max_expires,
        state.max_size,
    );
    if let Some(passed_expires) = passed_expires {
        if passed_expires <= TimeDelta::zero() {
            return Err(BadRequest(DumpError::new(
                "The expires duration must be larger than 0".to_string(),
            )));
        }
        expires = expires.min(passed_expires);
    }
    Ok(Dump {
        details: DumpDetails {
            file_name,
            secret,
            expires,
        },
        file_bytes,
    })
}

#[handler]
async fn dump_file(multipart: Multipart, state: Data<&Arc<ServeArgs>>) -> Result<String> {
    let dump = dump_parse_multipart(multipart, state.clone()).await?;
    let file = File::from_dump(&dump, &state.data_directory);
    let connection = state
        .create_connection()
        .map_err(|e| InternalServerError(e))?;
    let found_file =
        File::search_file_by_hash(&connection, &file.hash).map_err(|x| InternalServerError(x))?;
    if found_file.is_none() {
        let size_sum = File::size_sum(&connection).map_err(|x| InternalServerError(x))?;
        if size_sum + dump.file_bytes.len() > state.disk_quota {
            return Err(InsufficientStorage(DumpError::new(
                "The quota has been exceeded".to_string(),
            )));
        }
        if state.blocked_groups.contains(&file.group) {
            return Err(Forbidden(DumpError::new(
                "This type of file is not allowed".to_string(),
            )));
        }
        file.create(&connection)
            .map_err(|x| InternalServerError(x))?;
        file.write(state.data_directory.clone(), dump.file_bytes)
            .map_err(|x| InternalServerError(x))?;
    }
    let url = Url::from_dump_details_and_file(&dump.details, &file);
    // TODO: fix duplicate tokens
    url.create(&connection)
        .map_err(|x| InternalServerError(x))?;
    // if search_result
    let mut access_url = state.url.clone();
    if !access_url.ends_with('/') {
        access_url.push('/');
    }
    access_url.push_str(&url.token);
    access_url.push('\n');
    Ok(access_url)
}

#[handler]
async fn get_file(Path(token): Path<String>, state: Data<&Arc<ServeArgs>>) -> Result<Response> {
    let connection = state
        .create_connection()
        .map_err(|e| InternalServerError(e))?;
    let url = Url::search_url_by_token(&connection, &token).map_err(|x| InternalServerError(x))?;
    if url.is_none() {
        return Err(NotFoundError {}.into());
    }
    let url = url.unwrap();
    if url.expired() {
        return Err(NotFoundError {}.into());
    }
    let file = url.file(&connection).map_err(|x| InternalServerError(x))?;
    let bytes = file
        .read(state.data_directory.clone())
        .map_err(|x| InternalServerError(x))?;
    // TODO: detection
    let body = Body::from_vec(bytes);
    let mut builder = Response::builder()
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, file.size as u64)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", url.file_name),
        )
        .header("X-Expires", url.expires.to_string())
        .content_type(file.mime);
    Ok(builder.body(body))
}

fn ensure_model_files(data_directory: &std::path::Path) {
    let model_directory = data_directory.join("model");
    if !model_directory.exists() {
        std::fs::create_dir_all(&model_directory).expect("Could not create model directory");
    }
    let model_file = model_directory.join("model.onnx");
    let model_config_file = model_directory.join("model_config.json");
    if !model_file.exists() {
        std::fs::write(&model_file, include_bytes!("../assets/model.onnx"))
            .expect("Could not write model file");
    }
    if !model_config_file.exists() {
        std::fs::write(
            &model_config_file,
            include_str!("../assets/model_config.json"),
        )
        .expect("Could not write model config file");
    }
}

macro_rules! create_rate_limit_layer {
    ($rate_limit_count:expr, $rate_limit_duration:expr) => {
        TowerLayerCompatExt::compat(RateLimitLayer::new($rate_limit_count, $rate_limit_duration))
    };
}

pub async fn serve(args: ServeArgs) {
    std::fs::create_dir_all(&args.data_directory.join("files"))
        .expect("Could not create files directory");
    ensure_model_files(&args.data_directory);
    let rate_limit_count = args.rate_limit_count;
    let rate_limit_duration = args.rate_limit_duration;
    let deny_ips_layer = build_deny_ips(&args);

    let address = args.address.clone();
    // let db_path = args.data_directory.join("db.sqlite3");
    let data = Arc::new(args);
    let app = Route::new()
        .at(
            "/",
            post(dump_file).with(create_rate_limit_layer!(
                rate_limit_count,
                rate_limit_duration
            )),
        )
        .at(
            "/:token",
            get(get_file).with(create_rate_limit_layer!(
                rate_limit_count,
                rate_limit_duration
            )),
        )
        .with(deny_ips_layer)
        .with(AddData::new(data));
    let _ = Server::new(TcpListener::bind(address))
        .name("dump")
        .run(app)
        .await;
}
