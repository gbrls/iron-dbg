use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, DeriveInput, Fields, FieldsNamed, Lit, Meta, MetaNameValue,
};

#[proc_macro_derive(FromMI, attributes(name, consume))]
pub fn from_mi(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input);

    let mut idents = vec![];

    for attr in attrs {
        if let Some(s) = name_from_attr(&attr) {
            //idents.push(format!("{}", quote! { (#s) }));
            idents.push(s);
        }
    }

    match data {
        syn::Data::Struct(s) => match s.fields {
            Fields::Named(FieldsNamed { named, .. }) => {
                for f in named {
                    //if let Some(id) = f.ident {
                    //    idents.push(format!("{}", quote! { (#id) }));
                    //}

                    for attr in f.attrs {
                        if let Some(s) = name_from_attr(&attr) {
                            //idents.push(format!("{}", quote! { (#s) }));
                            idents.push(s);
                        }
                    }
                }
            }
            Fields::Unnamed(f) => {}
            Fields::Unit => {}
        },
        _ => {}
    }

    let idents = format!("{:?}", idents);

    let output = quote! {
        impl #ident {
            pub fn print_fields() {
                println!("Fields of {} are {}", stringify!(#ident), #idents);
            }
        }
    };

    output.into()
}

fn name_from_attr(attr: &Attribute) -> Option<String> {
    if !attr.path.is_ident("name") {
        return None;
    }

    match attr.parse_meta().unwrap() {
        Meta::NameValue(MetaNameValue {
            lit: Lit::Str(s), ..
        }) => Some(s.value()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
