use common::components::ComponentId;
use keybroker::Identity;

/// We only let auth propagate across function calls within the root component,
/// with the special case of permitting admin auth to always propagate.
pub fn propagate_component_auth(
    identity: &Identity,
    caller: ComponentId,
    callee_is_root: bool,
) -> Identity {
    if identity.is_admin() {
        return identity.clone();
    }
    if caller.is_root() && callee_is_root {
        identity.clone()
    } else {
        Identity::Unknown
    }
}
