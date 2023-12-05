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

enum Enumifiable {
    Struct(ItemStruct),
    Enum(ItemEnum),
}

impl Parse for Enumifiable {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        match input.parse::<Item>()? {
            Item::Struct(item) => Ok(Self::Struct(item)),
            Item::Enum(item) => Ok(Self::Enum(item)),
            _ => Err(input.error("not `struct` or `enum`")),
        }
    }
}

impl Enumifiable {
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
                    "at most one `#[enumify()]` attribute is allowed per
                    type corresponding to a variant",
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

    fn construct_enum_variant(&mut self) -> Result<Variant, Error> {
        let (attrs, ident, generics) = match self {
            Enumifiable::Struct(item) => (&mut item.attrs, &item.ident, &item.generics),
            Enumifiable::Enum(item) => (&mut item.attrs, &item.ident, &item.generics),
        };

        match Self::extract_variant_wrapper(attrs) {
            None => Ok(parse_quote! { #ident(#ident #generics) }),
            Some(Ok(path)) => Ok(parse_quote! { #ident(#path<#ident #generics>) }),
            Some(Err(error)) => Err(error),
        }
    }
}

impl From<Enumifiable> for Item {
    fn from(enumifiable: Enumifiable) -> Self {
        match enumifiable {
            Enumifiable::Struct(item) => Self::Struct(item),
            Enumifiable::Enum(item) => Self::Enum(item),
        }
    }
}

struct Enumify {
    enum_attributes: Vec<Attribute>,
    enum_visibility: Visibility,
    enum_token: Token![enum],
    enum_ident: Ident,
    enum_generics: Generics,
    enumifiables: Vec<Enumifiable>,
}

impl Parse for Enumify {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let enum_attributes = input.call(Attribute::parse_outer)?;
        let enum_visibility = input.parse::<Visibility>()?;
        let enum_token = input.parse::<Token![enum]>()?;
        let enum_ident = input.parse::<Ident>()?;
        let enum_generics = input.parse::<Generics>()?;
        input.parse::<Token![;]>()?;

        let enumifiables = iter::from_fn(|| match input.is_empty() {
            true => None,
            false => Some(input.parse::<Enumifiable>()),
        })
        .collect::<Result<Vec<_>, Error>>()?;

        Ok(Self {
            enum_attributes,
            enum_visibility,
            enum_token,
            enum_ident,
            enum_generics,
            enumifiables,
        })
    }
}

impl Enumify {
    fn construct_enum_item(&mut self) -> (ItemEnum, Vec<Error>) {
        let Self {
            enum_attributes,
            enum_visibility,
            enum_token,
            enum_ident,
            enum_generics,
            enumifiables,
        } = self;

        let mut enum_variants = Vec::new();
        let mut enum_errors = Vec::new();

        for result in enumifiables
            .iter_mut()
            .map(Enumifiable::construct_enum_variant)
        {
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
        let Self {
            enum_ident,
            enum_generics,
            ..
        } = self;

        self.enumifiables
            .iter()
            .map(|enumifiable| {
                let (ident, generics) = match enumifiable {
                    Enumifiable::Struct(item) => (&item.ident, &item.generics),
                    Enumifiable::Enum(item) => (&item.ident, &item.generics),
                };

                parse_quote! {
                    impl #enum_generics ::core::convert::From<#ident #generics> for #enum_ident #enum_generics {
                        fn from(value: #ident #generics) -> Self {
                            #enum_ident::#ident(value.into())
                        }
                    }
                }
            })
            .collect()
    }

    fn generate_tokens(mut self) -> TokenStream2 {
        let (enum_item, enum_errors) = self.construct_enum_item();
        let impl_items = self.construct_impl_items();
        let Self { enumifiables, .. } = self;

        iter::once(Item::Enum(enum_item))
            .chain(enumifiables.into_iter().map(Item::from))
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
