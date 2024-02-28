use dynisland_abi::ActivityIdentifier;
use gtk::Widget;
use linkme::distributed_slice;
use ron::Value;

use anyhow::Result;

//TODO switch to cdylib
pub type LayoutDefinition = (&'static str, fn() -> Box<dyn LayoutManager>);

#[distributed_slice]
pub static LAYOUTS: [LayoutDefinition];

pub trait LayoutManager {
    #[allow(clippy::new_ret_no_self)]
    fn new() -> Box<dyn LayoutManager>
    where
        Self: Sized;

    fn parse_config(&mut self, config: Value) -> Result<()>;

    fn get_name(&self) -> &'static str;
    fn get_primary_widget(&self) -> gtk::Widget;
    fn add_activity(&mut self, activity_id: &ActivityIdentifier, widget: Widget);
    fn get_activity(&self, activity: &ActivityIdentifier) -> Option<&Widget>;
    fn remove_activity(&mut self, activity: &ActivityIdentifier);
    fn list_activities(&self) -> Vec<&ActivityIdentifier>;
    fn set_focus(&mut self, identifier: &ActivityIdentifier) -> Result<()>;
    fn get_focused(&self) -> Option<&ActivityIdentifier>;
    fn cycle_focus_next(&mut self) -> Result<()>;
    fn cycle_focus_previous(&mut self) -> Result<()>;
}
