//! Event manager - registration, dispatch, and hooks
//!
//! Hooks IGameEventManager2::FireEvent to intercept game events.

use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::LazyLock;

use parking_lot::RwLock;

use cs2rust_sdk::{IGameEvent, IGameEventManager2};

use super::raw::GameEventRef;
use super::types::{EventCallback, EventInfo, HookResult};
use crate::hooks::{HookError, VTableHookKey};

/// VTable indices for IGameEventManager2 (Linux)
mod vtable {
    pub const LOAD_EVENTS_FROM_FILE: usize = 1;
    pub const RESET: usize = 2;
    pub const ADD_LISTENER: usize = 3;
    pub const FIND_LISTENER: usize = 4;
    pub const REMOVE_LISTENER: usize = 5;
    pub const CREATE_EVENT: usize = 6;
    pub const FIRE_EVENT: usize = 7;
    pub const FIRE_EVENT_CLIENT_SIDE: usize = 8;
    pub const DUPLICATE_EVENT: usize = 9;
    pub const FREE_EVENT: usize = 10;
}

/// Global game event manager pointer (set when LoadEventsFromFile is called)
static GAME_EVENT_MANAGER: AtomicPtr<IGameEventManager2> = AtomicPtr::new(std::ptr::null_mut());

/// Hook keys for cleanup
static HOOK_KEYS: LazyLock<RwLock<EventHookKeys>> =
    LazyLock::new(|| RwLock::new(EventHookKeys::default()));

#[derive(Default)]
struct EventHookKeys {
    load_events_hook: Option<VTableHookKey>,
    fire_event_hook: Option<VTableHookKey>,
}

/// Function pointer types for IGameEventManager2 methods
type LoadEventsFromFileFn = extern "C" fn(*mut IGameEventManager2, *const c_char, bool) -> i32;
type FireEventFn = extern "C" fn(*mut IGameEventManager2, *mut IGameEvent, bool) -> bool;
type DuplicateEventFn = extern "C" fn(*mut IGameEventManager2, *mut IGameEvent) -> *mut IGameEvent;
type FreeEventFn = extern "C" fn(*mut IGameEventManager2, *mut IGameEvent);

/// Original function pointers
static ORIGINAL_FIRE_EVENT: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

/// Storage for an event hook
struct EventHook {
    name: String,
    pre_hooks: Vec<EventCallback>,
    post_hooks: Vec<EventCallback>,
}

/// Global event manager
pub static EVENTS: LazyLock<RwLock<EventManager>> =
    LazyLock::new(|| RwLock::new(EventManager::new()));

/// Event manager for registering and dispatching event handlers
pub struct EventManager {
    /// Map of event name to hook data
    hooks: HashMap<String, EventHook>,

    /// Stack of event names for tracking nested events
    event_stack: Vec<Option<String>>,

    /// Stack of duplicated events for post-hooks
    event_copies: Vec<*mut IGameEvent>,
}

// SAFETY: Accessed only from game thread
unsafe impl Send for EventManager {}
unsafe impl Sync for EventManager {}

impl EventManager {
    fn new() -> Self {
        Self {
            hooks: HashMap::new(),
            event_stack: Vec::new(),
            event_copies: Vec::new(),
        }
    }

    /// Get the game event manager pointer
    pub fn game_event_manager() -> Option<NonNull<IGameEventManager2>> {
        NonNull::new(GAME_EVENT_MANAGER.load(Ordering::Acquire))
    }

    /// Duplicate an event for post-hook processing
    fn duplicate_event(&self, event: *mut IGameEvent) -> *mut IGameEvent {
        let manager = match Self::game_event_manager() {
            Some(m) => m.as_ptr(),
            None => return std::ptr::null_mut(),
        };

        unsafe {
            let vtable = *(manager as *const *const *const c_void);
            let duplicate_fn: DuplicateEventFn =
                std::mem::transmute(*vtable.add(vtable::DUPLICATE_EVENT));
            duplicate_fn(manager, event)
        }
    }

