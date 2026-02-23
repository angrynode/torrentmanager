use askama::Template;
use askama_web::WebTemplate;

// TUTORIAL: https://github.com/SeaQL/sea-orm/blob/master/examples/axum_example/
use crate::extractors::category_request::CategoriesRequest;
use crate::filesystem::FileSystemEntry;
use crate::state::AppStateContext;
use crate::state::flash_message::{FallibleTemplate, OperationStatus};

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
    pub fn new(context: AppStateContext, categories: CategoriesRequest) -> Self {
        Self {
            state: context,
            flash: None,
            children: categories.children,
        }
    }
}

pub async fn index(context: AppStateContext, categories: CategoriesRequest) -> IndexTemplate {
    IndexTemplate::new(context, categories)
}
