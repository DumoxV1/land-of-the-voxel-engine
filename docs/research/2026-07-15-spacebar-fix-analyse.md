# winit 0.30 + wgpu 0.30: Spacebar crash diagnosis & fix

## Why Spacebar crashes it

Spacebar is not special to wgpu — it is a reliable *trigger*. Pressing any key (or an OS focus/occlusion change) makes the compositor
reconfigure or hide the window surface. Because you drive a **continuous** redraw from `about_to_wait → request_redraw()`,
`get_current_texture()` runs again immediately against a surface that is now `Lost`/`Outdated`/`Occluded`. Your `render_frame` only
matches `Success`/`Suboptimal` and `_ => return`s for the rest — that skip is safe, **but** you never reconfigure or recreate the
surface, and you keep rendering into a stale/destroyed `Surface`. On DXGI (Windows) and Vulkan/Wayland, calling `get_current_texture()`
or `present()` after the surface is gone raises a wgpu validation error that becomes a **panic**, closing the window. A second common
cause: creating the window/surface *outside* `resumed()` (old pre-0.30 style) — winit 0.30 has no live window until `resumed()`, so the
first render panics.

## Correct ControlFlow (ApplicationHandler)

Use `ControlFlow::Wait`; redraw only on demand (input/resize). Create window + surface inside `resumed()`.

```rust
impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        let win = el.create_window(Window::default_attributes()).unwrap();
        self.window = Some(win);
        self.surface = Some(instance.create_surface(self.window.as_ref().unwrap()));
    }
    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        el.set_control_flow(ControlFlow::Wait);
    }
    fn window_event(&mut self, el, _id, ev) {
        match ev {
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    self.keys.insert(event.physical_key);     // physical_key: Option<PhysicalKey>
                    self.window.as_ref().unwrap().request_redraw();
                } else { self.keys.remove(&event.physical_key); }
            }
            WindowEvent::Resized(s) => self.resize(s),        // MUST reconfigure surface
            WindowEvent::RedrawRequested => self.render_frame(),
            WindowEvent::CloseRequested => el.exit(),
            _ => {}
        }
    }
    fn suspended(&mut self, _el) { self.surface = None; }     // drop on suspend
}
```

## Surface-loss handling (wgpu 0.30)

```rust
fn render_frame(&mut self) {
    let surface = match &self.surface { Some(s) => s, None => return };
    match surface.get_current_texture() {
        Success(t) | Suboptimal(t) => { self.scene.render_to_view(&t.view(), ...); self.queue.present(); t.present(); }
        Timeout | Occluded => return,                 // skip this frame
        Outdated => { self.configure_surface(); return; }   // reconfigure, retry next redraw
        Lost    => { self.recreate_surface(); return; }     // recreate surface + configure
        Validation(e) => { log::error!("{e:?}"); return; }
    }
}
```

Never `.unwrap()` `get_current_texture()`. Reconfigure on `Resized`; recreate on `Lost`; always guard rendering behind
`self.surface.is_some()`.
