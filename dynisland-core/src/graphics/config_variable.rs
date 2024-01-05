use dyn_clone::DynClone;

#[derive(Clone, Debug)]
pub struct ConfigVariable<T>
where
    T: DynClone,
{
    pub value: T,
    pub set_by_module: bool,
}

impl<T: DynClone> ConfigVariable<T> {
    pub fn new(value: T) -> Self {
        ConfigVariable {
            value,
            set_by_module: false,
        }
    }
}

#[macro_export]
macro_rules! implement_get_set {
    ($vis:vis, $val:tt, $type:ty) => {
        concat_idents::concat_idents!(name = get_, $val {
            $vis fn name(&self)-> $type{
                self.$val.value.clone()
            }
        });
        concat_idents::concat_idents!(name = get_, $val, _set_by_module {
            $vis fn name(&self) -> bool {
                self.$val.set_by_module
            }
        });
        concat_idents::concat_idents!(name = set_, $val {
            $vis fn name(&mut self, value: $type, module: bool) -> Result<()> {
                // trace!("tried to set value {:?}", value);
                // if self.$val.value.eq(&value) {
                //     return Ok(());
                // }
                if module {
                    self.$val.set_by_module = true;
                } else if self.$val.set_by_module {
                    return Ok(());
                }
                self.$val.value = value;
                self.update_provider()
            }
        });
    };
}
