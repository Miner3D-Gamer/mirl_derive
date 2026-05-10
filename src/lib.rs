//! Derive values for structs and enums

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitBool, Token, parse::Parse, parse_macro_input};

/// A definition of a configurably derivative value
struct CfgDefinitionValue {
    feature: &'static str,
    streams: Vec<proc_macro2::TokenStream>,
    is_derive: bool,
    dependencies: Vec<&'static str>,
}
impl CfgDefinitionValue {
    fn new(
        feature: &'static str,
        streams: Vec<proc_macro2::TokenStream>,
        is_derive: bool,
        dependencies: Option<Vec<&'static str>>,
    ) -> Self {
        Self {
            feature,
            streams,
            is_derive,
            dependencies: dependencies.unwrap_or_default(),
        }
    }
}

fn get_codec_definition() -> Vec<CfgDefinitionValue> {
    vec![
        CfgDefinitionValue::new(
            "serde",
            vec![quote! { serde::Serialize }, quote! { serde::Deserialize }],
            true,
            None,
        ),
        CfgDefinitionValue::new(
            "bitcode",
            vec![quote! { bitcode::Encode }, quote! { bitcode::Decode }],
            true,
            None,
        ),
        CfgDefinitionValue::new(
            "wincode",
            vec![
                // quote! { wincode::SchemaRead },
                // quote! { wincode::SchemaWrite },
            ],
            true,
            None,
        ),
        CfgDefinitionValue::new(
            "compactly",
            vec![
                quote! { compactly::v1::Encode },
                quote! { compactly::v2::Encode },
            ],
            true,
            None,
        ),
        CfgDefinitionValue::new(
            "zerocopy",
            vec![
                quote! { zerocopy::TryFromBytes },
                quote! { zerocopy::IntoBytes },
            ],
            true,
            Some(vec!["c_compatible"]),
        ),
    ]
}

