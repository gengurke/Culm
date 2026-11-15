fn main() {
    rune::set_app_id(addon_app::spec_app_id());
    rune::run(addon_app::spec());
}
