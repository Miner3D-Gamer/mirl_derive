//! Derive values for structs and enums
// TODO: Add `read_only` flag to [`derive_all`]

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Token, parse::Parse, parse_macro_input};

#[derive(Debug, Clone)]
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
    pub is_derive: DeriveLevel,
    /// What dependencies are required
    pub dependencies: Vec<&'static str>,
    /// If the given name should also be treated as a dependency
    pub name_is_dependency: bool,
}
/// How the given item is applied
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum DeriveLevel {
    /// `#[item]`
    Attribute,
    /// `#[derive(item)]`
    Derive,
    /// `#[const_derive(item)]`
    ConstDerive,
}
impl DeriveLevel {
    const fn is_derived(self) -> bool {
        match self {
            Self::Attribute => false,
            Self::ConstDerive | Self::Derive => true,
        }
    }
    const fn is_const(self) -> bool {
        matches!(self, Self::ConstDerive)
    }
}
impl CfgDefinitionValue {
    fn new(
        feature: &'static str,
        read_streams: Vec<proc_macro2::TokenStream>,
        write_streams: Vec<proc_macro2::TokenStream>,
        is_derive: DeriveLevel,
        dependencies: Option<Vec<&'static str>>,
        name_is_dependency: bool,
    ) -> Self {
        Self {
            feature,
            read_streams,
            write_streams,
            is_derive,
            dependencies: dependencies.unwrap_or_default(),
            name_is_dependency,
        }
    }
}

fn get_codec_definition() -> Vec<CfgDefinitionValue> {
    vec![
        CfgDefinitionValue::new(
            "serde",
            vec![quote! { serde::Serialize }],
            vec![quote! { serde::Deserialize }],
            DeriveLevel::Derive,
            None,
            true,
        ),
        CfgDefinitionValue::new(
            "bitcode",
            vec![quote! { bitcode::Encode }],
            vec![quote! { bitcode::Decode }],
            DeriveLevel::Derive,
            None,
            true,
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
            DeriveLevel::Derive,
            None,
            true,
        ),
        CfgDefinitionValue::new(
            "compactly",
            vec![],
            vec![
                quote! { compactly::v1::Encode },
                quote! { compactly::v2::Encode },
            ],
            DeriveLevel::Derive,
            None,
            true,
        ),
        CfgDefinitionValue::new(
            "zerocopy",
            vec![quote! { zerocopy::IntoBytes }],
            vec![quote! { zerocopy::TryFromBytes }],
            DeriveLevel::Derive,
            Some(vec!["c_compatible"]),
            true,
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
            vec![],
            DeriveLevel::Derive,
            None,
            true,
        ),
        CfgDefinitionValue::new(
            "enum_ext",
            vec![],
            vec![],
            DeriveLevel::Attribute,
            None,
            true,
        ),
    ]
}

fn get_core_definition() -> Vec<CfgDefinitionValue> {
    vec![
        CfgDefinitionValue::new(
            "default",
            vec![quote! { core::default::Default }],
            vec![],
            DeriveLevel::ConstDerive,
            None,
            false,
        ),
        CfgDefinitionValue::new(
            "clone",
            vec![quote! { core::clone::Clone }],
            vec![],
            DeriveLevel::ConstDerive,
            None,
            false,
        ),
        CfgDefinitionValue::new(
            "eq",
            vec![quote! { core::cmp::Eq }],
            vec![],
            DeriveLevel::ConstDerive,
            None,
            false,
        ),
        CfgDefinitionValue::new(
            "partial_eq",
            vec![quote! { core::cmp::PartialEq }],
            vec![],
            DeriveLevel::ConstDerive,
            None,
            false,
        ),
        CfgDefinitionValue::new(
            "hash",
            vec![quote! { std::hash::Hash }],
            vec![],
            DeriveLevel::Derive,
            None,
            false,
        ),
        CfgDefinitionValue::new(
            "ord",
            vec![quote! { core::cmp::Ord }],
            vec![],
            DeriveLevel::ConstDerive,
            None,
            false,
        ),
        CfgDefinitionValue::new(
            "partial_ord",
            vec![quote! { core::cmp::PartialOrd }],
            vec![],
            DeriveLevel::ConstDerive,
            None,
            false,
        ),
        CfgDefinitionValue::new(
            "debug",
            vec![quote! { core::fmt::Debug }],
            vec![],
            DeriveLevel::Derive,
            None,
            false,
        ),
        CfgDefinitionValue::new(
            "copy",
            vec![quote! { core::marker::Copy }],
            vec![],
            DeriveLevel::Derive,
            None,
            false,
        ),
    ]
}

