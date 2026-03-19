// panel.rs
use iced::{
    Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Size, Theme, advanced::{Widget, layout, renderer, widget}
};
use iced::advanced::Renderer as _;

pub struct Panel<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
}

impl<'a, Message> Panel<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            width: Length::Shrink,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<'a, Message> Widget<Message, Theme, Renderer> for Panel<'a, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut widget::Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let inner_padding = Padding { top: 4.0, bottom: 4.0, left: 20.0, right: 20.0 };
        let limits = limits.width(self.width);

        let child_node = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &limits.shrink(inner_padding),
        );

        let child_size = child_node.size();
        layout::Node::with_children(
            Size {
                width: child_size.width,
                height: child_size.height + inner_padding.top + inner_padding.bottom,
            },
            vec![child_node.move_to(Point::new(0.0, inner_padding.top))],
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let inner_padding = 10.0;

        // blue outer box
        renderer.fill_quad(
            renderer::Quad { 
                bounds, 
                    border: Border {
                    color: Color::from_rgba8(77, 238, 255, 0.8),
                    width: 6.0,
                    radius: 8.0.into()
                },
                shadow: Shadow {
                    offset: iced::Vector { x: 0.0, y: 0.0 },
                    blur_radius: 5.0,
                    color: Color::from_rgba8(0, 238, 255, 1.0)
                },
                ..Default::default() },
        Color::from_rgba8(45, 190, 255, 0.9)
        );

        // white inner box
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle { 
                    x: bounds.x + inner_padding * 5.0, 
                    y: bounds.y + inner_padding / 2.0, 
                    width: bounds.width - inner_padding * 10.0, 
                    height: bounds.height - inner_padding 
                },
                border: Border {
                    color: Color::BLACK,
                    width: 0.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            },
            Color::WHITE
        );

        // draw content
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().unwrap(),
            cursor,
            viewport,
        );
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }
}

impl<'a, Message: 'a> From<Panel<'a, Message>> for Element<'a, Message> {
    fn from(panel: Panel<'a, Message>) -> Self {
        Element::new(panel)
    }
}