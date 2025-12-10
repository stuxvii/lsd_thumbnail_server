use base64::Engine;
use dotenv::dotenv;
use macroquad::prelude::*;
use memory_stats::memory_stats;
use png::{BitDepth, ColorType, Encoder};
use rouille::{post_input, router};
use sqlx::mysql::MySqlPool;
use std::sync::mpsc::{Sender, channel};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, thread};
use chrono::{DateTime, Utc};

mod utility;
use crate::utility::{
    BodyColors, ItemAsset, fetch_accessories_info, fetch_avatar, from_brickcolor, from_hex,
    load_resources_and_mesh, load_static_mesh_from_bytes, load_static_mesh, process_img, process_mesh,
    replace_transparent_with_color,
};

const PROGRAM_NAME: &str = "LSDBLOX Avatar Server 1.1";
const BASE_HTTP_PATH: &str = "/srv/http";

const DEFAULT_MESH_BYTES: &[u8] = include_bytes!("default.obj");
const RARM_MESH_BYTES: &[u8]    = include_bytes!("rightarm.obj");
const LARM_MESH_BYTES: &[u8]    = include_bytes!("leftarm.obj");
const RLEG_MESH_BYTES: &[u8]    = include_bytes!("rightleg.obj");
const LLEG_MESH_BYTES: &[u8]    = include_bytes!("leftleg.obj");
const TRSO_MESH_BYTES: &[u8]    = include_bytes!("torso.obj");
const TSHIRT_MESH_BYTES: &[u8]  = include_bytes!("tshirt.obj");

pub struct StaticMeshes {
    pub head: Option<tobj::Mesh>,
    pub rarm: Option<tobj::Mesh>,
    pub larm: Option<tobj::Mesh>,
    pub rleg: Option<tobj::Mesh>,
    pub lleg: Option<tobj::Mesh>,
    pub trso: Option<tobj::Mesh>,
    pub tshirt: Option<tobj::Mesh>,
}

struct HexBodyColors {
    head: u32,
    trso: u32,
    larm: u32,
    rarm: u32,
    lleg: u32,
    rleg: u32,
}

