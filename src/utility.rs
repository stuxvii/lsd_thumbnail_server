use image::{GenericImageView, ImageError, ImageReader};
use macroquad::prelude::*;
use serde::Deserialize;
use sqlx::prelude::FromRow;
use sqlx::{MySql, Pool};
use std::error::Error;
use std::path::Path;
use std::io::Cursor;

#[derive(Debug, FromRow)]
struct JsonString {
    json: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BodyColors {
    pub trso: u16,
    pub head: u16,
    pub lleg: u16,
    pub larm: u16,
    pub rarm: u16,
    pub rleg: u16,
}

impl Default for BodyColors {
    fn default() -> Self {
        Self {
            trso: 1001,
            head: 1001,
            lleg: 1001,
            larm: 1001,
            rarm: 1001,
            rleg: 1001,
        }
    }
}

pub async fn fetch_avatar(
    pool: &Pool<MySql>,
    user_id: i32,
) -> Result<(BodyColors, Vec<i32>), Box<dyn Error>> {
    let colors_row: Option<JsonString> = sqlx::query_as(
        r#"SELECT colors as json FROM profiles WHERE id = ?"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let body_colors: BodyColors = match colors_row {
        Some(row) => serde_json::from_str(&row.json).unwrap_or_else(|err| {
            eprintln!("Failed to parse body colors for user {}: {}", user_id, err);
            BodyColors::default()
        }),
        None => BodyColors::default(),
    };

    let items_row: Option<JsonString> = sqlx::query_as(
        r#"SELECT equipped as json FROM profiles WHERE id = ?"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let items: Vec<i32> = match items_row {
        Some(row) => serde_json::from_str(&row.json).unwrap_or_else(|err| {
            eprintln!("Failed to parse items for user {}: {}", user_id, err);
            vec![0]
        }),
        None => vec![0],
    };

    Ok((body_colors, items))
}

#[derive(Debug, FromRow, Clone)]
pub struct ItemAsset {
    pub item_type: i8,
    pub location: Option<String>,
    pub texture_path: Option<String>,
}

pub async fn fetch_accessories_info(
    pool: &Pool<MySql>,
    item_ids: Vec<i32>,
) -> Result<Vec<ItemAsset>, Box<dyn Error>> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders: String = (0..item_ids.len())
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        r#"
        SELECT
            i.type AS item_type,
            i.asset AS location,
            a.asset AS texture_path
        FROM items i
        LEFT JOIN items a ON i.hat_texture = a.id
        WHERE i.id IN ({}) AND i.approved = 1
        "#,
        placeholders
    );

    let mut query = sqlx::query_as::<_, ItemAsset>(&sql);
    for id in item_ids {
        query = query.bind(id);
    }

    let item_assets: Vec<ItemAsset> = query.fetch_all(pool).await?;
    Ok(item_assets)
}

pub fn process_img(img_path: &Path) -> Result<(u32, u32, Vec<u8>), ImageError> {
    let img = ImageReader::open(img_path)?.decode()?;
    let bytes = img.to_rgba8().into_vec();
    let (width, height) = img.dimensions();
    Ok((width, height, bytes))
}

pub fn replace_transparent_with_color(mut bytes: Vec<u8>, hex_color: u32) -> Vec<u8> {
    let bg_r = ((hex_color >> 16) & 0xFF) as u32;
    let bg_g = ((hex_color >> 8) & 0xFF) as u32;
    let bg_b = (hex_color & 0xFF) as u32;

    for pixel in bytes.chunks_exact_mut(4) {
        let alpha = pixel[3] as u32;

        if alpha == 255 {
            continue;
        }

        if alpha == 0 {
            pixel[0] = bg_r as u8;
            pixel[1] = bg_g as u8;
            pixel[2] = bg_b as u8;
            pixel[3] = 255;
            continue;
        }

        let inv_alpha = 255 - alpha;
        pixel[0] = ((pixel[0] as u32 * alpha + bg_r * inv_alpha) / 255) as u8;
        pixel[1] = ((pixel[1] as u32 * alpha + bg_g * inv_alpha) / 255) as u8;
        pixel[2] = ((pixel[2] as u32 * alpha + bg_b * inv_alpha) / 255) as u8;
        pixel[3] = 255;
    }

    bytes
}

