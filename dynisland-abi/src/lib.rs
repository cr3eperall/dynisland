use abi_stable::StableAbi;
use glib::translate::{FromGlibPtrNone, ToGlibPtr};
use gtk::{Application, Widget};

pub mod activity_identifier;
pub mod layout;
pub mod module;

#[repr(C)]
#[derive(StableAbi)]
pub struct SabiWidget {
    //FIXME check if lifetimes are needed
    #[sabi(last_prefix_field)]
    pub widget_ref: *mut core::ffi::c_void,
}

// this can be send, because gtk::Widget can be processed only in the UI thread
//TODO add a better explanation for why this is necessary
unsafe impl Send for SabiWidget {}

impl From<Widget> for SabiWidget {
    fn from(widget: Widget) -> Self {
        let widget_ptr: *mut gtk::ffi::GtkWidget = widget.to_glib_none().0;
        Self {
            widget_ref: widget_ptr as *mut core::ffi::c_void,
        }
    }
}

impl TryInto<Widget> for SabiWidget {
    type Error = String;
    fn try_into(self) -> Result<Widget, Self::Error> {
        unsafe {
            let widget: *mut gtk::ffi::GtkWidget = self.widget_ref as _;
            if widget.is_null() {
                return Err(String::from("SabiWidget pointer is NULL"));
            }
            if !glib::types::instance_of::<gtk::Widget>(widget as *const _) {
                return Err(String::from(
                    "SabiWidget pointer is not a widget, maybe it was already deallocated",
                ));
            }
            Ok(gtk::Widget::from_glib_none(widget))
        }
    }
}

#[repr(C)]
#[derive(StableAbi)]
pub struct SabiApplication {
    //FIXME check if lifetimes are needed
    #[sabi(last_prefix_field)]
    pub application_ref: *mut core::ffi::c_void,
}

// this can be send, because gtk::Application can be processed only in the UI thread
//TODO check if send is necessary
// unsafe impl Send for SabiApplication {}

impl From<Application> for SabiApplication {
    fn from(app: Application) -> Self {
        let application_ptr: *mut gtk::ffi::GtkApplication = app.to_glib_none().0;
        Self {
            application_ref: application_ptr as *mut core::ffi::c_void,
        }
    }
}

impl TryInto<Application> for SabiApplication {
    type Error = String;
    fn try_into(self) -> Result<Application, Self::Error> {
        unsafe {
            let application: *mut gtk::ffi::GtkApplication = self.application_ref as _;
            if application.is_null() {
                return Err(String::from("SabiApplication pointer is NULL"));
            }
            if !glib::types::instance_of::<gtk::Application>(application as *const _) {
                return Err(String::from(
                    "SabiApplication pointer is not an application, maybe it was already deallocated",
                ));
            }
            Ok(gtk::Application::from_glib_none(application))
        }
    }
}
