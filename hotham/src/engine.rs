use crate::{
    resources::{
        AudioContext, GuiContext, HapticContext, PhysicsContext, RenderContext, VulkanContext,
        XrContext,
    },
    HothamError, HothamResult, VIEW_TYPE,
};
use openxr as xr;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

use xr::{ActiveActionSet, EventDataBuffer, SessionState};

#[cfg(target_os = "android")]
pub static ANDROID_LOOPER_ID_MAIN: u32 = 0;
#[cfg(target_os = "android")]
pub static ANDROID_LOOPER_NONBLOCKING_TIMEOUT: Duration = Duration::from_millis(0);
#[cfg(target_os = "android")]
pub static ANDROID_LOOPER_BLOCKING_TIMEOUT: Duration = Duration::from_millis(i32::MAX as _);

/// The Hotham Engine
/// A wrapper around the "external world" from the perspective of the engine, eg. renderer, XR, etc.
/// **IMPORTANT**: make sure you call `update` each tick
pub struct Engine {
    should_quit: Arc<AtomicBool>,
    #[allow(dead_code)]
    resumed: bool,
    event_data_buffer: EventDataBuffer,
    /// OpenXR context
    pub xr_context: XrContext,
    /// Vulkan context
    pub vulkan_context: VulkanContext,
    /// Renderer context
    pub render_context: RenderContext,
    /// Physics context
    pub physics_context: PhysicsContext,
    /// Audio context
    pub audio_context: AudioContext,
    /// GUI context
    pub gui_context: GuiContext,
    /// Haptics context
    pub haptic_context: HapticContext,
}

impl Engine {
    /// Create a new instance of the engine
    /// NOTE: only one instance may be running at any one time
    pub fn new() -> Self {
        #[allow(unused_mut)] // Only Android mutates this.
        let mut resumed = false;
        let should_quit = Arc::new(AtomicBool::from(false));

        // Before we do ANYTHING - we should process android events
        #[cfg(target_os = "android")]
        process_android_events(&mut resumed, &should_quit);

        // On desktop, register a Ctrl-C handler.
        #[cfg(not(target_os = "android"))]
        {
            let should_quit = should_quit.clone();
            ctrlc::set_handler(move || should_quit.store(true, Ordering::Relaxed)).unwrap();
        }

        // Now initialise the engine.
        let (xr_context, vulkan_context) =
            XrContext::new().expect("!!FATAL ERROR - Unable to initialise OpenXR!!");
        let render_context = RenderContext::new(&vulkan_context, &xr_context)
            .expect("!!FATAL ERROR - Unable to initialise renderer!");
        let gui_context = GuiContext::new(&vulkan_context);

        let mut engine = Self {
            should_quit,
            resumed,
            event_data_buffer: Default::default(),
            xr_context,
            vulkan_context,
            render_context,
            physics_context: Default::default(),
            audio_context: Default::default(),
            gui_context,
            haptic_context: Default::default(),
        };

        engine.update().unwrap();
        engine
    }

    /// IMPORTANT: Call this function each tick to update the engine's running state with the underlying OS
    pub fn update(&mut self) -> HothamResult<(xr::SessionState, xr::SessionState)> {
        #[cfg(target_os = "android")]
        process_android_events(&mut self.resumed, &self.should_quit);

        let (previous_state, current_state) = {
            let previous_state = self.xr_context.session_state.clone();
            let current_state = self.xr_context.poll_xr_event(&mut self.event_data_buffer)?;
            (previous_state, current_state)
        };

        match (previous_state, current_state) {
            (SessionState::STOPPING, SessionState::IDLE) => {
                // Do nothing so we can process further events.
            }
            (_, SessionState::IDLE) => {
                sleep(Duration::from_millis(100)); // Sleep to avoid thrasing the CPU
            }
            (SessionState::IDLE, SessionState::READY) => {
                self.xr_context.session.begin(VIEW_TYPE)?;
            }
            (_, SessionState::EXITING) => {
                // Show's over
                println!("[HOTHAM_ENGINE] State is now exiting!");
                return Err(HothamError::ShuttingDown);
            }
            (_, SessionState::STOPPING) => {
                self.xr_context.end_session()?;
            }
            _ => {}
        }

        if self.should_quit.load(Ordering::Relaxed) {
            return Err(HothamError::ShuttingDown);
        }

        Ok((previous_state, current_state))
    }

