use proc_macro::TokenStream;
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
    let r#gen = quote! {
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
    r#gen.into()
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
    let is_pauseable = if let Some(arg1) = args.get(1) {
        assert!(
            matches!(arg1, FnArg::Typed(pat) if matches!(&*pat.ty, syn::Type::Path(p) if p.path.is_ident("PauseController")))
        );
        true
    } else {
        false
    };
    let run_test = if is_pauseable {
        quote! {
            let (__pause_controller, __pause_client) = ::common::pause::PauseController::new();
            let mut __test_driver = ::runtime::testing::TestDriver::new_with_pause_client(
                __pause_client
            );
            let rt = __test_driver.rt();
            let test_future = #name(rt, __pause_controller);
            __test_driver.run_until(test_future)
        }
    } else {
        quote! {
            let mut __test_driver = ::runtime::testing::TestDriver::new();
            let rt = __test_driver.rt();
            let test_future = #name(rt);
            __test_driver.run_until(test_future)
        }
    };

    let attrs = ast.attrs.iter();
    let r#gen = quote! {
        #[test]
        #( #attrs )*
        fn #name() #output {
            #ast
            // Set a consistent thread stack size regardless of environment.
            let builder = std::thread::Builder::new().stack_size(
                *::common::knobs::RUNTIME_STACK_SIZE);
            let handler = builder
                .spawn(|| {
                    #run_test
                })
                .unwrap();
            handler.join().unwrap()
        }
    };
    r#gen.into()
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
        ident,
        generics,
        inputs,
        output,
        ..
    } = sig;

    let r#gen = quote! {
        #(#attrs)*
        #vis async fn #ident #generics (#inputs) #output {
            ::common::run_instrumented!(
                #ident,
                #block
            )
        }
    };
    r#gen.into()
}

/// Use as #[convex_macro::v8_op] to annotate "ops" (Rust code callable from
/// JavaScript that is shipped with backend).
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
        ident,
        generics,
        inputs,
        output,
        ..
    } = sig;

    let Some(FnArg::Typed(first_pat_type)) = inputs.first() else {
        panic!("op should take a first argument for its op provider");
    };
    let Pat::Ident(first_pat_ident) = &*first_pat_type.pat else {
        panic!("op's first argument should be a plain identifier");
    };
    let provider_ident = &first_pat_ident.ident;

    let arg_pats: Vec<_> = inputs
        .iter()
        .skip(1)
        .map(|input| {
            let FnArg::Typed(pat) = input else {
                panic!("input must be typed")
            };
            &pat.pat
        })
        .collect();
    let arg_parsing: Vec<_> = inputs
        .iter()
        .enumerate()
        .skip(1)
        .map(|(idx, input)| {
            let idx = idx as i32;
            let arg_info = format!("{ident} arg{idx}");
            let FnArg::Typed(pat) = input else {
                panic!("input must be typed")
            };
            let ty = &pat.ty;
            // NOTE: deno has special case when pat.ty is &mut [u8].
            // While that would make some ops more efficient, it also makes them
            // unsafe because it's hard to prove that the same buffer isn't
            // being mutated from multiple ops in parallel or multiple arguments
            // on the same op.
            //
            // Forego all special casing and just use serde_v8.
            quote! {
                {
                    let __raw_arg = __args.get(#idx);
                    let __arg: #ty = ::deno_core::serde_v8::from_v8(
                        __scope,
                        __raw_arg,
                    ).context(#arg_info)?;
                    __arg
                }
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

    let r#gen = quote! {
        #(#attrs)*
        #vis fn #ident #generics (
            #first_pat_type,
            __args: ::deno_core::v8::FunctionCallbackArguments,
            mut __rv: ::deno_core::v8::ReturnValue,
        ) -> ::anyhow::Result<()> {
            #[allow(clippy::unused_unit)]
            let ( #(#arg_pats,)*) = {
                let mut __scope = OpProvider::scope(#provider_ident);
                ::deno_core::v8::scope!(let __scope, &mut __scope);
                (#(#arg_parsing,)*)
            };
            let __result_v = (|| #output { #block })()?;
            {
                let mut __scope = OpProvider::scope(#provider_ident);
                ::deno_core::v8::scope!(let __scope, &mut __scope);
                let __value_v8 = deno_core::serde_v8::to_v8(__scope, __result_v)?;
                __rv.set(__value_v8);
            }
            Ok(())
        }
    };
    r#gen.into()
}