pub fn load_static_mesh_from_bytes(name: &str, bytes: &[u8]) -> Option<tobj::Mesh> {
    let mut cursor = Cursor::new(bytes);
    match tobj::load_obj_buf(&mut cursor, &tobj::GPU_LOAD_OPTIONS, |p| {
        // this is gonna return JACKSHIT bro
        tobj::load_mtl(p)
    }) {
        Ok((meshes, _)) if !meshes.is_empty() => Some(meshes[0].mesh.clone()),
        Ok(_) => {
            eprintln!("Loaded obj from bytes {} but it contained no meshes.", name);
            None
        }
        Err(err) => {
            eprintln!("Failed to load static mesh '{}' from bytes: {}", name, err);
            None
        }
    }
}

pub fn load_static_mesh(path: &str) -> Option<tobj::Mesh> {
    match tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS) {
        Ok((meshes, _)) if !meshes.is_empty() => Some(meshes[0].mesh.clone()),
        Ok(_) => {
            eprintln!("Loaded obj {} but it contained no meshes.", path);
            None
        }
        Err(err) => {
            eprintln!("Failed to load static mesh '{}': {}", path, err);
            None
        }
    }
}

pub fn process_mesh(mesh: &tobj::Mesh, texture: &Texture2D) -> macroquad::models::Mesh {
    let vertex_positions: Vec<Vec3> = mesh
        .positions
        .chunks(3)
        .map(|x| Vec3::new(x[0], x[1], x[2]))
        .collect();

    let texcoords: Vec<Vec2> = mesh
        .texcoords
        .chunks(2)
        .map(|x| Vec2::new(x[0], x[1]))
        .collect();

    let normals: Vec<Vec3> = mesh
        .normals
        .chunks(3)
        .map(|x| Vec3::new(x[0], x[1], x[2]))
        .collect();

    let mut vertices = Vec::new();
    let count = vertex_positions.len();

    for i in 0..count {
        let uv = *texcoords.get(i).unwrap_or(&Vec2::ZERO);
        
        let normal = if i < normals.len() {
            vec4(normals[i].x, normals[i].y, normals[i].z, 1.0)
        } else {
            vec4(0.0, 1.0, 0.0, 1.0)
        };

        vertices.push(Vertex {
            position: vertex_positions[i],
            uv,
            color: [255, 255, 255, 255],
            normal,
        });
    }

    macroquad::models::Mesh {
        vertices,
        indices: mesh.indices.iter().map(|x| *x as u16).collect(),
        texture: Some(texture.clone()),
    }
}

pub fn load_resources_and_mesh(
    mesh_filename: &str,
    texture_filename: &str,
) -> Result<macroquad::models::Mesh, Box<dyn Error>> {
    // Ideally, the base path should be configurable, not hardcoded.
    let texture_full_path = format!("/srv/http/{}", texture_filename);
    let mesh_full_path = format!("/srv/http/{}", mesh_filename);

    let img_path = Path::new(&texture_full_path);

    let texture = if img_path.exists() {
        match process_img(img_path) {
            Ok((w, h, bytes)) => {
                let img = Image {
                    bytes,
                    width: w as u16,
                    height: h as u16,
                };
                Texture2D::from_image(&img)
            }
            Err(_) => Texture2D::from_file_with_format(include_bytes!("checker.png"), None),
        }
    } else {
        Texture2D::from_file_with_format(include_bytes!("checker.png"), None)
    };

    let (meshes, _) = tobj::load_obj(&mesh_full_path, &tobj::GPU_LOAD_OPTIONS)?;

    if meshes.is_empty() {
        return Err("No data found in obj file.".into());
    }

    let mesh_data = meshes[0].mesh.clone();
    Ok(process_mesh(&mesh_data, &texture))
}

