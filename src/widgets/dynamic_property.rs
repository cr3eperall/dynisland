use anyhow::{bail, Result};

use crate::modules::base_module::{
    DynamicProperty, PropertyUpdate, ValidDynType, ValidDynamicClosure,
};

impl Clone for Box<dyn ValidDynamicClosure> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

impl Clone for DynamicProperty {
    fn clone(&self) -> Self {
        Self {
            backend_channel: self.backend_channel.clone(),
            name: self.name.clone(),
            value: dyn_clone::clone_box(&*self.value),
        }
    }
}

impl DynamicProperty {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn get(&self) -> &dyn ValidDynType {
        &*self.value
    }

    /// returns Err if the value is of the wrong type
    pub fn set<T>(&mut self, value: T) -> Result<()>
    where
        T: ValidDynType,
    {
        if (*self.value).type_id() != value.type_id() {
            //checks if it's the same type, doesn't check enum subtype
            bail!("tried to set wrong type")
        }
        self.value = Box::new(value);
        match self.backend_channel.send(PropertyUpdate(
            self.name.clone(),
            dyn_clone::clone_box(&*self.value),
        )) {
            Ok(_) => Ok(()),
            Err(err) => bail!("error sending update request to ui: {:?}", err),
        }
    }
}

#[macro_export]
macro_rules! cast_dyn_any {
    ($val:expr, $type:ty) => {
        ($val as &dyn std::any::Any).downcast_ref::<$type>()
    };
}
