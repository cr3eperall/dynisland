use std::{rc::Rc, sync::Arc};

use tokio::sync::Mutex;

use crate::base_module::{ActivityMap, DynamicActivity, DynamicProperty};
use anyhow::{anyhow, bail, Result};

impl ActivityMap {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get_activity(&self, identifier: &str) -> Result<Rc<Mutex<DynamicActivity>>> {
        self.map
            .get(identifier)
            .cloned()
            .ok_or_else(|| anyhow!("Activity {} not found", identifier))
    }
    pub async fn insert_activity(&mut self, activity: Rc<Mutex<DynamicActivity>>) -> Result<()> {
        let activity_id = activity.lock().await.get_identifier();
        if self.map.contains_key(&activity_id.activity()) {
            bail!("activity {} was already registered", activity_id);
        }
        self.map.insert(activity_id.activity(), activity.clone());
        Ok(())
    }
    pub fn get_property_blocking(
        &self,
        activity_id: &str,
        property_name: &str,
    ) -> Result<Arc<Mutex<DynamicProperty>>> {
        self.get_activity(activity_id)?
            .blocking_lock()
            .get_property(property_name)
    }
    pub async fn get_property(
        &self,
        activity_id: &str,
        property_name: &str,
    ) -> Result<Arc<Mutex<DynamicProperty>>> {
        self.get_activity(activity_id)?
            .lock()
            .await
            .get_property(property_name)
    }
}
