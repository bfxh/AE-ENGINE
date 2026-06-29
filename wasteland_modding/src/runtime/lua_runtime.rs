//! Lua 运行时：基于 mlua 的沙箱化 Lua 5.4 执行环境

use crate::sandbox::SandboxConfig;
use mlua::{Lua, MultiValue, Value};
use parking_lot::Mutex;
use std::sync::Arc;

/// Lua 脚本执行错误
#[derive(Debug)]
pub struct LuaError(pub String);

impl std::fmt::Display for LuaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lua error: {}", self.0)
    }
}

impl std::error::Error for LuaError {}

impl From<mlua::Error> for LuaError {
    fn from(e: mlua::Error) -> Self {
        Self(e.to_string())
    }
}

/// Mod 脚本回调注册表
#[derive(Default)]
pub struct CallbackRegistry {
    /// on_init 回调
    pub on_init: Vec<String>,
    /// on_load 回调
    pub on_load: Vec<String>,
    /// on_update 回调（参数：dt）
    pub on_update: Vec<String>,
    /// on_unload 回调
    pub on_unload: Vec<String>,
    /// 自定义事件回调：event_name -> [function_name]
    pub event_handlers: hashbrown::HashMap<String, Vec<String>>,
}

impl CallbackRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_event(&mut self, event: &str, func: &str) {
        self.event_handlers.entry(event.to_string()).or_default().push(func.to_string());
    }

    pub fn get_handlers(&self, event: &str) -> &[String] {
        self.event_handlers.get(event).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

/// Lua 运行时实例：一个 Mod 对应一个 LuaRuntime
pub struct LuaRuntime {
    lua: Lua,
    config: SandboxConfig,
    callbacks: Arc<Mutex<CallbackRegistry>>,
    mod_name: String,
}

impl LuaRuntime {
    /// 创建新的 Lua 运行时
    pub fn new(mod_name: &str, config: SandboxConfig) -> Result<Self, LuaError> {
        let lua = Lua::new();
        let callbacks = Arc::new(Mutex::new(CallbackRegistry::new()));

        // 注册引擎 API
        Self::register_api(&lua, mod_name, &callbacks)?;

        Ok(Self { lua, config, callbacks, mod_name: mod_name.to_string() })
    }

    /// 注册引擎 API 到 Lua 全局表
    fn register_api(
        lua: &Lua,
        mod_name: &str,
        callbacks: &Arc<Mutex<CallbackRegistry>>,
    ) -> Result<(), LuaError> {
        let globals = lua.globals();

        // engine.log(level, message)
        let log_fn = lua.create_function(|_, (level, msg): (String, String)| {
            match level.as_str() {
                "error" => log::error!("[mod] {}", msg),
                "warn" => log::warn!("[mod] {}", msg),
                "info" => log::info!("[mod] {}", msg),
                "debug" => log::debug!("[mod] {}", msg),
                "trace" => log::trace!("[mod] {}", msg),
                _ => log::info!("[mod] {}", msg),
            }
            Ok(())
        })?;
        let engine = lua.create_table()?;
        engine.set("log", log_fn)?;

        // engine.on_init(function)
        let cb_clone = callbacks.clone();
        let on_init_fn = lua.create_function(move |lua, func: mlua::Function| {
            let name = format!("__on_init_{}", cb_clone.lock().on_init.len());
            lua.globals().set(name.as_str(), func)?;
            cb_clone.lock().on_init.push(name.clone());
            Ok(name)
        })?;
        engine.set("on_init", on_init_fn)?;

        // engine.on_update(function)
        let cb_clone = callbacks.clone();
        let on_update_fn = lua.create_function(move |lua, func: mlua::Function| {
            let name = format!("__on_update_{}", cb_clone.lock().on_update.len());
            lua.globals().set(name.as_str(), func)?;
            cb_clone.lock().on_update.push(name.clone());
            Ok(name)
        })?;
        engine.set("on_update", on_update_fn)?;

        // engine.on_event(event_name, function)
        let cb_clone = callbacks.clone();
        let on_event_fn =
            lua.create_function(move |lua, (event, func): (String, mlua::Function)| {
                let name = format!("__on_event_{}_{}", event, cb_clone.lock().event_handlers.len());
                lua.globals().set(name.as_str(), func)?;
                cb_clone.lock().register_event(&event, &name);
                Ok(name)
            })?;
        engine.set("on_event", on_event_fn)?;

        // engine.mod_name
        engine.set("mod_name", mod_name)?;

        globals.set("engine", engine)?;

        // 注册 print 函数（重定向到 log）
        let print_fn = lua.create_function(|_, args: MultiValue| {
            let msg: Vec<String> = args
                .into_iter()
                .map(|v| match v {
                    Value::String(s) => s
                        .to_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| "<invalid>".to_string()),
                    Value::Integer(i) => i.to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::Boolean(b) => b.to_string(),
                    Value::Nil => "nil".to_string(),
                    _ => format!("{:?}", v),
                })
                .collect();
            log::info!("[lua] {}", msg.join("\t"));
            Ok(())
        })?;
        globals.set("print", print_fn)?;

        // 限制不安全的函数
        Self::sandbox_globals(lua)?;

        Ok(())
    }

    /// 沙箱化：移除/替换不安全的全局函数
    fn sandbox_globals(lua: &Lua) -> Result<(), LuaError> {
        let globals = lua.globals();

        // 移除 os.execute, os.exit, os.getenv 等
        if let Ok(os) = globals.get::<mlua::Table>("os") {
            let _ = os.set("execute", mlua::Value::Nil);
            let _ = os.set("exit", mlua::Value::Nil);
            let _ = os.set("getenv", mlua::Value::Nil);
            let _ = os.set("remove", mlua::Value::Nil);
            let _ = os.set("rename", mlua::Value::Nil);
            let _ = os.set("tmpname", mlua::Value::Nil);
        }

        // 移除 io 库（文件 IO）
        let _ = globals.set("io", mlua::Value::Nil);

        // 移除 loadfile, dofile
        let _ = globals.set("loadfile", mlua::Value::Nil);
        let _ = globals.set("dofile", mlua::Value::Nil);

        // 移除 package 库（防止 require 加载外部模块）
        let _ = globals.set("package", mlua::Value::Nil);

        Ok(())
    }

    /// 加载并执行 Lua 脚本
    pub fn load_script(&self, source: &str, chunk_name: &str) -> Result<(), LuaError> {
        let chunk = self.lua.load(source).set_name(chunk_name);
        chunk.exec()?;
        Ok(())
    }

    /// 调用 on_init 回调
    pub fn call_on_init(&self) -> Result<(), LuaError> {
        let cbs = self.callbacks.lock();
        for name in &cbs.on_init {
            if let Ok(func) = self.lua.globals().get::<mlua::Function>(name.as_str()) {
                func.call::<()>(())?;
            }
        }
        Ok(())
    }

    /// 调用 on_update 回调
    pub fn call_on_update(&self, dt: f32) -> Result<(), LuaError> {
        let cbs = self.callbacks.lock();
        for name in &cbs.on_update {
            if let Ok(func) = self.lua.globals().get::<mlua::Function>(name.as_str()) {
                func.call::<()>(dt)?;
            }
        }
        Ok(())
    }

    /// 调用 on_unload 回调
    pub fn call_on_unload(&self) -> Result<(), LuaError> {
        let cbs = self.callbacks.lock();
        for name in &cbs.on_unload {
            if let Ok(func) = self.lua.globals().get::<mlua::Function>(name.as_str()) {
                func.call::<()>(())?;
            }
        }
        Ok(())
    }

    /// 触发事件
    pub fn fire_event(&self, event: &str, args: MultiValue) -> Result<(), LuaError> {
        let cbs = self.callbacks.lock();
        let handlers = cbs.get_handlers(event).to_vec();
        drop(cbs);

        for name in handlers {
            if let Ok(func) = self.lua.globals().get::<mlua::Function>(name.as_str()) {
                func.call::<()>(args.clone())?;
            }
        }
        Ok(())
    }

    /// 获取 Mod 名称
    pub fn mod_name(&self) -> &str {
        &self.mod_name
    }

    /// 获取沙箱配置
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// 获取回调注册表
    pub fn callbacks(&self) -> &Arc<Mutex<CallbackRegistry>> {
        &self.callbacks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SandboxConfig {
        SandboxConfig {
            memory_limit_mb: 16,
            fuel_limit: 10_000,
            allowed_dirs: Vec::new(),
            network_allowed: false,
            file_system_allowed: false,
            max_execution_time_ms: 100,
        }
    }

    #[test]
    fn lua_runtime_basic_execution() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        rt.load_script("x = 42", "test").unwrap();
        let x: i64 = rt.lua.globals().get("x").unwrap();
        assert_eq!(x, 42);
    }

    #[test]
    fn lua_runtime_print_redirect() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        // print 不应该 panic
        rt.load_script("print('hello world')", "test").unwrap();
    }

    #[test]
    fn lua_runtime_engine_api() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        rt.load_script("engine.log('info', 'test message')", "test").unwrap();
        let name: String =
            rt.lua.globals().get::<mlua::Table>("engine").unwrap().get("mod_name").unwrap();
        assert_eq!(name, "test_mod");
    }

    #[test]
    fn lua_runtime_sandbox_removes_io() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        // io 应该被移除
        let io: mlua::Value = rt.lua.globals().get("io").unwrap();
        assert_eq!(io, mlua::Value::Nil);
    }

    #[test]
    fn lua_runtime_sandbox_removes_os_execute() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        // os.execute 应该被移除
        let os: mlua::Table = rt.lua.globals().get("os").unwrap();
        let execute: mlua::Value = os.get("execute").unwrap();
        assert_eq!(execute, mlua::Value::Nil);
    }

    #[test]
    fn lua_runtime_sandbox_removes_package() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        let pkg: mlua::Value = rt.lua.globals().get("package").unwrap();
        assert_eq!(pkg, mlua::Value::Nil);
    }

    #[test]
    fn lua_runtime_on_init_callback() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        rt.load_script("engine.on_init(function() _G.init_called = true end)", "test").unwrap();
        rt.call_on_init().unwrap();
        let called: bool = rt.lua.globals().get("init_called").unwrap();
        assert!(called);
    }

    #[test]
    fn lua_runtime_on_update_callback() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        rt.load_script("engine.on_update(function(dt) _G.last_dt = dt end)", "test").unwrap();
        rt.call_on_update(0.016).unwrap();
        let dt: f64 = rt.lua.globals().get("last_dt").unwrap();
        assert!((dt - 0.016).abs() < 1e-6);
    }

    #[test]
    fn lua_runtime_event_handlers() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        rt.load_script(
            "engine.on_event('player_join', function(name) _G.joined = name end)",
            "test",
        )
        .unwrap();
        rt.fire_event(
            "player_join",
            mlua::MultiValue::from_vec(vec![mlua::Value::String(
                rt.lua.create_string("Alice").unwrap(),
            )]),
        )
        .unwrap();
        let joined: String = rt.lua.globals().get("joined").unwrap();
        assert_eq!(joined, "Alice");
    }

    #[test]
    fn lua_runtime_callback_registry() {
        let mut reg = CallbackRegistry::new();
        reg.register_event("test", "handler1");
        reg.register_event("test", "handler2");
        reg.register_event("other", "handler3");
        assert_eq!(reg.get_handlers("test").len(), 2);
        assert_eq!(reg.get_handlers("other").len(), 1);
        assert_eq!(reg.get_handlers("nonexistent").len(), 0);
    }

    #[test]
    fn lua_runtime_error_handling() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        let result = rt.load_script("error('test error')", "test");
        assert!(result.is_err());
    }

    #[test]
    fn lua_runtime_math_available() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        rt.load_script("result = math.floor(3.7)", "test").unwrap();
        let result: i64 = rt.lua.globals().get("result").unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn lua_runtime_string_available() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        rt.load_script("result = string.upper('hello')", "test").unwrap();
        let result: String = rt.lua.globals().get("result").unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn lua_runtime_table_available() {
        let rt = LuaRuntime::new("test_mod", test_config()).unwrap();
        rt.load_script("t = {1, 2, 3}; result = table.concat(t, ',')", "test").unwrap();
        let result: String = rt.lua.globals().get("result").unwrap();
        assert_eq!(result, "1,2,3");
    }
}
