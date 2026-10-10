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
        json!({"type":"stop","taskId":"task","expectedRevision":1,"cancel":true}),
        json!({"type":"decideApproval","taskId":"task","approvalId":"approval","expectedHash":"exact","approve":true}),
        json!({"type":"accept","taskId":"task","expectedRevision":1,"digest":"checked"}),
        json!({"type":"requestChanges","taskId":"task","expectedRevision":1,"instructions":"Revise citations"}),
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
        assert_eq!(
            variant["properties"]
                .as_object()
                .unwrap()
                .keys()
                .collect::<BTreeSet<_>>(),
            wire.as_object().unwrap().keys().collect::<BTreeSet<_>>()
        );
    }
    let mut expanded = authorization.clone();
    expanded["scope"]["administrator"] = json!(true);
    assert!(serde_json::from_value::<OrganizationExecutionMutation>(expanded).is_err());
    let mut misspelled = authorization;
    misspelled["expected_revision"] = json!(1);
    assert!(serde_json::from_value::<OrganizationExecutionMutation>(misspelled).is_err());
}
