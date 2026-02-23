use askama::Template;
use askama_web::WebTemplate;
use snafu::prelude::*;

// TUTORIAL: https://github.com/SeaQL/sea-orm/blob/master/examples/axum_example/
use crate::filesystem::FileSystemEntry;
use crate::state::flash_message::{FallibleTemplate, FlashTemplate, OperationStatus, StatusCookie};
use crate::state::{AppStateContext, error::*};

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Categories
    pub children: Vec<FileSystemEntry>,
    /// Operation status for UI confirmation
    pub flash: Option<OperationStatus>,
}

impl FallibleTemplate for IndexTemplate {
    fn with_optional_flash(&mut self, flash: Option<OperationStatus>) {
        self.flash = flash;
    }
}

impl IndexTemplate {
    pub async fn new(context: AppStateContext) -> Result<Self, AppStateError> {
        let categories = context.db.category().list().await.context(CategorySnafu)?;
        let children = FileSystemEntry::from_categories(&categories);

        Ok(Self {
            state: context,
            flash: None,
            children,
        })
    }
}

pub async fn index(
    context: AppStateContext,
    status: StatusCookie,
) -> Result<FlashTemplate<IndexTemplate>, AppStateError> {
    let template = IndexTemplate::new(context).await?;
    Ok(status.with_template(template))
}
