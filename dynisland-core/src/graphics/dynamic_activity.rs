use anyhow::{anyhow, bail, Ok, Result};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc::UnboundedSender, Mutex};

use crate::base_module::{
    DynamicActivity, DynamicProperty, PropertyUpdate, SubscribableProperty, ValidDynType,
    ValidDynamicClosure,
};

use super::activity_widget::ActivityWidget;

impl DynamicActivity {
    pub fn new(ui_send: UnboundedSender<PropertyUpdate>, name: &str) -> Self {
        Self {
            widget: ActivityWidget::new(name),
            property_dictionary: HashMap::new(),
            ui_send,
        }
    }

    pub fn set_activity_widget(&mut self, widget: ActivityWidget) {
        widget.set_name(self.widget.name());
        self.widget = widget;
    }
    pub fn get_activity_widget(&self) -> ActivityWidget {
        self.widget.clone()
    }
    pub fn get_identifier(&self) -> String {
        self.widget.name().to_string()
    }

    /// Returns Err if the property already exists
    pub fn add_dynamic_property<T>(&mut self, name: &str, initial_value: T) -> Result<()>
    where
        T: ValidDynType,
    {
        if self.property_dictionary.contains_key(name) {
            bail!("propery already added")
        }
        let prop = DynamicProperty {
            backend_channel: self.ui_send.clone(),
            activity_id: self.get_identifier(),
            property_name: name.to_string(),
            value: Box::new(initial_value),
        };
        let subs_prop = SubscribableProperty {
            property: Arc::new(Mutex::new(prop)),
            subscribers: Vec::new(),
        };
        self.property_dictionary.insert(name.to_string(), subs_prop);
        Ok(())
    }

    /// Returns Err if the property doesn't exist
    pub fn subscribe_to_property<F>(&mut self, name: &str, callback: F) -> Result<()>
    where
        F: ValidDynamicClosure + 'static,
    {
        let prop = self
            .property_dictionary
            .get_mut(name)
            .ok_or_else(|| anyhow!("property {} doesn't exist on this activity", name))?;
        prop.subscribers.push(Box::new(callback));
        Ok(())
    }

    pub fn get_subscribers(&self, name: &str) -> Result<&[Box<dyn ValidDynamicClosure>]> {
        let prop = self
            .property_dictionary
            .get(name)
            .ok_or_else(|| anyhow!("property {} doesn't exist on this activity", name))?;
        Ok(prop.subscribers.as_slice())
    }

    /// for producer, returns Err if the property doesn't exist
    pub fn get_property(&self, name: &str) -> Result<Arc<Mutex<DynamicProperty>>> {
        match self.property_dictionary.get(name) {
            Some(property) => Ok(property.property.clone()),
            None => bail!("property {} doesn't exist on this activity", name),
        }
    }
}
