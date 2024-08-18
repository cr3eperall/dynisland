// Recursive expansion of object_subclass macro
// =============================================

use std::ffi::CString;

use glib::{ffi::GType, subclass::types::ObjectSubclass, translate::FromGlib};
use gtk::subclass::widget::WidgetClassExt;

use super::{imp::RollingCharPriv, RollingChar};

impl ObjectSubclass for RollingCharPriv {
    type Interfaces = ();
    type Class = glib::subclass::basic::ClassStruct<Self>;
    type Instance = glib::subclass::basic::InstanceStruct<Self>;
    #[inline]
    fn new() -> Self {
        ::std::default::Default::default()
    }
    const NAME: &'static str = "RollingChar";
    type Type = RollingChar;
    type ParentType = gtk::Widget;
    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<gtk::BinLayout>();
        klass.set_css_name("rolling-char");
    }
}
unsafe impl glib::subclass::types::ObjectSubclassType for RollingCharPriv {
    #[inline]
    fn type_data() -> ::std::ptr::NonNull<glib::subclass::TypeData> {
        static mut DATA: glib::subclass::TypeData = glib::subclass::types::INIT_TYPE_DATA;
        unsafe { ::std::ptr::NonNull::from(&mut DATA) }
    }
    #[inline]
    fn type_() -> glib::Type {
        static ONCE: ::std::sync::Once = ::std::sync::Once::new();
        ONCE.call_once(|| {
            unsafe {
                let type_name = CString::new(Self::NAME).unwrap();
                let gtype: GType = glib::gobject_ffi::g_type_from_name(type_name.as_ptr());

                if gtype == glib::gobject_ffi::G_TYPE_INVALID {
                    // type needs to be registered
                    glib::subclass::register_type::<Self>();
                } else {
                    // type was already registered by another module, it should be safe to not register it
                    let type_ = glib::Type::from_glib(gtype);
                    let mut data = Self::type_data();
                    data.as_mut().type_ = type_;
                    //FIXME set other type data like private_offset, this could cause crashes in the future
                    // data.as_mut().private_offset = std::mem::size_of::<glib::subclass::types::PrivateStruct<Self>>()
                }
            }
        });
        unsafe {
            let data = Self::type_data();
            let type_ = data.as_ref().type_();
            type_
        }
    }
}
#[doc(hidden)]
impl glib::subclass::types::FromObject for RollingCharPriv {
    type FromObjectType = <Self as glib::subclass::types::ObjectSubclass>::Type;
    #[inline]
    fn from_object(obj: &Self::FromObjectType) -> &Self {
        <Self as glib::subclass::types::ObjectSubclassExt>::from_obj(obj)
    }
}
#[doc(hidden)]
impl glib::clone::Downgrade for RollingCharPriv {
    type Weak = glib::subclass::ObjectImplWeakRef<RollingCharPriv>;
    #[inline]
    fn downgrade(&self) -> Self::Weak {
        let ref_counted = glib::subclass::prelude::ObjectSubclassExt::ref_counted(self);
        glib::clone::Downgrade::downgrade(&ref_counted)
    }
}
impl RollingCharPriv {
    #[inline]
    pub fn downgrade(&self) -> <Self as glib::clone::Downgrade>::Weak {
        glib::clone::Downgrade::downgrade(self)
    }
}
#[doc(hidden)]
impl ::std::borrow::ToOwned for RollingCharPriv {
    type Owned = glib::subclass::ObjectImplRef<RollingCharPriv>;
    #[inline]
    fn to_owned(&self) -> Self::Owned {
        glib::subclass::prelude::ObjectSubclassExt::ref_counted(self)
    }
}
#[doc(hidden)]
impl ::std::borrow::Borrow<RollingCharPriv> for glib::subclass::ObjectImplRef<RollingCharPriv> {
    #[inline]
    fn borrow(&self) -> &RollingCharPriv {
        self
    }
}
