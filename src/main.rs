#[macro_use]
extern crate validator_derive;
use bytes::buf::{Buf, BufExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::error::Error as StdError;
use thiserror::Error;
use validator::{Validate, ValidationErrors, ValidationErrorsKind};
use warp::{http::StatusCode, reject, Filter, Rejection, Reply};

type Result<T> = std::result::Result<T, Rejection>;

#[derive(Deserialize, Debug, Validate)]
struct CreateRequest {
    #[validate(email)]
    pub email: String,
    #[validate]
    pub address: Address,
    #[validate]
    pub pets: Vec<Pet>,
}

#[derive(Deserialize, Debug, Validate)]
struct Address {
    #[validate(length(min = 2, max = 10))]
    pub street: String,
    #[validate(range(min = 1))]
    pub street_no: usize,
}

#[derive(Deserialize, Serialize, Debug, Validate)]
struct Pet {
    #[validate(length(min = 3, max = 20))]
    pub name: String,
}

#[tokio::main]
async fn main() {
    let basic = warp::path!("create-basic")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(create_handler);

    let basic_path = warp::path!("create-path")
        .and(warp::post())
        .and(warp::body::aggregate())
        .and_then(create_handler_path);

    let basic_path_validator = warp::path!("create-validator")
        .and(warp::post())
        .and(warp::body::aggregate())
        .and_then(create_handler_validator);

    let routes = basic
        .or(basic_path)
        .or(basic_path_validator)
        .recover(handle_rejection);

    println!("Server started at localhost:8080!");
    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}

async fn create_handler(body: CreateRequest) -> Result<impl Reply> {
    Ok(format!("called with: {:?}", body))
}

async fn create_handler_path(buf: impl Buf) -> Result<impl Reply> {
    let des = &mut serde_json::Deserializer::from_reader(buf.reader());
    let body: CreateRequest = serde_path_to_error::deserialize(des)
        .map_err(|e| reject::custom(Error::JSONPathError(e.to_string())))?;
    Ok(format!("called with: {:?}", body))
}

async fn create_handler_validator(buf: impl Buf) -> Result<impl Reply> {
    let des = &mut serde_json::Deserializer::from_reader(buf.reader());
    let body: CreateRequest = serde_path_to_error::deserialize(des)
        .map_err(|e| reject::custom(Error::JSONPathError(e.to_string())))?;

    body.validate()
        .map_err(|e| reject::custom(Error::ValidationError(e)))?;
    Ok(format!("called with: {:?}", body))
}

#[derive(Error, Debug)]
enum Error {
    #[error("JSON path error: {0}")]
    JSONPathError(String),
    #[error("validation error: {0}")]
    ValidationError(ValidationErrors),
}

impl warp::reject::Reject for Error {}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
    errors: Option<Vec<FieldError>>,
}

#[derive(Serialize)]
struct FieldError {
    field: String,
    field_errors: Vec<String>,
}

pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let (code, message, errors) = if err.is_not_found() {
        (StatusCode::NOT_FOUND, "Not Found".to_string(), None)
    } else if let Some(e) = err.find::<Error>() {
        match e {
            Error::JSONPathError(_) => (StatusCode::BAD_REQUEST, e.to_string(), None),
            Error::ValidationError(val_errs) => {
                let errors: Vec<FieldError> = val_errs
                    .errors()
                    .iter()
                    .map(|error_kind| FieldError {
                        field: error_kind.0.to_string(),
                        field_errors: match error_kind.1 {
                            ValidationErrorsKind::Struct(struct_err) => {
                                validation_errs_to_str_vec(struct_err)
                            }
                            ValidationErrorsKind::Field(field_errs) => field_errs
                                .iter()
                                .map(|fe| format!("{}: {:?}", fe.code, fe.params))
                                .collect(),
                            ValidationErrorsKind::List(vec_errs) => vec_errs
                                .iter()
                                .map(|ve| {
                                    format!(
                                        "{}: {:?}",
                                        ve.0,
                                        validation_errs_to_str_vec(ve.1).join(" | "),
                                    )
                                })
                                .collect(),
                        },
                    })
                    .collect();

                (
                    StatusCode::BAD_REQUEST,
                    "field errors".to_string(),
                    Some(errors),
                )
            }
        }
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        (
            StatusCode::BAD_REQUEST,
            e.source()
                .map(|cause| cause.to_string())
                .unwrap_or_else(|| "BAD_REQUEST".to_string()),
            None,
        )
    } else {
        eprintln!("unhandled error: {:?}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
            None,
        )
    };

    let json = warp::reply::json(&ErrorResponse {
        message: message.into(),
        errors,
    });

    Ok(warp::reply::with_status(json, code))
}

fn validation_errs_to_str_vec(ve: &ValidationErrors) -> Vec<String> {
    ve.field_errors()
        .iter()
        .map(|fe| {
            format!(
                "{}: errors: {}",
                fe.0,
                fe.1.iter()
                    .map(|ve| format!("{}: {:?}", ve.code, ve.params))
                    .collect::<Vec<String>>()
                    .join(", ")
            )
        })
        .collect()
}
