// panel.rs
use iced::Vector;
use iced::advanced::Renderer as _;
use iced::{
    Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Size, Theme,
    advanced::{Widget, layout, renderer, widget},
};

/// Width the design values below were authored at. Everything scales from this.
pub const REFERENCE_WIDTH: f32 = 600.0;

#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub inset: Vector,  // outer edge -> white box
    pub margin: Vector, // white box edge -> content
    pub border_width: f32,
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub glow: f32,
}

impl Metrics {
    pub const REFERENCE: Self = Self {
        inset: Vector::new(35.0, 7.0),
        margin: Vector::new(16.0, 10.0),
        border_width: 6.0,
        outer_radius: 8.0,
        inner_radius: 12.0,
        glow: 5.0,
    };

    pub fn scaled(self, s: f32) -> Self {
        Self {
            inset: Vector::new(self.inset.x * s, self.inset.y * s),
            margin: Vector::new(self.margin.x * s, self.margin.y * s),
            border_width: self.border_width * s,
            outer_radius: self.outer_radius * s,
            inner_radius: self.inner_radius * s,
            glow: self.glow * s,
        }
    }

    /// Distance from the outer edge to the content area.
    fn content_padding(&self) -> Padding {
        Padding {
            top: self.inset.y + self.margin.y,
            bottom: self.inset.y + self.margin.y,
            left: self.inset.x + self.margin.x,
            right: self.inset.x + self.margin.x,
        }
    }

    /// Size available to content for a panel of `outer` size.
    /// Use this for `shrink_text_to_fit`'s max_width / max_height.
    pub fn content_size(outer: Size, base: Self) -> Size {
        let m = base.scaled(scale_for(outer.width));
        let p = m.content_padding();
        Size::new(
            (outer.width - p.left - p.right).max(0.0),
            (outer.height - p.top - p.bottom).max(0.0),
        )
    }
}

pub fn scale_for(width: f32) -> f32 {
    if width.is_finite() && width > 0.0 {
        width / REFERENCE_WIDTH
    } else {
        1.0
    }
}

pub struct Panel<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
    metrics: Metrics,
}

impl<'a, Message> Panel<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            width: Length::Shrink,
            height: Length::Shrink,
            metrics: Metrics::REFERENCE,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
    pub fn metrics(mut self, metrics: Metrics) -> Self {
        self.metrics = metrics;
        self
    }
}

impl<'a, Message> Widget<Message, Theme, Renderer> for Panel<'a, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);

        // Resolve the outer width first so we know the scale before laying out the child.
        let outer = limits.resolve(self.width, self.height, Size::ZERO);
        let pad = self
            .metrics
            .scaled(scale_for(outer.width))
            .content_padding();

        let child = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &limits.shrink(pad),
        );
        let intrinsic = Size::new(
            child.size().width + pad.left + pad.right,
            child.size().height + pad.top + pad.bottom,
        );
        let size = limits.resolve(self.width, self.height, intrinsic);

        layout::Node::with_children(size, vec![child.move_to(Point::new(pad.left, pad.top))])
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
        let m = self.metrics.scaled(bounds.width / REFERENCE_WIDTH);

        // blue outer box
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    color: Color::from_rgba8(77, 238, 255, 0.8),
                    width: m.border_width,
                    radius: m.outer_radius.into(),
                },
                shadow: Shadow {
                    offset: iced::Vector { x: 0.0, y: 0.0 },
                    blur_radius: m.glow,
                    color: Color::from_rgba8(0, 238, 255, 1.0),
                },
                ..Default::default()
            },
            Color::from_rgba8(45, 190, 255, 0.9),
        );

        // white inner box
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x + m.inset.x,
                    y: bounds.y + m.inset.y,
                    width: bounds.width - m.inset.x * 2.0,
                    height: bounds.height - m.inset.y * 2.0,
                },
                border: Border {
                    color: Color::BLACK,
                    width: 0.0,
                    radius: m.inner_radius.into(),
                },
                ..Default::default()
            },
            Color::WHITE,
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
