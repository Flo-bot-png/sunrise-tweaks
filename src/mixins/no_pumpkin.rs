fn no_pumpkin(
    width: i32,
    height: i32,
    overlays: &hud_overlays::CameraOverlays,
    scope: &mut f32,
    operation: impl FnOnce(i32, i32, &hud_overlays::CameraOverlays, &mut f32) -> Vec<sunrise_client::hud_layout::GuiDraw>,
) -> Vec<sunrise_client::hud_layout::GuiDraw> {
    if !::mod_sunrise_tweaks::no_pumpkin() {
        return operation(width, height, overlays, scope);
    }
    let mut without = overlays.clone();
    without.equipment.retain(|texture| !texture.contains("pumpkinblur"));
    operation(width, height, &without, scope)
}
