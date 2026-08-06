use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $($variant:ident => $value:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = ();

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($value => Ok(Self::$variant)),+,
                    _ => Err(()),
                }
            }
        }
    };
}

string_enum! {
    pub enum Visibility {
        Private => "private",
        Unlisted => "unlisted",
        Public => "public",
    }
}

string_enum! {
    pub enum MediaStatus {
        Importing => "importing",
        Queued => "queued",
        Processing => "processing",
        Ready => "ready",
        Failed => "failed",
        Deleting => "deleting",
    }
}

string_enum! {
    pub enum JobStatus {
        Queued => "queued",
        Running => "running",
        Succeeded => "succeeded",
        Failed => "failed",
        Cancelled => "cancelled",
    }
}

string_enum! {
    pub enum JobKind {
        RemoteImport => "remote_import",
        Transcode => "transcode",
        Delete => "delete",
    }
}
