use super::{Auth, Operation, Pending, Screen, client};
use client::{AuthSession, Outcome};

fn designflow_app() -> crate::FigApp {
    let mut app = crate::FigApp::new(designflow_engine::Session::empty(), Box::new(crate::NoServices));
    app.session.prefs.language = "pt-BR".into();
    app.session.prefs.ui_scale = 1.25;
    let mut auth = locked();
    auth.complete(Operation::SignIn, Ok(Outcome::Session(session(client::unix_seconds() + 3600))));
    app.auth = Some(auth);
    app.ui.home = true;
    app.ui.right_width = 280.0;
    app
}

fn designflow_frame(ctx: &egui::Context, app: &mut crate::FigApp, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut frame = designflow_frame_with_textures(ctx, app, events);
    frame.textures_delta.clear();
    frame
}

fn designflow_frame_with_textures(ctx: &egui::Context, app: &mut crate::FigApp, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0) / ctx.pixels_per_point())),
            events,
            ..Default::default()
        },
        |ui| app.ui(ui),
    )
}

#[test]
fn designflow_home_creates_an_editable_post_and_editor_keeps_real_actions() {
    let ctx = egui::Context::default();
    let mut app = designflow_app();
    for _ in 0..4 {
        designflow_frame(&ctx, &mut app, vec![]);
    }
    let output = designflow_frame(&ctx, &mut app, vec![]);
    let shapes = crate::live_tests::flat(&output.shapes);
    assert!(!crate::live_tests::has_text(&shapes, "Criar com IA"));
    let point = crate::live_tests::text_center(&shapes, "Post");
    for pressed in [true, false] {
        designflow_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton { pos: point, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
            ],
        );
    }
    assert!(!app.ui.home);
    assert_eq!(app.session.docs.len(), 1);
    let doc = app.session.doc().unwrap();
    let frame = doc.nodes().find(|node| node.name == "Post").unwrap();
    assert_eq!(frame.width, 1080.0);
    assert_eq!(frame.height, 1350.0);
    let output = designflow_frame(&ctx, &mut app, vec![]);
    let shapes = crate::live_tests::flat(&output.shapes);
    for label in ["Arquivo", "Editar", "Propriedades", "Estilos", "Exportar"] {
        assert!(crate::live_tests::has_text(&shapes, label));
    }
    let uid = app.session.ds().unwrap().uid;
    app.ui.home = true;
    for _ in 0..3 {
        designflow_frame(&ctx, &mut app, vec![]);
    }
    let frame = designflow_frame(&ctx, &mut app, vec![]);
    let shapes = crate::live_tests::flat(&frame.shapes);
    let point = shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(text) if text.galley.job.text.starts_with("Post") => Some(text.pos + text.galley.size() * 0.5),
            _ => None,
        })
        .find(|point| point.y < crate::tabs::HEIGHT)
        .expect("open files remain accessible from Home");
    for pressed in [true, false] {
        designflow_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton { pos: point, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
            ],
        );
    }
    assert!(!app.ui.home);
    assert_eq!(app.session.ds().unwrap().uid, uid);
    assert_eq!(app.session.docs.len(), 1);
}

#[test]
fn designflow_first_login_opens_home_but_refresh_keeps_the_editor() {
    let ctx = egui::Context::default();
    let mut app = crate::FigApp::new(designflow_engine::Session::new(), Box::new(crate::NoServices));
    app.auth = Some(locked());
    for operation in [Operation::SignIn, Operation::Refresh] {
        app.ui.home = false;
        let (sender, receiver) = std::sync::mpsc::channel();
        sender.send(Ok(Outcome::Session(session(client::unix_seconds() + 3600)))).unwrap();
        app.auth.as_mut().unwrap().pending = Some(Pending { operation, started: web_time::Instant::now(), receiver });
        app.background_tick(&ctx);
        assert_eq!(app.ui.home, operation == Operation::SignIn);
        assert!(app.auth.as_ref().unwrap().unlocked());
    }
}

