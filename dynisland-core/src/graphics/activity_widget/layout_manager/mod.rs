mod imp;

glib::wrapper! {
    pub struct ActivityLayoutManager(ObjectSubclass<imp::ActivityLayoutManagerPriv>)
        @extends gtk::LayoutManager;
}