    /// Begin a frame
    /// Make sure to call this BEFORE beginning any renderpasses.
    pub fn begin_frame(&mut self) {
        let active_action_set = ActiveActionSet::new(&self.xr_context.action_set);
        self.xr_context
            .session
            .sync_actions(&[active_action_set])
            .unwrap();

        // Wait for a frame to become available from the runtime
        self.xr_context.begin_frame().unwrap();

        let (view_state_flags, views) = self
            .xr_context
            .session
            .locate_views(
                VIEW_TYPE,
                self.xr_context.frame_state.predicted_display_time,
                &self.xr_context.reference_space,
            )
            .unwrap();
        self.xr_context.views = views;
        self.xr_context.view_state_flags = view_state_flags;

        // If the shouldRender flag is set, start rendering
        if self.xr_context.frame_state.should_render {
            self.render_context
                .begin_frame(&self.vulkan_context, self.xr_context.frame_index);
        } else {
            println!(
                "[HOTHAM_BEGIN_FRAME] - Session is runing but shouldRender is false - not rendering"
            );
        }
    }

    /// End the current frame
    /// Make sure to ONLY call this AFTER `begin_frame` and DO NOT issue any further rendering commands this frame
    pub fn end_frame(&mut self) {
        // Check if we should be rendering.
        if self.xr_context.frame_state.should_render {
            self.render_context
                .end_frame(&self.vulkan_context, self.xr_context.frame_index);
        } else {
            println!(
                "[HOTHAM_END_FRAME] - Session is runing but shouldRender is false - not rendering"
            );
        }
        self.xr_context.end_frame().unwrap();
    }
}

#[cfg(target_os = "android")]
pub fn process_android_events(resumed: &mut bool, should_quit: &Arc<AtomicBool>) -> bool {
    while let Some(event) = poll_android_events(*resumed) {
        println!("[HOTHAM_ANDROID] Received event {:?}", event);
        match event {
            ndk_glue::Event::Resume => *resumed = true,
            ndk_glue::Event::Destroy => {
                should_quit.store(true, Ordering::Relaxed);
                return true;
            }
            ndk_glue::Event::Pause => *resumed = false,
            _ => {}
        }
    }

    false
}

#[cfg(target_os = "android")]
pub fn poll_android_events(resumed: bool) -> Option<ndk_glue::Event> {
    use ndk::looper::{Poll, ThreadLooper};

    let looper = ThreadLooper::for_thread().unwrap();
    let timeout = if resumed {
        ANDROID_LOOPER_NONBLOCKING_TIMEOUT
    } else {
        ANDROID_LOOPER_BLOCKING_TIMEOUT
    };
    let result = looper.poll_all_timeout(timeout);

    match result {
        Ok(Poll::Event { ident, .. }) => {
            let ident = ident as u32;
            if ident == ANDROID_LOOPER_ID_MAIN {
                ndk_glue::poll_events()
            } else {
                None
            }
        }
        _ => None,
    }
}

/*#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use crate::resources::{RenderContext, XrContext};

    use super::begin_frame;

    #[test]

    pub fn test_begin_frame() {
        let (mut xr_context, vulkan_context) = XrContext::new().unwrap();
        let render_context = RenderContext::new(&vulkan_context, &xr_context).unwrap();
        xr_context.frame_index = 100;

        begin_frame(&mut xr_context, &vulkan_context, &render_context);
        assert_eq!(xr_context.frame_index, 0);
    }

    #[test]
    pub fn test_end_frame() {
        let (mut xr_context, vulkan_context) = XrContext::new().unwrap();
        let mut render_context = RenderContext::new(&vulkan_context, &xr_context).unwrap();
        begin_frame(&mut xr_context, &vulkan_context, &render_context);
        end_frame(&mut xr_context, &vulkan_context, &mut render_context);
    }
}*/
