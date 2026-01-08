use std::fmt::Write;

use arboard::Clipboard;

use crate::error::{Result, TuicrError};
use crate::model::{CommentType, ReviewSession};

pub fn export_to_clipboard(session: &ReviewSession) -> Result<()> {
    let content = generate_markdown(session);

    let mut clipboard = Clipboard::new()
        .map_err(|e| TuicrError::Clipboard(format!("Failed to access clipboard: {}", e)))?;

    clipboard
        .set_text(content)
        .map_err(|e| TuicrError::Clipboard(format!("Failed to copy to clipboard: {}", e)))?;

    Ok(())
}

fn generate_markdown(session: &ReviewSession) -> String {
    let mut md = String::new();

    let repo_name = session
        .repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Header
    let _ = writeln!(md, "# Code Review: {}", repo_name);
    let _ = writeln!(md);
    let _ = writeln!(
        md,
        "**Reviewed:** {}",
        session.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    let _ = writeln!(md, "**Base Commit:** `{}`", session.base_commit);

    let reviewed = session.reviewed_count();
    let total = session.files.len();
    let _ = writeln!(md, "**Files Reviewed:** {}/{}", reviewed, total);
    let _ = writeln!(md);

    // Session notes
    if let Some(notes) = &session.session_notes {
        let _ = writeln!(md, "## Summary");
        let _ = writeln!(md);
        let _ = writeln!(md, "{}", notes);
        let _ = writeln!(md);
    }

    // Files
    let _ = writeln!(md, "## Files");
    let _ = writeln!(md);

    // Collect all action items for the summary
    let mut action_items: Vec<(String, Option<u32>, String)> = Vec::new();

    // Sort files by path for consistent output
    let mut files: Vec<_> = session.files.iter().collect();
    files.sort_by_key(|(path, _)| path.to_string_lossy().to_string());

    for (path, review) in files {
        let status_char = review.status.as_char();
        let reviewed_mark = if review.reviewed {
            "REVIEWED"
        } else {
            "PENDING"
        };

        let _ = writeln!(
            md,
            "### {} `{}` [{}]",
            status_char,
            path.display(),
            reviewed_mark
        );
        let _ = writeln!(md);

        // File comments
        if !review.file_comments.is_empty() {
            let _ = writeln!(md, "#### File Comments");
            let _ = writeln!(md);

            for comment in &review.file_comments {
                let _ = writeln!(
                    md,
                    "> **[{}]** {}",
                    comment.comment_type.as_str(),
                    comment.content
                );
                let _ = writeln!(md);

                // Track issues and suggestions as action items
                if matches!(
                    comment.comment_type,
                    CommentType::Issue | CommentType::Suggestion
                ) {
                    action_items.push((path.display().to_string(), None, comment.content.clone()));
                }
            }
        }

        // Line comments
        if !review.line_comments.is_empty() {
            let _ = writeln!(md, "#### Line Comments");
            let _ = writeln!(md);

            // Sort by line number
            let mut line_comments: Vec<_> = review.line_comments.iter().collect();
            line_comments.sort_by_key(|(line, _)| *line);

            for (line, comments) in line_comments {
                for comment in comments {
                    let _ = writeln!(md, "**Line {}:**", line);
                    let _ = writeln!(md);
                    let _ = writeln!(
                        md,
                        "> **[{}]** {}",
                        comment.comment_type.as_str(),
                        comment.content
                    );
                    let _ = writeln!(md);

                    // Track issues and suggestions as action items
                    if matches!(
                        comment.comment_type,
                        CommentType::Issue | CommentType::Suggestion
                    ) {
                        action_items.push((
                            path.display().to_string(),
                            Some(*line),
                            comment.content.clone(),
                        ));
                    }
                }
            }
        }

        let _ = writeln!(md, "---");
        let _ = writeln!(md);
    }

    // Action Items summary
    if !action_items.is_empty() {
        let _ = writeln!(md, "## Action Items");
        let _ = writeln!(md);

        for (i, (file, line, content)) in action_items.iter().enumerate() {
            let location = match line {
                Some(l) => format!("**`{}`:{}**", file, l),
                None => format!("**`{}`**", file),
            };
            let _ = writeln!(md, "{}. {} - {}", i + 1, location, content);
        }
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Comment, FileStatus};
    use std::path::PathBuf;

    fn create_test_session() -> ReviewSession {
        let mut session =
            ReviewSession::new(PathBuf::from("/tmp/test-repo"), "abc1234def".to_string());
        session.add_file(PathBuf::from("src/main.rs"), FileStatus::Modified);

        // Add a file comment
        if let Some(review) = session.get_file_mut(&PathBuf::from("src/main.rs")) {
            review.reviewed = true;
            review.add_file_comment(Comment::new(
                "Consider adding documentation".to_string(),
                CommentType::Suggestion,
            ));
            review.add_line_comment(
                42,
                Comment::new(
                    "Magic number should be a constant".to_string(),
                    CommentType::Issue,
                ),
            );
        }

        session
    }

    #[test]
    fn should_generate_valid_markdown() {
        // given
        let session = create_test_session();

        // when
        let markdown = generate_markdown(&session);

        // then
        assert!(markdown.contains("# Code Review: test-repo"));
        assert!(markdown.contains("**Base Commit:** `abc1234def`"));
        assert!(markdown.contains("src/main.rs"));
        assert!(markdown.contains("[SUGGESTION]"));
        assert!(markdown.contains("Consider adding documentation"));
        assert!(markdown.contains("Line 42"));
        assert!(markdown.contains("[ISSUE]"));
        assert!(markdown.contains("Magic number"));
        assert!(markdown.contains("## Action Items"));
    }

    #[test]
    fn should_include_action_items_for_issues_and_suggestions() {
        // given
        let session = create_test_session();

        // when
        let markdown = generate_markdown(&session);

        // then
        // Should have 2 action items (1 suggestion + 1 issue)
        assert!(markdown.contains("1."));
        assert!(markdown.contains("2."));
    }
}