#[test]
fn designflow_editor_menu_is_anchored_and_can_undo_and_redo() {
    let ctx = egui::Context::default();
    let mut app = designflow_app();
    app.run("file.new", serde_json::json!({}));
    app.ui.home = false;
    assert!(!crate::menus::command_enabled(&app, "edit.undo"));
    app.run("shape.rectangle", serde_json::json!({"x":0,"y":0,"width":100,"height":100}));
    assert!(crate::menus::command_enabled(&app, "edit.undo"));
    for _ in 0..4 {
        designflow_frame(&ctx, &mut app, vec![]);
    }
    for (command, label) in [("edit.undo", "Undo"), ("edit.redo", "Redo")] {
        let frame = designflow_frame(&ctx, &mut app, vec![]);
        let shapes = crate::live_tests::flat(&frame.shapes);
        let menu = crate::live_tests::text_center(&shapes, "Editar");
        for pressed in [true, false] {
            designflow_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(menu),
                    egui::Event::PointerButton { pos: menu, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
                ],
            );
        }
        for _ in 0..3 {
            designflow_frame(&ctx, &mut app, vec![]);
        }
        assert!(app.ui.main_menu);
        let frame = designflow_frame(&ctx, &mut app, vec![]);
        let shapes = crate::live_tests::flat(&frame.shapes);
        let name = crate::i18n::menu_item(command, label);
        let item = crate::live_tests::text_center(&shapes, &name);
        assert!(item.x >= menu.x - 40.0, "the dropdown stays below its own menu button");
        for pressed in [true, false] {
            designflow_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(item),
                    egui::Event::PointerButton { pos: item, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
                ],
            );
        }
        for _ in 0..2 {
            designflow_frame(&ctx, &mut app, vec![]);
        }
        let rectangles = app.session.doc().unwrap().nodes().filter(|node| matches!(node.kind, designflow_doc::NodeKind::Rectangle)).count();
        assert_eq!(rectangles, usize::from(command == "edit.redo"));
        assert!(!app.ui.main_menu);
    }
}

