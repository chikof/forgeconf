use proc_macro::TokenStream;
use syn::{ItemStruct, Result};

mod model;
mod render;

use model::{ForgeconfAttr, collect_fields};

/// Derive loader and parsing logic for a configuration struct.
#[proc_macro_attribute]
pub fn forgeconf(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand(attr, item) {
        Ok(ts) => ts,
        Err(err) => err
            .to_compile_error()
            .into(),
    }
}

fn expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let args = syn::parse::<ForgeconfAttr>(attr)?;
    let mut item = syn::parse::<ItemStruct>(item)?;
    let fields = collect_fields(&mut item)?;
    let tokens = render::render(&item, &args, &fields)?;
    Ok(tokens.into())
}
