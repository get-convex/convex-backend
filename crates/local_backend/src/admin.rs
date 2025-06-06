use authentication::application_auth::ApplicationAuth;
use common::types::MemberId;
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use keybroker::{
    AdminIdentityPrincipal,
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

pub fn must_be_admin_member_with_write_access(identity: &Identity) -> anyhow::Result<MemberId> {
    must_be_admin_member_internal(identity, true)
}

pub fn must_be_admin_member(identity: &Identity) -> anyhow::Result<MemberId> {
    must_be_admin_member_internal(identity, false)
}

fn must_be_admin_member_internal(
    identity: &Identity,
    needs_write_access: bool,
) -> anyhow::Result<MemberId> {
    if let Identity::InstanceAdmin(admin_identity) = identity {
        if let AdminIdentityPrincipal::Member(member_id) = admin_identity.principal() {
            if needs_write_access && admin_identity.is_read_only() {
                return Err(read_only_admin_key_error().into());
            }
            Ok(*member_id)
        } else {
            Err(bad_admin_key_error(identity.instance_name()).into())
        }
    } else {
        Err(bad_admin_key_error(identity.instance_name()).into())
    }
}

pub fn bad_admin_key_error(instance_name: Option<String>) -> ErrorMetadata {
    let msg = match instance_name {
        Some(name) => format!(
            "The provided deploy key was invalid for deployment '{}'. Double check that the \
             environment this key was generated for matches the desired deployment.",
            name
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
