-- Page-level comments (phase 1: one thread per page, single-level replies).
-- There is no inline anchoring: a comment belongs to the whole view.
CREATE TABLE IF NOT EXISTS af_page_comment (
  comment_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  workspace_id UUID NOT NULL REFERENCES af_workspace(workspace_id) ON DELETE CASCADE,
  view_id UUID NOT NULL,
  uid BIGINT NOT NULL REFERENCES af_user(uid) ON DELETE CASCADE,
  content TEXT NOT NULL CHECK (char_length(content) > 0 AND char_length(content) <= 10000),
  reply_to UUID REFERENCES af_page_comment(comment_id) ON DELETE CASCADE,
  resolved BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_af_page_comment_workspace_view
  ON af_page_comment (workspace_id, view_id);
