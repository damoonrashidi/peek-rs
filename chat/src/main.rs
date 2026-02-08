use gpui::{
    App, Application, Bounds, Context, Entity, KeyBinding, SharedString, TitlebarOptions, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size,
};
use gpui_component::{
    h_flex,
    select::{Select, SelectEvent, SelectState},
    v_flex,
};

#[derive(Clone, Debug)]
struct ConnectionInfo {
    workspace_name: String,
    connection_name: String,
    url: String,
}

impl ConnectionInfo {
    fn display_name(&self) -> String {
        format!("[{}] {}", self.workspace_name, self.connection_name)
    }

    fn connection_url(&self) -> &str {
        &self.url
    }
}

struct ChatWindow {
    connections: Vec<ConnectionInfo>,
    connection_selector: Entity<SelectState<Vec<SharedString>>>,
    selected_index: usize,
}

impl ChatWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let conf = config::PeekConfig::get_or_default();

        let connections: Vec<ConnectionInfo> = conf
            .workspaces
            .iter()
            .flat_map(|workspace| {
                workspace
                    .connections
                    .iter()
                    .map(|connection| ConnectionInfo {
                        workspace_name: workspace.name.clone(),
                        connection_name: connection.name.clone(),
                        url: connection.url.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        let display_names: Vec<SharedString> = connections
            .iter()
            .map(|conn| SharedString::from(conn.display_name()))
            .collect();

        let connection_selector = cx.new(|cx| SelectState::new(display_names, None, window, cx));

        cx.subscribe_in(
            &connection_selector,
            window,
            |view, state, event, _window, cx| match event {
                SelectEvent::Confirm(_value) => {
                    let selected_index_path = state.read(cx).selected_index(cx);
                    if let Some(index_path) = selected_index_path {
                        let index = index_path.row;
                        view.selected_index = index;
                        if let Some(connection) = view.connections.get(index) {
                            println!(
                                "Selected connection: [{}] {} ({})",
                                connection.workspace_name,
                                connection.connection_name,
                                connection.connection_url()
                            );
                        }
                    }
                    cx.notify();
                }
            },
        )
        .detach();

        Self {
            connections,
            connection_selector,
            selected_index: 0,
        }
    }

    fn get_current_connection(&self) -> Option<&ConnectionInfo> {
        self.connections.get(self.selected_index)
    }

    fn render_titlebar(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .w_full()
            .h(px(40.))
            .items_center()
            .justify_between()
            .px_4()
            .pl_12()
            .bg(rgb(0x001e_1e1e))
            .child(div().text_sm().text_color(rgb(0x00cc_cccc)).child("Peek"))
            .child(
                h_flex().items_center().gap_2().child(
                    Select::new(&self.connection_selector)
                        .size_1()
                        .placeholder("Select...")
                        .w(px(200.)),
                ),
            )
    }
}

impl Render for ChatWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let display_name = self
            .get_current_connection()
            .map_or_else(|| "No connection".to_string(), ConnectionInfo::display_name);

        v_flex()
            .size_full()
            .child(self.render_titlebar(window, cx))
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .p_4()
                    .items_center()
                    .justify_center()
                    .child(format!("Chat interface for: {display_name}")),
            )
    }
}

actions!(window, [Quit]);

fn main() {
    Application::new().run(|cx: &mut App| {
        // Initialize gpui-component theme system
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(900.0), px(600.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    traffic_light_position: Some(gpui::Point::new(px(8.), px(8.))),
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| ChatWindow::new(window, cx)),
        )
        .unwrap();

        cx.activate(true);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    });
}
