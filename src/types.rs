#![allow(unused)]

use std::{cmp, fmt};

macro_rules! structs {
    ($($(#[$attr:meta])* struct $name:ident { $($body:tt)* })*) => {
        $(
            $(#[$attr])*
            #[derive(Clone, Deserialize)]
            #[serde(rename_all = "camelCase")]
            pub struct $name { $($body)* }
        )*
    }
}

structs! {
    struct Annotations {
        pub text_annotations: Vec<Annotation>,
        full_text_annotation: FullAnnotation,
    }

    struct Annotation {
        #[serde(default = "default_locale")] pub locale: String,
        pub description: String,
        pub bounding_poly: BoundingBox,
    }

    struct FullAnnotation {
        text: String,
        pages: Vec<Page>,
    }

    struct Page {
        width: i32,
        height: i32,
        property: Property,
        blocks: Vec<Block>,
    }

    struct Block {
        block_type: BlockType,
        property: Property,
        bounding_box: BoundingBox,
        paragraphs: Vec<Paragraph>,
    }

    struct Paragraph {
        property: Property,
        bounding_box: BoundingBox,
        words: Vec<Word>,
    }

    struct Word {
        bounding_box: BoundingBox,
        symbols: Vec<Symbol>,
    }

    struct Symbol {
        property: Property,
        bounding_box: BoundingBox,
        text: String,
    }

    struct Property {
        detected_languages: Vec<Language>,
    }

    struct Language {
        language_code: String,
    }

    struct BoundingBox {
        vertices: BoundingBoxVertices,
    }

    struct BoundingBoxVertices {
        sw: Vertex,
        se: Vertex,
        ne: Vertex,
        nw: Vertex,
    }

    #[derive(Copy)]
    struct Vertex {
        // values could be missing: https://stackoverflow.com/a/39378944/1114328
        #[serde(default)] x: i32,
        #[serde(default)] y: i32,
    }
}

#[derive(Copy, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlockType {
    Unknown,
    Text,
    Table,
    Picture,
    Ruler,
    Barcode,
}

fn default_locale() -> String {
    "en".into()
}

impl BoundingBox {
    pub fn left(&self) -> i32 {
        cmp::min(self.vertices.sw.x,
                 self.vertices.nw.x)
    }
    pub fn right(&self) -> i32 {
        cmp::max(self.vertices.se.x,
                 self.vertices.ne.x)
    }
    pub fn top(&self) -> i32 {
        cmp::max(self.vertices.ne.y,
                 self.vertices.nw.y)
    }
    pub fn bottom(&self) -> i32 {
        cmp::min(self.vertices.sw.y,
                 self.vertices.se.y)
    }
    
    pub fn width(&self) -> u32 {
        (self.right() - self.left()) as u32
    }
    pub fn height(&self) -> u32 {
        (self.top() - self.bottom()) as u32
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self {
            vertices: BoundingBoxVertices {
                sw: Vertex {
                    x: cmp::min(self.vertices.sw.x, other.vertices.sw.x),
                    y: cmp::min(self.vertices.sw.y, other.vertices.sw.y),
                },
                se: Vertex {
                    x: cmp::max(self.vertices.se.x, other.vertices.se.x),
                    y: cmp::min(self.vertices.se.y, other.vertices.se.y),
                },
                ne: Vertex {
                    x: cmp::max(self.vertices.ne.x, other.vertices.ne.x),
                    y: cmp::max(self.vertices.ne.y, other.vertices.ne.y),
                },
                nw: Vertex {
                    x: cmp::min(self.vertices.nw.x, other.vertices.nw.x),
                    y: cmp::max(self.vertices.nw.y, other.vertices.nw.y),
                },
            },
        }
    }

    pub fn area(&self) -> u32 {
        self.width() * self.height()
    }
}

impl fmt::Display for Annotation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{:25} ({:4}, {:4}) ({:4}, {:4})",
               self.description,
               self.bounding_poly.left(), self.bounding_poly.top(),
               self.bounding_poly.right(), self.bounding_poly.bottom())
    }
}

