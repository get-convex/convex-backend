use authentication::application_auth::ApplicationAuth;
use errors::ErrorMetadataAnyhowExt;
use keybroker::{
    bad_admin_key_error,
    AdminIdentityPrincipal,
    Identity,
};

pub async fn must_be_admin_from_key(
    app_auth: &ApplicationAuth,
    instance_name: String,
    admin_key: String,
) -> anyhow::Result<Identity> {
    let identity = app_auth
        .check_key(admin_key, instance_name.clone())
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
    Ok(identity)
}

pub fn must_be_admin(identity: &Identity) -> anyhow::Result<AdminIdentityPrincipal> {
    let admin_identity = match identity {
        Identity::InstanceAdmin(admin_identity) => admin_identity,
        Identity::ActingUser(admin_identity, _user_identity_attributes) => admin_identity,
        Identity::System(_) | Identity::User(_) | Identity::Unknown(_) => {
            return Err(bad_admin_key_error(identity.instance_name()).into());
        },
    };
    Ok(admin_identity.principal().clone())
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
        let result = identity.require_operation(DeploymentOp::RunTestQuery);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_operations_allows_all() {
        let identity = admin_identity_with_operations(vec![]);
        assert!(identity
            .require_operation(DeploymentOp::RunTestQuery)
            .is_ok());
        assert!(identity.require_operation(DeploymentOp::Deploy).is_ok());
    }

    #[test]
    fn test_specific_operation_allowed() {
        let identity = admin_identity_with_operations(vec![DeploymentOp::RunTestQuery]);
        assert!(identity
            .require_operation(DeploymentOp::RunTestQuery)
            .is_ok());
    }

    #[test]
    fn test_specific_operation_denied() {
        let identity = admin_identity_with_operations(vec![DeploymentOp::ViewLogs]);
        let result = identity.require_operation(DeploymentOp::RunTestQuery);
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
        assert!(identity
            .require_operation(DeploymentOp::RunTestQuery)
            .is_ok());
        assert!(identity.require_operation(DeploymentOp::Deploy).is_ok());
        assert!(identity.require_operation(DeploymentOp::ViewLogs).is_ok());
        // An operation not in the list should be denied.
        assert!(identity.require_operation(DeploymentOp::WriteData).is_err());
    }

    #[test]
    fn test_unknown_identity_rejected() {
        let identity = Identity::Unknown(None);
        let result = identity.require_operation(DeploymentOp::RunTestQuery);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_forbidden());
        assert_eq!(err.short_msg(), "BadDeployKey");
    }
}
