fn no_darkness_fog(value: Option<f32>) -> Option<f32> {
    if ::mod_sunrise_tweaks::no_darkness() {
        None
    } else {
        value
    }
}