    /// Free a duplicated event
    fn free_event(&self, event: *mut IGameEvent) {
        if event.is_null() {
            return;
        }

        let manager = match Self::game_event_manager() {
            Some(m) => m.as_ptr(),
            None => return,
        };

        unsafe {
            let vtable = *(manager as *const *const *const c_void);
            let free_fn: FreeEventFn = std::mem::transmute(*vtable.add(vtable::FREE_EVENT));
            free_fn(manager, event);
        }
    }

    /// Handle pre-fire event
    ///
    /// Returns (should_continue, modified_dont_broadcast)
    fn on_fire_event(&mut self, event: *mut IGameEvent, dont_broadcast: bool) -> (bool, bool) {
        let event_ref = match unsafe { GameEventRef::from_ptr(event) } {
            Some(e) => e,
            None => {
                self.event_stack.push(None);
                return (true, dont_broadcast);
            }
        };

        let name = event_ref.get_name().to_string();
        let mut local_dont_broadcast = dont_broadcast;

        if let Some(hook) = self.hooks.get(&name) {
            self.event_stack.push(Some(name.clone()));

            // Run pre-hooks
            for callback in &hook.pre_hooks {
                let mut info = EventInfo::new(local_dont_broadcast);
                let result = callback(&event_ref, &mut info);
                local_dont_broadcast = info.dont_broadcast;

                if result >= HookResult::Handled {
                    // Block the event, but duplicate for post-hooks
                    self.event_copies.push(self.duplicate_event(event));
                    self.free_event(event);
                    return (false, local_dont_broadcast);
                }
            }

            // Duplicate for post-hook access
            self.event_copies.push(self.duplicate_event(event));
        } else {
            self.event_stack.push(None);
        }

        (true, local_dont_broadcast)
    }

    /// Handle post-fire event
    fn on_fire_event_post(&mut self, _event: *mut IGameEvent, dont_broadcast: bool) {
        let hook_name = self.event_stack.pop();

        if let Some(Some(name)) = hook_name {
            if let Some(hook) = self.hooks.get(&name) {
                if !hook.post_hooks.is_empty() {
                    if let Some(event_copy) = self.event_copies.pop() {
                        if let Some(event_ref) = unsafe { GameEventRef::from_ptr(event_copy) } {
                            let mut info = EventInfo::new(dont_broadcast);
                            for callback in &hook.post_hooks {
                                callback(&event_ref, &mut info);
                            }
                        }
                        self.free_event(event_copy);
                    }
                } else {
                    // No post hooks, just free the copy
                    if let Some(event_copy) = self.event_copies.pop() {
                        self.free_event(event_copy);
                    }
                }
            }
        }
    }
}

/// Register an event handler
///
/// # Arguments
/// * `name` - Event name (e.g., "player_death", "round_start")
/// * `post` - If true, handler runs after event fires; otherwise before
/// * `callback` - Function to call when event fires
pub fn register_event<F>(name: &str, post: bool, callback: F)
where
    F: Fn(&GameEventRef, &mut EventInfo) -> HookResult + Send + Sync + 'static,
{
    let mut manager = EVENTS.write();

    let hook = manager.hooks.entry(name.to_string()).or_insert_with(|| {
        tracing::debug!("Registering new event hook: {}", name);
        EventHook {
            name: name.to_string(),
            pre_hooks: Vec::new(),
            post_hooks: Vec::new(),
        }
    });

    if post {
        hook.post_hooks.push(Box::new(callback));
    } else {
        hook.pre_hooks.push(Box::new(callback));
    }

    tracing::trace!(
        "Added {} handler for event '{}' (total: {} pre, {} post)",
        if post { "post" } else { "pre" },
        name,
        hook.pre_hooks.len(),
        hook.post_hooks.len()
    );
}

/// Unregister all handlers for an event
///
/// # Arguments
/// * `name` - Event name to unregister
///
/// # Returns
/// true if the event was found and removed
pub fn unregister_event(name: &str) -> bool {
    let mut manager = EVENTS.write();
    let removed = manager.hooks.remove(name).is_some();
    if removed {
        tracing::debug!("Unregistered all handlers for event: {}", name);
    }
    removed
}

