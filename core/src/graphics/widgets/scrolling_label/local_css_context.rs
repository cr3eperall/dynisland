use gtk::CssProvider;
use rand::{distributions::Alphanumeric, Rng};

use crate::{
    config_variable::ConfigVariable, graphics::util::CssSize, implement_config_get_set,
    randomize_name,
};

#[derive(Clone, Debug)]
pub struct ScrollingLabelLocalCssContext {
    //TODO add some way to globally configure and swap the animations (maybe get a string to format from a config)
    css_provider: CssProvider,
    name: String,
    size: i32,
    animation_name: String,
    active: bool,

    config_fade_size: ConfigVariable<CssSize>,
    config_speed: ConfigVariable<f32>, //pixels per second
    config_delay: ConfigVariable<u64>, //millis
}

impl ScrollingLabelLocalCssContext {
    pub fn new(name: &str) -> Self {
        Self {
            css_provider: gtk::CssProvider::new(),
            name: name.to_string(),
            animation_name: "scroll".to_string(),
            size: 0,
            active: true,
            config_fade_size: ConfigVariable::new(CssSize::Percent(4.0)),
            config_speed: ConfigVariable::new(40.0),
            config_delay: ConfigVariable::new(5000),
        }
    }

    implement_config_get_set!(pub, config_fade_size, CssSize);
    implement_config_get_set!(pub, config_speed, f32, self=>{self.set_new_animation_name("scroll"); self.update_provider();});
    implement_config_get_set!(pub, config_delay, u64, self=>{self.set_new_animation_name("scroll"); self.update_provider();});

    // GET
    pub fn get_css_provider(&self) -> &CssProvider {
        &self.css_provider
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_size(&self) -> i32 {
        self.size
    }
    pub fn get_active(&self) -> bool {
        self.active
    }

    // SET
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
        self.update_provider()
    }
    pub fn set_active(&mut self, active: bool, size: i32) {
        if self.active == active && self.size == size {
            return;
        }
        if active && self.size != size {
            self.set_new_animation_name("scroll")
        }
        self.active = active;
        self.size = size;
        self.update_provider()
    }

    fn set_new_animation_name(&mut self, prefix: &str) {
        self.animation_name = prefix
            .chars()
            .chain(
                rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(6)
                    .map(char::from),
            )
            .collect::<String>();
    }

    fn update_provider(&self) {
        // let border_radius = self.border_radius;
        let name = self.name.as_str();
        let active = self.active;
        let size = self.size;
        let mut duration = (size as f32 / self.config_speed.value) * 1000.0; //millis
        let delay = self.config_delay.value as f32;
        duration += delay;
        let start_percentage = (delay / duration) * 100.0;

        // log::debug!("size: {} flag: {}",size, self.anim_restart_flag);
        // debug!("{size_timing_function}");
        let scroll_anim = &self.animation_name;
        let css = if active {
            // log::debug!("active");
            format!(
                r"@keyframes {scroll_anim} {{ /* need 2 animations to swap when i want to reset it */
                    0%    {{ transform: translateX(0px); }}
                    {start_percentage:.3}% {{ transform: translateX(0px); }}
                    100%  {{ transform: translateX(-{size}px); }}
                }}
                .{name} > box {{
                    animation: none;
                    transform: translateX(0px);
                    animation-name: {scroll_anim};
                    animation-duration: {duration}ms;
                    animation-iteration-count: infinite;
                    animation-timing-function: linear;
                    /* animation-delay: 1s; */
                }}"
            )
        } else {
            format!(
                r".{name}> box{{ 
                    animation: none;
                }}"
            )
        };
        // log::debug!("{css}");
        self.css_provider.load_from_string(&css);
    }
}

