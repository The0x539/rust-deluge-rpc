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
        #[derive(TryFromPrimitive, IntoPrimitive)]
        #[derive(Serialize, Deserialize)]
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

// TODO: renaming?
macro_rules! string_enum {
    (
        $(#[$attr:meta])*
        $vis:vis enum $name:ident $(= $default:ident)?;
        $( $(#[$variant_attr:meta])* $variant:ident ),+$(,)?
    ) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        #[derive(Serialize, Deserialize)]
        #[serde(try_from = "String", into = "&'static str")]
        $(#[$attr])*
        $vis enum $name {$( $(#[$variant_attr])* $variant ),+}
        impl FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> std::result::Result<Self, String> {
                match s {
                    $(stringify!($variant) => Ok(Self::$variant),)+
                    s => Err(format!("Invalid {} value: {:?}", stringify!($name), s)),
                }
            }
        }
        impl Into<&'static str> for $name {
            fn into(self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant),)+
                }
            }
        }
        // It's incredibly dumb that there's no blanket impl for this.
        impl TryFrom<String> for $name {
            type Error = String;
            fn try_from(s: String) -> std::result::Result<Self, String> {
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
