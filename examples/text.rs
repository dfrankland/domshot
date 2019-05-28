use domshot::spawn_dom;
use image;
use tokio::{
    prelude::*,
    sync::{mpsc, oneshot},
};

fn main() {
    let (render_sender, render_reciever) = mpsc::unbounded_channel();
    let (close_sender, close_reciever) = oneshot::channel();

    let mut count = 0;
    let handle_render = render_reciever
        .take(3)
        .for_each(move |render: image::DynamicImage| {
            count += 1;
            render.save(&format!("screenshot-{}.png", count)).unwrap();
            Ok(())
        })
        .then(|_| close_sender.send(()));

    let dom = spawn_dom(
        include_str!("text.xml"),
        render_sender,
        close_reciever,
        Some(include_str!("text.css")),
        Some(vec![
            "Noto Sans CJK JP",
            "Noto Color Emoji",
            "Ubuntu",
            "Sahadeva",
        ]),
    );

    tokio::run(future::lazy(|| {
        tokio::spawn(handle_render);
        dom
    }));
}
