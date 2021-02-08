use proc_macro::TokenStream;
/// Tonic middleware macro
use proc_macro_error::abort;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{
    parse_macro_input, AttributeArgs, FnArg, GenericArgument, Ident, ImplItem, ImplItemMethod,
    ItemImpl, Meta, NestedMeta, Path, PathArguments, ReturnType, Type, TypePath,
};

/// Implement one RPC method
struct Rpc {
    name: Ident,
    middlewares: Vec<Path>,
    request_type: Type,
    response_type: Type,
    error_type: Type,
    real_impl: ImplItemMethod,
    all_middlewares: Option<Vec<Path>>,
}

impl Rpc {
    fn from_input(input: ImplItemMethod) -> Self {
        let mut real_impl = input.clone();
        // Remove macro attributes
        real_impl.attrs.clear();

        let signature = input.sig;
        let attrs = input.attrs;
        let name = signature.ident;
        let args_type = signature.inputs;
        let ret_type = signature.output;

        let request_type = match &args_type[2] {
            FnArg::Typed(pt) => (&*pt.ty).clone(),
            t => {
                abort! { t, "Only 'normal' function arguments are authorized here"; note = ""; help = ""; }
            }
        };
        let ret_type = match ret_type {
            ReturnType::Type(_, t) => (&*t).clone(),
            t => {
                abort! { t, "Method need a return type"; note = ""; help = ""; }
            }
        };
        let ret_type = match ret_type {
            Type::Path(tp) => tp,
            t => {
                abort! { t, "Need to return a Result"; note = ""; help = ""; }
            }
        };

        let (response_type, error_type) = match &ret_type.path.segments[0].arguments {
            PathArguments::AngleBracketed(aba) => {
                let response_type = match &aba.args[0] {
                    GenericArgument::Type(t) => t,
                    t => {
                        abort! { t, "Should be a type"; note = ""; help = ""; }
                    }
                };
                let error_type = match &aba.args[1] {
                    GenericArgument::Type(t) => t,
                    t => {
                        abort! { t, "Should be a type"; note = ""; help = ""; }
                    }
                };
                (response_type.clone(), error_type.clone())
            }
            t => {
                abort! { t, "Return type should be Result<Response, Error>"; note = ""; help = ""; }
            }
        };

        // Middlewares
        let mut middlewares = Vec::new();
        for attr in attrs {
            match attr.parse_meta() {
                Ok(meta) => {
                    let p = meta.path();

                    // We're only interested in middleware attributes
                    if quote! { #p }.to_string() == "middleware" {
                        match meta {
                            Meta::List(mts) => {
                                for mt in mts.nested.iter() {
                                    match mt {
                                        NestedMeta::Meta(m) => {
                                            middlewares.push(m.path().clone());
                                        }
                                        t => {
                                            abort! {t, "Needs to impl middleware trait"; note = ""; help = ""; }
                                        }
                                    };
                                }
                            }
                            t => {
                                abort! { t, "Need to be a middleware type list (A, B, ...)"; note = ""; help = ""; }
                            }
                        };
                    }
                }
                Err(_) => {}
            };
        }

        Self {
            name,
            middlewares,
            request_type,
            response_type,
            error_type,
            real_impl,
            all_middlewares: None,
        }
    }
}

