macro_rules! u8_enum {
    (
        $(#[$attr:meta])*
        $vis:vis enum $name:ident $(= $default:ident)?;
        $(
            $(#[$variant_attr:meta])*
            $variant:ident = $discrim:literal
        ),+$(,)?
    ) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[derive(::num_enum::TryFromPrimitive, ::num_enum::IntoPrimitive)]
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(try_from = "u8", into = "u8")]
        #[repr(u8)]
        $(#[$attr])*
        $vis enum $name {$(
            $(#[$variant_attr])*
            $variant = $discrim
        ),+}
        $(impl Default for $name { fn default() -> Self { Self::$default } })?
    }
}

macro_rules! string_enum {
    (
        $(#[$attr:meta])*
        $vis:vis enum $name:ident $(= $default:ident)?;
        $(
            $(#[$variant_attr:meta])*
            $variant:ident $(= $discrim:literal)?
        ),+$(,)?
    ) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(try_from = "String", into = "&'static str")]
        $(#[$attr])*
        $vis enum $name {$( $(#[$variant_attr])* $variant ),+}
        impl $name {
            pub fn as_str(&self) -> &'static str {
                // stick to variant names, matching Debug impl
                match self {
                    $(Self::$variant => stringify!($variant)),+
                }
            }
        }
        impl ::std::str::FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> ::std::result::Result<Self, String> {
                // Accept either version for FromStr (and thus for Deserialize)
                match s {
                    $(stringify!($variant) $( | $discrim)? => Ok(Self::$variant),)+
                    s => Err(format!("Invalid {} value: {:?}", stringify!($name), s)),
                }
            }
        }
        impl ::std::convert::From<$name> for &'static str {
            fn from(value: $name) -> Self {
                // If provided, use discriminants for Into (and thus for Serialize)
                match value {
                    $(
                        $name::$variant => {
                            let s = $($discrim; let _ = )? stringify!($variant);
                            s
                        }
                    ),+
                }
            }
        }
        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
        // It's incredibly dumb that there's no blanket impl for this.
        impl ::std::convert::TryFrom<String> for $name {
            type Error = String;
            fn try_from(s: String) -> std::result::Result<Self, String> {
                // Forward to FromStr
                s.as_str().parse()
            }
        }
        $(impl Default for $name { fn default() -> Self { Self::$default } })?
    }
}

macro_rules! option_struct {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident;
        $(
            $(#[$field_attr:meta])*
            $field_vis:vis $field:ident: $type:ty
        ),+$(,)?
    ) => {
        #[derive(Serialize, Deserialize)]
        $(#[$attr])*
        $vis struct $name {$(
            #[serde(default, skip_serializing_if = "Option::is_none")]
            $(#[$field_attr])*
            $field_vis $field: Option<$type>,
        )*}
    }
}
