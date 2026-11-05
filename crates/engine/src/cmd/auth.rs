//! Authentication belongs to the application shell. Commands request the current form's action;
//! credentials never enter command parameters, the journal, document history or UI requests.
//! Headless/programmatic callers may discard these requests, as they do dialog requests.

use serde_json::{Value, json};

use super::{CommandSpec, ok};
use crate::{Result, Session, bad, spec};

fn request(s: &mut Session, p: &Value, command: &str) -> Result<Value> {
    // The registry allows generic layer id(s), but an account action has no layer parameters.
    // Refuse every value except an empty object, rather than retaining ignored data in a journal.
    if !p.as_object().is_some_and(|object| object.is_empty()) {
        return Err(bad(command, "account actions take an empty object; enter credentials in the account form"));
    }
    s.request_ui(command, json!({}));
    ok()
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!("auth.signIn", "Sign in to DesignFlow", "{}", false, |s, p| request(s, p, "auth.signIn")),
        spec!("auth.signUp", "Create a DesignFlow account", "{}", false, |s, p| request(s, p, "auth.signUp")),
        spec!("auth.sendRecovery", "Send password recovery code", "{}", false, |s, p| request(s, p, "auth.sendRecovery")),
        spec!("auth.verifySignup", "Confirm account email", "{}", false, |s, p| request(s, p, "auth.verifySignup")),
        spec!("auth.verifyRecovery", "Verify password recovery code", "{}", false, |s, p| request(s, p, "auth.verifyRecovery")),
        spec!("auth.updatePassword", "Set a new account password", "{}", false, |s, p| request(s, p, "auth.updatePassword")),
        spec!("auth.signOut", "Sign out of DesignFlow", "{}", false, |s, p| request(s, p, "auth.signOut")),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::specs;
    use crate::Session;

    #[test]
    fn account_actions_request_the_shell_without_a_document_or_credentials() {
        for command in specs() {
            assert!(!command.history);
            let mut session = Session::empty();
            session.execute(command.id, &json!({})).unwrap();
            assert!(session.docs.is_empty());
            assert_eq!(session.ui_requests.len(), 1);
            assert_eq!(session.ui_requests[0].kind, command.id);
            assert_eq!(session.ui_requests[0].data, json!({}));
            assert_eq!(session.journal, vec![(command.id.to_string(), json!({}))]);
        }
    }

    #[test]
    fn account_actions_preserve_design_edits_and_the_undo_stack() {
        let mut session = Session::new();
        session.execute("shape.rectangle", &json!({"width": 80, "height": 40})).unwrap();
        let document = session.doc().unwrap().clone();
        let selection = session.selection();
        let undo = session.undo_depth();
        let repeat = session.last_command.clone();
        for command in specs() {
            session.execute(command.id, &json!({})).unwrap();
            assert_eq!(session.doc().unwrap(), &document);
            assert_eq!(session.selection(), selection);
            assert_eq!(session.undo_depth(), undo);
            assert_eq!(session.last_command, repeat);
        }
    }

    #[test]
    fn credentials_and_ignored_parameters_never_reach_the_journal_or_ui() {
        for command in specs() {
            for params in [
                json!({"email": "synthetic@example.invalid", "password": "test-only-secret"}),
                json!({"access_token": "test-only-token"}),
                json!({"id": "test-only-secret"}),
                json!({"ids": ["test-only-secret"]}),
                json!(["test-only-secret"]),
                json!(null),
            ] {
                let mut session = Session::empty();
                assert!(session.execute(command.id, &params).is_err());
                assert!(session.ui_requests.is_empty());
                assert!(session.journal.is_empty());
                assert!(session.docs.is_empty());
            }
        }
    }
}
