use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::Path;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::category;
use crate::database::content_folder::PathBreadcrumb;
use crate::extractors::normalized_path::*;
use crate::filesystem::FileSystemEntry;
use crate::state::flash_message::{
    FallibleTemplate, FlashRedirect, FlashTemplate, OperationStatus, StatusCookie,
};
use crate::state::{AppStateContext, error::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CategoryForm {
    pub name: NormalizedPathComponent,
    pub path: NormalizedPathAbsolute,
}

#[derive(Template, WebTemplate)]
#[template(path = "categories/new.html")]
pub struct NewCategoryTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// Default form with value
    pub category_form: Option<CategoryForm>,
}

pub async fn new(app_state_context: AppStateContext) -> NewCategoryTemplate {
    NewCategoryTemplate {
        state: app_state_context,
        category_form: None,
    }
}

pub async fn delete(
    context: AppStateContext,
    Path(id): Path<i32>,
    jar: CookieJar,
) -> FlashRedirect {
    let status = match context.db.category().delete(id).await {
        Ok(name) => StatusCookie::success(
            jar,
            format!("The category {} has been successfully deleted", name),
        ),
        Err(error) => StatusCookie::error(jar, error.to_string()),
    };

    status.redirect("/categories")
}

pub async fn create(
    context: AppStateContext,
    jar: CookieJar,
    Form(form): Form<CategoryForm>,
) -> FlashRedirect {
    let status = match context.db.category().create(&form).await {
        Ok(created) => StatusCookie::success(
            jar,
            format!(
                "The category {} has been successfully created (ID {})",
                created.name, created.id
            ),
        ),
        Err(error) => StatusCookie::error(jar, error.to_string()),
    };

    status.redirect("/")
}

#[derive(Template, WebTemplate)]
#[template(path = "categories/show.html")]
pub struct CategoryShowTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// Categories found in database
    pub children: Vec<FileSystemEntry>,
    /// Category
    category: category::Model,
    /// Operation status for UI confirmation (Cookie)
    pub flash: Option<OperationStatus>,
    /// Breadcrumbs navigation
    pub breadcrumbs: Vec<PathBreadcrumb>,
}

impl FallibleTemplate for CategoryShowTemplate {
    fn with_optional_flash(&mut self, flash: Option<OperationStatus>) {
        self.flash = flash;
    }
}

pub async fn show(
    context: AppStateContext,
    Path(category_name): Path<String>,
    status: StatusCookie,
) -> Result<FlashTemplate<CategoryShowTemplate>, AppStateError> {
    let categories = context.db.category();

    let category = categories
        .find_by_name(category_name.to_string())
        .await
        .context(CategorySnafu)?;

    // get all content folders in this category
    let content_folders = categories
        .list_folders(category.id)
        .await
        .context(CategorySnafu)?;

    let children = FileSystemEntry::from_content_folders(&category, &content_folders);

    let breadcrumbs = PathBreadcrumb::for_filesystem_path(category.name.as_str());

    Ok(status.with_template(CategoryShowTemplate {
        category,
        children,
        state: context,
        flash: None,
        breadcrumbs,
    }))
}
