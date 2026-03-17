// hex_button.rs
use iced::{
    Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Size, Theme, advanced::{Widget, layout, renderer, widget}, mouse
};
use iced::widget::canvas::{Frame, Stroke};
use iced::widget::canvas::path::Builder;

pub struct HexButton<Message> {
    label: String,
    on_press: Option<Message>,
    edge_width: f32,
    normal_color: Color,
    hover_color: Color,
    press_color: Color,
}

impl<Message: Clone> HexButton<Message> {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_press: None,
            edge_width: 20.0,
            normal_color: Color::from_rgb(0.2, 0.2, 0.2),
            hover_color: Color::from_rgb(0.4, 0.4, 0.4),
            press_color: Color::from_rgb(0.1, 0.1, 0.1),
        }
    }

    pub fn on_press(mut self, msg: Message) -> Self {
        self.on_press = Some(msg);
        self
    }

    pub fn colors(mut self, normal: Color, hover: Color, press: Color) -> Self {
        self.normal_color = normal;
        self.hover_color = hover;
        self.press_color = press;
        self
    }
}

// internal state for hover/press tracking
#[derive(Default)]
pub struct HexButtonInternalState {
    is_hovered: bool,
    is_pressed: bool,
}

impl<Message: Clone + 'static> Widget<Message, Theme, Renderer> for HexButton<Message> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Fixed(40.0))
    }

    fn layout(&mut self, _tree: &mut widget::Tree, _renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let padding = Padding::new(8.0).left(self.edge_width + 8.0).right(self.edge_width + 8.0);
        let text_limits = limits.shrink(padding);
        
        // measure text
        let text_size = text_limits.resolve(Length::Shrink, Length::Shrink, Size::new(
            text_limits.max().width,
            40.0,
        ));

        layout::Node::new(Size::new(
            text_size.width + padding.left + padding.right,
            40.0,
        ))
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: layout::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<HexButtonInternalState>();
        let bounds = layout.bounds();

        let color = if state.is_pressed {
            self.press_color
        } else if state.is_hovered {
            self.hover_color
        } else {
            self.normal_color
        };

        use iced::advanced::Renderer as _;
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    color: Color::BLACK,
                    width: 2.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            },
            color,
        );

        use iced::advanced::text::Renderer as _;
        renderer.fill_text(
            iced::advanced::text::Text {
                content: self.label.clone(),
                bounds: bounds.size(),
                size: iced::Pixels(16.0),
                line_height: iced::widget::text::LineHeight::default(),
                font: renderer.default_font(),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center,
                shaping: iced::widget::text::Shaping::default(),
                wrapping: iced::widget::text::Wrapping::default(),
            },
            Point::new(bounds.x, bounds.y),
            Color::WHITE,
            *viewport,
        );
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<HexButtonInternalState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(HexButtonInternalState::default())
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<HexButtonInternalState>();
        let bounds = layout.bounds();
        let is_over = cursor.is_over(bounds);

        match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.is_hovered != is_over {
                    state.is_hovered = is_over;
                    shell.request_redraw();
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if is_over => {
                state.is_pressed = true;
                shell.request_redraw();
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_pressed && is_over {
                    if let Some(msg) = self.on_press.clone() {
                        shell.publish(msg);
                    }
                }
                if state.is_pressed {
                    state.is_pressed = false;
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &widget::Tree,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Message: Clone + 'static> From<HexButton<Message>> for Element<'a, Message> {
    fn from(btn: HexButton<Message>) -> Self {
        Element::new(btn)
    }
}