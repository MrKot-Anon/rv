use super::{Phase, SessionOptions, SessionView};
use crate::actions::*;
use gpui::{
    AppContext, Bounds, Entity, Modifiers, MouseButton, Pixels, TestAppContext, VisualTestContext,
    point, px, size,
};
use rv_core::{ConnectRequest, ScaleMode};
use rv_session::{SessionCommand, SessionEvent, SessionHandle, SessionTestPeer};

fn setup(
    cx: &mut TestAppContext,
    view_only: bool,
) -> (Entity<SessionView>, &mut VisualTestContext, SessionTestPeer) {
    setup_with_scale(cx, view_only, ScaleMode::Fit)
}

fn setup_with_scale(
    cx: &mut TestAppContext,
    view_only: bool,
    scale: ScaleMode,
) -> (Entity<SessionView>, &mut VisualTestContext, SessionTestPeer) {
    setup_with_frame(cx, view_only, scale, 800, 400)
}

fn setup_with_frame(
    cx: &mut TestAppContext,
    view_only: bool,
    scale: ScaleMode,
    fb_w: u16,
    fb_h: u16,
) -> (Entity<SessionView>, &mut VisualTestContext, SessionTestPeer) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::bind_keys(cx);
    });
    let mut connection = rv_core::Connection::new("Offscreen desktop", "example.test", 5900);
    connection.view_only = view_only;
    let request = ConnectRequest::from_connection(&connection, None);
    let (handle, peer) = SessionHandle::test_pair();
    handle.framebuffer.lock().unwrap().resize(fb_w, fb_h);
    peer.events
        .send(SessionEvent::Connected {
            width: fb_w,
            height: fb_h,
        })
        .unwrap();
    peer.events
        .send(SessionEvent::FrameReady { generation: 1 })
        .unwrap();
    let mut entity = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| {
            let mut view = SessionView::with_handle(
                request,
                "Offscreen desktop".into(),
                SessionOptions {
                    scale,
                    pin_toolbar: true,
                    menu_key: "f8".into(),
                    hide_shots: true,
                    thumb_path: None,
                    remember_password: false,
                },
                handle,
                window,
                cx,
            );
            view.pump(cx);
            view
        });
        entity = Some(view.clone());
        gpui_component::Root::new(view, window, cx)
    });
    cx.simulate_resize(size(px(1000.), px(800.)));
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    (entity.unwrap(), cx, peer)
}

#[gpui::test]
fn offscreen_full_hd_shrunk_to_window_receives_mouse_and_wheel(cx: &mut TestAppContext) {
    let (view, cx, mut peer) = setup_with_frame(cx, false, ScaleMode::Fit, 1920, 1080);
    let bounds = picture(&view, cx);
    assert!(bounds.size.width <= px(1000.), "picture bounds: {bounds:?}");
    assert!(bounds.size.height <= px(800.), "picture bounds: {bounds:?}");
    let center = bounds.center();
    cx.simulate_mouse_move(center, None, Modifiers::default());
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: center,
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-40.))),
        modifiers: Modifiers::default(),
        touch_phase: gpui::TouchPhase::Moved,
    });
    let events = commands(&mut peer);
    assert!(events.len() >= 7, "pointer events: {events:?}");
}

fn picture(view: &Entity<SessionView>, cx: &VisualTestContext) -> Bounds<Pixels> {
    view.read_with(cx, |view, _| view.image_box.get())
}

fn commands(peer: &mut SessionTestPeer) -> Vec<SessionCommand> {
    let mut commands = vec![];
    while let Ok(command) = peer.commands.try_recv() {
        commands.push(command);
    }
    commands
}

#[gpui::test]
fn offscreen_pointer_maps_to_framebuffer_and_releases_buttons(cx: &mut TestAppContext) {
    let (view, cx, mut peer) = setup(cx, false);
    let center = picture(&view, cx).center();
    cx.simulate_mouse_move(center, None, Modifiers::default());
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    let pointers: Vec<_> = commands(&mut peer)
        .into_iter()
        .map(|command| {
            let SessionCommand::Input(vnc::X11Event::PointerEvent(event)) = command else {
                panic!("unexpected command: {command:?}");
            };
            (event.position_x, event.position_y, event.bottons)
        })
        .collect();
    assert_eq!(pointers, [(400, 200, 0), (400, 200, 1), (400, 200, 0)]);
    view.read_with(cx, |view, _| {
        assert_eq!(view.map_pointer(center), Some((400, 200)));
        assert_eq!(view.buttons, 0);
    });
}

