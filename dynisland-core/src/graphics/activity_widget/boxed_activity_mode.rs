use crate::randomize_name;

#[derive(Clone, Debug, Copy)]
pub enum ActivityMode {
    Minimal = 0,
    Compact = 1,
    Expanded = 2,
    Overlay = 3,
}

impl ToString for ActivityMode {
    fn to_string(&self) -> String {
        match self {
            ActivityMode::Minimal => "minimal".to_string(),
            ActivityMode::Compact => "compact".to_string(),
            ActivityMode::Expanded => "expanded".to_string(),
            ActivityMode::Overlay => "overlay".to_string(),
        }
    }
}

//TODO add explanation for why this is necessary
// Recursive expansion of Boxed macro
// ===================================

impl glib::subclass::boxed::BoxedType for ActivityMode {
    const NAME: &'static ::core::primitive::str = randomize_name!("BoxedActivityMode");
}
impl glib::prelude::StaticType for ActivityMode {
    #[inline]
    fn static_type() -> glib::Type {
        static TYPE: ::std::sync::OnceLock<glib::Type> = ::std::sync::OnceLock::new();
        *TYPE.get_or_init(glib::subclass::register_boxed_type::<ActivityMode>)
    }
}
impl glib::value::ValueType for ActivityMode {
    type Type = ActivityMode;
}
impl glib::value::ToValue for ActivityMode {
    #[inline]
    fn to_value(&self) -> glib::Value {
        unsafe {
            let ptr: *mut ActivityMode = ::std::boxed::Box::into_raw(::std::boxed::Box::new(*self));
            let mut value = glib::Value::from_type_unchecked(
                <ActivityMode as glib::prelude::StaticType>::static_type(),
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
        <ActivityMode as glib::prelude::StaticType>::static_type()
    }
}
impl ::std::convert::From<ActivityMode> for glib::Value {
    #[inline]
    fn from(v: ActivityMode) -> Self {
        unsafe {
            let mut value = glib::Value::from_type_unchecked(
                <ActivityMode as glib::prelude::StaticType>::static_type(),
            );
            glib::gobject_ffi::g_value_take_boxed(
                glib::translate::ToGlibPtrMut::to_glib_none_mut(&mut value).0,
                glib::translate::IntoGlibPtr::<*mut ActivityMode>::into_glib_ptr(v) as *mut _,
            );
            value
        }
    }
}
unsafe impl<'a> glib::value::FromValue<'a> for ActivityMode {
    type Checker = glib::value::GenericValueTypeChecker<Self>;
    #[inline]
    unsafe fn from_value(value: &'a glib::Value) -> Self {
        let ptr =
            glib::gobject_ffi::g_value_dup_boxed(glib::translate::ToGlibPtr::to_glib_none(value).0);
        debug_assert!(!ptr.is_null());
        *::std::boxed::Box::from_raw(ptr as *mut ActivityMode)
    }
}
unsafe impl<'a> glib::value::FromValue<'a> for &'a ActivityMode {
    type Checker = glib::value::GenericValueTypeChecker<Self>;
    #[inline]
    unsafe fn from_value(value: &'a glib::Value) -> Self {
        let ptr =
            glib::gobject_ffi::g_value_get_boxed(glib::translate::ToGlibPtr::to_glib_none(value).0);
        debug_assert!(!ptr.is_null());
        &*(ptr as *mut ActivityMode)
    }
}
impl glib::translate::GlibPtrDefault for ActivityMode {
    type GlibType = *mut ActivityMode;
}
impl glib::translate::FromGlibPtrBorrow<*const ActivityMode> for ActivityMode {
    #[inline]
    unsafe fn from_glib_borrow(ptr: *const ActivityMode) -> glib::translate::Borrowed<Self> {
        glib::translate::FromGlibPtrBorrow::from_glib_borrow(ptr as *mut _)
    }
}
impl glib::translate::FromGlibPtrBorrow<*mut ActivityMode> for ActivityMode {
    #[inline]
    unsafe fn from_glib_borrow(ptr: *mut ActivityMode) -> glib::translate::Borrowed<Self> {
        debug_assert!(!ptr.is_null());
        glib::translate::Borrowed::new(std::ptr::read(ptr))
    }
}
impl glib::translate::FromGlibPtrNone<*const ActivityMode> for ActivityMode {
    #[inline]
    unsafe fn from_glib_none(ptr: *const ActivityMode) -> Self {
        debug_assert!(!ptr.is_null());
        *ptr
    }
}
impl glib::translate::FromGlibPtrNone<*mut ActivityMode> for ActivityMode {
    #[inline]
    unsafe fn from_glib_none(ptr: *mut ActivityMode) -> Self {
        glib::translate::FromGlibPtrNone::from_glib_none(ptr as *const _)
    }
}
impl glib::translate::FromGlibPtrFull<*mut ActivityMode> for ActivityMode {
    #[inline]
    unsafe fn from_glib_full(ptr: *mut ActivityMode) -> Self {
        debug_assert!(!ptr.is_null());
        *::std::boxed::Box::from_raw(ptr)
    }
}
impl glib::translate::IntoGlibPtr<*mut ActivityMode> for ActivityMode {
    #[inline]
    unsafe fn into_glib_ptr(self) -> *mut ActivityMode {
        ::std::boxed::Box::into_raw(::std::boxed::Box::new(self)) as *mut _
    }
}
impl<'a> glib::translate::ToGlibPtr<'a, *const ActivityMode> for ActivityMode {
    type Storage = std::marker::PhantomData<&'a Self>;
    #[inline]
    fn to_glib_none(&'a self) -> glib::translate::Stash<'a, *const ActivityMode, Self> {
        glib::translate::Stash(self as *const ActivityMode, std::marker::PhantomData)
    }
    #[inline]
    fn to_glib_full(&self) -> *const ActivityMode {
        ::std::boxed::Box::into_raw(::std::boxed::Box::new(*self))
    }
}
impl<'a> glib::translate::ToGlibPtr<'a, *mut ActivityMode> for ActivityMode {
    type Storage = std::marker::PhantomData<&'a Self>;
    #[inline]
    fn to_glib_none(&'a self) -> glib::translate::Stash<'a, *mut ActivityMode, Self> {
        glib::translate::Stash(
            self as *const ActivityMode as *mut _,
            std::marker::PhantomData,
        )
    }
    #[inline]
    fn to_glib_full(&self) -> *mut ActivityMode {
        ::std::boxed::Box::into_raw(::std::boxed::Box::new(*self)) as *mut _
    }
}
impl glib::HasParamSpec for ActivityMode {
    type ParamSpec = glib::ParamSpecBoxed;
    type SetValue = Self;
    type BuilderFn = fn(&::core::primitive::str) -> glib::ParamSpecBoxedBuilder<Self>;
    fn param_spec_builder() -> Self::BuilderFn {
        Self::ParamSpec::builder
    }
}
