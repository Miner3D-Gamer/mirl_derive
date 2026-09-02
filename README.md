# Mirl Derive (0.0.0-alpha)

#### Mirive - A lib for easily deriving other derivatives

> This lib unlike every other `mirl` crate does not require nightly as it is used in `mirl_build_tools`

<sub>No flags</sub>

### Entry points

The macro `#[mirl_derive::derive_all]`, see its documentation for more information.

The code above any struct/enum:

```rust
#[mirl_derive::derive_all]
struct MyStruct {}

#[mirl_derive::derive_all]
enum MyEnum {}
```

Unions are unsupported. Considering that they are almost unused anyways, this shouldn't be too much of an issue.

<details>
<summary>

**⮤ Supported Crates:**

</summary>

Inside `cargo.toml`:

```toml
[features]
serde = ["serde", "serde/std", "std"]
bitcode = ["dep:bitcode"]
wincode = ["dep:wincode"]
compactly = ["dep:compactly"]
zerocopy = ["dep:zerocopy"]

strum = ["dep:strum"]
enum_ext = ["dep:enum_ext"]
```

```toml
[dependencies]
mirl_derive = {version = "0.0.0-alpha"}

# Codec
compactly = {version = "0.1.6", optional = true}
serde = {version = ">=1.0", optional = true, features = ["derive"]}
bitcode = {version = "0.6.9", optional = true, features = ["derive"]}
wincode = {version = "0.5.3", optional = true, features = ["derive"]}
zerocopy = {version = "0.8.48", optional = true, features = ["float-nightly", "derive"]}

# Enum functionality
strum = {version = ">=0.28", optional = true, features = ["derive"]}
enum_ext = {version = "0.6.0", optional = true}

```

</details>

### Purpose

Conveniently saving lines on not repeating the same 5 derive lines on every struct and enum

### Disclaimer

No Disclaimers for now

### Origin

Copy pasting 3 to 5 `cfg_attr` above every struct and enum was getting way more annoying than worth it so I explored the world of procedural macros.
It's a weird world for sure but also a very powerful one.
