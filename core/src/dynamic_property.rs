use std::{any::Any, marker::PhantomData};

use anyhow::{bail, Result};
use dyn_clone::DynClone;
use dynisland_abi::module::ActivityIdentifier;

pub trait ValidDynType: Any + Sync + Send + DynClone {
    fn as_any(&self) -> &dyn Any;
}
impl<T: Any + Sync + Send + Clone> ValidDynType for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct PropertyUpdate {
    pub activity_id: ActivityIdentifier,
    pub property_name: String,
    pub value: Box<dyn ValidDynType>,
}
pub struct DynamicPropertyAny {
    pub backend_channel: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    pub activity_id: ActivityIdentifier,
    pub property_name: String,
    pub value: Box<dyn ValidDynType>,
}
pub struct DynamicProperty<T: ValidDynType> {
    property: DynamicPropertyAny,
    t: PhantomData<T>,
}

impl<T: ValidDynType> Clone for DynamicProperty<T> {
    fn clone(&self) -> Self {
        Self {
            property: self.property.clone(),
            t: PhantomData,
        }
    }
}

impl Clone for DynamicPropertyAny {
    fn clone(&self) -> Self {
        Self {
            backend_channel: self.backend_channel.clone(),
            property_name: self.property_name.clone(),
            activity_id: self.activity_id.clone(),
            value: dyn_clone::clone_box(&*self.value),
        }
    }
}

impl DynamicPropertyAny {
    pub fn name(&self) -> &str {
        self.property_name.as_str()
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
            let tried_type = std::any::type_name_of_val(&value);
            //checks if it's the same type, doesn't check enum subtype
            bail!("tried to set wrong type:(tried to set type: {tried_type})")
        }
        self.value = Box::new(value);
        match self.backend_channel.send(PropertyUpdate {
            activity_id: self.activity_id.clone(),
            property_name: self.property_name.clone(),
            value: dyn_clone::clone_box(&*self.value),
        }) {
            Ok(_) => Ok(()),
            Err(err) => bail!("error sending update request to ui: {:?}", err),
        }
    }
}

impl<T: ValidDynType> DynamicProperty<T> {
    pub fn name(&self) -> &str {
        self.property.property_name.as_str()
    }

    pub fn get(&self) -> &T {
        crate::cast_dyn_any!(self.property.value, T).unwrap()
    }

    /// returns Err if the value is of the wrong type
    pub fn set(&mut self, value: T) -> Result<()> {
        self.property.set(value)
    }
}

impl<T: ValidDynType> From<DynamicPropertyAny> for DynamicProperty<T> {
    fn from(value: DynamicPropertyAny) -> Self {
        Self {
            property: value,
            t: PhantomData,
        }
    }
}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}
impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[macro_export]
macro_rules! cast_dyn_any {
    ($val:expr, $type:ty) => {
        $val.as_any().downcast_ref::<$type>()
    };
}

// #[macro_export]
// macro_rules! cast_dyn_any_mut {
//     ($val:expr, $type:ty) => {
//         (&mut $val).as_any().downcast_mut::<$type>()
//     };
// }
