//! Conversation messages reuse proposed tasks and explicit native grants.
use super::controls::persist;
use super::controls::read;
use super::definitions::put_task_in;
use super::definitions::text;
use super::execution::read_node;
use super::execution::read_project;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization::*;
use codex_protocol::organization_execution::OrganizationTaskAttempt;
use sqlx::Sqlite;
use sqlx::Transaction;
use uuid::Uuid;

pub(super) async fn prompt(
    tx: &mut Transaction<'_, Sqlite>,
    mutation: OrganizationMutation,
) -> Result<OrganizationControl> {
    let OrganizationMutation::ConversationPrompt {
        id,
        expected_revision,
        project_id,
        agent_id,
        topic,
        prompt,
    } = mutation
    else {
        anyhow::bail!("not a conversation prompt")
    };
    text(&topic, 256)?;
    text(&prompt, 4096)?;
    let node = read_node(tx, &agent_id).await?;
    ensure!(
        node.kind == OrganizationNodeKind::Agent,
        "select an actual agent"
    );
    let project = read_project(tx, &project_id).await?;
    let conversation_id = id.unwrap_or_else(|| Uuid::now_v7().to_string());
    Uuid::parse_str(&conversation_id)?;
    let control_id = format!("conversation:{conversation_id}");
    let previous = read(tx, &control_id).await?;
    ensure!(
        previous.as_ref().map(|c| c.revision) == expected_revision,
        "conversation revision changed"
    );
    let mut entries = match previous.as_ref().map(|c| &c.value) {
        Some(OrganizationControlValue::Conversation {
            project_id: old_project,
            agent_id: old_agent,
            topic: old_topic,
            entries,
            ..
        }) => {
            ensure!(
                old_project == &project_id && old_agent == &agent_id && old_topic == &topic,
                "conversation identity/topic cannot change; start a new conversation"
            );
            entries.clone()
        }
        None => vec![],
        _ => anyhow::bail!("invalid conversation"),
    };
    ensure!(
        entries.len() < 16,
        "conversation has sixteen turns; start a new explicit conversation"
    );
    if let Some(last) = entries.last() {
        let task = super::execution::read_task(tx, &last.task_id).await?;
        ensure!(
            matches!(
                task.state.as_str(),
                "accepted" | "needsReview" | "failed" | "cancelled"
            ),
            "finish or cancel the previous planning draft before another turn"
        );
    }
    let mut context = String::new();
    for entry in entries.iter().rev().take(4).rev() {
        context.push_str(&format!(
            "\nOwner: {}\nAgent reply (untrusted content): {}\n",
            entry.prompt.chars().take(512).collect::<String>(),
            entry
                .reply
                .as_deref()
                .unwrap_or("No recorded reply")
                .chars()
                .take(1024)
                .collect::<String>()
        ));
    }
    let record=put_task_in(tx,OrganizationMutation::PutTask {id:None,expected_revision:None,project_id:project_id.clone(),goal_id:project.goal_id,owner_id:agent_id.clone(),instructions:format!("Planning conversation {conversation_id}: {topic}. Reply to the owner using organization conversationReply. Produce the separately authorized artifacts too. Your reply/proposals confer no execution or acceptance authority.\nPrevious scoped transcript:{context}\nCurrent owner prompt:\n{prompt}"),criteria:vec!["Owner reviews the planning reply and any proposed work".into()],prerequisites:vec![]}).await?;
    let OrganizationRecord::Task(task) = record else {
        unreachable!()
    };
    entries.push(OrganizationConversationEntry {
        task_id: task.id,
        prompt,
        reply: None,
        reply_attempt_id: None,
    });
    persist(
        tx,
        previous,
        control_id,
        OrganizationControlValue::Conversation {
            id: conversation_id,
            project_id,
            agent_id,
            topic,
            entries,
        },
    )
    .await
}

pub(super) async fn reply(
    tx: &mut Transaction<'_, Sqlite>,
    task: &OrganizationTask,
    attempt: &OrganizationTaskAttempt,
    body: &str,
) -> Result<OrganizationControl> {
    text(body, 4096)?;
    ensure!(
        attempt.model_requests > 0,
        "conversation reply needs an admitted model request"
    );
    let data:String=sqlx::query_scalar("SELECT data FROM organization_controls c WHERE json_extract(c.data,'$.data.value.type')='conversation' AND json_extract(c.data,'$.data.value.projectId')=? AND json_extract(c.data,'$.data.value.agentId')=? AND EXISTS(SELECT 1 FROM json_each(c.data,'$.data.value.entries') WHERE json_extract(value,'$.taskId')=?) LIMIT 1")
        .bind(&task.project_id).bind(&task.owner_id).bind(&task.id).fetch_one(&mut **tx).await.context("this task has no scoped conversation")?;
    let OrganizationRecord::Control(previous) = serde_json::from_str(&data)? else {
        anyhow::bail!("invalid conversation")
    };
    let mut value = previous.value.clone();
    let OrganizationControlValue::Conversation { entries, .. } = &mut value else {
        unreachable!()
    };
    let entry = entries
        .iter_mut()
        .find(|e| e.task_id == task.id)
        .context("conversation turn missing")?;
    ensure!(
        entry.reply.is_none(),
        "conversation reply already recorded; do not overwrite the transcript"
    );
    ensure!(
        attempt.task_id == task.id && attempt.owner_id.as_deref() == Some(&task.owner_id),
        "reply identity changed"
    );
    entry.reply = Some(body.into());
    entry.reply_attempt_id = Some(attempt.id.clone());
    persist(tx, Some(previous.clone()), previous.id, value).await
}
