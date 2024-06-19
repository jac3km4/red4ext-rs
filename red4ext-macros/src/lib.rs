#![allow(clippy::manual_unwrap_or_default)]
use darling::ast::NestedMeta;
use darling::FromMeta;
use heck::ToPascalCase;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse_macro_input;
use syn::spanned::Spanned;

const ATTR_KEY: &str = "redscript";

#[derive(Debug, FromMeta)]
struct FunctionAttrs {
    name: Option<String>,
    full_name: Option<String>,
    #[darling(default)]
    native: bool,
    #[darling(default)]
    cb: bool,
    #[darling(default)]
    operator: bool,
}

#[proc_macro_attribute]
pub fn redscript_global(attr: TokenStream, item: TokenStream) -> TokenStream {
    let fn_item = parse_macro_input!(item as syn::ForeignItemFn);
    let args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };
    let name = fn_item.sig.ident.to_string().to_pascal_case();
    generate_forwader(&name, None, fn_item.attrs, args, fn_item.vis, fn_item.sig).into()
}

#[proc_macro_attribute]
pub fn redscript_import(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemImpl);

    let syn::ItemImpl {
        attrs,
        defaultness,
        unsafety,
        impl_token,
        generics,
        trait_,
        self_ty,
        items,
        ..
    } = item;
    if let Some((_, path, _)) = trait_ {
        return syn::Error::new(path.span(), "Cannot import a trait impl")
            .to_compile_error()
            .into();
    }

    let syn::Type::Path(self_path) = &*self_ty else {
        return syn::Error::new(self_ty.span(), "Expected a path")
            .to_compile_error()
            .into();
    };
    let Some(self_ident) = self_path.path.get_ident() else {
        return syn::Error::new(self_path.path.span(), "Expected a type identifier")
            .to_compile_error()
            .into();
    };

    let items = items.into_iter().map(|i| match i {
        syn::ImplItem::Verbatim(tokens) => match syn::parse2::<syn::ForeignItemFn>(tokens) {
            Ok(fn_item) => generate_forwader(
                &fn_item.sig.ident.to_string().to_pascal_case(),
                Some(&self_ident.to_string()),
                fn_item.attrs,
                vec![],
                fn_item.vis,
                fn_item.sig,
            ),
            Err(err) => err.to_compile_error(),
        },
        other => other.into_token_stream(),
    });

    quote! {
        #(#attrs)* #defaultness #unsafety #impl_token #generics #self_ty {
            #(#items)*
        }
    }
    .into()
}

fn generate_forwader(
    fn_name: &str,
    parent: Option<&str>,
    attrs: Vec<syn::Attribute>,
    meta: Vec<NestedMeta>,
    vis: syn::Visibility,
    sig: syn::Signature,
) -> proc_macro2::TokenStream {
    let (attrs, other_attrs): (Vec<_>, Vec<_>) = attrs.into_iter().partition(|el| {
        el.meta
            .require_list()
            .ok()
            .and_then(|l| l.path.get_ident())
            .map(ToString::to_string)
            .as_deref()
            == Some(ATTR_KEY)
    });

    let nested_meta = attrs.into_iter().flat_map(|attr| {
        attr.meta
            .require_list()
            .ok()
            .and_then(|l| NestedMeta::parse_meta_list(l.tokens.clone()).ok())
            .unwrap_or_default()
    });
    let meta: Vec<_> = meta.into_iter().chain(nested_meta).collect();

    let attrs = match FunctionAttrs::from_list(&meta) {
        Ok(res) => res,
        Err(err) => return err.write_errors(),
    };

    let mut types = vec![];
    let mut idents = vec![];
    for arg in &sig.inputs {
        match arg {
            syn::FnArg::Typed(pat) => match &*pat.pat {
                syn::Pat::Ident(id) => {
                    idents.push(id);
                    types.push(&*pat.ty);
                }
                _ => {
                    return syn::Error::new(arg.span(), "Only plain parameters are supported")
                        .to_compile_error()
                }
            },
            syn::FnArg::Receiver(_) => {}
        }
    }

    let name = attrs
        .full_name
        .as_deref()
        .or(attrs.name.as_deref())
        .unwrap_or(fn_name);

    let receiver = sig.inputs.first().and_then(|arg| match arg {
        syn::FnArg::Receiver(receiver) => Some(receiver),
        _ => None,
    });
    let signature = generate_name(name, receiver, parent, &types, &sig.output, &attrs);
    let ret = &sig.output;
    let body = match (receiver, parent, &attrs) {
        (Some(syn::Receiver { self_token, .. }), Some(_), _) => {
            quote!(::red4ext_rs::call!(#self_token, [#signature] (#(#idents),*) #ret))
        }
        (None, Some(_), FunctionAttrs { native: false, .. }) => {
            quote!(::red4ext_rs::call!([#signature] (#(#idents),*) #ret))
        }
        (None, Some(_), _) => {
            quote!(::red4ext_rs::call!([#parent] :: [#name] (#(#idents),*) #ret))
        }
        _ => {
            quote!(::red4ext_rs::call!([#signature] (#(#idents),*) #ret))
        }
    };
    quote! {
        #(#other_attrs)* #vis #sig {
            #body
        }
    }
}

fn generate_name(
    name: &str,
    receiver: Option<&syn::Receiver>,
    parent: Option<&str>,
    args: &[&syn::Type],
    ret: &syn::ReturnType,
    attrs: &FunctionAttrs,
) -> proc_macro2::TokenStream {
    fn into_repr_name(typ: &syn::Type) -> proc_macro2::TokenStream {
        quote!(<<#typ as ::red4ext_rs::conv::IntoRepr>::Repr as ::red4ext_rs::conv::NativeRepr>::MANGLED_NAME)
    }

    fn from_repr_name(typ: &syn::Type) -> proc_macro2::TokenStream {
        quote!(<<#typ as ::red4ext_rs::conv::FromRepr>::Repr as ::red4ext_rs::conv::NativeRepr>::MANGLED_NAME)
    }

    let mut components = vec![];
    if let (None, Some(parent), FunctionAttrs { native: false, .. }) = (receiver, parent, &attrs) {
        components.extend([quote!(#parent), quote!("::")]);
    }
    components.push(quote!(#name));
    if !attrs.cb && !attrs.native || attrs.operator {
        components.push(quote!(";"));
        components.extend(args.iter().map(|typ| into_repr_name(typ)));
    }
    match &ret {
        syn::ReturnType::Type(_, typ) if attrs.operator => {
            components.extend([quote!(";"), from_repr_name(typ)]);
        }
        _ => {}
    };

    concat_str(components)
}

fn concat_str(exprs: Vec<proc_macro2::TokenStream>) -> proc_macro2::TokenStream {
    let len = exprs
        .iter()
        .fold(quote!(0), |acc, e| quote!(#acc + #e.len()));
    let it = exprs.iter();
    let array = quote! {{
        let mut buf = [0u8; #len];
        let mut i = 0;
        #(
            buf[i..i + #it.len()].copy_from_slice(#it.as_bytes());
            i += #it.len();
        )*
        buf
    }};
    quote! {unsafe { ::std::str::from_utf8_unchecked(&#array) }}
}