fn render_scene(
    accessories: Vec<ItemAsset>,
    colors: HexBodyColors,
    static_meshes: &StaticMeshes,
) -> String {
    let now: DateTime<Utc> = Utc::now();
    println!("[{}] STARTED RENDER", now.format("%d-%m-%Y %H:%M:%S"));

    let yaw: f32 = 1.0;
    let pitch: f32 = 0.4;
    let radius: f32 = 10.0;
    let target = vec3(-0.25, -1.75, -1.0);
    let world_up = vec3(0.0, 1.0, 0.0);

    let new_pos = vec3(
        radius * yaw.cos() * pitch.cos(),
        radius * pitch.sin(),
        radius * yaw.sin() * pitch.cos(),
    ) + target;

    clear_background(Color::with_alpha(&Color::from_hex(0x000000), 0.0));

    set_camera(&Camera3D {
        position: new_pos,
        up: world_up,
        target,
        ..Default::default()
    });

    let face_loc = std::path::Path::new("src/face.png");
    let mut face_texture = match process_img(face_loc) {
        Ok((w, h, bytes)) => Texture2D::from_rgba8(
            w as u16,
            h as u16,
            &replace_transparent_with_color(bytes, colors.head),
        ),
        Err(e) => {
            eprintln!("Default face couldn't be loaded: {}", e);
            Texture2D::from_rgba8(1, 1, &[255, 0, 0, 255])
        }
    };

    let mut head_mesh_data: Option<tobj::Mesh> = static_meshes.head.clone();
    let rarm_mesh_data: Option<tobj::Mesh> = static_meshes.rarm.clone();
    let larm_mesh_data: Option<tobj::Mesh> = static_meshes.larm.clone();
    let rleg_mesh_data: Option<tobj::Mesh> = static_meshes.rleg.clone();
    let lleg_mesh_data: Option<tobj::Mesh> = static_meshes.lleg.clone();
    let trso_mesh_data: Option<tobj::Mesh> = static_meshes.trso.clone();

    let mut rarm_texture = Texture2D::from_rgba8(1, 1, &from_hex(colors.rarm));
    let mut larm_texture = Texture2D::from_rgba8(1, 1, &from_hex(colors.larm));
    let mut rleg_texture = Texture2D::from_rgba8(1, 1, &from_hex(colors.rleg));
    let mut lleg_texture = Texture2D::from_rgba8(1, 1, &from_hex(colors.lleg));
    let mut trso_texture = Texture2D::from_rgba8(1, 1, &from_hex(colors.trso));

    let mut tshirt_meshes = Vec::new();

    for accessory in accessories {
        let loc = accessory.location.clone().unwrap_or_default();
        if loc.is_empty() {
            continue;
        }

        match accessory.item_type {
            9 => {
                // HAT
                let tex_path = accessory.texture_path.clone().unwrap_or_default();
                if let Ok(m) = load_resources_and_mesh(&loc, &tex_path) {
                    draw_mesh(&m);
                }
            }
            8 => {
                // HEAD SWAP
                let mesh_full_path = format!("{}/{}", BASE_HTTP_PATH, loc);
                if let Some(new_mesh) = load_static_mesh(&mesh_full_path) {
                    head_mesh_data = Some(new_mesh);
                }
            }
            7 => {
                // FACE TEXTURE
                let tmp_path = format!("{}/{}", BASE_HTTP_PATH, loc);
                if let Ok((w, h, bytes)) = process_img(std::path::Path::new(&tmp_path)) {
                    face_texture = Texture2D::from_rgba8(
                        w as u16,
                        h as u16,
                        &replace_transparent_with_color(bytes, colors.head),
                    );
                }
            }
            6 => {
                // PANTS
                let tmp_path = format!("{}/{}", BASE_HTTP_PATH, loc);
                if let Ok((w, h, bytes)) = process_img(std::path::Path::new(&tmp_path)) {
                    rleg_texture = Texture2D::from_rgba8(
                        w as u16,
                        h as u16,
                        &replace_transparent_with_color(bytes.clone(), colors.rleg),
                    );
                    lleg_texture = Texture2D::from_rgba8(
                        w as u16,
                        h as u16,
                        &replace_transparent_with_color(bytes, colors.lleg),
                    );
                }
            }
            5 => {
                // SHIRT
                let tmp_path = format!("{}/{}", BASE_HTTP_PATH, loc);
                if let Ok((w, h, bytes)) = process_img(std::path::Path::new(&tmp_path)) {
                    trso_texture = Texture2D::from_rgba8(
                        w as u16,
                        h as u16,
                        &replace_transparent_with_color(bytes.clone(), colors.trso),
                    );
                    rarm_texture = Texture2D::from_rgba8(
                        w as u16,
                        h as u16,
                        &replace_transparent_with_color(bytes.clone(), colors.rarm),
                    );
                    larm_texture = Texture2D::from_rgba8(
                        w as u16,
                        h as u16,
                        &replace_transparent_with_color(bytes, colors.larm),
                    );
                }
            }
            4 => {
                // T-SHIRT
                let tmp_path = format!("{}/{}", BASE_HTTP_PATH, loc);
                if let Ok((w, h, bytes)) = process_img(std::path::Path::new(&tmp_path)) {
                    if let Some(tshirt_mesh) = static_meshes.tshirt.clone() {
                        let texture = Texture2D::from_rgba8(w as u16, h as u16, &bytes);
                        let final_mesh = process_mesh(&tshirt_mesh, &texture);
                        tshirt_meshes.push(final_mesh);
                    }
                }
            }
            _ => {
                eprintln!("Item Type {} not implemented.", accessory.item_type)
            }
        }
    }

    gl_use_default_material();

    if let Some(mesh) = trso_mesh_data {
        draw_mesh(&process_mesh(&mesh, &trso_texture));
    }
    if let Some(mesh) = rarm_mesh_data {
        draw_mesh(&process_mesh(&mesh, &rarm_texture));
    }
    if let Some(mesh) = larm_mesh_data {
        draw_mesh(&process_mesh(&mesh, &larm_texture));
    }
    if let Some(mesh) = head_mesh_data {
        draw_mesh(&process_mesh(&mesh, &face_texture));
    }
    if let Some(mesh) = lleg_mesh_data {
        draw_mesh(&process_mesh(&mesh, &lleg_texture));
    }
    if let Some(mesh) = rleg_mesh_data {
        draw_mesh(&process_mesh(&mesh, &rleg_texture));
    }
    for mesh in tshirt_meshes {
        draw_mesh(&mesh);
    }

    let img_data = get_screen_data();
    let width = img_data.width as u32;
    let height = img_data.height as u32;

    let Some(image) = image::RgbaImage::from_raw(width, height, img_data.bytes) else {
        eprintln!("Failed to create image from screen data.");
        return String::new();
    };

    let flipped_bytes = image::imageops::flip_vertical(&image).into_vec();
    let mut png_data = Vec::new();
    {
        let mut encoder = Encoder::new(&mut png_data, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        match encoder.write_header() {
            Ok(mut writer) => {
                if let Err(e) = writer.write_image_data(&flipped_bytes) {
                    eprintln!("Failed to write PNG data: {}", e);
                    return String::new();
                }
            }
            Err(e) => {
                eprintln!("Failed to write PNG header: {}", e);
                return String::new();
            }
        }
    }

    base64::engine::general_purpose::STANDARD.encode(png_data)
}

fn window_conf() -> Conf {
    Conf {
        window_title: PROGRAM_NAME.to_owned(),
        window_width: 1024,
        window_height: 1024,
        window_resizable: false,
        ..Default::default()
    }
}

struct RenderRequest {
    accessories: Vec<ItemAsset>,
    bodycolors: Option<BodyColors>,
    job_type: u8,
    response_sender: Sender<String>,
    request_time: f64,
}

#[macroquad::main(window_conf)]
async fn main() {
    dotenv().ok();
    println!("{}", PROGRAM_NAME);
    println!("Licensed under the GPLv3.\n");

    let (tx_work, rx_work) = channel::<RenderRequest>();
    
    let static_meshes = StaticMeshes {
        head: load_static_mesh_from_bytes("default", DEFAULT_MESH_BYTES),
        rarm: load_static_mesh_from_bytes("rightarm", RARM_MESH_BYTES),
        larm: load_static_mesh_from_bytes("leftarm", LARM_MESH_BYTES),
        rleg: load_static_mesh_from_bytes("rightleg", RLEG_MESH_BYTES),
        lleg: load_static_mesh_from_bytes("leftleg", LLEG_MESH_BYTES),
        trso: load_static_mesh_from_bytes("torso", TRSO_MESH_BYTES),
        tshirt: load_static_mesh_from_bytes("tshirt", TSHIRT_MESH_BYTES),
    };

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

        let db_password = env::var("DB_PASSWORD").expect("DB_PASSWORD not set");
        let url = format!("mysql://usr:{}@localhost:3306/appdb", db_password);

        let pool = rt.block_on(async {
            MySqlPool::connect(&url)
                .await
                .expect("Failed to connect to DB")
        });

        let now: DateTime<Utc> = Utc::now();
        println!("[{}] STARTED SERVER ON PORT 6767 (unfunny)", now.format("%d-%m-%Y %H:%M:%S"));

        rouille::start_server("127.0.0.1:6767", move |request| {
            router!(request,
                (POST) (/) => {
                    let current_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64();
                    let now: DateTime<Utc> = Utc::now();
                    println!("[{}] INCOMING -- FROM {:?}", now.format("%d-%m-%Y %H:%M:%S"), request.remote_addr());


                    let body = match post_input!(request, { id: String, job_type: String }) {
                        Ok(d) => d,
                        Err(_) => return rouille::Response::empty_400(),
                    };

                    let type_val = match body.job_type.parse::<i32>() {
                        Ok(i) => i,
                        Err(_) => return rouille::Response::text("Invalid Number").with_status_code(400),
                    };

                    let id_val = match body.id.parse::<i32>() {
                        Ok(i) => i,
                        Err(_) => return rouille::Response::text("Invalid Number").with_status_code(400),
                    };

                    let (tx_answer, rx_answer) = channel();
                    let now: DateTime<Utc> = Utc::now();
                    println!("[{}] JOB TYPE: {}, ID: {}. REQUESTING RENDER", now.format("%d-%m-%Y %H:%M:%S"), type_val, id_val);

                    match type_val {
                        1 => {
                            let avatar_result = rt.block_on(async {
                                fetch_avatar(&pool, id_val).await
                            });

                            let (bodycolors, accessory_ids) = match avatar_result {
                                Ok(data) => data,
                                Err(e) => {
                                    eprintln!("DB Error for user {}: {}", id_val, e);
                                    return rouille::Response::text("User not found").with_status_code(404);
                                }
                            };

                            let accessories = rt.block_on(async {
                                match fetch_accessories_info(&pool, accessory_ids).await {
                                    Ok(a) => a,
                                    Err(e) => {
                                        eprintln!("Failed to fetch accessories for user {}: {}", id_val, e);
                                        Vec::new()
                                    }
                                }
                            });

                            let req = RenderRequest {
                                accessories,
                                bodycolors: Some(bodycolors),
                                job_type: 1,
                                response_sender: tx_answer,
                                request_time: current_time
                            };

                            if tx_work.send(req).is_err() {
                                return rouille::Response::text("Fatal error, server shutting down.").with_status_code(500);
                            }

                            match rx_answer.recv() {
                                Ok(base64_img) if !base64_img.is_empty() => rouille::Response::text(base64_img),
                                _ => rouille::Response::text("Render Failed").with_status_code(500),
                            }
                        },
                        2 => {
                            let accessories = rt.block_on(async {
                                match fetch_accessories_info(&pool, vec![id_val]).await {
                                    Ok(a) => a,
                                    Err(e) => {
                                        eprintln!("Failed to fetch accessories for user {}: {}", id_val, e);
                                        Vec::new()
                                    }
                                }
                            });

                            let req = RenderRequest {
                                accessories,
                                bodycolors: None,
                                job_type: 2,
                                response_sender: tx_answer,
                                request_time: current_time
                            };

                            if tx_work.send(req).is_err() {
                                return rouille::Response::text("Fatal error, server shutting down.").with_status_code(500);
                            }

                            match rx_answer.recv() {
                                Ok(base64_img) if !base64_img.is_empty() => rouille::Response::text(base64_img),
                                _ => rouille::Response::text("Render Failed").with_status_code(500),
                            }
                        },
                        _ => {
                            println!("they just tried requesting a bunch of hippy dippy baloney");
                            rouille::Response::text("Invalid job type").with_status_code(400)
                        }
                    }

                },
                _ => rouille::Response::empty_404()
            )
        });
    });

    let mut last_request_time: f64;
    loop {
        if let Ok(work) = rx_work.try_recv() {
            match work.job_type {
                1=> {
                    let body_colors = work.bodycolors.unwrap_or_default();
                    let hex_body_colors: HexBodyColors = HexBodyColors { 
                        head: from_brickcolor(body_colors.head).unwrap_or_default(), 
                        trso: from_brickcolor(body_colors.trso).unwrap_or_default(), 
                        larm: from_brickcolor(body_colors.larm).unwrap_or_default(), 
                        rarm: from_brickcolor(body_colors.rarm).unwrap_or_default(), 
                        lleg: from_brickcolor(body_colors.lleg).unwrap_or_default(), 
                        rleg: from_brickcolor(body_colors.rleg).unwrap_or_default()
                    };
                    let result_b64 = render_scene(work.accessories, hex_body_colors, &static_meshes);
                    let now: DateTime<Utc> = Utc::now();
                    println!("[{}] SUCCESS", now.format("%d-%m-%Y %H:%M:%S"));
                    let now: DateTime<Utc> = Utc::now();
                    println!("[{}] SENDING...", now.format("%d-%m-%Y %H:%M:%S"));
                    let _ = work.response_sender.send(result_b64);
                },
                2 => {
                    let accessory: ItemAsset = match work.accessories.first() {
                        Some(a) => a.clone(),
                        None => {
                            eprintln!("What the fuck? ok this should never happen this is so fucking weird bro HELPPPPPPP I'M GONNA EXPLODE!!!");
                            unreachable!();
                        }
                    };

                    let colors: HexBodyColors = HexBodyColors { trso: 0xbfbfbf, head: 0xbfbfbf, lleg: 0xbfbfbf, larm: 0xbfbfbf, rarm: 0xbfbfbf, rleg: 0xbfbfbf };
                    let result_b64 = render_scene(vec![accessory], colors, &static_meshes);
                    let now: DateTime<Utc> = Utc::now();
                    println!("[{}] SUCCESS", now.format("%d-%m-%Y %H:%M:%S"));
                    let now: DateTime<Utc> = Utc::now();
                    println!("[{}] SENDING...", now.format("%d-%m-%Y %H:%M:%S"));
                    let _ = work.response_sender.send(result_b64);
                }
                _ => {
                    unreachable!()
                }
            }

            let current_time: f64 = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            last_request_time = current_time - work.request_time;
            let now: DateTime<Utc> = Utc::now();
            println!("[{}] FINISHED -- TOOK {}s.", now.format("%d-%m-%Y %H:%M:%S"), last_request_time);

        }

        set_default_camera();
        clear_background(BLACK);

        if let Some(usage) = memory_stats() {
            draw_text(
                format!("MEM: {}K", usage.physical_mem / 1024).as_str(),
                10.0,
                16.0,
                24.0,
                WHITE,
            );
            draw_text(
                format!(
                    "SWAP: {}K",
                    usage.virtual_mem / 1024
                )
                .as_str(),
                10.0,
                32.0,
                24.0,
                WHITE,
            );
        } else {
            println!("Couldn't get the current memory usage :(");
        }

        next_frame().await;
    }
}
