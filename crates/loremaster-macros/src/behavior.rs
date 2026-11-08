//! `#[behavior]`: a behavior from a function (DSL step L5). The macro keeps
//! the function and adds a unit struct with a `Behavior` impl that calls it.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, Parser};
use syn::{Error, Result, Token};

/// The event kinds that `on` takes, as `EventKind` names them.
const KINDS: &[&str] = &[
    "start", "turn", "enter", "leave", "time", "verb", "timer", "custom", "call", "talk",
];

pub fn expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let f: syn::ItemFn = syn::parse2(item)?;
    let on = on.parse2(attr)?;
    if f.sig.inputs.len() != 1 {
        return Err(Error::new_spanned(
            &f.sig.inputs,
            "a behavior function takes one argument: `cx: &mut Cx<'_>`",
        ));
    }
    let name = &f.sig.ident;
    let id = name.to_string().trim_start_matches("r#").to_string();
    let ty = format_ident!("{}", camel(&id), span = name.span());
    let doc = f
        .attrs
        .iter()
        .filter_map(|a| match &a.meta {
            syn::Meta::NameValue(nv) if nv.path.is_ident("doc") => match &nv.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => Some(s.value().trim().to_string()),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    let vis = &f.vis;
    let rt = quote!(::loremaster::dsl::runtime);
    let (manifest, guard) = match &on {
        None => (quote!(#rt::Manifest::opaque()), quote!()),
        Some(kinds) => (
            quote!(#rt::Manifest::opaque().events([#(#rt::EventKind::#kinds),*])),
            quote! {
                if !::core::matches!(event.kind(), #(#rt::EventKind::#kinds)|*) {
                    return #rt::Flow::Continue;
                }
            },
        ),
    };
    let struct_doc = format!("The behavior `{id}` of the function [`{id}`].");
    Ok(quote! {
        #f

        #[doc = #struct_doc]
        #[derive(Debug, Clone, Copy, Default)]
        #vis struct #ty;

        impl #rt::Behavior for #ty {
            fn id(&self) -> &str {
                #id
            }

            fn description(&self) -> &str {
                #doc
            }

            fn manifest(&self) -> #rt::Manifest {
                #manifest
            }

            #[allow(unused_variables)]
            fn handle(&self, event: &#rt::Event, cx: &mut #rt::Cx<'_>) -> #rt::Flow {
                #guard
                ::loremaster::dsl::IntoFlow::into_flow(#name(cx))
            }
        }
    })
}

/// `on = turn`, `on = [turn, enter]` or nothing (every event).
fn on(input: syn::parse::ParseStream) -> Result<Option<Vec<Ident>>> {
    if input.is_empty() {
        return Ok(None);
    }
    let key: Ident = input.parse()?;
    if key != "on" {
        return Err(Error::new(key.span(), "expected `on = <event kind>`"));
    }
    input.parse::<Token![=]>()?;
    let words: Vec<Ident> = if input.peek(syn::token::Bracket) {
        let content;
        syn::bracketed!(content in input);
        content
            .parse_terminated(Ident::parse, Token![,])?
            .into_iter()
            .collect()
    } else {
        vec![input.parse()?]
    };
    if !input.is_empty() {
        return Err(input.error("expected the end of `on = ..`"));
    }
    words
        .iter()
        .map(|w| {
            let word = w.to_string();
            if KINDS.contains(&word.as_str()) {
                Ok(Ident::new(&camel(&word), w.span()))
            } else {
                Err(Error::new(
                    w.span(),
                    format!("expected an event kind: {}", KINDS.join(", ")),
                ))
            }
        })
        .collect::<Result<_>>()
        .map(Some)
}

/// `weigh_gold` to `WeighGold`.
fn camel(id: &str) -> String {
    id.split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect()
}
