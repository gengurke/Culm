mod brand;

use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, col, text};

#[derive(Default)]
pub struct State {}

pub fn view(_s: &State, _: &mut Cx<State>) -> El {
    let t = tok::theme(true);
    col()
        .w_full()
        .h_full()
        .center()
        .fill(t.bg)
        .child(text(brand::APP_NAME).font(type_::TITLE).color(t.text))
}

fn main() {
    rune::run(AppSpec::<State> {
        ..AppSpec::new(brand::APP_NAME, (1100.0, 720.0), State::default, view)
    });
}
