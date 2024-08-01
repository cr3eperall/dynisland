use std::ffi::CString;

use glib::{ffi::GType, subclass::boxed::BoxedType, translate::FromGlib};
use gtk::CssProvider;
use rand::{distributions::Alphanumeric, Rng};

use crate::{config_variable::ConfigVariable, implement_config_get_set};

use super::boxed_activity_mode::ActivityMode;

#[derive(Clone, Debug)]
pub struct ActivityWidgetLocalCssContext {
    css_provider: CssProvider,
    name: String,

    size: (i32, i32),
    opacity: [f64; 4],
    stretch: [(f64, f64); 4],
    blur: [f64; 4],
    stretch_on_resize: bool,

    config_minimal_height: ConfigVariable<i32>,
    config_minimal_width: ConfigVariable<i32>,
    config_blur_radius: ConfigVariable<f64>,
    config_enable_drag_stretch: ConfigVariable<bool>,
}

#[allow(unused_braces)]
impl ActivityWidgetLocalCssContext {
    pub fn new(name: &str) -> Self {
        Self {
            css_provider: gtk::CssProvider::new(),
            name: name.to_string(),
            size: (40, 40),
            opacity: [1.0, 0.0, 0.0, 0.0],
            stretch: [(1.0, 1.0), (1.0, 1.0), (1.0, 1.0), (1.0, 1.0)],
            blur: [0.0, 1.0, 1.0, 1.0],
            stretch_on_resize: true,

            config_minimal_height: ConfigVariable::new(40),
            config_minimal_width: ConfigVariable::new(60),
            config_blur_radius: ConfigVariable::new(6.0),
            config_enable_drag_stretch: ConfigVariable::new(false),
        }
    }

    implement_config_get_set!(pub, config_minimal_height, i32);
    implement_config_get_set!(pub, config_minimal_width, i32);
    implement_config_get_set!(pub, config_blur_radius, f64);
    implement_config_get_set!(pub, config_enable_drag_stretch, bool);

    // GET
    pub fn get_css_provider(&self) -> CssProvider {
        self.css_provider.clone()
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_size(&self) -> (i32, i32) {
        self.size
    }
    pub fn get_opacity(&self, mode: ActivityMode) -> f64 {
        self.opacity[mode as usize]
    }
    pub fn get_stretch(&self, mode: ActivityMode) -> (f64, f64) {
        self.stretch[mode as usize]
    }
    pub fn get_blur(&self, mode: ActivityMode) -> f64 {
        self.blur[mode as usize]
    }
    pub fn get_stretch_on_resize(&self) -> bool {
        self.stretch_on_resize
    }

    // SET
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
        self.update_provider()
    }
    pub fn set_size(&mut self, size: (i32, i32)) {
        if self.size == size {
            return;
        }
        self.size = (
            i32::max(size.0, self.config_minimal_height.value),
            i32::max(size.1, self.config_minimal_height.value),
        );
        self.update_provider()
    }
    pub fn set_opacity(&mut self, mode: ActivityMode, opacity: f64) {
        if self.opacity[mode as usize] == opacity {
            return;
        }
        self.opacity[mode as usize] = opacity;
        self.update_provider()
    }
    pub fn set_opacity_all(&mut self, opacity: [f64; 4]) {
        if self.opacity == opacity {
            return;
        }
        self.opacity = opacity;
        self.update_provider()
    }
    pub fn set_stretch(&mut self, mode: ActivityMode, stretch: (f64, f64)) {
        if self.stretch[mode as usize] == stretch {
            return;
        }
        self.stretch[mode as usize] = stretch;
        self.update_provider()
    }
    pub fn set_stretch_all(&mut self, stretch: [(f64, f64); 4]) {
        if self.stretch == stretch {
            return;
        }
        self.stretch = stretch;
        self.update_provider()
    }
    pub fn set_blur(&mut self, mode: ActivityMode, blur: f64) {
        if self.blur[mode as usize] == blur {
            return;
        }
        self.blur[mode as usize] = blur;
        self.update_provider()
    }
    pub fn set_blur_all(&mut self, blur: [f64; 4]) {
        if self.blur == blur {
            return;
        }
        self.blur = blur;
        self.update_provider()
    }
    pub fn set_stretch_on_resize(&mut self, stretch: bool) {
        if self.stretch_on_resize == stretch {
            return;
        }
        self.stretch_on_resize = stretch;
        self.update_provider()
    }

