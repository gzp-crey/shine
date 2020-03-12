use actix_web::{http::header, http::StatusCode, Error as ActixError, HttpResponse, ResponseError};
use std::fmt;

/// Helper to handle redirect responses
#[derive(Debug, Clone)]
pub enum Redirect {
    Permanent(String), // redirect, resource moved 301
    Found(String),     // redirect 302
    SeeOther(String),  // redirect and change method to get 303
    Temporary(String), // redirect and preseve method 307
}

impl From<&Redirect> for HttpResponse {
    fn from(value: &Redirect) -> HttpResponse {
        match value {
            Redirect::Permanent(uri) => {
                log::error!("RedirectPermanent to {}", uri);
                HttpResponse::MovedPermanently()
                    .header(header::LOCATION, uri.as_str())
                    .finish()
                    .into_body()
            }
            Redirect::Found(uri) => {
                log::error!("RedirectFound to {}", uri);
                HttpResponse::Found()
                    .header(header::LOCATION, uri.as_str())
                    .finish()
                    .into_body()
            }
            Redirect::SeeOther(uri) => {
                log::error!("RedirectSeeOther to {}", uri);
                HttpResponse::SeeOther()
                    .header(header::LOCATION, uri.as_str())
                    .finish()
                    .into_body()
            }
            Redirect::Temporary(uri) => {
                log::error!("RedirectTemporary to {}", uri);
                HttpResponse::TemporaryRedirect()
                    .header(header::LOCATION, uri.as_str())
                    .finish()
                    .into_body()
            }
        }
    }
}

impl From<Redirect> for HttpResponse {
    fn from(value: Redirect) -> HttpResponse {
        (&value).into()
    }
}

#[derive(Debug, Clone)]
pub enum PageError {
    Internal(String),
    Response(StatusCode, String),
    RedirectOnError(String, Redirect),
    Home,
    Login,
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
            PageError::RedirectOnError(err, redirect) => {
                log::error!("Redirect on error: {}", err);
                redirect.into()
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
