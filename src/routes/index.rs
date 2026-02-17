use askama::Template;
use askama_web::WebTemplate;
use axum_extra::extract::CookieJar;
use snafu::prelude::*;

// TUTORIAL: https://github.com/SeaQL/sea-orm/blob/master/examples/axum_example/
use crate::database::category::{self, CategoryOperator};
use crate::state::flash_message::{OperationStatus, get_cookie};
use crate::state::{AppStateContext, error::*};

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Categories
    pub categories: Vec<category::Model>,
    /// Operation status for UI confirmation
    pub flash: Option<OperationStatus>,
}

#[derive(Template, WebTemplate)]
#[template(path = "upload.html")]
pub struct UploadTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Categories
    pub categories: Vec<String>,
}

impl IndexTemplate {
    pub async fn new(
        context: AppStateContext,
        jar: CookieJar,
    ) -> Result<(CookieJar, Self), AppStateError> {
        let categories = CategoryOperator::new(context.state.clone(), context.user.clone())
            .list()
            .await
            .context(CategorySnafu)?;

        let (jar, operation_status) = get_cookie(jar);

        Ok((
            jar,
            IndexTemplate {
                state: context,
                categories,
                flash: operation_status,
            },
        ))
    }
}

impl UploadTemplate {
    pub async fn new(context: AppStateContext) -> Result<Self, AppStateError> {
        let categories: Vec<String> =
            CategoryOperator::new(context.state.clone(), context.user.clone())
                .list()
                .await
                .context(CategorySnafu)?
                .into_iter()
                .map(|x| x.name.to_string())
                .collect();

        Ok(UploadTemplate {
            state: context,
            categories,
        })
    }
}

pub async fn index(
    context: AppStateContext,
    jar: CookieJar,
) -> Result<(CookieJar, IndexTemplate), AppStateError> {
    IndexTemplate::new(context, jar).await
}

pub async fn upload(context: AppStateContext) -> Result<UploadTemplate, AppStateError> {
    UploadTemplate::new(context).await
}
