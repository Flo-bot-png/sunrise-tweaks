fn fullbright(game: &mut Game<'_>) -> ::core::ops::ControlFlow<()> {
    if ::mod_sunrise_tweaks::fullbright() {
        if let Some(renderer) = game.renderer.as_mut() {
            renderer.fullbright = true;
        }
        return ::core::ops::ControlFlow::Break(());
    }
    ::core::ops::ControlFlow::Continue(())
}
