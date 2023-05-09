use darling::FromMeta;
use heck::ToPascalCase;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::Parser;
use syn::parse_macro_input;
use syn::parse_macro_input::ParseMacroInput;
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
pub fn redscript_global(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let fn_item = parse_macro_input!(item as syn::ForeignItemFn);
    let args = parse_macro_input!(_attr as syn::AttributeArgs);
    let name = fn_item.sig.ident.to_string().to_pascal_case();
    generate_forwader(&name, fn_item.attrs, args, fn_item.vis, fn_item.sig).into()
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

    let items = items.into_iter().map(|i| match i {
        syn::ImplItem::Method(method) => generate_forwader(
            &method.sig.ident.to_string().to_pascal_case(),
            method.attrs,
            vec![],
            method.vis,
            method.sig,
        ),
        syn::ImplItem::Verbatim(tokens) => match syn::parse2::<syn::ForeignItemFn>(tokens) {
            Ok(fn_item) => generate_forwader(
                &fn_item.sig.ident.to_string().to_pascal_case(),
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
    attrs: Vec<syn::Attribute>,
    meta: Vec<syn::NestedMeta>,
    vis: syn::Visibility,
    sig: syn::Signature,
) -> proc_macro2::TokenStream {
    let (attrs, other_attrs): (Vec<_>, Vec<_>) = attrs
        .into_iter()
        .partition(|el| el.path.get_ident().map(ToString::to_string).as_deref() == Some(ATTR_KEY));

    let meta: Vec<_> = attrs
        .into_iter()
        .flat_map(|attr| {
            attr.parse_args_with(syn::AttributeArgs::parse)
                .unwrap_or_default()
        })
        .chain(meta)
        .collect();

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
                    types.push(pat.ty.clone());
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
    let name = syn::LitStr::new(name, sig.span());

    fn into_repr_name(typ: &syn::Type) -> proc_macro2::TokenStream {
        quote!(<<#typ as ::red4ext_rs::conv::IntoRepr>::Repr as ::red4ext_rs::conv::NativeRepr>::MANGLED_NAME)
    }

    fn from_repr_name(typ: &syn::Type) -> proc_macro2::TokenStream {
        quote!(<<#typ as ::red4ext_rs::conv::FromRepr>::Repr as ::red4ext_rs::conv::NativeRepr>::MANGLED_NAME)
    }

    let signature = if attrs.operator {
        let args = types.iter().map(|typ| into_repr_name(typ));
        let ret = match &sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, typ) => Some(from_repr_name(typ)),
        };
        quote!(::red4ext_rs::macros::concat_str!(#name, ";", #(#args),*, ";", #ret))
    } else if attrs.cb || attrs.native {
        name.to_token_stream()
    } else {
        let args = types.iter().map(|typ| into_repr_name(typ));
        quote!(::red4ext_rs::macros::concat_str!(#name, ";", #(#args),*))
    };

    let ret = &sig.output;
    let body = match sig.inputs.first() {
        Some(syn::FnArg::Receiver(syn::Receiver { self_token, .. })) => {
            quote!(::red4ext_rs::call!(#self_token.0.clone(), [#signature] (#(#idents),*) #ret))
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

#[proc_macro]
pub fn concat_str(ts: TokenStream) -> TokenStream {
    let exprs = match syn::punctuated::Punctuated::<syn::Expr, syn::token::Comma>::parse_terminated
        .parse(ts)
    {
        Ok(res) => res,
        Err(err) => return err.to_compile_error().into(),
    };
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
    quote! {unsafe { ::std::str::from_utf8_unchecked(&#array) }}.into()
}
