use actix_web::web::{Data, Json, Path};
use actix_web::HttpRequest;
use app_error::AppError;
use database::page_comment::{
  create_page_comment, delete_page_comment, list_page_comments, update_page_comment,
};
use database_entity::dto::{AFRole, PageComment};
use serde::Deserialize;
use shared_entity::response::{AppResponse, JsonAppResponse};
use uuid::Uuid;

use crate::biz::authentication::jwt::UserUuid;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreatePageCommentRequest {
  pub content: String,
  pub reply_to: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePageCommentRequest {
  pub content: Option<String>,
  pub resolved: Option<bool>,
}

fn check_content(content: &str) -> Result<String, AppError> {
  let trimmed = content.trim().to_string();
  if trimmed.is_empty() || trimmed.len() > 10000 {
    return Err(AppError::InvalidRequest(
      "comment content must be 1-10000 characters".to_string(),
    ));
  }
  Ok(trimmed)
}

pub(crate) async fn post_page_comment_handler(
  user_uuid: UserUuid,
  path: Path<(Uuid, Uuid)>,
  state: Data<AppState>,
  payload: Json<CreatePageCommentRequest>,
) -> actix_web::Result<JsonAppResponse<PageComment>> {
  let (workspace_id, view_id) = path.into_inner();
  let uid = state.user_cache.get_user_uid(&user_uuid).await?;
  state
    .workspace_access_control
    .enforce_role_weak(&uid, &workspace_id, AFRole::Member)
    .await?;
  let content = check_content(&payload.content)?;
  let comment = create_page_comment(
    &state.pg_pool,
    workspace_id,
    view_id,
    uid,
    &content,
    payload.reply_to,
  )
  .await?;
  Ok(AppResponse::Ok().with_data(comment).into())
}

pub(crate) async fn list_page_comments_handler(
  user_uuid: UserUuid,
  path: Path<(Uuid, Uuid)>,
  state: Data<AppState>,
  _req: HttpRequest,
) -> actix_web::Result<JsonAppResponse<Vec<PageComment>>> {
  let (workspace_id, view_id) = path.into_inner();
  let uid = state.user_cache.get_user_uid(&user_uuid).await?;
  state
    .workspace_access_control
    .enforce_role_weak(&uid, &workspace_id, AFRole::Member)
    .await?;
  let comments = list_page_comments(&state.pg_pool, workspace_id, view_id).await?;
  Ok(AppResponse::Ok().with_data(comments).into())
}

pub(crate) async fn patch_page_comment_handler(
  user_uuid: UserUuid,
  path: Path<(Uuid, Uuid)>,
  state: Data<AppState>,
  payload: Json<UpdatePageCommentRequest>,
) -> actix_web::Result<JsonAppResponse<PageComment>> {
  let (workspace_id, comment_id) = path.into_inner();
  let uid = state.user_cache.get_user_uid(&user_uuid).await?;
  state
    .workspace_access_control
    .enforce_role_weak(&uid, &workspace_id, AFRole::Member)
    .await?;
  let content = payload
    .content
    .as_deref()
    .map(check_content)
    .transpose()?;
  let comment = update_page_comment(
    &state.pg_pool,
    comment_id,
    uid,
    content.as_deref(),
    payload.resolved,
  )
  .await?
  .ok_or_else(|| {
    AppError::RecordNotFound("comment not found or not owned by you".to_string())
  })?;
  Ok(AppResponse::Ok().with_data(comment).into())
}

pub(crate) async fn delete_page_comment_handler(
  user_uuid: UserUuid,
  path: Path<(Uuid, Uuid)>,
  state: Data<AppState>,
) -> actix_web::Result<JsonAppResponse<()>> {
  let (workspace_id, comment_id) = path.into_inner();
  let uid = state.user_cache.get_user_uid(&user_uuid).await?;
  state
    .workspace_access_control
    .enforce_role_weak(&uid, &workspace_id, AFRole::Member)
    .await?;
  if !delete_page_comment(&state.pg_pool, comment_id, uid).await? {
    return Err(
      AppError::RecordNotFound("comment not found or not owned by you".to_string()).into(),
    );
  }
  Ok(AppResponse::Ok().with_data(()).into())
}
