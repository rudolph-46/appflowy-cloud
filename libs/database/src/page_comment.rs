use app_error::AppError;
use chrono::{DateTime, Utc};
use database_entity::dto::PageComment;
use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct PageCommentRow {
  comment_id: Uuid,
  workspace_id: Uuid,
  view_id: Uuid,
  uid: i64,
  user_name: Option<String>,
  content: String,
  reply_to: Option<Uuid>,
  resolved: bool,
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
}

impl From<PageCommentRow> for PageComment {
  fn from(row: PageCommentRow) -> Self {
    Self {
      comment_id: row.comment_id,
      workspace_id: row.workspace_id,
      view_id: row.view_id,
      uid: row.uid,
      user_name: row.user_name,
      content: row.content,
      reply_to: row.reply_to,
      resolved: row.resolved,
      created_at: row.created_at,
      updated_at: row.updated_at,
    }
  }
}

pub async fn create_page_comment<'a, E: Executor<'a, Database = Postgres>>(
  executor: E,
  workspace_id: Uuid,
  view_id: Uuid,
  uid: i64,
  content: &str,
  reply_to: Option<Uuid>,
) -> Result<PageComment, AppError> {
  let row = sqlx::query_as::<_, PageCommentRow>(
    r#"
    WITH inserted AS (
      INSERT INTO af_page_comment (workspace_id, view_id, uid, content, reply_to)
      VALUES ($1, $2, $3, $4, $5)
      RETURNING comment_id, workspace_id, view_id, uid, content, reply_to, resolved, created_at, updated_at
    )
    SELECT i.comment_id, i.workspace_id, i.view_id, i.uid,
           u.name AS user_name,
           i.content, i.reply_to, i.resolved, i.created_at, i.updated_at
    FROM inserted i
    LEFT JOIN af_user u ON u.uid = i.uid
    "#,
  )
  .bind(workspace_id)
  .bind(view_id)
  .bind(uid)
  .bind(content)
  .bind(reply_to)
  .fetch_one(executor)
  .await
  .map_err(|err| anyhow::anyhow!("failed to create page comment: {}", err))?;
  Ok(row.into())
}

pub async fn list_page_comments(
  pool: &PgPool,
  workspace_id: Uuid,
  view_id: Uuid,
) -> Result<Vec<PageComment>, AppError> {
  let rows = sqlx::query_as::<_, PageCommentRow>(
    r#"
    SELECT c.comment_id, c.workspace_id, c.view_id, c.uid,
           u.name AS user_name,
           c.content, c.reply_to, c.resolved, c.created_at, c.updated_at
    FROM af_page_comment c
    LEFT JOIN af_user u ON u.uid = c.uid
    WHERE c.workspace_id = $1 AND c.view_id = $2
    ORDER BY c.created_at ASC
    "#,
  )
  .bind(workspace_id)
  .bind(view_id)
  .fetch_all(pool)
  .await
  .map_err(|err| anyhow::anyhow!("failed to list page comments: {}", err))?;
  Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn update_page_comment<'a, E: Executor<'a, Database = Postgres>>(
  executor: E,
  comment_id: Uuid,
  uid: i64,
  content: Option<&str>,
  resolved: Option<bool>,
) -> Result<Option<PageComment>, AppError> {
  let row = sqlx::query_as::<_, PageCommentRow>(
    r#"
    WITH updated AS (
      UPDATE af_page_comment
      SET content = COALESCE($3, content),
          resolved = COALESCE($4, resolved),
          updated_at = NOW()
      WHERE comment_id = $1 AND uid = $2
      RETURNING comment_id, workspace_id, view_id, uid, content, reply_to, resolved, created_at, updated_at
    )
    SELECT i.comment_id, i.workspace_id, i.view_id, i.uid,
           u.name AS user_name,
           i.content, i.reply_to, i.resolved, i.created_at, i.updated_at
    FROM updated i
    LEFT JOIN af_user u ON u.uid = i.uid
    "#,
  )
  .bind(comment_id)
  .bind(uid)
  .bind(content)
  .bind(resolved)
  .fetch_optional(executor)
  .await
  .map_err(|err| anyhow::anyhow!("failed to update page comment: {}", err))?;
  Ok(row.map(Into::into))
}

pub async fn delete_page_comment<'a, E: Executor<'a, Database = Postgres>>(
  executor: E,
  comment_id: Uuid,
  uid: i64,
) -> Result<bool, AppError> {
  let result = sqlx::query(
    "DELETE FROM af_page_comment WHERE comment_id = $1 AND uid = $2",
  )
  .bind(comment_id)
  .bind(uid)
  .execute(executor)
  .await
  .map_err(|err| anyhow::anyhow!("failed to delete page comment: {}", err))?;
  Ok(result.rows_affected() > 0)
}
