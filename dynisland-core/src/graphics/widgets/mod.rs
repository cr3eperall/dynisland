pub mod rolling_number;
pub mod scrolling_label;

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedOrientation")]
pub enum Orientation {
    Horizontal,
    Vertical,
}
