use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    FnArg,
    GenericArgument,
    ItemFn,
    Pat,
    PathArguments,
    PathSegment,
    ReturnType,
    Signature,
    Type,
};

/// Macro to use for tests that need ProdRuntime and tokio runtime.
/// Example:
/// ```
/// #[convex_macro::prod_rt_test]
/// async fn test_database(rt: ProdRuntime) -> anyhow::Result<()> {
///     // Supports tokio-postgres and await.
///     let TestDbSetup { _postgres_url, .. } = setup_db().await?;
///     // Gives a runtime argument for passing to libraries.
///     let _db = new_test_database(rt).await?;
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn prod_rt_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = syn::parse(item).unwrap();
    let sig = &ast.sig;
    let name = &sig.ident;
    sig.asyncness
        .expect("#[prod_rt_test] only works on async functions");
    let args = &sig.inputs;
    let output = &sig.output;
    let Some(FnArg::Typed(_)) = args.first() else {
        panic!("#[prod_rt_test] requires `{name}` to have `rt: ProdRuntime` as the first arg");
    };
    let attrs = ast.attrs.iter();
    let gen = quote! {
        #[test]
        #( #attrs )*
        fn #name() #output {
            #ast
            // Set a consistent thread stack size regardless of environment.
            let builder = std::thread::Builder::new().stack_size(
                *::common::knobs::RUNTIME_STACK_SIZE);
            let handler = builder
                .spawn(|| {
                    let tokio = ::runtime::prod::ProdRuntime::init_tokio()?;
                    let rt = ::runtime::prod::ProdRuntime::new(&tokio);
                    let rt2 = rt.clone();
                    let test_future = #name(rt);
                    rt2.block_on("test", test_future)
                })
                .unwrap();
            handler.join().unwrap()
        }
    };
    gen.into()
}

/// Macro to use for tests that need TestRuntime.
/// Example:
/// ```
/// #[convex_macro::test_runtime]
/// async fn test_database(rt: TestRuntime) -> anyhow::Result<()> {
///     // Gives a runtime argument for passing to libraries, and supports await.
///     let _db = new_test_database(rt).await?;
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn test_runtime(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = syn::parse(item).unwrap();
    let sig = &ast.sig;
    let name = &sig.ident;
    sig.asyncness
        .expect("#[test_runtime] only works on async functions");
    let args = &sig.inputs;
    let output = &sig.output;
    let Some(FnArg::Typed(_)) = args.first() else {
        panic!("#[test_runtime] requires `{name}` to have `rt: TestRuntime` as the first arg");
    };
    let attrs = ast.attrs.iter();
    let gen = quote! {
        #[test]
        #( #attrs )*
        fn #name() #output {
            #ast
            // Set a consistent thread stack size regardless of environment.
            let builder = std::thread::Builder::new().stack_size(
                *::common::knobs::RUNTIME_STACK_SIZE);
            let handler = builder
                .spawn(|| {
                    let mut __test_driver = ::runtime::testing::TestDriver::new();
                    let rt = __test_driver.rt();
                    let test_future = #name(rt);
                    __test_driver.run_until(test_future)
                })
                .unwrap();
            handler.join().unwrap()
        }
    };
    gen.into()
}

#[proc_macro_attribute]
pub fn instrument_future(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        ref attrs,
        ref vis,
        ref sig,
        ref block,
    } = syn::parse(item).unwrap();

    assert!(sig.constness.is_none(), "Can't instrument const fn");
    assert!(sig.asyncness.is_some(), "Can only instrument async fn");
    assert!(sig.unsafety.is_none(), "Can't instrument unsafe fn");
    assert!(sig.abi.is_none(), "Can't instrument fn with explicit ABI");
    assert!(
        sig.variadic.is_none(),
        "Can't instrument fn with variadic arguments"
    );

    let Signature {
        ref ident,
        ref generics,
        ref inputs,
        ref output,
        ..
    } = sig;

    let gen = quote! {
        #(#attrs)*
        #vis async fn #ident #generics (#inputs) #output {
            let __instrument_name = ::common::tracing::cstr!(#ident);
            let __instrument_loc = ::common::span_location!();
            let future = async move {
                #block
            };
            ::common::tracing::InstrumentedFuture::new(
                future,
                __instrument_name,
                __instrument_loc,
            ).await
        }
    };
    gen.into()
}

