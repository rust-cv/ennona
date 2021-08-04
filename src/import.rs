use eyre::{eyre, Result};
use std::path::Path;

use crate::state::Vertex;
use nalgebra::{distance, Point3, Vector3};
use ply_rs::{parser::Parser, ply};

pub enum Import {
    Ply(Vec<Vertex>),
    Image(image::DynamicImage),
}

pub fn import(path: &Path) -> Result<Import> {
    let f = std::fs::File::open(path).unwrap();
    let mut buf_read = std::io::BufReader::new(f);
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("no_extension");

    match extension {
        "ply" => {
            let vertex_parser = Parser::<Vertex>::new();
            let header = vertex_parser.read_header(&mut buf_read)?;

            let mut vertex_list = Vec::new();

            for (_, element) in &header.elements {
                if element.name == "vertex" {
                    vertex_list =
                        vertex_parser.read_payload_for_element(&mut buf_read, element, &header)?;
                }
            }
            Ok(Import::Ply(vertex_list))
        }
        "png" | "jpg" | "jpeg" => {
            // let img = image::load_from_memory(buf_read)?;
            let img = image::io::Reader::new(buf_read)
                .with_guessed_format()?
                .decode()?;
            Ok(Import::Image(img))
        }
        _ => Err(eyre!(
            "Unknown file extension '{}' in file: '{:?}'",
            extension,
            path
        )),
    }
}

impl ply::PropertyAccess for Vertex {
    fn new() -> Self {
        Vertex {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0],
        }
    }

    fn set_property(&mut self, key: String, property: ply::Property) {
        match (key.as_ref(), property) {
            ("x", ply::Property::Double(v)) => self.position[0] = v as f32,
            ("y", ply::Property::Double(v)) => self.position[1] = v as f32,
            ("z", ply::Property::Double(v)) => self.position[2] = v as f32,
            ("red", ply::Property::UChar(v)) => self.color[0] = v as f32 / 255.,
            ("green", ply::Property::UChar(v)) => self.color[1] = v as f32 / 255.,
            ("blue", ply::Property::UChar(v)) => self.color[2] = v as f32 / 255.,
            (k, _) => panic!("Unexpected key/value combination: key: {}", k),
        }
    }
}

/// Calculates the average position of all vertices provided in the list
pub fn avg_vertex_position(vertices: &[Vertex]) -> Point3<f32> {
    let mut center = Point3::<f64>::new(0.0, 0.0, 0.0);
    let length = vertices.len() as f64;
    for v in vertices {
        center.coords += Vector3::from(v.position).map(|x| x as f64);
    }

    center.coords /= length;

    center.map(|x| x as f32)
}

/// Calculates the average distance of all vertices to the average vertex position
pub fn avg_vertex_distance(avg_vertex_position: Point3<f32>, vertices: &[Vertex]) -> f32 {
    let mut sum = 0.0;
    let length = vertices.len() as f32;
    for v in vertices {
        let pos = Point3::<f32>::from(v.position);
        sum += distance(&avg_vertex_position, &pos);
    }

    sum / length
}
