use codex_protocol::organization_coordination::OrganizationCoordinationCall;
use codex_protocol::organization_execution::OrganizationExecutionMutation;
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn execution_mutation_schema_matches_owner_wire_fields_and_rejects_unknown_grants() {
    let schema =
        serde_json::to_value(schemars::schema_for!(OrganizationExecutionMutation)).unwrap();
    let authorization = json!({
        "type":"authorize", "taskId":"task", "expectedRevision":1,
        "scope":{
            "inputFiles":["oracle.py"], "immutableFiles":["oracle.py"],
            "artifacts":["report.txt"], "checks":[["python3","oracle.py"]],
            "model":"gpt-5.5", "maxTokens":20000, "maxSeconds":60,
            "maxModelRequests":2, "maxAttempts":1, "requireToolApproval":true,
            "delegates":[], "maxChildren":0
        }
    });
    for example in [
        authorization.clone(),
        json!({"type":"handoff","taskId":"task","expectedRevision":2,"checkpointId":"checkpoint","ownerId":"receiver","expectedOwnerRevision":3,"instructions":"Continue remaining work"}),
        json!({"type":"stop","taskId":"task","expectedRevision":1,"cancel":true}),
        json!({"type":"decideApproval","taskId":"task","approvalId":"approval","expectedHash":"exact","approve":true}),
        json!({"type":"accept","taskId":"task","expectedRevision":1,"digest":"checked"}),
        json!({"type":"requestChanges","taskId":"task","expectedRevision":1,"instructions":"Revise citations"}),
        json!({"type":"apply","taskId":"task","expectedRevision":1,"digest":"checked"}),
        json!({"type":"apply","taskId":"task","expectedRevision":1,"digest":"checked","paths":["report.txt"]}),
    ] {
        let mutation: OrganizationExecutionMutation = serde_json::from_value(example).unwrap();
        assert_eq!(mutation.task_id(), "task");
        let wire = serde_json::to_value(mutation).unwrap();
        let variant = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|variant| variant["properties"]["type"]["enum"][0] == wire["type"])
            .unwrap();
        let allowed = variant["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let required = variant["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|field| field.as_str().unwrap())
            .collect::<BTreeSet<_>>();
        let present = wire
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert!(
            present.is_subset(&allowed),
            "wire contains fields outside its schema"
        );
        assert!(
            required.is_subset(&present),
            "wire omits a required schema field"
        );
        if wire["type"] != "apply" || wire.get("paths").is_some() {
            assert_eq!(allowed, present);
        } else {
            assert_eq!(
                allowed.difference(&present).copied().collect::<Vec<_>>(),
                ["paths"]
            );
            assert!(!required.contains("paths"));
        }
    }
    let mut expanded = authorization.clone();
    expanded["scope"]["administrator"] = json!(true);
    assert!(serde_json::from_value::<OrganizationExecutionMutation>(expanded).is_err());
    let mut misspelled = authorization;
    misspelled["expected_revision"] = json!(1);
    assert!(serde_json::from_value::<OrganizationExecutionMutation>(misspelled).is_err());
}

#[test]
fn coordination_wire_matches_schema_without_agent_authority_fields() {
    let schema = serde_json::to_value(schemars::schema_for!(OrganizationCoordinationCall)).unwrap();
    for example in [
        json!({"type":"checkpoint","content":{"completedSteps":["Prepared"],"remainingSteps":["Check"],"blocker":"Owner decision"}}),
        json!({"type":"wait","taskIds":["child"]}),
        json!({"type":"proposePlan","items":[{"key":"one","ownerId":"named-agent","instructions":"Check","criteria":["Checked"],"prerequisites":[]}]}),
        json!({"type":"delegate","taskId":"child"}),
        json!({"type":"writeNote","scope":"agent","body":"Context"}),
        json!({"type":"readNotes","scope":"project"}),
        json!({"type":"send","taskId":"child","body":"Context"}),
        json!({"type":"inbox"}),
        json!({"type":"acknowledge","messageId":"message"}),
    ] {
        let call: OrganizationCoordinationCall = serde_json::from_value(example.clone()).unwrap();
        assert_eq!(serde_json::to_value(&call).unwrap(), example);
        let variant = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|variant| variant["properties"]["type"]["enum"][0] == example["type"])
            .unwrap();
        assert_eq!(
            variant["properties"]
                .as_object()
                .unwrap()
                .keys()
                .collect::<BTreeSet<_>>(),
            example.as_object().unwrap().keys().collect::<BTreeSet<_>>()
        );
        for field in [
            "scopeOverride",
            "workerId",
            "attemptId",
            "model",
            "maxTokens",
            "approve",
        ] {
            let mut expanded = example.clone();
            expanded[field] = json!("injected");
            assert!(serde_json::from_value::<OrganizationCoordinationCall>(expanded).is_err());
        }
    }
}