fn apply_derive(
    item: &mut DeriveInput,
    flags: &mut Vec<(String, UserSetting)>,
    derive_config: Vec<CfgDefinitionValue>,
    allow_unused_config: bool,
    allow_unused_flag: bool,
    read_only: bool,
) -> Option<proc_macro2::TokenStream> {
    let mut codec_derives = derive_config;
    let mut unused_flags = Vec::with_capacity(flags.len());

    for _ in 0..flags.len() {
        let flag = unsafe { vec_unchecked_swap_remove(flags, 0) };

        let Some(idx) = codec_derives.iter().position(|x| flag.0.eq(x.feature)) else {
            unused_flags.push(flag);
            continue;
        };
        let info = unsafe { vec_unchecked_swap_remove(&mut codec_derives, idx) };

        if !flag.1.as_bool() {
            continue;
        }
        if info.is_derive.is_derived() {
            let const_derive = info.is_derive.is_const() && flag.1.allow_const();
            // Add all derives as separate #[derive(...)] attributes
            for derive in &info.read_streams {
                add_derive(
                    item,
                    info.feature,
                    &info.dependencies,
                    info.name_is_dependency,
                    const_derive,
                    derive,
                );
            }
            if !read_only {
                for derive in &info.write_streams {
                    add_derive(
                        item,
                        info.feature,
                        &info.dependencies,
                        info.name_is_dependency,
                        const_derive,
                        derive,
                    );
                }
            }
        } else {
            for derive in info.read_streams {
                let feature = info.feature;
                item.attrs.push(syn::parse_quote! {
                    #[cfg_attr(feature = #feature, #derive)]
                });
            }
            if !read_only {
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
                                .map(|x| format!("`{}` ({:?})", x.0, x.1))
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
                            .map(|x| format!("`{}` ({:?})", x.0, x.1))
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
                    "Not all fields have been defined in the `mirl_derive` macro, missing fields: {}",
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
    item: &mut DeriveInput,
    name: &'static str,
    dependencies: &[&'static str],
    name_is_dependency: bool,
    const_derive: bool,
    derive: &proc_macro2::TokenStream,
) {
    let mut features: Vec<&str> = if name_is_dependency {
        vec![name]
    } else {
        Vec::new()
    };
    features.extend_from_slice(dependencies);

    let requirements: Vec<String> = features
        .iter()
        .map(|x| format!("feature = \"{x}\""))
        .collect::<Vec<String>>();
    let requirements = requirements.join(", ");

    let (feature, const_feature) = if const_derive {
        (
            format!("all(not(IS_NIGHTLY), {requirements})"),
            Some(format!("all(IS_NIGHTLY, {requirements})")),
        )
    } else {
        (format!("all({requirements})"), None)
    };

    let Ok(stream): Result<proc_macro2::TokenStream, proc_macro2::LexError> = feature.parse()
    else {
        panic!(
            "Error at {}:{}:{} \nSome feature was not defined properly: {}",
            file!(),
            line!(),
            column!(),
            feature
        )
    };

    if let Some(const_feature) = const_feature {
        let Ok(const_stream): Result<proc_macro2::TokenStream, proc_macro2::LexError> =
            const_feature.parse()
        else {
            panic!(
                "Error at {}:{}:{} \nSome feature was not defined properly: {}",
                file!(),
                line!(),
                column!(),
                feature
            )
        };
        // panic!("### #[cfg_attr({}, derive({}))]", const_feature, derive.to_string());
        item.attrs.push(syn::parse_quote! {
            #[cfg_attr(#const_stream, derive_const(#derive))]
        });
    }
    item.attrs.push(syn::parse_quote! {
        #[cfg_attr(#stream, derive(#derive))]
    });
}

fn add_flag_to_vec<T: Into<UserSetting>>(
    vec: &mut Vec<(String, UserSetting)>,
    flag: &str,
    bool: T,
) {
    if !vec_contains_flag(vec, flag) {
        vec.push((flag.to_string(), bool.into()));
    }
}
fn remove_flag_from_vec(vec: &mut Vec<(String, UserSetting)>, flag: &str) {
    vec.retain(|x| x.0.ne(flag));
}
fn vec_get_flag(vec: &[(String, UserSetting)], flag: &str) -> Option<UserSetting> {
    vec.iter().find(|x| x.0.eq(flag)).map(|x| x.1)
}
#[allow(dead_code)]
fn vec_contains_flag(vec: &[(String, UserSetting)], flag: &str) -> bool {
    vec.iter().any(|x| x.0.eq(flag))
}

fn camel_to_snake_casing(camel: &str) -> String {
    let chars = camel.chars();
    let mut output: Vec<char> =
        Vec::with_capacity(camel.len() + camel.chars().filter(|x| x.is_uppercase()).count());

    let mut first = true;
    for char in chars {
        if char.is_uppercase() {
            if !first {
                output.push('_');
            }
            output.push(char.to_ascii_lowercase());
        } else {
            output.push(char);
        }
        first = false;
    }
    // let o: String = output.iter().collect();
    // panic!("### {}", o);

    output.iter().collect()
}

/// Attribute macro to conditionally derive codec traits.
/// If you use this in conjunction with the builtin `derive` and `derive_const`, place this macro above the derive calls.
///
/// ---
///
/// Applies serialization/deserialization derives based on enabled features.
///
/// Automatically chooses which features to enable/disable based on what the item the derive is used on.
///
/// Options:
/// - Disable: `false`
/// - Non const version: `nonconst`/`non_const`
///
/// Configurable attributes:
/// - Supported crates: `wincode`, `bitcode`, `serde`, `strum`, `enum_ext`, `c_compatible`, `zerocopy`, and `compactly`
/// - Attributes: `read_only`
/// - Builtin: `Debug`, `Clone`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Default`, and `Hash` (requires snake case instead of camel case)
///
/// ### Warning:
/// `wincode` has been temporarily disabled until the author fixes their `overly complex generic constant` problem. Please use `bitcode` in the meanwhile.
///
/// # Example
/// If `wincode`, `serde`, and `zerocopy` were to give issues, you could just disable them
/// ``` ignore
/// #[mirl_derive::derive_all(wincode = false, serde = false, zerocopy = false)]
/// pub struct MyData {
///     value: i32,
/// }
/// ```
///
/// # Panics
/// When it cannot parse the given item
// TODO: Automatically add `default=true` if `#[default]` is used in an enum
#[proc_macro_attribute]
pub fn derive_all(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as DeriveInput);
    let FlagList(mut flags) = parse_macro_input!(args as FlagList);

    // TODO: For the "core" items a very nice algorithm was used. Add the other thingies to the same list.
    let c_compatible = flags
        .iter()
        .find(|x| x.0.eq("c_compatible"))
        .is_none_or(|x| x.1.as_bool());

    let vals = ["serde", "bitcode", "zerocopy"];
    for val in vals {
        add_flag_to_vec(&mut flags, val, UserSetting::True);
    }
    add_flag_to_vec(&mut flags, "wincode", item.generics.lt_token.is_none());

    let core_values: [(&str, bool, &[_]); _] = [
        ("debug", true, &[]),
        ("clone", true, &[]),
        ("default", false, &[]),
        ("copy", true, &[]),
        ("partial_eq", true, &[]),
        ("eq", true, &["partial_eq"]),
        ("partial_ord", true, &["partial_eq"]),
        ("ord", true, &["partial_ord", "eq"]),
        ("hash", true, &[]),
    ];

    let be_nice_to_derive = !vec_get_flag(&flags, "ignore_derive")
        .unwrap_or(UserSetting::True)
        .as_bool();
    remove_flag_from_vec(&mut flags, "ignore_derive");

    let already_derived = if be_nice_to_derive {
        let mut already_derived = Vec::new();
        for attr in &item.attrs {
            if attr.path().is_ident("derive") || attr.path().is_ident("derive_const") {
                let Ok(derives) = attr.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
                ) else {
                    panic!("INSERT ERROR MESSAGE")
                };

                for derived in derives {
                    if let Some(ident) = derived.get_ident() {
                        // panic!("### '{}'", ident.to_string());
                        already_derived.push(camel_to_snake_casing(&ident.to_string()));
                    }
                }
            }
        }
        already_derived
    } else {
        Vec::new()
    };
    for (val, activate, _) in core_values {
        if already_derived.iter().any(|x| x.eq(val)) {
            add_flag_to_vec(&mut flags, val, false);
            continue;
        }
        add_flag_to_vec(&mut flags, val, activate);
    }

    for (flag, _, dependencies) in core_values {
        for dep in dependencies {
            match vec_get_flag(&flags, dep).unwrap() {
                UserSetting::False => {
                    remove_flag_from_vec(&mut flags, flag);
                    add_flag_to_vec(&mut flags, flag, false);
                    break;
                }
                UserSetting::NonConst => {
                    if vec_get_flag(&flags, flag).is_none_or(|x| matches!(x, UserSetting::False)) {
                        continue;
                    }
                    remove_flag_from_vec(&mut flags, flag);
                    add_flag_to_vec(&mut flags, flag, UserSetting::NonConst);
                }
                UserSetting::True => {}
            }
        }
    }

    // if !flags.iter().any(|x| x.0.eq("wincode")) {
    //     if item.generics.lt_token.is_None {
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
                flags.push((k.to_string(), false.into()));
            }
        }
        add_flag_to_vec(&mut flags, "compactly", false);
        cfg.extend(get_enum_definition());
    } else {
        add_flag_to_vec(&mut flags, "compactly", true);
    }
    add_flag_to_vec(&mut flags, "read_only", false);
    cfg.extend(get_core_definition());
    // println!(
    //     "----\n{:#?} vs {:?}",
    //     flags,
    //     cfg.iter().map(|x| x.feature).collect::<Vec<&str>>()
    // );

    // TODO: Check if read_only was defined instead of adding it. & Use updated functions for getting/removing it instead of this hacky mess.
    // Safety: As we are adding read_only when it isn't defined, it will always exist
    let read_only_pos = unsafe {
        flags
            .iter()
            .position(|x| x.0 == "read_only")
            .unwrap_unchecked()
    };
    let read_only = flags.remove(read_only_pos).1;
    if let Some(err) = apply_derive(
        &mut item,
        &mut flags,
        cfg,
        false,
        false,
        read_only.as_bool(),
    ) {
        return err.into();
    }
    if c_compatible {
        item.attrs.push(syn::parse_quote! {
            #[cfg_attr(feature = "c_compatible", repr(C))]
        });
    }
    quote! { #item }.into()
}
// /// Attribute macro to conditionally derive codec traits.
// ///
// /// Applies serialization/deserialization derives based on enabled features.
// /// Supports: `serde`, `bitcode`, `wincode`, `compactly`
// /// Optionally: `strum`, `enum_ext`
// ///
// /// # Example
// /// ```ignore
// /// #[derive_codec(serde = true, bitcode = false, wincode = true, compactly = true)]
// /// pub struct MyData {
// ///     value: i32,
// /// }
// /// ```
// // #[proc_macro_attribute]
// fn derive_possible_configured(args: TokenStream, input: TokenStream) -> TokenStream {
//     let mut item = parse_macro_input!(input as DeriveInput);
//     let FlagList(mut flags) = parse_macro_input!(args as FlagList);
//     if let Some(err) = apply_derive(
//         &mut item,
//         &mut flags,
//         get_codec_definition(),
//         false,
//         true,
//         false,
//     ) {
//         return err.into();
//     }
//     if let syn::Data::Enum(_) = item.data
//         && let Some(err) = apply_derive(
//             &mut item,
//             &mut flags,
//             get_enum_definition(),
//             false,
//             false,
//             false,
//         )
//     {
//         return err.into();
//     }
//     quote! { #item }.into()
// }

