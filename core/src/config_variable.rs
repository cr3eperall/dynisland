use dyn_clone::DynClone;

// #[deprecated]
//TODO remove
#[derive(Clone, Debug)]
pub struct ConfigVariable<T>
where
    T: DynClone,
{
    pub(super) value: T,
    pub(super) set_by_user: bool,
}

impl<T: DynClone> ConfigVariable<T> {
    pub fn new(value: T) -> Self {
        ConfigVariable {
            value,
            set_by_user: false,
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
        concat_idents::concat_idents!(name = get_, $val, _set_by_user {
            $vis fn name(&self) -> bool {
                self.$val.set_by_user
            }
        });
        concat_idents::concat_idents!(name = set_, $val {
            $vis fn name(&mut self, value: $type, user: bool){
                if user {
                    self.$val.set_by_user = true;
                } else if self.$val.set_by_user {
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
                self.$val.set_by_user
            }
        });
        concat_idents::concat_idents!(name = set_, $val {
            $vis fn name(&mut $_self, value: $type, user: bool) {
                if user {
                    $_self.$val.set_by_user = true;
                } else if $_self.$val.set_by_user {
                    return
                }
                $_self.$val.value = value;
                $body
            }
        });
    };
}
