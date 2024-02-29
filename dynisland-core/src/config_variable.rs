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
macro_rules! implement_config_get_set {
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
            $vis fn name(&mut self, value: $type, module: bool){
                // trace!("tried to set value {:?}", value);
                // if self.$val.value.eq(&value) {
                //     return Ok(());
                // }
                if module {
                    self.$val.set_by_module = true;
                } else if self.$val.set_by_module {
                    return
                }
                self.$val.value = value;
            }
        });
    };
    ($vis:vis, $val:tt, $type:ty, $_self:ident => $body:block) => {
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
            $vis fn name(&mut $_self, value: $type, module: bool) {
                // trace!("tried to set value {:?}", value);
                // if self.$val.value.eq(&value) {
                //     return Ok(());
                // }
                if module {
                    $_self.$val.set_by_module = true;
                } else if $_self.$val.set_by_module {
                    return
                }
                $_self.$val.value = value;
                $body
            }
        });
    };
}