#[test]
#[ignore = "Writes visual QA artifacts only when explicitly requested"]
fn designflow_export_visuals() {
    use std::collections::HashMap;
    designflow_text::scan_system_fonts();
    let directory = std::path::PathBuf::from(std::env::var("DESIGNFLOW_CAPTURE_DIR").expect("capture directory"));
    std::fs::create_dir_all(&directory).unwrap();
    for name in ["home", "editor"] {
        let ctx = egui::Context::default();
        let mut app = designflow_app();
        if name == "editor" {
            app.run("file.new", serde_json::json!({"name":"Identidade visual"}));
            app.ui.right_width = 280.0;
            app.run("page.setBackground", serde_json::json!({"color":"#F8F7FC"})).expect("capture background");
            let frame = app
                .run("frame.create", serde_json::json!({"x":320,"y":360,"width":1360,"height":760,"name":"Frame – Hero","props":{"fills":[]}}))
                .expect("capture frame")["id"]
                .as_u64()
                .unwrap();
            app.run("text.create",serde_json::json!({"parent":frame,"x":100,"y":76,"text":"Título da seção","props":{"fontSize":54,"fontStyle":"SemiBold","fills":["#171428"]}})).expect("capture heading");
            app.run("text.create",serde_json::json!({"parent":frame,"x":100,"y":164,"text":"Uma breve descrição que explica\no conteúdo desta seção.","props":{"fontSize":32,"fills":["#636171"]}})).expect("capture description");
            for index in 0..3 {
                let card=app.run("node.create",serde_json::json!({"type":"frame","parent":frame,"x":100+index*410,"y":304,"width":365,"height":342,"name":format!("Card {}",index+1),"props":{"fills":["#FDFCFF"],"cornerRadius":28,"strokes":["#E4E0EE"],"strokeWeight":2}})).expect("capture card")["id"].as_u64().unwrap();
                app.run(
                    "shape.rectangle",
                    serde_json::json!({"parent":card,"x":48,"y":42,"width":96,"height":96,"props":{"fills":["#EEE3FF"],"cornerRadius":24}}),
                )
                .expect("capture tile");
                app.run("text.create",serde_json::json!({"parent":card,"x":48,"y":172,"text":format!("Card {}",index+1),"props":{"fontSize":32,"fontStyle":"SemiBold","fills":["#171428"]}})).expect("capture card label");
                for (row, width) in [(0, 258), (1, 190)] {
                    app.run("shape.rectangle",serde_json::json!({"parent":card,"x":48,"y":240+row*34,"width":width,"height":16,"props":{"fills":["#DBD9E4"],"cornerRadius":8}})).expect("capture skeleton");
                }
            }
            app.run("select.set", serde_json::json!({"ids":[frame]})).expect("capture selection");
            app.ui.expanded.insert(designflow_doc::NodeId(frame));
            app.run("view.rulers", serde_json::json!({"on":true})).expect("capture rulers");
            app.run("view.zoomFit", serde_json::json!({}));
            app.run("view.zoom50", serde_json::json!({}));
            app.ui.home = false;
        }
        let mut images: HashMap<egui::TextureId, egui::ColorImage> = HashMap::new();
        let mut output = None;
        for capture_frame in 0..20 {
            if name == "editor" && capture_frame == 8 {
                app.run("view.set", serde_json::json!({"zoom":0.5,"cx":1000,"cy":740})).expect("settled capture camera");
            }
            let mut frame = designflow_frame_with_textures(&ctx, &mut app, vec![]);
            for (id, deltas) in &frame.textures_delta.set {
                for delta in deltas {
                    let egui::ImageData::Color(image) = &delta.image;
                    if let Some([x, y]) = delta.pos {
                        let target = images.get_mut(id).unwrap();
                        for row in 0..image.size[1] {
                            let start = (y + row) * target.size[0] + x;
                            target.pixels[start..start + image.size[0]]
                                .copy_from_slice(&image.pixels[row * image.size[0]..(row + 1) * image.size[0]]);
                        }
                    } else {
                        images.insert(*id, image.as_ref().clone());
                    }
                }
            }
            frame.textures_delta.clear();
            output = Some(frame);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let output = output.unwrap();
        let density = ctx.pixels_per_point();
        let primitives:Vec<_>=ctx.tessellate(output.shapes,density).into_iter().filter_map(|primitive|{
            let egui::epaint::Primitive::Mesh(mesh)=primitive.primitive else {return None};
            let vertices:Vec<_>=mesh.vertices.iter().map(|v|serde_json::json!([v.pos.x*density,v.pos.y*density,v.uv.x,v.uv.y,v.color.r(),v.color.g(),v.color.b(),v.color.a()])).collect();
            Some(serde_json::json!({"clip":[primitive.clip_rect.min.x*density,primitive.clip_rect.min.y*density,primitive.clip_rect.max.x*density,primitive.clip_rect.max.y*density],"texture":format!("{:?}",mesh.texture_id),"vertices":vertices,"indices":mesh.indices}))
        }).collect();
        let mut textures = serde_json::Map::new();
        for (index, (id, image)) in images.iter().enumerate() {
            let filename = format!("{name}-texture-{index}.png");
            let rgba: Vec<u8> = image.pixels.iter().flat_map(|color| color.to_array()).collect();
            image::save_buffer(directory.join(&filename), &rgba, image.size[0] as u32, image.size[1] as u32, image::ColorType::Rgba8).unwrap();
            textures.insert(format!("{id:?}"), filename.into());
        }
        std::fs::write(
            directory.join(format!("{name}.json")),
            serde_json::to_vec(&serde_json::json!({"width":1920,"height":1080,"primitives":primitives,"textures":textures})).unwrap(),
        )
        .unwrap();
    }
}

fn locked() -> Auth {
    Auth::new(Err("Serviço não configurado".into()))
}

fn session(expires_at: u64) -> AuthSession {
    AuthSession {
        access_token: "synthetic-access-token".into(),
        refresh_token: "synthetic-refresh-token".into(),
        user_id: "synthetic-user".into(),
        email: "synthetic@example.invalid".into(),
        expires_at,
    }
}

#[test]
fn missing_configuration_or_rejected_credentials_never_unlocks_the_editor() {
    let mut auth = locked();
    assert!(!auth.unlocked());
    auth.complete(Operation::SignIn, Err("Dados incorretos".into()));
    assert!(!auth.unlocked());
    assert!(auth.message.as_ref().is_some_and(|(error, _)| *error));
}

#[test]
fn recovery_verification_cannot_open_the_editor_and_password_update_returns_to_sign_in() {
    let mut auth = locked();
    auth.complete(Operation::VerifyRecovery, Ok(Outcome::Session(session(client::unix_seconds() + 3600))));
    assert!(auth.screen == Screen::NewPassword);
    assert!(auth.recovery.is_some());
    assert!(!auth.unlocked());
    auth.password = "synthetic-password".into();
    auth.complete(Operation::UpdatePassword, Ok(Outcome::Message("Senha atualizada".into())));
    assert!(auth.screen == Screen::SignIn);
    assert!(auth.password.is_empty());
    assert!(auth.recovery.is_none());
    assert!(!auth.unlocked());
}

#[test]
fn confirmed_sign_in_clears_fields_and_refresh_failure_relocks_at_sign_in() {
    let mut auth = locked();
    auth.screen = Screen::VerifySignup;
    auth.password = "synthetic-password".into();
    auth.confirmation = "synthetic-password".into();
    auth.code = "123456".into();
    auth.complete(Operation::VerifySignup, Ok(Outcome::Session(session(client::unix_seconds() + 3600))));
    assert!(auth.unlocked());
    assert!(auth.password.is_empty() && auth.confirmation.is_empty() && auth.code.is_empty());
    auth.complete(Operation::Refresh, Err("Sessão revogada".into()));
    assert!(!auth.unlocked());
    assert!(auth.screen == Screen::SignIn);
}

#[test]
fn expired_session_is_locked_even_before_a_poll_and_retains_no_pending_result() {
    let mut auth = locked();
    auth.screen = Screen::VerifySignup;
    auth.session = Some(session(client::unix_seconds().saturating_sub(1)));
    assert!(!auth.unlocked());
    auth.poll(&egui::Context::default());
    assert!(auth.screen == Screen::SignIn);
    assert!(auth.session.is_none() && auth.pending.is_none());
}

#[test]
fn logout_discards_a_late_successful_login() {
    let mut auth = locked();
    let (sender, receiver) = std::sync::mpsc::channel();
    auth.pending = Some(Pending { operation: Operation::SignIn, started: web_time::Instant::now(), receiver });
    let ctx = egui::Context::default();
    auth.sign_out(&ctx);
    assert!(sender.send(Ok(Outcome::Session(session(client::unix_seconds() + 3600)))).is_err());
    auth.poll(&ctx);
    assert!(!auth.unlocked());
}

#[test]
fn locked_frontend_has_login_fields_and_hides_document_tabs_without_leaking_passwords() {
    let mut ui = crate::layers_drag_tests::Ui::new(designflow_engine::Session::new());
    ui.app.auth = Some(locked());
    let auth = ui.app.auth.as_mut().unwrap();
    auth.email = "synthetic@example.invalid".into();
    auth.password = "synthetic-test-secret".into();
    for _ in 0..3 {
        ui.frame(vec![]);
    }
    let shapes = crate::live_tests::flat(&ui.frame_output(vec![]).shapes);
    assert!(crate::live_tests::has_text(&shapes, "Entrar no DesignFlow"));
    assert!(!crate::live_tests::has_text(&shapes, "Untitled"));
    for text in ["Entrar no DesignFlow", "Criar conta", "Esqueci minha senha"] {
        let center = crate::live_tests::text_center(&shapes, text);
        assert!(ui.ctx.content_rect().contains(center), "account action outside viewport: {text}");
    }
    let inspect = ui.app.inspect().to_string();
    assert!(!inspect.contains("synthetic-test-secret"));
    assert!(!inspect.contains("synthetic@example.invalid"));
}

fn option3_ui() -> crate::layers_drag_tests::Ui {
    let mut ui = crate::layers_drag_tests::Ui::new(designflow_engine::Session::empty());
    let config = client::Config::new("https://synthetic-project.supabase.co", "sb_publishable_synthetic_test_key_1234");
    ui.app.auth = Some(Auth::new(config));
    for _ in 0..4 {
        ui.frame(vec![]);
    }
    ui
}

fn option3_click(ui: &mut crate::layers_drag_tests::Ui, position: egui::Pos2) {
    for pressed in [true, false] {
        ui.frame(vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton { pos: position, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
        ]);
    }
    ui.frame(vec![]);
}

#[test]
fn option3_tabs_preserve_email_and_clear_passwords_only_when_switching_screens() {
    let mut ui = option3_ui();
    let auth = ui.app.auth.as_mut().unwrap();
    auth.email = "synthetic@example.invalid".into();
    auth.password = "synthetic-secret".into();
    let shapes = crate::live_tests::flat(&ui.frame_output(vec![]).shapes);
    option3_click(&mut ui, crate::live_tests::text_center(&shapes, "Criar conta"));
    let auth = ui.app.auth.as_mut().unwrap();
    assert!(auth.screen == Screen::SignUp);
    assert_eq!(auth.email, "synthetic@example.invalid");
    assert!(auth.password.is_empty());
    auth.password = "second-synthetic-secret".into();
    let shapes = crate::live_tests::flat(&ui.frame_output(vec![]).shapes);
    assert!(crate::live_tests::has_text(&shapes, "Confirmar senha"));
    assert!(crate::live_tests::has_text(&shapes, "Criar minha conta"));
    option3_click(&mut ui, crate::live_tests::text_center(&shapes, "Criar conta"));
    assert_eq!(ui.app.auth.as_ref().unwrap().password, "second-synthetic-secret");
    let shapes = crate::live_tests::flat(&ui.frame_output(vec![]).shapes);
    option3_click(&mut ui, crate::live_tests::text_center(&shapes, "Entrar"));
    let auth = ui.app.auth.as_ref().unwrap();
    assert!(auth.screen == Screen::SignIn);
    assert_eq!(auth.email, "synthetic@example.invalid");
    assert!(auth.password.is_empty());
    assert!(auth.pending.is_none());
}

#[test]
fn option3_eye_toggles_visibility_without_changing_the_password_or_starting_a_request() {
    let mut ui = option3_ui();
    ui.ctx.enable_accesskit();
    ui.app.auth.as_mut().unwrap().password = "synthetic-eye-secret".into();
    ui.frame(vec![]);
    let update = ui.frame_output(vec![]).platform_output.accesskit_update.unwrap();
    let bounds = update.nodes.iter().find(|(_, node)| node.label() == Some("Mostrar senha")).and_then(|(_, node)| node.bounds()).unwrap();
    option3_click(&mut ui, egui::pos2(((bounds.x0 + bounds.x1) * 0.5) as f32, ((bounds.y0 + bounds.y1) * 0.5) as f32));
    let auth = ui.app.auth.as_ref().unwrap();
    assert!(auth.reveal);
    assert_eq!(auth.password, "synthetic-eye-secret");
    assert!(auth.pending.is_none());
    assert!(ui.ctx.egui_wants_keyboard_input());
}

#[test]
fn designflow_compact_tools_expand_and_windows_uses_system_pointer() {
    let ctx = egui::Context::default();
    let mut app = designflow_app();
    app.run("file.new", serde_json::json!({})).unwrap();
    app.ui.home = false;
    for _ in 0..5 {
        designflow_frame(&ctx, &mut app, vec![]);
    }
    let document = app.session.ds().unwrap().uid;
    for label in ["Estilos", "Propriedades"] {
        let output = designflow_frame(&ctx, &mut app, vec![]);
        let shapes = crate::live_tests::flat(&output.shapes);
        let at = crate::live_tests::text_center(&shapes, label);
        for pressed in [true, false] {
            designflow_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
                ],
            );
        }
        let styles = ctx.data(|d| d.get_temp::<bool>(egui::Id::unique("designflow_styles_tab"))).unwrap_or(false);
        assert_eq!(styles, label == "Estilos");
    }
    let bar = crate::toolbar::pill_rect(app.canvas.rect, 288.0);
    let text = egui::pos2(bar.min.x + 8.0 + 4.0 * 40.0 + 16.0, bar.center().y);
    for pressed in [true, false] {
        designflow_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(text),
                egui::Event::PointerButton { pos: text, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
            ],
        );
    }
    assert_eq!(app.session.tool.tool, designflow_engine::ToolId::Text);
    let more = egui::pos2(bar.min.x + 8.0 + 6.0 * 40.0 + 16.0, bar.center().y);
    for pressed in [true, false] {
        designflow_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(more),
                egui::Event::PointerButton { pos: more, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
            ],
        );
    }
    designflow_frame(&ctx, &mut app, vec![]);
    assert!(app.ui.actions_rect.is_some(), "expanded toolbar keeps advanced actions accessible");
    assert_eq!(app.session.ds().unwrap().uid, document);
    app.run("tool.set", serde_json::json!({"tool":"pen"})).unwrap();
    let position = app.canvas.rect.center();
    let output = designflow_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(position)]);
    if cfg!(target_os = "windows") {
        assert_eq!(output.platform_output.cursor_icon, egui::CursorIcon::Crosshair);
        assert!(matches!(app.canvas.cursor, Some(crate::cursors::Shown::System(_))));
    }
}
