fn no_darkness_light(value: (f32, f32)) -> (f32, f32) {
    if ::mod_sunrise_tweaks::no_darkness() {
        (0.0, 0.0)
    } else {
        value
    }
}