impl ToTokens for Rpc {
    fn to_tokens(&self, tokens: &mut ::proc_macro2::TokenStream) {
        let rpc_name = &self.name;
        let request_type = &self.request_type;
        let response_type = &self.response_type;
        let _error_type = &self.error_type;

        let md_mappings: Vec<usize> = self
            .middlewares
            .iter()
            .map(|md| {
                self.all_middlewares.as_ref().unwrap().iter().position(|a| a == md).unwrap_or_else(
                    || {
                        abort! { md, "Impossible macro error"; note = ""; help = ""; }
                    },
                )
            })
            .collect();

        let md_calls: Vec<(::proc_macro2::TokenStream, ::proc_macro2::TokenStream, ::proc_macro2::TokenStream)> = md_mappings.iter().map(|i| {
            let i = format_ident!("md_{}", i);
            (quote! { let mut #i = self.#i.before_request(&request).await.map_err(|e| tonic::Status::from(e))?; },
             quote! { &mut #i },
             quote! { self.#i.after_request(#i, &ret).await; })
        }).collect();
        let md_calls_before = md_calls.iter().map(|i| &i.0);
        let md_calls_env = md_calls.iter().map(|i| &i.1);
        let md_calls_after = md_calls.iter().map(|i| &i.2);

        let res = quote! {
            async fn #rpc_name(&self, request: #request_type) -> ::core::result::Result<#response_type, ::tonic::Status> {
                //Per middleware
                #(#md_calls_before)
                *
                let envs = (#(#md_calls_env),*);
                // Call impl
                let ret = self.service_impl.#rpc_name(envs, request).await.map_err(|e| e.into());

                //Per middleware
                #(#md_calls_after)
                *
                ret
            }
        };
        tokens.append_all(res);
    }
}

/// Impl service
struct Service {
    service_name_type: TypePath,
    rpcs: Vec<Rpc>,
    middlewares: Vec<Path>,
}

struct TMGImpl {
    args: TMGArgs,
    service: Service,
}

impl TMGImpl {
    fn from_args_and_input(args: TMGArgs, input: ItemImpl) -> Self {
        let service_name_type = match *input.self_ty {
            Type::Path(tp) => tp,
            t => {
                abort! { t, "Need to implement a type"; note = ""; help = "Define an impl block implementing rpc methods"; }
            }
        };

        let mut rpcs: Vec<Rpc> = input
            .items
            .into_iter()
            .filter_map(|i| {
                let m = match i {
                    ImplItem::Method(m) => m,
                    ImplItem::Macro(m) => {
                        abort! {m, "Macro not implemented yet"; note = ""; help = ""; }
                    }
                    t => {
                        abort! {t, "Nothing but rpc method in this block"; note = ""; help = ""; }
                    }
                };
                Some(Rpc::from_input(m))
            })
            .collect();

        let mut middlewares: Vec<Path> =
            rpcs.iter().map(|r| r.middlewares.clone()).fold(Vec::new(), |mut acc, v| {
                acc.extend(v);
                acc
            });
        middlewares.sort_by(|a, b| {
            let a = quote! { #a }.to_string();
            let b = quote! { #b }.to_string();
            a.cmp(&b)
        });
        middlewares.dedup();
        for mut r in rpcs.iter_mut() {
            r.all_middlewares = Some(middlewares.clone());
        }

        let service = Service { service_name_type, rpcs, middlewares };

        Self { args, service }
    }
}

impl ToTokens for TMGImpl {
    fn to_tokens(&self, tokens: &mut ::proc_macro2::TokenStream) {
        let Self { ref args, ref service } = *self;
        let Service { ref service_name_type, ref rpcs, ref middlewares } = *service;
        let tonic_service_trait = &args.tonic_service_trait;
        let tonic_server = &args.tonic_server;
        let service_name_type_str = quote! { #service_name_type }.to_string();
        let middleware_service_wrapper_name =
            format_ident!("{}TMProcMacroMiddlewareWrapper", service_name_type_str);

        let method_real_impls: Vec<::proc_macro2::TokenStream> = rpcs
            .iter()
            .map(|r| {
                let r = &r.real_impl;
                quote! { #r }
            })
            .collect();
        let methods: Vec<::proc_macro2::TokenStream> = rpcs.iter().map(|r| quote! { #r }).collect();

        // All middlewares
        let middlewares_def: Vec<(::proc_macro2::TokenStream, ::proc_macro2::TokenStream)> =
            middlewares
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let i = format_ident!("md_{}", i);
                    (quote! { #i }, quote! { #i: #t })
                })
                .collect();
        let middlewares_def_names = middlewares_def.iter().map(|i| &i.0);
        let middlewares_def_types = middlewares_def.iter().map(|i| &i.1);
        let middlewares_def_types2 = middlewares_def_types.clone();

        // Here some of the magic happens
        // Expand methods see https://docs.rs/proc-quote/0.3.2/proc_quote/macro.quote.html
        let res = quote! {
            /// Middleware wrapper
            pub struct #middleware_service_wrapper_name {
                service_impl: #service_name_type,
                #(#middlewares_def_types),
                *
            }

            /// Middleware boilerplate impl
            #[::tonic::async_trait]
            impl #tonic_service_trait for #middleware_service_wrapper_name {
                #(#methods)
                *
            }

            // Convert to service middleware
            impl #service_name_type {
                pub fn to_service_middleware(self, #(#middlewares_def_types2),*) -> #tonic_server<#middleware_service_wrapper_name> {
                    #tonic_server::new(#middleware_service_wrapper_name {
                        service_impl: self,
                        #(#middlewares_def_names),*
                    })
                }
            }

            // Real impls
            impl #service_name_type {
                #(#method_real_impls)
                *
            }

        };
        tokens.append_all(res);
    }
}

struct TMGArgs {
    tonic_service_trait: Ident,
    tonic_server: Ident,
}

impl TMGArgs {
    fn from_args(tokens: AttributeArgs) -> Self {
        let tonic_service_trait = match &tokens[0] {
            NestedMeta::Meta(m) => match m {
                Meta::Path(p) => p.segments[0].ident.clone(),
                t => {
                    abort! {t, "Needs to be tonic service trait"; note = ""; help = ""; }
                }
            },
            t => {
                abort! {t, "Needs to be tonic service trait"; note = ""; help = ""; }
            }
        };

        let tonic_server = match &tokens[1] {
            NestedMeta::Meta(m) => match m {
                Meta::Path(p) => p.segments[0].ident.clone(),
                t => {
                    abort! {t, "Needs to be tonic service trait"; note = ""; help = ""; }
                }
            },
            t => {
                abort! {t, "Needs to be tonic service trait"; note = ""; help = ""; }
            }
        };

        Self { tonic_service_trait, tonic_server }
    }
}

pub(crate) fn tonic_middleware(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemImpl);
    let args = TMGArgs::from_args(args);
    let tmg = TMGImpl::from_args_and_input(args, input);
    let res = quote! { #tmg };
//     eprintln!("{}", res.to_string());
    TokenStream::from(res)
}