impl Default for ScrollingLabelLocalCssContext {
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

impl glib::subclass::boxed::BoxedType for ScrollingLabelLocalCssContext {
    const NAME: &'static ::core::primitive::str =
        randomize_name!("BoxedScrollingLabelLocalCssContext");
}
impl glib::prelude::StaticType for ScrollingLabelLocalCssContext {
    #[inline]
    fn static_type() -> glib::Type {
        static TYPE: ::std::sync::OnceLock<glib::Type> = ::std::sync::OnceLock::new();
        *TYPE.get_or_init(glib::subclass::register_boxed_type::<ScrollingLabelLocalCssContext>)
    }
}
impl glib::value::ValueType for ScrollingLabelLocalCssContext {
    type Type = ScrollingLabelLocalCssContext;
}
impl glib::value::ToValue for ScrollingLabelLocalCssContext {
    #[inline]
    fn to_value(&self) -> glib::Value {
        unsafe {
            let ptr: *mut ScrollingLabelLocalCssContext =
                ::std::boxed::Box::into_raw(::std::boxed::Box::new(self.clone()));
            let mut value = glib::Value::from_type_unchecked(
                <ScrollingLabelLocalCssContext as glib::prelude::StaticType>::static_type(),
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
        <ScrollingLabelLocalCssContext as glib::prelude::StaticType>::static_type()
    }
}
impl ::std::convert::From<ScrollingLabelLocalCssContext> for glib::Value {
    #[inline]
    fn from(v: ScrollingLabelLocalCssContext) -> Self {
        unsafe {
            let mut value = glib::Value::from_type_unchecked(
                <ScrollingLabelLocalCssContext as glib::prelude::StaticType>::static_type(),
            );
            glib::gobject_ffi::g_value_take_boxed(
                glib::translate::ToGlibPtrMut::to_glib_none_mut(&mut value).0,
                glib::translate::IntoGlibPtr::<*mut ScrollingLabelLocalCssContext>::into_glib_ptr(v)
                    as *mut _,
            );
            value
        }
    }
}
unsafe impl<'a> glib::value::FromValue<'a> for ScrollingLabelLocalCssContext {
    type Checker = glib::value::GenericValueTypeChecker<Self>;
    #[inline]
    unsafe fn from_value(value: &'a glib::Value) -> Self {
        let ptr =
            glib::gobject_ffi::g_value_dup_boxed(glib::translate::ToGlibPtr::to_glib_none(value).0);
        debug_assert!(!ptr.is_null());
        *::std::boxed::Box::from_raw(ptr as *mut ScrollingLabelLocalCssContext)
    }
}
unsafe impl<'a> glib::value::FromValue<'a> for &'a ScrollingLabelLocalCssContext {
    type Checker = glib::value::GenericValueTypeChecker<Self>;
    #[inline]
    unsafe fn from_value(value: &'a glib::Value) -> Self {
        let ptr =
            glib::gobject_ffi::g_value_get_boxed(glib::translate::ToGlibPtr::to_glib_none(value).0);
        debug_assert!(!ptr.is_null());
        &*(ptr as *mut ScrollingLabelLocalCssContext)
    }
}
impl glib::translate::GlibPtrDefault for ScrollingLabelLocalCssContext {
    type GlibType = *mut ScrollingLabelLocalCssContext;
}
impl glib::translate::FromGlibPtrBorrow<*const ScrollingLabelLocalCssContext>
    for ScrollingLabelLocalCssContext
{
    #[inline]
    unsafe fn from_glib_borrow(
        ptr: *const ScrollingLabelLocalCssContext,
    ) -> glib::translate::Borrowed<Self> {
        glib::translate::FromGlibPtrBorrow::from_glib_borrow(ptr as *mut _)
    }
}
impl glib::translate::FromGlibPtrBorrow<*mut ScrollingLabelLocalCssContext>
    for ScrollingLabelLocalCssContext
{
    #[inline]
    unsafe fn from_glib_borrow(
        ptr: *mut ScrollingLabelLocalCssContext,
    ) -> glib::translate::Borrowed<Self> {
        debug_assert!(!ptr.is_null());
        glib::translate::Borrowed::new(std::ptr::read(ptr))
    }
}
impl glib::translate::FromGlibPtrNone<*const ScrollingLabelLocalCssContext>
    for ScrollingLabelLocalCssContext
{
    #[inline]
    unsafe fn from_glib_none(ptr: *const ScrollingLabelLocalCssContext) -> Self {
        debug_assert!(!ptr.is_null());
        (*ptr).clone()
    }
}
impl glib::translate::FromGlibPtrNone<*mut ScrollingLabelLocalCssContext>
    for ScrollingLabelLocalCssContext
{
    #[inline]
    unsafe fn from_glib_none(ptr: *mut ScrollingLabelLocalCssContext) -> Self {
        glib::translate::FromGlibPtrNone::from_glib_none(ptr as *const _)
    }
}
impl glib::translate::FromGlibPtrFull<*mut ScrollingLabelLocalCssContext>
    for ScrollingLabelLocalCssContext
{
    #[inline]
    unsafe fn from_glib_full(ptr: *mut ScrollingLabelLocalCssContext) -> Self {
        debug_assert!(!ptr.is_null());
        *::std::boxed::Box::from_raw(ptr)
    }
}
impl glib::translate::IntoGlibPtr<*mut ScrollingLabelLocalCssContext>
    for ScrollingLabelLocalCssContext
{
    #[inline]
    unsafe fn into_glib_ptr(self) -> *mut ScrollingLabelLocalCssContext {
        ::std::boxed::Box::into_raw(::std::boxed::Box::new(self)) as *mut _
    }
}
impl<'a> glib::translate::ToGlibPtr<'a, *const ScrollingLabelLocalCssContext>
    for ScrollingLabelLocalCssContext
{
    type Storage = std::marker::PhantomData<&'a Self>;
    #[inline]
    fn to_glib_none(
        &'a self,
    ) -> glib::translate::Stash<'a, *const ScrollingLabelLocalCssContext, Self> {
        glib::translate::Stash(
            self as *const ScrollingLabelLocalCssContext,
            std::marker::PhantomData,
        )
    }
    #[inline]
    fn to_glib_full(&self) -> *const ScrollingLabelLocalCssContext {
        ::std::boxed::Box::into_raw(::std::boxed::Box::new(self.clone()))
    }
}
impl<'a> glib::translate::ToGlibPtr<'a, *mut ScrollingLabelLocalCssContext>
    for ScrollingLabelLocalCssContext
{
    type Storage = std::marker::PhantomData<&'a Self>;
    #[inline]
    fn to_glib_none(
        &'a self,
    ) -> glib::translate::Stash<'a, *mut ScrollingLabelLocalCssContext, Self> {
        glib::translate::Stash(
            self as *const ScrollingLabelLocalCssContext as *mut _,
            std::marker::PhantomData,
        )
    }
    #[inline]
    fn to_glib_full(&self) -> *mut ScrollingLabelLocalCssContext {
        ::std::boxed::Box::into_raw(::std::boxed::Box::new(self.clone())) as *mut _
    }
}
impl glib::HasParamSpec for ScrollingLabelLocalCssContext {
    type ParamSpec = glib::ParamSpecBoxed;
    type SetValue = Self;
    type BuilderFn = fn(&::core::primitive::str) -> glib::ParamSpecBoxedBuilder<Self>;
    fn param_spec_builder() -> Self::BuilderFn {
        Self::ParamSpec::builder
    }
}
