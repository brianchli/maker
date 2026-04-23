#[macro_export]
macro_rules! ok_or_http_response {
    ($expr:expr, $status:expr) => {
        match $expr {
            Ok(ok) => ok,
            Err(e) => {
                ::tracing::error!("[{}] {}", $status, e);
                return Ok($crate::service::http::error_response($status));
            }
        }
    };
}

#[macro_export]
macro_rules! some_or_http_response {
    ($expr:expr, $reason:literal, $status:expr) => {
        match $expr {
            Some(ok) => ok,
            None => {
                ::tracing::error!("[{}] {}", $status, $reason);
                return Ok($crate::service::http::error_response($status));
            }
        }
    };
}

#[macro_export]
macro_rules! some_or_err {
    ($expr:expr, $reason:literal) => {
        $crate::some_or_http_response!($expr, $reason, StatusCode::INTERNAL_SERVER_ERROR)
    };
}

#[macro_export]
macro_rules! server_err {
    ($expr:expr) => {
        $crate::ok_or_http_response!($expr, StatusCode::INTERNAL_SERVER_ERROR)
    };
}

#[macro_export]
macro_rules! bad_request {
    ($expr:expr) => {
        $crate::ok_or_http_response!($expr, StatusCode::BAD_REQUEST)
    };
}