    fn update_provider(&self) {
        let (w, h) = self.size;
        // let border_radius = self.border_radius;
        let name = self.name.as_str();
        let (min_opacity, com_opacity, exp_opacity, ove_opacity) = (
            self.opacity[0],
            self.opacity[1],
            self.opacity[2],
            self.opacity[3],
        );
        let stretches = self.stretch.map(|(x, y)| {
            let x = if !x.is_finite() { 1.0 } else { x };
            let y = if !y.is_finite() { 1.0 } else { y };
            (x, y)
        });
        let (min_stretch_x, com_stretch_x, exp_stretch_x, ove_stretch_x) = (
            stretches[0].0,
            stretches[1].0,
            stretches[2].0,
            stretches[3].0,
        );
        let (min_stretch_y, com_stretch_y, exp_stretch_y, ove_stretch_y) = (
            stretches[0].1,
            stretches[1].1,
            stretches[2].1,
            stretches[3].1,
        );
        let (min_blur, com_blur, exp_blur, ove_blur) =
            (self.blur[0], self.blur[1], self.blur[2], self.blur[3]);
        // debug!("{size_timing_function}");
        let css = if self.stretch_on_resize {
            format!(
                r".{name} .activity-background, .{name} .activity-background * {{ 
                    min-width: {w}px; 
                    min-height: {h}px;
                }}
                
                .{name} .mode-minimal{{
                    opacity: {min_opacity};
                    transform: scale({min_stretch_x}, {min_stretch_y});
                    filter: blur({min_blur}px);
                }}
                .{name} .mode-compact{{
                    opacity: {com_opacity};
                    transform: scale({com_stretch_x}, {com_stretch_y});
                    filter: blur({com_blur}px);
                }}
                .{name} .mode-expanded{{
                    opacity: {exp_opacity};
                    transform: scale({exp_stretch_x}, {exp_stretch_y});
                    filter: blur({exp_blur}px);
                }}
                .{name} .mode-overlay{{
                    opacity: {ove_opacity};
                    transform: scale({ove_stretch_x}, {ove_stretch_y});
                    filter: blur({ove_blur}px);
                }}"
            )
        } else {
            format!(
                r".{name} .activity-background, .{name} .activity-background * {{ 
                    min-width: {w}px; 
                    min-height: {h}px;
                }}

                .{name} .mode-minimal{{
                    opacity: {min_opacity};
                    filter: blur({min_blur}px);
                    transform: scale(1,1);
                }}
                .{name} .mode-compact{{
                    opacity: {com_opacity};
                    filter: blur({com_blur}px);
                    transform: scale(1,1);
                }}
                .{name} .mode-expanded{{
                    opacity: {exp_opacity};
                    filter: blur({exp_blur}px);
                    transform: scale(1,1);
                }}
                .{name} .mode-overlay{{
                    opacity: {ove_opacity};
                    filter: blur({ove_blur}px);
                    transform: scale(1,1);
                }}"
            )
        };
        // log::debug!("{css}");
        self.css_provider.load_from_string(&css);
    }
}

impl Default for ActivityWidgetLocalCssContext {
    fn default() -> Self {
        Self::new(
            "c".chars()
                .chain(
                    rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(6)
                        .map(char::from),
                )
                .collect::<String>()
                .as_str(),
        )
    }
}

// Recursive expansion of glib::Boxed macro
// =========================================

