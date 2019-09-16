use std::sync::{Arc, Mutex};

use rbx_dom_weak::{RbxId, RbxTree};
use rlua::{FromLua, MetaMethod, ToLua, UserData, UserDataMethods};

#[derive(Clone)]
pub struct LuaInstance {
    pub tree: Arc<Mutex<RbxTree>>,
    pub id: RbxId,
}

impl LuaInstance {
    pub fn new(tree: Arc<Mutex<RbxTree>>, id: RbxId) -> Self {
        LuaInstance { tree, id }
    }
}

impl UserData for LuaInstance {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("FindFirstChild", |_context, this, arg: String| {
            let tree = this.tree.lock().unwrap();

            let instance = tree
                .get_instance(this.id)
                .ok_or_else(|| rlua::Error::external("Instance was destroyed"))?;

            let child = instance
                .get_children_ids()
                .into_iter()
                .copied()
                .find(|id| {
                    if let Some(child_instance) = tree.get_instance(*id) {
                        return child_instance.name == arg;
                    }

                    return false;
                })
                .map(|id| LuaInstance::new(Arc::clone(&this.tree), id));

            Ok(child)
        });

        methods.add_method("GetChildren", |_context, this, _args: ()| {
            let tree = this.tree.lock().unwrap();

            let instance = tree
                .get_instance(this.id)
                .ok_or_else(|| rlua::Error::external("Instance was destroyed"))?;

            let children: Vec<LuaInstance> = instance
                .get_children_ids()
                .into_iter()
                .map(|id| LuaInstance::new(Arc::clone(&this.tree), *id))
                .collect();

            Ok(children)
        });

        methods.add_meta_method(MetaMethod::ToString, |context, this, _arg: ()| {
            let tree = this.tree.lock().unwrap();

            let instance = tree
                .get_instance(this.id)
                .ok_or_else(|| rlua::Error::external("Instance was destroyed"))?;

            Ok(instance.name.as_str().to_lua(context))
        });

        methods.add_meta_method(MetaMethod::Index, |context, this, key: String| {
            let tree = this.tree.lock().unwrap();

            let instance = tree
                .get_instance(this.id)
                .ok_or_else(|| rlua::Error::external("Instance was destroyed"))?;

            match key.as_str() {
                "Name" => instance.name.as_str().to_lua(context),
                "ClassName" => instance.class_name.as_str().to_lua(context),
                "Parent" => match instance.get_parent_id() {
                    Some(parent_id) => {
                        if parent_id == tree.get_root_id() {
                            Ok(rlua::Value::Nil)
                        } else {
                            LuaInstance::new(Arc::clone(&this.tree), parent_id).to_lua(context)
                        }
                    }
                    None => Ok(rlua::Value::Nil),
                },
                _ => instance
                    .get_children_ids()
                    .into_iter()
                    .copied()
                    .find(|id| {
                        if let Some(child_instance) = tree.get_instance(*id) {
                            return child_instance.name == key;
                        }

                        return false;
                    })
                    .map(|id| LuaInstance::new(Arc::clone(&this.tree), id))
                    .ok_or_else(|| {
                        rlua::Error::external(format!(
                            "'{}' is not a valid member of Instance",
                            key
                        ))
                    })?
                    .to_lua(context),
            }
        });

        methods.add_meta_method(
            MetaMethod::NewIndex,
            |context, this, (key, value): (String, rlua::Value)| {
                let mut tree = this.tree.lock().unwrap();

                let instance = tree
                    .get_instance_mut(this.id)
                    .ok_or_else(|| rlua::Error::external("Instance was destroyed"))?;

                match key.as_str() {
                    "Name" => match value {
                        rlua::Value::String(lua_str) => {
                            instance.name = lua_str.to_str()?.to_string();
                            Ok(())
                        }
                        _ => Err(rlua::Error::external(format!("'Name' must be a string."))),
                    },
                    "ClassName" => Err(rlua::Error::external("'ClassName' is read-only.")),
                    "Parent" => {
                        match Option::<LuaInstance>::from_lua(value, context)? {
                            Some(new_parent) => {
                                tree.set_parent(this.id, new_parent.id);
                            }
                            None => {
                                let root_id = tree.get_root_id();
                                tree.set_parent(this.id, root_id);
                            }
                        }

                        Ok(())
                    }
                    _ => Err(rlua::Error::external(format!(
                        "'{}' is not a valid member of Instance",
                        key
                    ))),
                }
            },
        );

        methods.add_meta_function(
            MetaMethod::Eq,
            |_context, (a, b): (LuaInstance, LuaInstance)| Ok(a.id == b.id),
        );
    }
}