#[gpui::test]
fn offscreen_stretch_maps_local_cursor_to_remote_absolute_position(cx: &mut TestAppContext) {
    let (view, cx, mut peer) = setup_with_scale(cx, false, ScaleMode::Stretch);
    cx.simulate_resize(size(px(1200.), px(650.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = picture(&view, cx);
    let position = bounds.origin + point(bounds.size.width / 4.0, bounds.size.height / 4.0);
    cx.simulate_mouse_move(position, None, Modifiers::default());
    cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
    let pointer_events: Vec<_> = commands(&mut peer)
        .into_iter()
        .map(|command| {
            let SessionCommand::Input(vnc::X11Event::PointerEvent(event)) = command else {
                panic!("unexpected command: {command:?}");
            };
            (event.position_x, event.position_y, event.bottons)
        })
        .collect();
    assert_eq!(
        pointer_events,
        [(200, 100, 0), (200, 100, 1), (200, 100, 0)]
    );
}

#[gpui::test]
fn offscreen_wheel_sends_remote_scroll(cx: &mut TestAppContext) {
    let (view, cx, mut peer) = setup(cx, false);
    let center = picture(&view, cx).center();
    cx.simulate_mouse_move(center, None, Modifiers::default());
    commands(&mut peer);
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: center,
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-40.))),
        modifiers: Modifiers::default(),
        touch_phase: gpui::TouchPhase::Moved,
    });
    let masks: Vec<_> = commands(&mut peer)
        .into_iter()
        .map(|command| {
            let SessionCommand::Input(vnc::X11Event::PointerEvent(event)) = command else {
                panic!("unexpected command: {command:?}");
            };
            event.bottons
        })
        .collect();
    assert_eq!(masks, [16, 0, 16, 0]);
}

#[gpui::test]
fn offscreen_missing_password_shows_prompt(cx: &mut TestAppContext) {
    let (view, cx, peer) = setup(cx, false);
    peer.events.send(SessionEvent::PasswordRequired).unwrap();
    view.update(cx, |view, cx| view.pump(cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    view.read_with(cx, |view, _| assert!(view.password_prompt));
    assert!(cx.debug_bounds("password-connect").is_some());
}

#[gpui::test]
fn offscreen_letterbox_does_not_send_outside_picture(cx: &mut TestAppContext) {
    let (view, cx, mut peer) = setup(cx, false);
    let bounds = picture(&view, cx);
    cx.simulate_mouse_move(bounds.center(), None, Modifiers::default());
    commands(&mut peer);
    cx.simulate_mouse_move(
        bounds.origin + point(px(2.), px(2.)),
        None,
        Modifiers::default(),
    );
    assert!(commands(&mut peer).is_empty());
    cx.simulate_mouse_move(bounds.center(), None, Modifiers::default());
    cx.simulate_keystrokes("f8");
    view.read_with(cx, |view, _| assert!(view.show_menu));
}

#[gpui::test]
fn offscreen_view_only_blocks_remote_input(cx: &mut TestAppContext) {
    let (view, cx, mut peer) = setup(cx, true);
    let center = picture(&view, cx).center();
    cx.simulate_mouse_move(center, None, Modifiers::default());
    cx.simulate_click(center, Modifiers::default());
    cx.simulate_keystrokes("a");
    assert!(commands(&mut peer).is_empty());
}

#[gpui::test]
fn offscreen_disconnect_stops_pointer_input(cx: &mut TestAppContext) {
    let (view, cx, mut peer) = setup(cx, false);
    let center = picture(&view, cx).center();
    cx.simulate_mouse_move(center, None, Modifiers::default());
    commands(&mut peer);
    peer.events.send(SessionEvent::Disconnected).unwrap();
    view.update(cx, |view, cx| view.pump(cx));
    cx.run_until_parked();
    cx.simulate_mouse_move(center, None, Modifiers::default());
    assert!(commands(&mut peer).is_empty());
    view.read_with(cx, |view, _| assert_eq!(view.phase, Phase::Ended));
}

#[gpui::test]
fn offscreen_close_closes_session(cx: &mut TestAppContext) {
    let (view, cx, mut peer) = setup(cx, false);
    cx.simulate_mouse_move(picture(&view, cx).center(), None, Modifiers::default());
    commands(&mut peer);
    cx.dispatch_action(SessionClose);
    assert!(matches!(
        peer.commands.try_recv(),
        Ok(SessionCommand::Close)
    ));
}

#[gpui::test]
fn offscreen_keyboard_forwards_pairs_and_consumes_menu_shortcuts(cx: &mut TestAppContext) {
    let (view, cx, mut peer) = setup(cx, false);
    for key in ["a", "f8", "escape"] {
        cx.simulate_keystrokes(key);
        // GPUI's typing helper sends key-down only; release explicitly to
        // verify the remote cannot be left with a stuck key.
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse(key).unwrap(),
        });
    }
    let keys: Vec<_> = commands(&mut peer)
        .into_iter()
        .map(|command| {
            let SessionCommand::Input(vnc::X11Event::KeyEvent(event)) = command else {
                panic!("unexpected command: {command:?}");
            };
            (event.keycode, event.down)
        })
        .collect();
    assert_eq!(keys, [(u32::from(b'a'), true), (u32::from(b'a'), false)]);
    view.read_with(cx, |view, _| assert!(!view.show_menu));
}
