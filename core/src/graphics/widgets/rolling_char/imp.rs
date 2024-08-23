use std::cell::RefCell;

use glib_macros::Properties;
use gtk::{prelude::*, subclass::prelude::*};

use super::RollingChar;

//TODO implement vertical orientation and builder

#[derive(Properties)]
#[properties(wrapper_type = RollingChar)]
pub struct RollingCharPriv {
    pub(super) primary_label: RefCell<gtk::Label>,

    pub(super) secondary_label: RefCell<gtk::Label>,

    primary_label_active: RefCell<bool>,

    #[property(get, set, nick = "Current Char", builder('-'))]
    pub(super) current_char: RefCell<char>,
}

impl Default for RollingCharPriv {
    fn default() -> Self {
        RollingCharPriv {
            primary_label: RefCell::new(gtk::Label::new(Some("-"))),
            secondary_label: RefCell::new(gtk::Label::new(Some("-"))),
            primary_label_active: RefCell::new(true),
            current_char: RefCell::new('-'),
        }
    }
}

#[glib::derived_properties]
impl ObjectImpl for RollingCharPriv {
    fn constructed(&self) {
        self.parent_constructed();

        let label_1 = self.primary_label.borrow();
        label_1.add_css_class("in");
        label_1.set_xalign(0.5);
        label_1.set_yalign(0.5);
        label_1.set_halign(gtk::Align::Center);
        label_1.set_valign(gtk::Align::Center);

        let label_2 = self.secondary_label.borrow();
        label_2.add_css_class("out");
        label_2.set_xalign(0.5);
        label_2.set_yalign(0.5);
        label_2.set_halign(gtk::Align::Center);
        label_2.set_valign(gtk::Align::Center);

        label_1.set_parent(self.obj().as_ref());
        label_2.set_parent(self.obj().as_ref());
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "current-char" => {
                let new_char: char = value.get().unwrap();
                if *self.current_char.borrow() == new_char {
                    return;
                }
                self.current_char.replace(new_char);

                let label_1 = self.primary_label.borrow();
                let label_2 = self.secondary_label.borrow();

                let primary_label_active = *self.primary_label_active.borrow();
                if primary_label_active {
                    label_2.set_text(&new_char.to_string());
                    label_1.set_css_classes(&["out"]);
                    label_2.set_css_classes(&["in"]);
                } else {
                    label_1.set_text(&new_char.to_string());
                    label_1.set_css_classes(&["in"]);
                    label_2.set_css_classes(&["out"]);
                }
                self.primary_label_active.replace(!primary_label_active);
            }
            x => panic!("Tried to set inexistant property of RollingChar: {}", x),
        }
    }

    fn dispose(&self) {
        let label_1 = self.primary_label.borrow();
        let label_2 = self.secondary_label.borrow();
        label_1.unparent();
        label_2.unparent();
    }
}

impl WidgetImpl for RollingCharPriv {}
