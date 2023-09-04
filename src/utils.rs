use actix_web::{http::header::LOCATION, HttpResponse};

pub fn internal_server_error<Error>(error: Error) -> actix_web::Error
where
    Error: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(error)
}

pub fn see_other<Url>(location: Url) -> HttpResponse
where
    Url: AsRef<str>,
{
    HttpResponse::SeeOther()
        .append_header((LOCATION, location.as_ref()))
        .finish()
}