fn get_enum_definition() -> Vec<CfgDefinitionValue> {
    vec![
        CfgDefinitionValue::new(
            "strum",
            vec![
                // quote! { strum::EnumIter },
                // quote! { strum::EnumCount },
                // quote! { strum::AsRefStr },
                // quote! { strum::Display },
                // quote! { strum::IntoStaticStr },
                // quote! { strum::VariantArray },
                // quote! { strum::VariantNames },
            ],
            true,
            None,
        ),
        CfgDefinitionValue::new("enum_ext", vec![], false, None),
    ]
}
fn apply_derive(
    item: &mut DeriveInput,
    flags: &mut Vec<(String, bool)>,
    derive_config: Vec<CfgDefinitionValue>,
    allow_unused_config: bool,
    allow_unused_flag: bool,
) -> Option<proc_macro2::TokenStream> {
    let mut codec_derives = derive_config;
    let mut unused_flags = Vec::with_capacity(flags.len());

    for _ in 0..flags.len() {
        let flag = unsafe { vec_unchecked_swap_remove(flags, 0) };

        let Some(idx) = codec_derives.iter().position(|x| flag.0.eq(x.feature))
        else {
            unused_flags.push(flag);
            continue;
        };
        let info =
            unsafe { vec_unchecked_swap_remove(&mut codec_derives, idx) };

        if info.is_derive {
            // Add all derives as separate #[derive(...)] attributes
            for derive in info.streams {
                let mut features: Vec<&str> = vec![info.feature];
                features.extend(info.dependencies.clone());
                let feature = format!(
                    "all({})",
                    features
                        .iter()
                        .map(|x| format!("feature = \"{x}\""))
                        .collect::<Vec<String>>()
                        .join(", ")
                );
                let Ok(stream): Result<
                    proc_macro2::TokenStream,
                    proc_macro2::LexError,
                > = feature.parse() else {
                    panic!(
                        "Error at {}:{}:{} \nSome feature was not defined properly: {}",
                        file!(),
                        line!(),
                        column!(),
                        feature
                    )
                };
                item.attrs.push(syn::parse_quote! {
                    #[cfg_attr(#stream, derive(#derive))]
                });
            }
        } else {
            for derive in info.streams {
                let feature = info.feature;
                item.attrs.push(syn::parse_quote! {
                    #[cfg_attr(feature = #feature, #derive)]
                });
            }
        }
    }
    if !unused_flags.is_empty() {
        if allow_unused_flag {
            *flags = unused_flags;
        } else {
            return Some(
                syn::Error::new_spanned(
                    &item,
                    format!(
                        "Unknown fields: {}. Expected one of: {}",
                        unused_flags
                            .iter()
                            .map(|x| format!("`{}` ({})", x.0, x.1))
                            .collect::<Vec<String>>()
                            .join(", "),
                        codec_derives
                            .iter()
                            .map(|x| format!("`{}`", x.feature))
                            .collect::<Vec<String>>()
                            .join(", ")
                    ),
                )
                .to_compile_error(),
            );
        }
    }

    if !allow_unused_config && !codec_derives.is_empty() {
        Some(
            syn::Error::new_spanned(
                &item,
                format!(
                    "Not all fields have been defined, missing fields: {}",
                    codec_derives
                        .iter()
                        .map(|x| format!("`{}`", x.feature))
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            )
            .to_compile_error(),
        )
    } else {
        None
    }
}
/// Attribute macro to conditionally derive codec traits.
///
/// Applies serialization/deserialization derives based on enabled features.
/// Supports: `wincode`
/// Optionally: `strum`, `enum_ext`
///
/// # Example
/// ```ignore
/// #[derive_codec(wincode = true, strum = false, enum_ext = true)]
/// pub struct MyData {
///     value: i32,
/// }
/// ```
#[proc_macro_attribute]
pub fn derive_all(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as DeriveInput);
    let FlagList(mut flags) = parse_macro_input!(args as FlagList);

    flags.extend([
        ("serde".to_string(), true),
        ("bitcode".to_string(), true),
        ("zerocopy".to_string(), true),
    ]);

    if item.generics.lt_token.is_none() {
        flags.push(("wincode".to_string(), true));
    } else {
        flags.push(("wincode".to_string(), false));
    }

    let mut cfg = get_codec_definition();

    if let syn::Data::Enum(data) = &item.data {
        let mut pure = true;
        for variant in &data.variants {
            #[allow(clippy::equatable_if_let)] // `==` cannot be used here
            if let syn::Fields::Unit = variant.fields {
                pure = false;
                break;
            }
        }
        let keys = ["strum", "enum_ext"];

        if pure {
            for k in keys {
                if !flags.iter().any(|x| x.0 == k) {
                    flags.push((k.to_string(), true));
                }
            }
        } else {
            flags.retain(|x| !keys.contains(&x.0.as_str()));
            for k in keys {
                flags.push((k.to_string(), false));
            }
        }

        flags.push(("compactly".to_string(), false));
        cfg.extend(get_enum_definition());
    } else {
        flags.push(("compactly".to_string(), true));
    }
    // println!(
    //     "----\n{:#?} vs {:?}",
    //     flags,
    //     cfg.iter().map(|x| x.feature).collect::<Vec<&str>>()
    // );
    if let Some(err) = apply_derive(&mut item, &mut flags, cfg, false, false) {
        return err.into();
    }

    item.attrs.push(syn::parse_quote! {
        #[cfg_attr(feature = "c_compatible", repr(C))]
    });
    quote! { #item }.into()
}
/// Attribute macro to conditionally derive codec traits.
///
/// Applies serialization/deserialization derives based on enabled features.
/// Supports: `serde`, `bitcode`, `wincode`, `compactly`
/// Optionally: `strum`, `enum_ext`
///
/// # Example
/// ```ignore
/// #[derive_codec(serde = true, bitcode = false, wincode = true, compactly = true)]
/// pub struct MyData {
///     value: i32,
/// }
/// ```
#[proc_macro_attribute]
pub fn derive_possible_configured(
    args: TokenStream,
    input: TokenStream,
) -> TokenStream {
    let mut item = parse_macro_input!(input as DeriveInput);
    let FlagList(mut flags) = parse_macro_input!(args as FlagList);
    if let Some(err) =
        apply_derive(&mut item, &mut flags, get_codec_definition(), false, true)
    {
        return err.into();
    }
    if let syn::Data::Enum(_) = item.data
        && let Some(err) = apply_derive(
            &mut item,
            &mut flags,
            get_enum_definition(),
            false,
            false,
        )
    {
        return err.into();
    }
    quote! { #item }.into()
}

/// Attribute macro to conditionally derive codec traits.
///
/// Applies serialization/deserialization derives based on enabled features.
/// Supports: serde, bitcode, wincode, compactly
///
/// # Example
/// ```ignore
/// #[derive_codec(serde = true, bitcode = false, wincode = true, compactly = true)]
/// pub struct MyData {
///     value: i32,
/// }
/// ```
#[proc_macro_attribute]
pub fn derive_codec(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as DeriveInput);
    let FlagList(mut flags) = parse_macro_input!(args as FlagList);
    if let Some(err) = apply_derive(
        &mut item,
        &mut flags,
        get_codec_definition(),
        false,
        false,
    ) {
        return err.into();
    }
    quote! { #item }.into()
}
struct FlagList(Vec<(String, bool)>);

impl Parse for FlagList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitBool = input.parse()?;

            items.push((key.to_string(), value.value()));

            let _ = input.parse::<Token![,]>();
        }

        Ok(Self(items))
    }
}