// /// Attribute macro to conditionally derive codec traits.
// ///
// /// Applies serialization/deserialization derives based on enabled features.
// /// Supports: serde, bitcode, wincode, compactly
// ///
// /// # Example
// /// ```ignore
// /// #[derive_codec(serde = true, bitcode = false, wincode = true, compactly = true)]
// /// pub struct MyData {
// ///     value: i32,
// /// }
// /// ```
// // #[proc_macro_attribute]
// fn derive_codec(args: TokenStream, input: TokenStream) -> TokenStream {
//     let mut item = parse_macro_input!(input as DeriveInput);
//     let FlagList(mut flags) = parse_macro_input!(args as FlagList);
//     if let Some(err) = apply_derive(
//         &mut item,
//         &mut flags,
//         get_codec_definition(),
//         false,
//         false,
//         false,
//     ) {
//         return err.into();
//     }
//     quote! { #item }.into()
// }
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Settings for the user
enum UserSetting {
    True,
    False,
    NonConst,
}
impl From<bool> for UserSetting {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}
impl UserSetting {
    const fn as_bool(self) -> bool {
        match self {
            Self::False => false,
            Self::True | Self::NonConst => true,
        }
    }
    const fn allow_const(self) -> bool {
        matches!(self, Self::True)
    }
}
impl Parse for UserSetting {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // `true` / `false` are keywords, so parse them as LitBool.
        if input.peek(syn::LitBool) {
            let value: syn::LitBool = input.parse()?;
            return Ok(if value.value() {
                Self::True
            } else {
                Self::False
            });
        }

