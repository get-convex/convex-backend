use value::identifier::Identifier;

// Path within the component tree for a particular component. Note that this
// path can potentially change when the component tree changes during a push, so
// we should resolve this path to a `ComponentId` within a transaction
// as soon as possible.
#[allow(unused)]
pub struct ComponentPath {
    path: Vec<Identifier>,
}