/// Attribute macro for better enum deriving.
///
/// Applies strum and optional `enum_ext` derives to enums.
/// Supports: serde, strum, `enum_ext`
///
/// # Example
/// ```ignore
/// #[derive_better_enum(serde = true, strum = true, enum_ext = false)]
/// pub enum Color {
///     Red,
///     Green,
///     Blue,
/// }
/// ```
#[proc_macro_attribute]
pub fn derive_better_enum(
    args: TokenStream,
    input: TokenStream,
) -> TokenStream {
    let mut item = parse_macro_input!(input as DeriveInput);
    let FlagList(mut flags) = parse_macro_input!(args as FlagList);
    if let Some(err) =
        apply_derive(&mut item, &mut flags, get_enum_definition(), false, false)
    {
        return err.into();
    }
    quote! { #item }.into()
}

/// Attribute macro: shorthand for `#[cfg_attr(feature = "c_compatible", repr(C))]`
///
/// Applies C-compatible memory layout when the "`c_compatible`" feature is enabled.
///
/// # Example
/// ```ignore
/// #[c_compatible]
/// pub struct CData {
///     field1: i32,
///     field2: u8,
/// }
/// ```
#[proc_macro_attribute]
pub fn c_compatible(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as DeriveInput);

    item.attrs.push(syn::parse_quote! {
        #[cfg_attr(feature = "c_compatible", repr(C))]
    });

    quote! { #item }.into()
}

/// Copied from `mirl_core`
/// Remove an item from a vec without shifting all values or retaining order
///
/// # Safety
/// The caller must ensure that `index` is strictly less than `vec.len()`
unsafe fn vec_unchecked_swap_remove<T>(vec: &mut Vec<T>, index: usize) -> T {
    let len = vec.len();

    // 1. Read the item out of the vector (takes ownership)
    let base_ptr = vec.as_mut_ptr();
    unsafe {
        let removed_item = std::ptr::read(base_ptr.add(index));

        // 2. If it's not the last element, move the last element to the cleared slot
        if index < len - 1 {
            std::ptr::copy_nonoverlapping(
                base_ptr.add(len - 1),
                base_ptr.add(index),
                1,
            );
        }

        vec.set_len(len - 1);

        removed_item
    }
}
