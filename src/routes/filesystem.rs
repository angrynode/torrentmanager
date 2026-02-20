use askama::Template;
use askama_web::WebTemplate;
use axum_extra::extract::CookieJar;

use crate::database::content_folder::PathBreadcrumb;
use crate::database::{category, content_folder};
use crate::extractors::filesystem::FileSystemView;
use crate::state::AppStateContext;
use crate::state::flash_message::{OperationStatus, get_cookie};

#[derive(Template, WebTemplate)]
#[template(path = "filesystem/show.html")]
pub struct FileSystemTemplate {
    pub state: AppStateContext,
    pub category: category::Model,
    pub folder: Option<content_folder::Model>,
    pub children: Vec<content_folder::Model>,
    pub ancestors: Vec<PathBreadcrumb>,
    pub flash: Option<OperationStatus>,
}

impl FileSystemTemplate {
    fn new(context: AppStateContext, view: FileSystemView) -> Self {
        let FileSystemView {
            category,
            folder,
            children,
            ancestors,
        } = view;

        Self {
            state: context,
            category,
            folder,
            children,
            ancestors,
            flash: None,
        }
    }

    pub fn with_flash(mut self, flash: Option<OperationStatus>) -> Self {
        self.flash = flash;
        self
    }

    fn title(&self) -> String {
        if let Some(folder) = &self.folder {
            folder.name.to_string()
        } else {
            self.category.name.to_string()
        }
    }
}

pub async fn filesystem(
    context: AppStateContext,
    view: FileSystemView,
    jar: CookieJar,
) -> (CookieJar, FileSystemTemplate) {
    let (jar, operation_status) = get_cookie(jar);
    (
        jar,
        FileSystemTemplate::new(context, view).with_flash(operation_status),
    )
}
