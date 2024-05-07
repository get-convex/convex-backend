// All components under a component have a unique `ComponentName`. For example,
// the root app component may have a waitlist component identified by
// "chatWaitlist".
#[allow(unused)]
pub struct ComponentName {
    name: String,
}

// Path within the component tree for a particular component. Note that this
// path can potentially change when the component tree changes during a push, so
// developers should resolve this path to a `ComponentId` within a transaction
// as soon as possible.
#[allow(unused)]
pub struct ComponentPath {
    path: Vec<ComponentName>,
}
