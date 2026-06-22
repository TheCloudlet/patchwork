use crate::shape::Rect;
use crate::Draw;

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
    fn draw(&self, buf: &mut crate::buffer::Buffer, area: Rect) {
        // This pane occupies `self.area`, expressed relative to the `area`
        // it was handed. Translate it into an absolute origin that content and
        // children are positioned against.
        let origin = Rect {
            x: area.x + self.area.x,
            y: area.y + self.area.y,
            w: self.area.w,
            h: self.area.h,
        };

        // Painter's algorithm: this pane's own content first (the background),
        // then each child on top, in order. Later items overwrite earlier ones.
        // Everything is drawn relative to this pane's origin.
        for item in &self.content {
            item.draw(buf, origin);
        }
        for child in &self.children {
            child.draw(buf, origin);
        }
    }
}
