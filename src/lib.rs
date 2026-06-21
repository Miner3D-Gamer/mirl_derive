//! Derive values for structs and enums
// TODO: Add `read_only` flag to [`derive_all`]

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitBool, Token, parse::Parse, parse_macro_input};

#[derive(Debug, Clone, )]
/// A definition of a configurably derivative value
struct CfgDefinitionValue {
    /// The feature name
    pub feature: &'static str,
    /// The derive parameter in quote! form
    pub read_streams: Vec<proc_macro2::TokenStream>,
    /// The derive parameter in quote! form
    pub write_streams: Vec<proc_macro2::TokenStream>,
    /// If it's a derive, else it'll be inputted raw
    // "That's what he said"
    pub is_derive: bool,
    /// What dependencies are required
    pub dependencies: Vec<&'static str>,
}
impl CfgDefinitionValue {
    fn new(
        feature: &'static str,
        read_streams: Vec<proc_macro2::TokenStream>,
        write_streams: Vec<proc_macro2::TokenStream>,
        is_derive: bool,
        dependencies: Option<Vec<&'static str>>,
    ) -> Self {
        Self {
            feature,
            read_streams,write_streams,
            is_derive,
            dependencies: dependencies.unwrap_or_default(),
        }
    }
}

fn get_codec_definition() -> Vec<CfgDefinitionValue> {
    vec![
        CfgDefinitionValue::new(
            "serde",
            vec![quote! { serde::Serialize }],
            vec![quote! { serde::Deserialize }],
            true,
            None,
        ),
        CfgDefinitionValue::new(
            "bitcode",
            vec![quote! { bitcode::Encode }],
            vec![ quote! { bitcode::Decode }],
            true,
            None,
        ),
        CfgDefinitionValue::new(
            "wincode",
                // Reenable wincode when
                // ```overly complex generic constant
                // consider moving this anonymous constant into a `const` function
                // this operation may be supported in the future```
                // Isn't an issue anymore. Latest broken version: 0.5.3

            vec![
                // quote! { wincode::SchemaRead },
            ],
            vec![
                // quote! { wincode::SchemaWrite },
            ],
            true,
            None,
        ),
        CfgDefinitionValue::new(
            "compactly",
            vec![],
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
                quote! { zerocopy::IntoBytes },
            ],
            vec![
                quote! { zerocopy::TryFromBytes },
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
                // quote! { strum::EnumIter }, This can only be implemented when a type has default
                quote! { strum::EnumCount },
                quote! { strum::AsRefStr },
                quote! { strum::IntoStaticStr },
                // quote! { strum::VariantArray },
                quote! { strum::VariantNames },
            ],
            vec![
            ],
            true,
            None,
        ),
        CfgDefinitionValue::new("enum_ext", vec![], vec![], false, None),
    ]
}
fn apply_derive(
    item: &mut DeriveInput,
    flags: &mut Vec<(String, bool)>,
    derive_config: Vec<CfgDefinitionValue>,
    allow_unused_config: bool,
    allow_unused_flag: bool,
    read_only: bool
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

        if !flag.1 {
            continue;
        }
        if info.is_derive {
            // Add all derives as separate #[derive(...)] attributes
            for derive in &info.read_streams {
                add_derive(item, info.feature, &info.dependencies, derive);
            }
            if !read_only{
            for derive in &info.write_streams {
                add_derive(item, info.feature, &info.dependencies, derive);
            }
            }
        } else {
            for derive in info.read_streams {
                let feature = info.feature;
                item.attrs.push(syn::parse_quote! {
                    #[cfg_attr(feature = #feature, #derive)]
                });
            }
            if !read_only{
                
            for derive in info.write_streams {
                let feature = info.feature;
                item.attrs.push(syn::parse_quote! {
                    #[cfg_attr(feature = #feature, #derive)]
                });
            }
            }
        }
    }
    if !unused_flags.is_empty() {
        if allow_unused_flag {
            *flags = unused_flags;
        } else {
            if codec_derives.is_empty() {
                return Some(
                    syn::Error::new_spanned(
                        &item,
                        format!(
                            "Unknown macro input: {}. All expected fields have been defined. Consider removing it?",
                            unused_flags
                                .iter()
                                .map(|x| format!("`{}` ({})", x.0, x.1))
                                .collect::<Vec<String>>()
                                .join(", "),
                            
                        ),
                    )
                    .to_compile_error(),
                );
            }
            return Some(
                syn::Error::new_spanned(
                    &item,
                    format!(
                        "Unknown macro input: {}. Expected one of: {}",
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

fn add_derive(
    item: &mut DeriveInput,name: &'static str, dependencies: &[&'static str], derive: &proc_macro2::TokenStream){
    
                let mut features: Vec<&str> = vec![name];
                features.extend_from_slice(dependencies);
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

fn add_flag_to_vec(vec: &mut Vec<(String, bool)>, flag: &str, bool: bool) {
    if !vec.iter().any(|x| x.0.eq(flag)) {
        vec.push((flag.to_string(), bool));
    }
}

/// Attribute macro to conditionally derive codec traits.
///
/// ---
/// 
/// Applies serialization/deserialization derives based on enabled features.
/// 
/// Automatically chooses which features to enable/disable based on what the item the derive is used on.
/// 
/// Optionally disableable: `wincode`, `bitcode`, `serde`, `strum`, `enum_ext`, `c_compatible`, `zerocopy`, and `compactly`
///
/// ### Warning: 
/// `wincode` has been temporarily disabled until the author fixes their `overly complex generic constant` problem. Please use `bitcode` in the meanwhile.
/// 
/// # Example
/// If `wincode`, `serde`, and `zerocopy` were to give issues, you could just disable them
/// ```
/// #[derive_all(wincode = false, c_compatible = false, zerocopy = false)]
/// pub struct MyData {
///     value: i32,
/// }
/// ```
#[proc_macro_attribute]
pub fn derive_all(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as DeriveInput);
    let FlagList(mut flags) = parse_macro_input!(args as FlagList);

    let c_compatible =
        flags.iter().find(|x| x.0.eq("c_compatible")).is_none_or(|x| x.1);

    let vals = ["serde", "bitcode", "zerocopy"];
    for val in vals {
        add_flag_to_vec(&mut flags, val, true);
    }
    add_flag_to_vec(&mut flags, "wincode", item.generics.lt_token.is_none());
    // if !flags.iter().any(|x| x.0.eq("wincode")) {
    //     if item.generics.lt_token.is_none() {
    //         flags.push(("wincode".to_string(), true));
    //     } else {
    //         flags.push(("wincode".to_string(), false));
    //     }
    // }
    let mut cfg = get_codec_definition();

    if let syn::Data::Enum(data) = &item.data {
        // println!("{:?}", flags);
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
                add_flag_to_vec(&mut flags, k, true);
            }
        } else {
            flags.retain(|x| !keys.contains(&x.0.as_str()));
            for k in keys {
                flags.push((k.to_string(), false));
            }
        }
        add_flag_to_vec(&mut flags, "compactly", false);
        cfg.extend(get_enum_definition());
    } else {
        add_flag_to_vec(&mut flags, "compactly", true);
    }
    add_flag_to_vec(&mut flags, "read_only",false);
    // println!(
    //     "----\n{:#?} vs {:?}",
    //     flags,
    //     cfg.iter().map(|x| x.feature).collect::<Vec<&str>>()
    // );   

    // TODO: Check if read_only was defined instead of adding it.
    // Safety: As we are adding read_only when it isn't defined, it will always exist
    let read_only_pos = unsafe{flags.iter().position(|x| x.0 == "read_only" ).unwrap_unchecked()};
    let read_only = flags.remove(read_only_pos).1;
    if let Some(err) = apply_derive(&mut item, &mut flags, cfg, false, false,read_only) {
        return err.into();
    }
    if c_compatible {
        item.attrs.push(syn::parse_quote! {
            #[cfg_attr(feature = "c_compatible", repr(C))]
        });
    }
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
        apply_derive(&mut item, &mut flags, get_codec_definition(), false, true, false)
    {
        return err.into();
    }
    if let syn::Data::Enum(_) = item.data
        && let Some(err) = apply_derive(
            &mut item,
            &mut flags,
            get_enum_definition(),
            false,
            false,false
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
        false,false
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
        apply_derive(&mut item, &mut flags, get_enum_definition(), false, false,false)
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