/// Our FireEvent detour
extern "C" fn fire_event_detour(
    this: *mut IGameEventManager2,
    event: *mut IGameEvent,
    dont_broadcast: bool,
) -> bool {
    // Get original function
    let original_ptr = ORIGINAL_FIRE_EVENT.load(Ordering::Acquire);
    if original_ptr.is_null() {
        tracing::error!("FireEvent original is null!");
        return false;
    }
    let original: FireEventFn = unsafe { std::mem::transmute(original_ptr) };

    // Pre-hook processing
    let (should_continue, new_dont_broadcast) = {
        let mut manager = EVENTS.write();
        manager.on_fire_event(event, dont_broadcast)
    };

    if !should_continue {
        // Event was blocked
        return false;
    }

    // Call original
    let result = original(this, event, new_dont_broadcast);

    // Post-hook processing
    {
        let mut manager = EVENTS.write();
        manager.on_fire_event_post(event, new_dont_broadcast);
    }

    result
}

/// Initialize event hooks
///
/// This needs to be called after the game event manager is available.
/// We hook LoadEventsFromFile to capture the IGameEventManager2 pointer,
/// then hook FireEvent for event interception.
pub fn init_event_hooks() -> Result<(), HookError> {
    // For now, we'll set up the infrastructure but the actual hooking
    // will happen when we detect the game event manager.
    // This is typically done via a LoadEventsFromFile hook or by
    // finding the CGameEventManager vtable in memory.

    tracing::info!("Event system initialized (waiting for game event manager)");
    Ok(())
}

/// Hook FireEvent on the game event manager
///
/// Called once we have the IGameEventManager2 pointer.
pub fn hook_fire_event(manager: *mut IGameEventManager2) -> Result<(), HookError> {
    if manager.is_null() {
        return Err(HookError::InvalidAddress(0));
    }

    // Store the manager pointer
    GAME_EVENT_MANAGER.store(manager, Ordering::Release);

    unsafe {
        // Get vtable
        let vtable = *(manager as *const *mut *const ());

        // Hook FireEvent
        let (key, original) = crate::hooks::vtable::create_vtable_hook_direct(
            "IGameEventManager2::FireEvent",
            vtable,
            vtable::FIRE_EVENT,
            fire_event_detour as *const (),
        )?;

        ORIGINAL_FIRE_EVENT.store(original as *mut c_void, Ordering::Release);
        HOOK_KEYS.write().fire_event_hook = Some(key);

        tracing::info!("Hooked IGameEventManager2::FireEvent at {:p}", original);
    }

    Ok(())
}

/// Set the game event manager pointer (called from external hook)
pub fn set_game_event_manager(manager: *mut IGameEventManager2) {
    let old = GAME_EVENT_MANAGER.swap(manager, Ordering::AcqRel);
    if old.is_null() && !manager.is_null() {
        tracing::info!("IGameEventManager2 acquired: {:p}", manager);

        // Try to hook FireEvent
        if let Err(e) = hook_fire_event(manager) {
            tracing::error!("Failed to hook FireEvent: {:?}", e);
        }
    }
}

/// Shutdown event hooks
pub fn shutdown_event_hooks() {
    let mut keys = HOOK_KEYS.write();

    if let Some(key) = keys.fire_event_hook.take() {
        if let Err(e) = crate::hooks::vtable::remove_vtable_hook(key) {
            tracing::warn!("Failed to remove FireEvent hook: {:?}", e);
        }
    }

    if let Some(key) = keys.load_events_hook.take() {
        if let Err(e) = crate::hooks::vtable::remove_vtable_hook(key) {
            tracing::warn!("Failed to remove LoadEventsFromFile hook: {:?}", e);
        }
    }

    GAME_EVENT_MANAGER.store(std::ptr::null_mut(), Ordering::Release);
    ORIGINAL_FIRE_EVENT.store(std::ptr::null_mut(), Ordering::Release);

    tracing::info!("Event system shutdown complete");
}
