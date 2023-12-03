extern crate proc_macro;

use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::ToTokens,
    std::{iter, mem},
    syn::{
        parse::{Parse, ParseStream},
        parse_macro_input, parse_quote, Attribute, Error, Generics, Ident, Item, ItemEnum,
        ItemImpl, ItemStruct, Meta, Path, Token, Variant, Visibility,
    },
};

struct Enumify {
    enum_attributes: Vec<Attribute>,
    enum_visibility: Visibility,
    enum_token: Token![enum],
    enum_ident: Ident,
    enum_generics: Generics,
    struct_items: Vec<ItemStruct>,
}

impl Parse for Enumify {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let enum_attributes = input.call(Attribute::parse_outer)?;
        let enum_visibility = input.parse::<Visibility>()?;
        let enum_token = input.parse::<Token![enum]>()?;
        let enum_ident = input.parse::<Ident>()?;
        let enum_generics = input.parse::<Generics>()?;
        input.parse::<Token![;]>()?;

        let struct_items = iter::from_fn(|| match input.is_empty() {
            true => None,
            false => Some(input.parse::<ItemStruct>()),
        })
        .collect::<Result<Vec<_>, Error>>()?;

        Ok(Self {
            enum_attributes,
            enum_visibility,
            enum_token,
            enum_ident,
            enum_generics,
            struct_items,
        })
    }
}

impl Enumify {
    fn extract_variant_wrapper(attributes: &mut Vec<Attribute>) -> Option<Result<Path, Error>> {
        let mut wrapper = None;

        for mut attribute in mem::take(attributes) {
            let list = match attribute.meta {
                Meta::List(list) => list,
                Meta::Path(_) | Meta::NameValue(_) => {
                    attributes.push(attribute);
                    continue;
                }
            };

            let ident = match list.path.require_ident() {
                Ok(ident) => ident,
                Err(_) => {
                    attribute.meta = Meta::List(list);
                    attributes.push(attribute);
                    continue;
                }
            };

            if ident != "enumify" {
                attribute.meta = Meta::List(list);
                attributes.push(attribute);
                continue;
            }

            if let Some(result) = &mut wrapper {
                let error = Error::new(
                    ident.span(),
                    "at most one `#[enumify()]` attribute is allowed per `struct`",
                );

                match result {
                    Ok(_) => wrapper = Some(Err(error)),
                    Err(previous_error) => previous_error.combine(error),
                }

                continue;
            }

            wrapper = Some(syn::parse2(list.tokens));
        }

        wrapper
    }

    fn construct_enum_variant(item: &mut ItemStruct) -> Result<Variant, Error> {
        let ItemStruct {
            ident, generics, ..
        } = item;

        match Self::extract_variant_wrapper(&mut item.attrs) {
            None => Ok(parse_quote! { #ident(#ident #generics) }),
            Some(Ok(path)) => Ok(parse_quote! { #ident(#path<#ident #generics>) }),
            Some(Err(error)) => Err(error),
        }
    }

    fn construct_enum_item(&mut self) -> (ItemEnum, Vec<Error>) {
        let Self {
            enum_attributes,
            enum_visibility,
            enum_token,
            enum_ident,
            enum_generics,
            struct_items,
        } = self;

        let mut enum_variants = Vec::new();
        let mut enum_errors = Vec::new();

        for result in struct_items.iter_mut().map(Self::construct_enum_variant) {
            match result {
                Ok(enum_variant) => enum_variants.push(enum_variant),
                Err(error) => enum_errors.push(error),
            }
        }

        let enum_item = parse_quote! {
            #( #enum_attributes )*
            #enum_visibility #enum_token #enum_ident #enum_generics {
                #( #enum_variants ),*
            }
        };

        (enum_item, enum_errors)
    }

    fn construct_impl_items(&self) -> Vec<ItemImpl> {
        self.struct_items
            .iter()
            .map(|struct_item| {
                let Self {
                    enum_ident,
                    enum_generics,
                    ..
                } = self;

                let ItemStruct {
                    ident: struct_ident,
                    generics: struct_generics,
                    ..
                } = struct_item;

                parse_quote! {
                    impl #enum_generics ::core::convert::From<#struct_ident #struct_generics> for #enum_ident #enum_generics {
                        fn from(value: #struct_ident #struct_generics) -> Self {
                            #enum_ident::#struct_ident(value.into())
                        }
                    }
                }
            })
            .collect()
    }

    fn generate_tokens(mut self) -> TokenStream2 {
        let (enum_item, enum_errors) = self.construct_enum_item();
        let impl_items = self.construct_impl_items();
        let Self { struct_items, .. } = self;

        iter::once(Item::Enum(enum_item))
            .chain(struct_items.into_iter().map(Item::Struct))
            .chain(impl_items.into_iter().map(Item::Impl))
            .map(Item::into_token_stream)
            .chain(enum_errors.into_iter().map(Error::into_compile_error))
            .collect()
    }
}

#[proc_macro]
pub fn enumify(stream: TokenStream) -> TokenStream {
    TokenStream::from(parse_macro_input!(stream as Enumify).generate_tokens())
}