/// Use as #[convex_macro::v8_op] to annotate "ops" (Rust code callable from
/// Javascript that is shipped with backend).
/// Must be used within the `isolate` crate.
///
/// Types:
/// Arguments and return value can be anything that implements
/// `serde::Serialize`. TODO: support &str and &mut [u8].
///
/// Note: Option::None in return values is encoded as `null` (not
/// undefined), while both `null` and `undefined` (and missing positional)
/// arguments become None.
///
/// The function should be called as `op_name(provider, args, rt)?`.
#[proc_macro_attribute]
pub fn v8_op(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        ref attrs,
        ref vis,
        ref sig,
        ref block,
    } = syn::parse(item).unwrap();

    assert!(sig.constness.is_none(), "const fn cannot be op");
    assert!(sig.asyncness.is_none(), "async fn cannot be op");
    assert!(sig.unsafety.is_none(), "unsafe fn cannot be op");
    assert!(sig.abi.is_none(), "fn with explicit ABI cannot be op");
    assert!(
        sig.variadic.is_none(),
        "fn with variadic arguments cannot be op"
    );

    let Signature {
        ref ident,
        ref generics,
        ref inputs,
        ref output,
        ..
    } = sig;

    let Some(FnArg::Typed(first_pat_type)) = inputs.first() else {
        panic!("op should take a first argument for its op provider");
    };
    let Pat::Ident(first_pat_ident) = &*first_pat_type.pat else {
        panic!("op's first argument should be a plain identifier");
    };
    let provider_ident = &first_pat_ident.ident;

    let arg_parsing: TokenStream2 = inputs
        .iter()
        .enumerate()
        .skip(1)
        .map(|(idx, input)| {
            let idx = idx as i32;
            let FnArg::Typed(pat) = input else {
                panic!("input must be typed")
            };
            let arg_info = format!("{} arg{}", ident, idx);
            // NOTE: deno has special case when pat.ty is &mut [u8].
            // While that would make some ops more efficient, it also makes them
            // unsafe because it's hard to prove that the same buffer isn't
            // being mutated from multiple ops in parallel or multiple arguments
            // on the same op.
            //
            // Forego all special casing and just use serde_v8.
            quote! {
                let #pat = {
                    let __raw_arg = __args.get(#idx);
                    ::deno_core::serde_v8::from_v8(
                        &mut __scope,
                        __raw_arg,
                    ).context(#arg_info)?
                };
            }
        })
        .collect();

    let ReturnType::Type(_, return_type) = output else {
        panic!("op needs return type");
    };
    let Type::Path(rtype_path) = &**return_type else {
        panic!("op must return anyhow::Result<...>")
    };
    let PathSegment {
        ident: retval_type,
        arguments: retval_arguments,
    } = rtype_path.path.segments.last().unwrap();
    assert_eq!(&retval_type.to_string(), "Result");
    let PathArguments::AngleBracketed(retval_arguments) = retval_arguments else {
        panic!("op must return anyhow::Result<...>")
    };
    let GenericArgument::Type(_retval_type) = retval_arguments
        .args
        .last()
        .expect("op must return anyhow::Result<...>")
    else {
        panic!("op must return anyhow::Result<...>");
    };

    let gen = quote! {
        #(#attrs)*
        #vis fn #ident #generics (
            #first_pat_type,
            __args: ::deno_core::v8::FunctionCallbackArguments,
            mut __rv: ::deno_core::v8::ReturnValue,
        ) -> ::anyhow::Result<()> {
            let mut __scope = ::deno_core::v8::HandleScope::new(OpProvider::scope(#provider_ident));
            #arg_parsing
            drop(__scope);
            let __result_v = (|| #output { #block })()?;
            {
                let mut __scope = ::deno_core::v8::HandleScope::new(
                    OpProvider::scope(#provider_ident),
                );
                let __value_v8 = deno_core::serde_v8::to_v8(&mut __scope, __result_v)?;
                __rv.set(__value_v8);
            }
            Ok(())
        }
    };
    gen.into()
}
