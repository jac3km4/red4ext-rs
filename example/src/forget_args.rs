use red4ext_rs::{
    prelude::{redscript_import, ClassType, NativeRepr},
    types::{IScriptable, Ref, TweakDbId},
};

#[derive(Debug)]
pub struct System;
impl ClassType for System {
    type BaseClass = IScriptable;
    const NAME: &'static str = "ForgetArgs.System";
}

#[redscript_import]
impl System {
    fn items(self: Ref<Self>) -> Ref<Items>;
}

impl System {
    pub(crate) fn on_increase(self: Ref<Self>, key: Key) {
        self.items().increase(key);
    }
    pub(crate) fn on_decrease(self: Ref<Self>) {
        self.items().decrease();
    }
}

#[derive(Debug, Default, PartialEq)]
#[repr(transparent)]
pub struct Key(TweakDbId);

impl PartialEq<Key> for TweakDbId {
    fn eq(&self, other: &Key) -> bool {
        self.eq(&other.0)
    }
}

unsafe impl NativeRepr for Key {
    const NAME: &'static str = TweakDbId::NAME;
    const MANGLED_NAME: &'static str = TweakDbId::MANGLED_NAME;
    const NATIVE_NAME: &'static str = TweakDbId::NATIVE_NAME;
}

#[derive(Debug)]
pub struct Item;
impl ClassType for Item {
    type BaseClass = IScriptable;
    const NAME: &'static str = "ForgetArgs.Item";
}

#[redscript_import]
impl Item {
    fn set(self: &mut Ref<Self>, value: i32) -> ();
    fn get(self: &Ref<Self>) -> i32;
}

#[derive(Debug)]
pub struct Items;
impl ClassType for Items {
    type BaseClass = IScriptable;
    const NAME: &'static str = "ForgetArgs.Items";
}

#[redscript_import]
impl Items {
    fn set_values(self: &mut Ref<Self>, values: Vec<Ref<Item>>) -> ();
    fn set_keys(self: &mut Ref<Self>, keys: Vec<Key>) -> ();
    fn values(self: &Ref<Self>) -> Vec<Ref<Item>>;
    fn keys(self: &Ref<Self>) -> Vec<Key>;
    fn create(self: &Ref<Self>, value: i32) -> Ref<Item>;
}

impl Items {
    fn position(self: &Ref<Self>, key: &Key) -> Option<usize> {
        self.keys().iter().position(|x| x == key)
    }
    pub fn increase(mut self: Ref<Self>, key: Key) {
        let mut values = self.values();
        if let Some(idx) = self.position(&key) {
            let existing = unsafe { values.get_unchecked_mut(idx) };
            existing.set(existing.get() + 1);
        } else {
            let value = self.create(1);
            let mut keys = self.keys();
            keys.push(key);
            values.push(value);
            self.set_keys(keys);
            self.set_values(values);
        }
    }
    pub fn decrease(mut self: Ref<Self>) {
        let mut keys = self.keys();
        let mut values = self.values();
        for (idx, value) in values.clone().iter().enumerate().rev() {
            if value.get() <= 0 {
                keys.remove(idx);
                values.remove(idx);
            } else {
                unsafe{ values.get_unchecked_mut(idx) }.set(-1);
            }
        }
        self.set_keys(keys);
        self.set_values(values);
    }
}
