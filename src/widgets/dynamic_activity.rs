use anyhow::{anyhow, bail, Ok, Result};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc::UnboundedSender, Mutex};

use super::{
    dynamic_property::{DynamicProperty, ValidDynType, ValidDynamicClosure, PropertyUpdate},
    activity_widget::ActivityWidget,
};


pub struct SubscribableProperty {
    pub property: Arc<Mutex<DynamicProperty>>,
    pub subscribers: Vec<Box<dyn ValidDynamicClosure>>,
}

pub struct DynamicActivity {
    pub widget: ActivityWidget,
    pub property_dictionary: HashMap<String, SubscribableProperty>,
    pub ui_send: UnboundedSender<PropertyUpdate>,
}

impl DynamicActivity {
    pub fn new(ui_send: UnboundedSender<PropertyUpdate>) -> Self {
        Self {
            widget: ActivityWidget::new(),
            property_dictionary: HashMap::new(),
            ui_send,
        }
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
            name: name.to_string(),
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
