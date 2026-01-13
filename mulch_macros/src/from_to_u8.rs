use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Expr, Lit};

pub fn derive_from_to_u8(item: DeriveInput) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let type_name = &item.ident;

    let Data::Enum(enum_data) = &item.data else {
        panic!("FromToU8 is only supported for enums")
    };

    let from_self_fn_body = to_u8_fn_body(enum_data);
    let try_from_u8_fn_body = from_u8_fn_body(enum_data);

    quote! {
        #[automatically_derived]
        impl #impl_generics #type_name #ty_generics #where_clause {
            pub const fn to_u8(&self) -> u8 { #from_self_fn_body }

            pub const fn from_u8(val: u8) -> ::std::option::Option<Self> { #try_from_u8_fn_body }
        }
    }
}

fn to_u8_fn_body(enum_data: &DataEnum) -> TokenStream {
    let mut discriminant = 0u8;
    let match_arms = enum_data.variants.iter().map(|variant| {
        if let Some((_, disc)) = variant.discriminant.as_ref() {
            if let Expr::Lit(disc) = disc
                && let Lit::Int(disc) = &disc.lit
            {
                let Ok(disc) = disc.base10_parse::<u8>() else {
                    panic!("Failed to parse discriminant");
                };

                discriminant = disc;
            } else {
                panic!("Only numeric literals are supported for discriminants with `FromToU8`")
            }
        }

        let variant_name = &variant.ident;

        let arm = quote! {
            Self::#variant_name => #discriminant
        };

        discriminant += 1;

        arm
    });

    quote! {
        match self {
            #(#match_arms),*
        }
    }
}

fn from_u8_fn_body(enum_data: &DataEnum) -> TokenStream {
    let mut discriminant = 0u8;
    let match_arms = enum_data.variants.iter().map(|variant| {
        if let Some((_, disc)) = variant.discriminant.as_ref() {
            if let Expr::Lit(disc) = disc
                && let Lit::Int(disc) = &disc.lit
            {
                let Ok(disc) = disc.base10_parse::<u8>() else {
                    panic!("Failed to parse discriminant");
                };

                discriminant = disc;
            } else {
                panic!("Only numeric literals are supported for discriminants with `FromToU8`")
            }
        }

        let variant_name = &variant.ident;

        let arm = quote! {
            #discriminant => Some(Self::#variant_name)
        };

        discriminant += 1;

        arm
    });

    quote! {
        match val {
            #(#match_arms,)*
            _ => None
        }
    }
}
