use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        FromRequest, FromRequestParts,
    },
    response::{IntoResponse, Response},
};
use axum_extra::extract::QueryRejection;
use serde::Serialize;

use crate::application::errors::{ApplicationError, DataError};

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApplicationError))]
pub struct Json<T>(pub T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(ApplicationError))]
pub struct Path<T>(pub T);

#[derive(FromRequestParts)]
#[from_request(via(axum_extra::extract::Query), rejection(ApplicationError))]
pub struct Query<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        let Self(value) = self;
        axum::Json(value).into_response()
    }
}

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Json(value)
    }
}

impl From<JsonRejection> for ApplicationError {
    fn from(rejection: JsonRejection) -> Self {
        DataError::Malformed(rejection.body_text()).into()
    }
}

impl From<PathRejection> for ApplicationError {
    fn from(rejection: PathRejection) -> Self {
        DataError::Malformed(rejection.body_text()).into()
    }
}

impl From<QueryRejection> for ApplicationError {
    fn from(rejection: QueryRejection) -> Self {
        DataError::Malformed(rejection.to_string()).into()
    }
}
