use axum_extra::extract::{CookieJar, cookie::Cookie};

#[derive(Debug)]
pub struct OperationStatus {
    /// Status of operation
    pub success: bool,
    /// Message for confirmation alert
    pub message: String,
}

impl OperationStatus {
    pub fn set_cookie(&self, jar: CookieJar) -> CookieJar {
        let mut cookie_operation_status_success =
            Cookie::new("operation_status_success", self.success.to_string());

        let mut cookie_operation_status_message =
            Cookie::new("operation_status_message", self.message.clone());
        cookie_operation_status_success.set_path("/");
        cookie_operation_status_message.set_path("/");

        jar.add(cookie_operation_status_success)
            .add(cookie_operation_status_message)
    }
}

pub fn get_cookie(jar: CookieJar) -> (CookieJar, Option<OperationStatus>) {
    let operation_status = match (
        jar.get("operation_status_success"),
        jar.get("operation_status_message"),
    ) {
        (Some(success), Some(message)) => Some(OperationStatus {
            success: if let Ok(success) = success.value().parse() {
                success
            } else {
                return (jar, None);
            },
            message: message.value().to_string(),
        }),
        _ => None,
    };

    let mut operation_status_success_cookie = Cookie::from("operation_status_success");
    operation_status_success_cookie.set_path("/");
    let mut operation_status_message_cookie = Cookie::from("operation_status_message");
    operation_status_message_cookie.set_path("/");

    let jar = jar
        .remove(operation_status_success_cookie)
        .remove(operation_status_message_cookie);

    (jar, operation_status)
}
