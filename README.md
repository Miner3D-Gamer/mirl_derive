# Mirl Derive (0.0.0-alpha)

#### Mirive - A lib for easily deriving other derivatives

> This lib unlike every other `mirl` crate does not require nightly as it is used in `mirl_build_tools`

<sub>No flags</sub>

### Entry points

The macro `#[mirl_derive::derive_all]`, see its documentation for more information.

**The suggested usage:**

Inside `cargo.toml`:

```toml
[features]
mirl_derive = ["dep:mirl_derive"]

serde = ["serde/derive", "serde/std", "std", "mirl_derive"]
bitcode = ["dep:bitcode", "mirl_derive"]
wincode = ["dep:wincode", "mirl_derive"]
compactly = ["dep:compactly", "mirl_derive"]
zerocopy = ["dep:zerocopy", "mirl_derive"]

strum = ["dep:strum", "mirl_derive"]
enum_ext = ["dep:enum_ext", "mirl_derive"]
```
<details>
<summary>[dependencies]</summary>

```toml
[dependencies]
mirl_derive = { version = "0.0.0-alpha", optional = true }

# Codec
compactly = { version = "0.1.6", optional = true }
serde = { version = ">=1.0", optional = true, default-features = false, features = [
    "derive",
] }
bitcode = { version = "0.6.9", optional = true, default-features = false, features = [
    "derive",
] }
wincode = { version = "0.5.3", optional = true, features = ["derive"] }
zerocopy = { version = "0.8.48", optional = true, default-features = false, features = [
    "float-nightly",
    "derive",
] }

# Enum functionality
strum = { version = ">=0.28", optional = true, default-features = false, features = [
    "derive",
] }
enum_ext = { version = "0.6.0", optional = true, default-features = false }

```

</details>


The code above any struct/enum:

```rust
#[cfg_attr(feature = "mirl_derive", mirl_derive::derive_all)]
struct MyStruct {}

#[cfg_attr(feature = "mirl_derive", mirl_derive::derive_all)]
enum MyEnum {}
```

Unions are unsupported. Considering that they are almost unused anyways, this shouldn't be too much of a hassle.

### Purpose

Conveniently saving lines on not repeating the same 5 derive lines on every struct and enum

### Disclaimer

No Disclaimers for now

### Origin

Copy pasting 3 to 5 `cfg_attr` above every struct and enum was getting way more annoying than worth it so I explored the world of procedural macros.
It's a weird world for sure but also a very powerful one.
