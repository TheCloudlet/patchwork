use crate::Draw;
use crate::shape::Rect;
use crate::surface::Surface;

pub struct Pane {
    pub area: Rect,
    pub content: Vec<Box<dyn Draw>>,
    pub children: Vec<Pane>,
    // pub popup: Vec<Pane>, // TODO
}

impl Pane {
    /// An empty pane covering `area`, with no content and no children.
    pub fn new(area: Rect) -> Self {
        Pane {
            area,
            content: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Adds a drawable to this pane's own content layer.
    pub fn push(&mut self, item: Box<dyn Draw>) {
        self.content.push(item);
    }

    /// Adds a child pane, drawn on top of this pane's content.
    pub fn add_child(&mut self, child: Pane) {
        self.children.push(child);
    }
}

impl Draw for Pane {
    fn draw(&self, surface: &mut Surface) {
        // Narrow to this pane's area — relative to the surface we were
        // handed, clamped to it. A pane can never paint outside what it was
        // given, however optimistic its own area is.
        let mut own = surface.sub(self.area);

        // Painter's algorithm: this pane's own content first (the background),
        // then each child on top, in order. Later items overwrite earlier ones.
        for item in &self.content {
            item.draw(&mut own);
        }
        for child in &self.children {
            child.draw(&mut own);
        }
    }
}