        // Parse the remaining forms.
        let first: syn::Ident = input.parse()?;
        let mut value = first.to_string();

        // Support `non-const`.
        if input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            let second: syn::Ident = input.parse()?;
            value.push('-');
            value.push_str(&second.to_string());
        }

        match value.to_ascii_lowercase().as_str() {
            "nonconst" | "non_const" | "non-const" => Ok(Self::NonConst),
            _ => Err(syn::Error::new(
                first.span(),
                "expected `true`, `false`, `nonconst`, `non_const`, or `non-const`",
            )),
        }
    }
}
struct FlagList(Vec<(String, UserSetting)>);

impl Parse for FlagList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: UserSetting = input.parse()?;

            items.push((key.to_string(), value));

            let _ = input.parse::<Token![,]>();
        }

        Ok(Self(items))
    }
}

// /// Attribute macro for better enum deriving.
// ///
// /// Applies strum and optional `enum_ext` derives to enums.
// /// Supports: serde, strum, `enum_ext`
// ///
// /// # Example
// /// ```ignore
// /// #[derive_better_enum(serde = true, strum = true, enum_ext = false)]
// /// pub enum Color {
// ///     Red,
// ///     Green,
// ///     Blue,
// /// }
// /// ```
// // #[proc_macro_attribute]
// fn derive_better_enum(args: TokenStream, input: TokenStream) -> TokenStream {
//     let mut item = parse_macro_input!(input as DeriveInput);
//     let FlagList(mut flags) = parse_macro_input!(args as FlagList);
//     if let Some(err) = apply_derive(
//         &mut item,
//         &mut flags,
//         get_enum_definition(),
//         false,
//         false,
//         false,
//     ) {
//         return err.into();
//     }
//     quote! { #item }.into()
// }

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
            std::ptr::copy_nonoverlapping(base_ptr.add(len - 1), base_ptr.add(index), 1);
        }

        vec.set_len(len - 1);

        removed_item
    }
}
