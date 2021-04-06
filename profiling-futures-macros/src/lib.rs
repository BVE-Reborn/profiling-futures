use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn wrap(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = syn::parse_macro_input!(item as syn::ItemFn);

    let attrs = function.attrs;
    let block = function.block;
    let sig = function.sig;
    let vis = function.vis;

    let res = quote! {
        #(#attrs)* #vis #sig {
            ::profiling_futures::FutureWrapper::new(async move #block).await
        }
    };

    res.into()
}
