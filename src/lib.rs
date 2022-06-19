use proc_macro::TokenStream;
use quote::quote;
use std::str::FromStr;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Ident, Type};

#[proc_macro_derive(FromPgRow)]
pub fn derive_from_pg_row(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let fields = get_struct_idents(&input.data);
    let v: Vec<_> = fields
        .into_iter()
        .map(|ident| {
            quote! {
                match row.try_get(stringify!(#ident)) {
                    Ok(v) => x.#ident = v,
                    Err(_) => {
                        let s: String = row.try_get(stringify!(#ident)).unwrap_or("".to_string());
                        if let Ok(t) =  serde_json::from_str(s.as_str()) {
                            x.#ident = t;
                        }
                    }
                }
            }
        })
        .collect();
    let expanded = quote! {
        impl std::convert::From<tokio_postgres::Row> for #struct_name {
            fn from(row: tokio_postgres::Row) -> Self {
                let mut x = Self::default();
               #(#v;)*
                x
            }
        }
    };
    expanded.into()
}

#[proc_macro_derive(IntoJsonMap)]
pub fn derive_to_json_map(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let fields = get_struct_idents(&input.data);
    let v: Vec<_> = fields
        .into_iter()
        .map(|x| {
            quote! {
                mp.insert(stringify!(#x).to_string(), serde_json::json!(self.#x));
            }
        })
        .collect();
    let expanded = quote! {
        impl std::convert::Into<serde_json::Map<String, serde_json::Value>> for #struct_name {
             fn into(self) -> serde_json::Map<String, serde_json::Value> {
                let mut mp = serde_json::Map::new();
                #(#v;)*
                mp
            }
        }
    };
    expanded.into()
}

#[proc_macro_derive(FromSql)]
pub fn derive_from_sql(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let expanded = quote! {
        impl<'a> tokio_postgres::types::FromSql<'a> for #struct_name {
            fn from_sql(ty: &tokio_postgres::types::Type, raw: &'a [u8]) -> std::result::Result<Self, Box<dyn std::error::Error + Sync + Send>> {
                Err(Box::new(crate::types::error::Error::system_error("")))
            }
            fn accepts(ty: &tokio_postgres::types::Type) -> bool {
                false
            }
        }
    };
    expanded.into()
}

#[proc_macro_derive(GetFieldNames)]
pub fn derive_struct_field_names(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let v = get_struct_idents(&input.data);
    let expanded = quote! {
        impl StructFieldNames for #struct_name {
            fn field_names() -> &'static[&'static str] {
                &[#(stringify!(#v),)*]
            }
        }
    };
    expanded.into()
}

#[proc_macro_derive(FromJsonMap)]
pub fn derive_from_json_map(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let v = get_struct_idents(&input.data);
    let xv: Vec<_> = v
        .iter()
        .map(|&ident| {
            proc_macro2::TokenStream::from_str(format!("x.{} = t;", ident).as_str()).unwrap()
        })
        .collect();

    let expanded = quote! {
        impl std::convert::From<serde_json::Map<String, serde_json::Value>> for #struct_name {
            fn from(mut mp: serde_json::Map<String, serde_json::Value>) -> Self {
                let mut x = Self::default();
                #(
                    if let Some(val) = mp.remove(stringify!(#v)) {
                        if let Ok(t) = serde_json::from_value(val) {
                            #xv
                        }
                    }
                )*
                x
            }
        }
    };
    expanded.into()
}

fn get_struct_idents(data: &Data) -> Vec<&Ident> {
    match data {
        Data::Struct(DataStruct { ref fields, .. }) => match fields {
            Fields::Named(ref named_fields) => named_fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().unwrap())
                .collect(),
            _ => panic!("Must Named Fields"),
        },
        _ => panic!("FromPgRow trait is Only for struct"),
    }
}

fn __get_struct_fields(data: &Data) -> Vec<(&Ident, &Type)> {
    match data {
        Data::Struct(DataStruct { ref fields, .. }) => match fields {
            Fields::Named(ref named_fields) => named_fields
                .named
                .iter()
                .map(|field| (field.ident.as_ref().unwrap(), &field.ty))
                .collect(),
            _ => panic!("Must Named Fields"),
        },
        _ => panic!("FromPgRow trait is Only for struct"),
    }
}