impl glib::subclass::boxed::BoxedType for ActivityWidgetLocalCssContext {
    const NAME: &'static ::core::primitive::str = "BoxedActivityWidgetLocalCssContext";
}
impl glib::prelude::StaticType for ActivityWidgetLocalCssContext {
    #[inline]
    fn static_type() -> glib::Type {
        static TYPE: ::std::sync::OnceLock<glib::Type> = ::std::sync::OnceLock::new();
        *TYPE.get_or_init(|| {
            unsafe {
                let type_name = CString::new(<Self as BoxedType>::NAME).unwrap();
                let gtype: GType = glib::gobject_ffi::g_type_from_name(type_name.as_ptr());

                if gtype == glib::gobject_ffi::G_TYPE_INVALID {
                    // type needs to be registered
                    glib::subclass::register_boxed_type::<ActivityWidgetLocalCssContext>()
                } else {
                    glib::Type::from_glib(gtype)
                    // type was already registered by another module, it should be safe to not register it
                }
            }
            // glib::subclass::register_boxed_type::<ActivityMode>()
        })
    }
}
impl glib::value::ValueType for ActivityWidgetLocalCssContext {
    type Type = ActivityWidgetLocalCssContext;
}
impl glib::value::ToValue for ActivityWidgetLocalCssContext {
    #[inline]
    fn to_value(&self) -> glib::Value {
        unsafe {
            let ptr: *mut ActivityWidgetLocalCssContext =
                ::std::boxed::Box::into_raw(::std::boxed::Box::new(self.clone()));
            let mut value = glib::Value::from_type_unchecked(
                <ActivityWidgetLocalCssContext as glib::prelude::StaticType>::static_type(),
            );
            glib::gobject_ffi::g_value_take_boxed(
                glib::translate::ToGlibPtrMut::to_glib_none_mut(&mut value).0,
                ptr as *mut _,
            );
            value
        }
    }
    #[inline]
    fn value_type(&self) -> glib::Type {
        <ActivityWidgetLocalCssContext as glib::prelude::StaticType>::static_type()
    }
}
impl ::std::convert::From<ActivityWidgetLocalCssContext> for glib::Value {
    #[inline]
    fn from(v: ActivityWidgetLocalCssContext) -> Self {
        unsafe {
            let mut value = glib::Value::from_type_unchecked(
                <ActivityWidgetLocalCssContext as glib::prelude::StaticType>::static_type(),
            );
            glib::gobject_ffi::g_value_take_boxed(
                glib::translate::ToGlibPtrMut::to_glib_none_mut(&mut value).0,
                glib::translate::IntoGlibPtr::<*mut ActivityWidgetLocalCssContext>::into_glib_ptr(v)
                    as *mut _,
            );
            value
        }
    }
}
unsafe impl<'a> glib::value::FromValue<'a> for ActivityWidgetLocalCssContext {
    type Checker = glib::value::GenericValueTypeChecker<Self>;
    #[inline]
    unsafe fn from_value(value: &'a glib::Value) -> Self {
        let ptr =
            glib::gobject_ffi::g_value_dup_boxed(glib::translate::ToGlibPtr::to_glib_none(value).0);
        debug_assert!(!ptr.is_null());
        *::std::boxed::Box::from_raw(ptr as *mut ActivityWidgetLocalCssContext)
    }
}
unsafe impl<'a> glib::value::FromValue<'a> for &'a ActivityWidgetLocalCssContext {
    type Checker = glib::value::GenericValueTypeChecker<Self>;
    #[inline]
    unsafe fn from_value(value: &'a glib::Value) -> Self {
        let ptr =
            glib::gobject_ffi::g_value_get_boxed(glib::translate::ToGlibPtr::to_glib_none(value).0);
        debug_assert!(!ptr.is_null());
        &*(ptr as *mut ActivityWidgetLocalCssContext)
    }
}
impl glib::translate::GlibPtrDefault for ActivityWidgetLocalCssContext {
    type GlibType = *mut ActivityWidgetLocalCssContext;
}
impl glib::translate::FromGlibPtrBorrow<*const ActivityWidgetLocalCssContext>
    for ActivityWidgetLocalCssContext
{
    #[inline]
    unsafe fn from_glib_borrow(
        ptr: *const ActivityWidgetLocalCssContext,
    ) -> glib::translate::Borrowed<Self> {
        glib::translate::FromGlibPtrBorrow::from_glib_borrow(ptr as *mut _)
    }
}
impl glib::translate::FromGlibPtrBorrow<*mut ActivityWidgetLocalCssContext>
    for ActivityWidgetLocalCssContext
{
    #[inline]
    unsafe fn from_glib_borrow(
        ptr: *mut ActivityWidgetLocalCssContext,
    ) -> glib::translate::Borrowed<Self> {
        debug_assert!(!ptr.is_null());
        glib::translate::Borrowed::new(std::ptr::read(ptr))
    }
}
impl glib::translate::FromGlibPtrNone<*const ActivityWidgetLocalCssContext>
    for ActivityWidgetLocalCssContext
{
    #[inline]
    unsafe fn from_glib_none(ptr: *const ActivityWidgetLocalCssContext) -> Self {
        debug_assert!(!ptr.is_null());
        (*ptr).clone()
    }
}
impl glib::translate::FromGlibPtrNone<*mut ActivityWidgetLocalCssContext>
    for ActivityWidgetLocalCssContext
{
    #[inline]
    unsafe fn from_glib_none(ptr: *mut ActivityWidgetLocalCssContext) -> Self {
        glib::translate::FromGlibPtrNone::from_glib_none(ptr as *const _)
    }
}
impl glib::translate::FromGlibPtrFull<*mut ActivityWidgetLocalCssContext>
    for ActivityWidgetLocalCssContext
{
    #[inline]
    unsafe fn from_glib_full(ptr: *mut ActivityWidgetLocalCssContext) -> Self {
        debug_assert!(!ptr.is_null());
        *::std::boxed::Box::from_raw(ptr)
    }
}
impl glib::translate::IntoGlibPtr<*mut ActivityWidgetLocalCssContext>
    for ActivityWidgetLocalCssContext
{
    #[inline]
    unsafe fn into_glib_ptr(self) -> *mut ActivityWidgetLocalCssContext {
        ::std::boxed::Box::into_raw(::std::boxed::Box::new(self)) as *mut _
    }
}
impl<'a> glib::translate::ToGlibPtr<'a, *const ActivityWidgetLocalCssContext>
    for ActivityWidgetLocalCssContext
{
    type Storage = std::marker::PhantomData<&'a Self>;
    #[inline]
    fn to_glib_none(
        &'a self,
    ) -> glib::translate::Stash<'a, *const ActivityWidgetLocalCssContext, Self> {
        glib::translate::Stash(
            self as *const ActivityWidgetLocalCssContext,
            std::marker::PhantomData,
        )
    }
    #[inline]
    fn to_glib_full(&self) -> *const ActivityWidgetLocalCssContext {
        ::std::boxed::Box::into_raw(::std::boxed::Box::new(self.clone()))
    }
}
impl<'a> glib::translate::ToGlibPtr<'a, *mut ActivityWidgetLocalCssContext>
    for ActivityWidgetLocalCssContext
{
    type Storage = std::marker::PhantomData<&'a Self>;
    #[inline]
    fn to_glib_none(
        &'a self,
    ) -> glib::translate::Stash<'a, *mut ActivityWidgetLocalCssContext, Self> {
        glib::translate::Stash(
            self as *const ActivityWidgetLocalCssContext as *mut _,
            std::marker::PhantomData,
        )
    }
    #[inline]
    fn to_glib_full(&self) -> *mut ActivityWidgetLocalCssContext {
        ::std::boxed::Box::into_raw(::std::boxed::Box::new(self.clone())) as *mut _
    }
}
impl glib::HasParamSpec for ActivityWidgetLocalCssContext {
    type ParamSpec = glib::ParamSpecBoxed;
    type SetValue = Self;
    type BuilderFn = fn(&::core::primitive::str) -> glib::ParamSpecBoxedBuilder<Self>;
    fn param_spec_builder() -> Self::BuilderFn {
        Self::ParamSpec::builder
    }
}
