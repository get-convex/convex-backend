use authentication::application_auth::ApplicationAuth;
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use keybroker::{
    AdminIdentityPrincipal,
    DeploymentOp,
    Identity,
};

pub async fn must_be_admin_from_key_with_write_access(
    app_auth: &ApplicationAuth,
    instance_name: String,
    admin_key: String,
) -> anyhow::Result<Identity> {
    must_be_admin_from_key_internal(app_auth, instance_name, admin_key, true).await
}

pub async fn must_be_admin_from_key(
    app_auth: &ApplicationAuth,
    instance_name: String,
    admin_key: String,
) -> anyhow::Result<Identity> {
    must_be_admin_from_key_internal(app_auth, instance_name, admin_key, false).await
}

async fn must_be_admin_from_key_internal(
    app_auth: &ApplicationAuth,
    instance_name: String,
    admin_key_or_access_token: String,
    needs_write_access: bool,
) -> anyhow::Result<Identity> {
    let identity = app_auth
        .check_key(admin_key_or_access_token, instance_name.clone())
        .await
        .map_err(|e| {
            if e.is_forbidden() {
                // This attaches a second ErrorMetadata to the error, but this
                // message is more helpful
                e.context(bad_admin_key_error(Some(instance_name)))
            } else {
                // This is a server error of some kind, not an issue with the
                // key itself
                e
            }
        })?;
    if needs_write_access {
        must_be_admin_with_write_access(&identity)?;
    }
    Ok(identity)
}

pub fn must_be_admin_with_write_access(
    identity: &Identity,
) -> anyhow::Result<AdminIdentityPrincipal> {
    must_be_admin_internal(identity, true)
}

pub fn must_be_admin_with_operation(
    identity: &Identity,
    operation: DeploymentOp,
) -> anyhow::Result<()> {
    let admin_identity = match identity {
        Identity::System(_) => return Ok(()),
        Identity::InstanceAdmin(admin_identity) | Identity::ActingUser(admin_identity, _) => {
            admin_identity
        },
        Identity::User(_) | Identity::Unknown(_) => {
            return Err(bad_admin_key_error(identity.instance_name()).into());
        },
    };
    if !admin_identity.is_operation_allowed(operation) {
        anyhow::bail!(ErrorMetadata::forbidden(
            "Unauthorized",
            "You do not have permission to perform this operation.",
        ));
    }
    Ok(())
}

pub fn must_be_admin(identity: &Identity) -> anyhow::Result<AdminIdentityPrincipal> {
    must_be_admin_internal(identity, false)
}

fn must_be_admin_internal(
    identity: &Identity,
    needs_write_access: bool,
) -> anyhow::Result<AdminIdentityPrincipal> {
    let admin_identity = match identity {
        Identity::InstanceAdmin(admin_identity) => admin_identity,
        Identity::ActingUser(admin_identity, _user_identity_attributes) => admin_identity,
        Identity::System(_) | Identity::User(_) | Identity::Unknown(_) => {
            return Err(bad_admin_key_error(identity.instance_name()).into());
        },
    };

    if needs_write_access && admin_identity.is_read_only() {
        return Err(read_only_admin_key_error().into());
    }
    Ok(admin_identity.principal().clone())
}

pub fn bad_admin_key_error(instance_name: Option<String>) -> ErrorMetadata {
    let msg = match instance_name {
        Some(name) => format!(
            "The provided deploy key was invalid for deployment '{name}'. Double check that the \
             environment this key was generated for matches the desired deployment."
        ),
        None => "The provided deploy key was invalid for this deployment. Double check that the \
                 environment this key was generated for matches the desired deployment."
            .to_string(),
    };
    ErrorMetadata::forbidden("BadDeployKey", msg)
}

pub fn read_only_admin_key_error() -> ErrorMetadata {
    ErrorMetadata::forbidden(
        "ReadOnlyAdminKey",
        "You do not have permission to perform this operation.",
    )
}

#[cfg(test)]
mod tests {
    use common::types::MemberId;
    use errors::ErrorMetadataAnyhowExt;
    use keybroker::{
        AdminIdentity,
        AdminIdentityPrincipal,
        DeploymentOp,
        Identity,
    };

    use super::must_be_admin_with_operation;

    fn admin_identity_with_operations(allowed_ops: Vec<DeploymentOp>) -> Identity {
        Identity::InstanceAdmin(AdminIdentity::new_for_access_token(
            "test-instance".to_string(),
            AdminIdentityPrincipal::Member(MemberId(1)),
            "test-token".to_string(),
            false,
            allowed_ops,
        ))
    }

    #[test]
    fn test_system_identity_always_allowed() {
        let identity = Identity::system();
        let result = must_be_admin_with_operation(&identity, DeploymentOp::RunTestQuery);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_operations_allows_all() {
        let identity = admin_identity_with_operations(vec![]);
        assert!(must_be_admin_with_operation(&identity, DeploymentOp::RunTestQuery).is_ok());
        assert!(must_be_admin_with_operation(&identity, DeploymentOp::Deploy).is_ok());
    }

    #[test]
    fn test_specific_operation_allowed() {
        let identity = admin_identity_with_operations(vec![DeploymentOp::RunTestQuery]);
        assert!(must_be_admin_with_operation(&identity, DeploymentOp::RunTestQuery).is_ok());
    }

    #[test]
    fn test_specific_operation_denied() {
        let identity = admin_identity_with_operations(vec![DeploymentOp::ViewLogs]);
        let result = must_be_admin_with_operation(&identity, DeploymentOp::RunTestQuery);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_forbidden());
        assert_eq!(err.short_msg(), "Unauthorized");
    }

    #[test]
    fn test_multiple_operations_allowed() {
        let identity = admin_identity_with_operations(vec![
            DeploymentOp::ViewLogs,
            DeploymentOp::RunTestQuery,
            DeploymentOp::Deploy,
        ]);
        assert!(must_be_admin_with_operation(&identity, DeploymentOp::RunTestQuery).is_ok());
        assert!(must_be_admin_with_operation(&identity, DeploymentOp::Deploy).is_ok());
        assert!(must_be_admin_with_operation(&identity, DeploymentOp::ViewLogs).is_ok());
        // An operation not in the list should be denied.
        assert!(must_be_admin_with_operation(&identity, DeploymentOp::WriteData).is_err());
    }

    #[test]
    fn test_unknown_identity_rejected() {
        let identity = Identity::Unknown(None);
        let result = must_be_admin_with_operation(&identity, DeploymentOp::RunTestQuery);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_forbidden());
        assert_eq!(err.short_msg(), "BadDeployKey");
    }
}
