#[test]
fn vectorization_keeps_original_and_is_one_undo_step() {
    use base64::Engine;
    let image = image::RgbaImage::from_fn(8, 8, |x, y| {
        if (2..6).contains(&x)
            && (2..6).contains(&y)
            && !((3..5).contains(&x) && (3..5).contains(&y))
        {
            image::Rgba([0, 0, 0, 255])
        } else {
            image::Rgba([255, 255, 255, 255])
        }
    });
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let mut session = designflow_engine::Session::default();
    session.execute("file.new", &serde_json::json!({})).unwrap();
    let placed = session
        .execute(
            "image.place",
            &serde_json::json!({
                "data": base64::engine::general_purpose::STANDARD.encode(bytes.into_inner()),
                "width": 80, "x": 10, "y": 20
            }),
        )
        .unwrap();
    let original = placed["id"].clone();
    let before = session.doc().unwrap().nodes().count();
    assert!(session.execute("image.vectorize", &serde_json::json!({"threshold":0})).is_err());
    assert!(session.execute("image.vectorize", &serde_json::json!({"threshold":256})).is_err());
    assert_eq!(session.doc().unwrap().nodes().count(), before);
    let traced = session
        .execute("image.vectorize", &serde_json::json!({"threshold":128}))
        .unwrap();
    assert_ne!(traced["id"], original);
    assert!(session.doc().unwrap().nodes().count() > before);
    assert!(
        session
            .doc()
            .unwrap()
            .nodes()
            .any(|node| matches!(node.kind, designflow_doc::NodeKind::Vector { .. }))
    );
    let exported = session
        .execute("file.export", &serde_json::json!({"format":"svg"}))
        .unwrap();
    let svg = base64::engine::general_purpose::STANDARD
        .decode(exported["data"].as_str().unwrap())
        .unwrap();
    let svg = String::from_utf8(svg).unwrap();
    assert!(!svg.contains("<image"));
    let png = session
        .execute("file.export", &serde_json::json!({"format":"png"}))
        .unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png["data"].as_str().unwrap())
        .unwrap();
    let raster = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(
        raster.get_pixel(raster.width() / 2, raster.height() / 2)[3],
        0
    );
    session
        .execute("edit.undo", &serde_json::json!({}))
        .unwrap();
    assert_eq!(session.doc().unwrap().nodes().count(), before);
    session
        .execute("edit.redo", &serde_json::json!({}))
        .unwrap();
    assert!(session.doc().unwrap().nodes().count() > before);
}
