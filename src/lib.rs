pub mod audio_conversion;
pub mod database;
pub mod decoders;
pub mod models;
pub mod playback;
pub mod playback_loop;
pub mod queue;
pub mod schema;

/// Shorthand for Result
pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

/// Create a macro to implement into for structs in enums
///
/// ```rust
///    # #[macro_use]
///    use rmusic::struct_in_enum;
///    struct One {}
///    struct Two {}
///    pub enum Enum {
///        A(One),
///        B(Two),
///    }
///
///    struct_in_enum!(Enum, link_enum);
///    // implements into for one to A etc.
///    link_enum!(A: One, B: Two);
/// ```
#[macro_export]
macro_rules! struct_in_enum {
    ($enum_name:ident, $name:ident) => {
        struct_in_enum!{#internal {$enum_name, $name} $}

    };
    (#internal {$enum_name:ident, $name:ident} $dollar:tt) => {
        /// Implement into for structs to the Enum
        /// ```rust
        ///    # use rmusic::struct_in_enum;
        ///    # #[macro_use]
        ///    # struct One {}
        ///    # struct Two {}
        ///    # pub enum Enum {
        ///    #     A(One),
        ///    #     B(Two),
        ///    # }
        ///
        ///    # struct_in_enum!(Enum, link_enum);
        ///
        ///     link_enum!(A: One, B: Two);
        /// ```
        macro_rules! $name {
            ($dollar ( $dollar variant:ident : $dollar struct_type:path), +) => { $dollar (
                #[allow(clippy::from_over_into)]
                impl Into<$enum_name> for  $dollar struct_type {
                    fn into(self) -> $enum_name {
                        $enum_name:: $dollar variant(self)
                    }
                }
            )+};
        }
    };
}
