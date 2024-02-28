pub mod base_module;
pub mod graphics;

#[macro_export]
macro_rules! randomize_name {
    ($name:literal) => {
        std::concat!($name, "_", const_random::const_random!(u16))
    };
}