pub fn from_hex(hex: u32) -> [u8; 4] {
    let byte_1 = ((hex >> 16) & 0xFF) as u8;
    let byte_2 = ((hex >> 8) & 0xFF) as u8;
    let byte_3 = (hex & 0xFF) as u8;
    [byte_1, byte_2, byte_3, 255]
}

pub fn from_brickcolor(id: u16) -> Option<u32> {
    // Optimized: Replaced HashMap construction with a match expression (Jump Table)
    match id {
        1003 => Some(0x111111),
        148 => Some(0x575857),
        2 => Some(0xA1A5A2),
        1002 => Some(0xCDCDCD),
        40 => Some(0xECECEC),
        1001 => Some(0xF8F8F8),
        348 => Some(0xEDEAEA),
        349 => Some(0xE9DADA),
        1025 => Some(0xFFC9C9),
        337 => Some(0xFF9494),
        344 => Some(0x965555),
        1007 => Some(0xA34B4B),
        350 => Some(0x883E3E),
        339 => Some(0x562424),
        331 => Some(0xFF5959),
        332 => Some(0x750000),
        327 => Some(0x970000),
        1004 => Some(0xFF0000),
        360 => Some(0x966766),
        338 => Some(0xBE6862),
        153 => Some(0x957977),
        41 => Some(0xCD544B),
        21 => Some(0xC4281C),
        101 => Some(0xDA867A),
        47 => Some(0xD9856C),
        176 => Some(0x97695B),
        100 => Some(0xEEC4B6),
        123 => Some(0xD36F4C),
        216 => Some(0x904C2A),
        345 => Some(0x8F4C2A),
        193 => Some(0xCF6024),
        133 => Some(0xD5733D),
        192 => Some(0x694028),
        18 => Some(0xCC8E69),
        361 => Some(0x564236),
        359 => Some(0xAF9483),
        128 => Some(0xAE7A59),
        38 => Some(0xA05F35),
        355 => Some(0x6C584B),
        217 => Some(0x7C5C46),
        364 => Some(0x5A4C42),
        137 => Some(0xE09864),
        125 => Some(0xEAB892),
        25 => Some(0x624732),
        106 => Some(0xDA8541),
        12 => Some(0xCB8442),
        178 => Some(0xB48455),
        365 => Some(0x6A3909),
        1014 => Some(0xAA5500),
        1030 => Some(0xFFCC99),
        168 => Some(0x756C62),
        225 => Some(0xEBB87F),
        105 => Some(0xE29B40),
        121 => Some(0xE7AC58),
        36 => Some(0xF3CF9B),
        127 => Some(0xDCBC81),
        362 => Some(0x7E683F),
        351 => Some(0xBC9B5D),
        356 => Some(0xA0844F),
        346 => Some(0xD3BE96),
        352 => Some(0xC7AC78),
        224 => Some(0xF0D5A0),
        180 => Some(0xD7A94B),
        191 => Some(0xE8AB2D),
        108 => Some(0x685C43),
        138 => Some(0x958A73),
        209 => Some(0xB08E44),
        1017 => Some(0xFFAF00),
        1005 => Some(0xFFB000),
        333 => Some(0xEFB838),
        5 => Some(0xD7C59A),
        353 => Some(0xCABFA3),
        340 => Some(0xF1E7C7),
        334 => Some(0xF8D96D),
        24 => Some(0xF5CD30),
        190 => Some(0xF9D62E),
        226 => Some(0xFDEA8D),
        3 => Some(0xF9E999),
        341 => Some(0xFEF3BB),
        347 => Some(0xE2DCBC),
        157 => Some(0xFFF67B),
        49 => Some(0xF8F184),
        44 => Some(0xF7F18D),
        1008 => Some(0xC1BE42),
        1029 => Some(0xFFFFCC),
        1009 => Some(0xFFFF00),
        134 => Some(0xD8DD56),
        115 => Some(0xC7D23C),
        200 => Some(0x828A5D),
        120 => Some(0xD9E4A7),
        119 => Some(0xA4BD47),
        1022 => Some(0x7F8E64),
        319 => Some(0xB9C4B1),
        324 => Some(0xA8BD99),
        29 => Some(0xA1C48C),
        1021 => Some(0x3A7D15),
        317 => Some(0x7C9C6B),
        323 => Some(0x94BE81),
        6 => Some(0xC2DAB8),
        304 => Some(0x2C651D),
        310 => Some(0x5B9A4C),
        328 => Some(0xB1E5A6),
        318 => Some(0x8AAB85),
        313 => Some(0x1F801D),
        1028 => Some(0xCCFFCC),
        37 => Some(0x4B974B),
        1020 => Some(0x00FF00),
        309 => Some(0x348E40),
        301 => Some(0x506D54),
        48 => Some(0x84B68D),
        141 => Some(0x27462D),
        210 => Some(0x709578),
        28 => Some(0x287F47),
        151 => Some(0x789082),
        1027 => Some(0x9FF3E9),
        1018 => Some(0x12EED4),
        118 => Some(0xB7D7D5),
        1019 => Some(0x00FFFF),
        107 => Some(0x008F9C),
        116 => Some(0x55A5AF),
        1013 => Some(0x04AFEC),
        315 => Some(0x0989CF),
        232 => Some(0x7DBBDD),
        11 => Some(0x80BBDC),
        42 => Some(0xC1DFF0),
        329 => Some(0x98C2DB),
        45 => Some(0xB4D2E4),
        23 => Some(0x0D69AC),
        26 => Some(0x1B2A35),
        1024 => Some(0xAFDDFF),
        43 => Some(0x7BB6E8),
        212 => Some(0x9FC3E9),
        140 => Some(0x203A56),
        143 => Some(0xCFE2F7),
        306 => Some(0x335882),
        102 => Some(0x6E99CA),
        305 => Some(0x527CAE),
        336 => Some(0xC7D4E4),
        135 => Some(0x74869D),
        314 => Some(0x9FADC0),
        145 => Some(0x7988A1),
        195 => Some(0x4667A4),
        196 => Some(0x23478B),
        1012 => Some(0x2154B9),
        1011 => Some(0x002060),
        213 => Some(0x6C81B7),
        149 => Some(0x161D32),
        110 => Some(0x435493),
        112 => Some(0x6874AC),
        307 => Some(0x102ADC),
        303 => Some(0x0010B0),
        220 => Some(0xA7A9CE),
        126 => Some(0xA5A5CB),
        1010 => Some(0x0000FF),
        1026 => Some(0xB1A7FF),
        268 => Some(0x342B75),
        219 => Some(0x6B629B),
        1031 => Some(0x6225D1),
        308 => Some(0x3D1585),
        1006 => Some(0xB480FF),
        1023 => Some(0x8C5B9F),
        104 => Some(0x6B327C),
        218 => Some(0x96709F),
        322 => Some(0x7B2F7B),
        312 => Some(0x592259),
        316 => Some(0x7B007B),
        1015 => Some(0xAA00AA),
        198 => Some(0x8E4285),
        321 => Some(0xA75E9B),
        1032 => Some(0xFF00BF),
        124 => Some(0x923978),
        1016 => Some(0xFF66CC),
        343 => Some(0xD490BD),
        330 => Some(0xFF98DC),
        342 => Some(0xE0B2D0),
        22 => Some(0xC470A0),
        221 => Some(0xCD6298),
        158 => Some(0xE1A4C2),
        222 => Some(0xE4ADC8),
        113 => Some(0xE5ADC8),
        9 => Some(0xE8BAC8),
        223 => Some(0xDC9095),
        _ => None,
    }
}