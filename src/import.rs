use crate::{face::Face, state::Vertex};
use eyre::{eyre, Result};
use itertools::Itertools;
use nalgebra::{distance, Point3, Vector3};
use ply_rs::{parser::Parser, ply};
use std::{collections::HashMap, path::Path};

pub enum Import {
    Ply(PlyData),
    Image(image::DynamicImage),
}

pub struct PlyData {
    pub point_vertices: Vec<Vertex>,
    pub face_vertices: Vec<Vertex>,
    pub face_indices: Vec<u32>,
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
            let face_parser = Parser::<Face>::new();
            let header = vertex_parser.read_header(&mut buf_read)?;

            let mut all_vertices = Vec::new();
            let mut faces = Vec::new();

            for (_, element) in &header.elements {
                if element.name == "vertex" {
                    all_vertices =
                        vertex_parser.read_payload_for_element(&mut buf_read, element, &header)?;
                }
                if element.name == "face" {
                    faces =
                        face_parser.read_payload_for_element(&mut buf_read, element, &header)?;
                }
            }

            // maps from ply vertex indices to face vertex indices
            let mut all_vertices_to_face_vertices = HashMap::new();
            let mut face_vertices = Vec::new();
            let mut face_indices = Vec::new();
            // looks up face vertex index or creates face vertex for ply vertex
            let mut get_or_insert_vertex = |index| {
                *all_vertices_to_face_vertices
                    .entry(index)
                    .or_insert_with(|| {
                        let pos = face_vertices.len();
                        face_vertices.push(all_vertices[index]);
                        pos
                    })
            };
            // tesselate faces
            for face in faces {
                // turns ply vertices into face vertices
                let mut face_iter = face
                    .vertex_index
                    .iter()
                    .map(|&i| get_or_insert_vertex(i as usize));
                // first vertex is center of triangle fan and first vertex
                // of all triangles
                if let Some(first) = face_iter.next() {
                    for (second, third) in face_iter.tuple_windows() {
                        // each set of 2 indicies is a new triangle in the triangle fan.
                        face_indices.extend_from_slice(&[
                            first as u32,
                            second as u32,
                            third as u32,
                        ]);
                    }
                }
            }

            // set of vertices that do not corrispond to faces.
            let point_vertices = all_vertices
                .iter()
                .enumerate()
                .filter(|(i, _)| !all_vertices_to_face_vertices.contains_key(i))
                .map(|(_, &v)| v)
                .collect_vec();

            let ply_data = PlyData {
                point_vertices,
                face_vertices,
                face_indices,
            };

            Ok(Import::Ply(ply_data))
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

impl ply::PropertyAccess for Face {
    fn new() -> Self {
        Face {
            vertex_index: Vec::new(),
        }
    }

    fn set_property(&mut self, key: String, property: ply::Property) {
        match (key.as_ref(), property) {
            ("vertex_index", ply::Property::ListInt(vec)) => self.vertex_index = vec,
            (k, _) => panic!("Face: Unexpected key/value combination: key: {}", k),
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
