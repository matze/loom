#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use yew::prelude::*;

struct Model {}

enum Message {}

impl Component for Model {
    type Message = Message;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
        true
    }

    fn changed(&mut self, _ctx: &Context<Self>) -> bool {
        false
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <h1>{"Today"}</h1>
                <p>{"Begin"} <button>{"Now"}</button> {"or"} <input type={"time"}/></p>
                <p>{"End"} <button>{"Now"}</button> {"or"} <input type={"time"}/></p>
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}
