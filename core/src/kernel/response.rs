use actix_web::{http::StatusCode, Error as ActixError, HttpResponse, ResponseError};
use std::fmt;

#[derive(Debug, Clone)]
pub enum PageError {
    Internal(String),
    Response(StatusCode, String),

    Home,
    Login,
    RedirectTo(String),
}

impl fmt::Display for PageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ActixError> for PageError {
    fn from(err: ActixError) -> PageError {
        PageError::Internal(format!("Actix error: {:?}", err))
    }
}

impl ResponseError for PageError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PageError::Response(code, body) => HttpResponse::build(code.clone()).body(body),
            PageError::Internal(err) => {
                log::error!("Internal server error: {}", err);
                HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).finish()
            }
            _ => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body("Internal error"),
        }
    }
}

pub type PageResult = Result<HttpResponse, PageError>;

#[derive(Debug, Clone)]
pub enum APIError {
    Internal(String),
    Response(StatusCode, String),

    BadRequest(String),
    Conflict(String),
    TooManyRequests(String),
    RespourceNotFound(String),
    Forbidden,
    Unauthorized,
}

impl fmt::Display for APIError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ActixError> for APIError {
    fn from(err: ActixError) -> APIError {
        APIError::Internal(format!("Actix error: {:?}", err))
    }
}

impl ResponseError for APIError {
    fn error_response(&self) -> HttpResponse {
        match self {
            APIError::Response(code, body) => HttpResponse::build(code.clone()).body(body),
            APIError::Internal(err) => {
                log::error!("Internal server error: {}", err);
                HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).finish()
            }
            APIError::BadRequest(body) => HttpResponse::build(StatusCode::BAD_REQUEST).body(body),
            APIError::Conflict(body) => HttpResponse::build(StatusCode::CONFLICT).body(body),
            APIError::TooManyRequests(body) => HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).body(body),
            APIError::RespourceNotFound(body) => HttpResponse::build(StatusCode::NOT_FOUND).body(body),
            APIError::Forbidden => HttpResponse::build(StatusCode::FORBIDDEN).finish(),
            APIError::Unauthorized => HttpResponse::build(StatusCode::UNAUTHORIZED).finish(),
        }
    }
}

pub type APIResult = Result<HttpResponse, APIError>;
