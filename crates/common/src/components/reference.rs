use std::str::FromStr;

use sync_types::UdfPath;
use value::identifier::Identifier;

/// `References` are relative paths to `Resources` that start at some
/// component.
#[derive(Clone, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(Debug, proptest_derive::Arbitrary)
)]
pub enum Reference {
    /// Reference originating from `component.args` at component definition time
    /// or `ctx.componentArgs` at runtime.
    ///
    /// Definition time:
    /// ```ts
    /// const component = new ComponentDefinition({ maxLength: v.number() });
    /// const { maxLength } = component.args;
    /// // => Reference::ComponentArgument { attributes: vec!["maxLength"]}
    /// ```
    ///
    /// Runtime:
    /// ```ts
    /// import { query } from "./_generated/server";
    ///
    /// export const f = query(async (ctx, args) => {
    ///   console.log(ctx.componentArgs.maxLength);
    /// })
    /// ```
    ComponentArgument { attributes: Vec<Identifier> },

    /// Reference originating from the `api` object, either at definition time
    /// or runtime.
    ///
    /// Definition time:
    /// ```ts
    /// import { api } from "./_generated/server";
    ///
    /// const f = api.foo.bar;
    /// // => Reference::Function("foo:bar")
    /// ```
    ///
    /// Runtime:
    /// ```ts
    /// import { api, mutation } from "./_generated/server";
    ///
    /// export const f = mutation(async (ctx, args) => {
    ///   await ctx.runAfter(0, api.foo.bar);
    /// });
    /// ```
    Function(UdfPath),

    /// Reference originating from `component.childComponents` at definition
    /// time or the generated component string builders in
    /// `_generated/server` at runtime.
    ///
    /// Definition time:
    /// ```ts
    /// const component = new ComponentDefinition();
    /// const wl = component.use(waitlist);
    /// // => Reference::Subcomponent { attributes: vec!["waitlist"]}
    ///
    /// const f = wl.foo.bar;
    /// // => Reference::Subcomponent { attributes: vec!["waitlist", "foo", "bar"]}
    /// ```
    Subcomponent { attributes: Vec<Identifier> },
}

impl FromStr for Reference {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut path_components = s.split('/');
        anyhow::ensure!(
            path_components.next() == Some("_reference"),
            "Invalid reference: {s}"
        );
        let result = match path_components.next() {
            Some("componentArgument") => {
                let attributes = path_components
                    .map(|s| s.parse())
                    .collect::<Result<_, _>>()?;
                Reference::ComponentArgument { attributes }
            },
            Some("function") => {
                let remainder = path_components
                    .remainder()
                    .ok_or_else(|| anyhow::anyhow!("Invalid reference {s}"))?;
                let path = remainder.parse()?;
                Reference::Function(path)
            },
            Some("subcomponent") => {
                let attributes = path_components
                    .map(|s| s.parse())
                    .collect::<Result<_, _>>()?;
                Reference::Subcomponent { attributes }
            },
            Some(_) | None => anyhow::bail!("Invalid reference {s}"),
        };
        Ok(result)
    }
}

impl From<Reference> for String {
    fn from(value: Reference) -> Self {
        let mut s = "_reference".to_string();
        match value {
            Reference::ComponentArgument { attributes } => {
                s.push_str("/componentArgument");
                for attribute in attributes {
                    s.push('/');
                    s.push_str(&attribute);
                }
            },
            Reference::Function(path) => {
                s.push_str("/function");
                s.push('/');
                s.push_str(&path.to_string());
            },
            Reference::Subcomponent { attributes } => {
                s.push_str("/subcomponent");
                for attribute in attributes {
                    s.push('/');
                    s.push_str(&attribute);
                }
            },
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::Reference;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_reference_roundtrips(reference in any::<Reference>()) {
            let s = String::from(reference.clone());
            assert_eq!(s.parse::<Reference>().unwrap(), reference);
        }
    }
}
