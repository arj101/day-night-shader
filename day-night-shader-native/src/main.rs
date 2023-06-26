use futures_channel;
use sfml::graphics::{
    Color, IntRect, RectangleShape, RenderStates, RenderTarget, RenderTexture, Shader, Texture,
};
use sfml::system::Vector2f;
use std::io::prelude::*;
use tokio::time::Instant;

use std::net::SocketAddr;

use std::io::BufReader;

use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use axum::{http::Request, routing::get_service, Router};
use clap::Parser;
use hyper::http::Response;
use hyper::Body;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use url::Url;

use log::debug;
use log::error;
use log::info;
use log::trace;
use log::warn;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    //address to listen at
    #[arg(short, long, default_value_t = SocketAddr::from(([127,0,0,1], 3000)))]
    addr: SocketAddr,

    ///temporary file used to save the rendered image
    #[arg(short, long, default_value_t = String::from("/tmp/__day_night_shader_rendered__.png"))]
    tmp_file: String,

    #[arg(short, long, default_value_t = 1400)]
    width: u32,
}

macro_rules! deref_mut {
    ($ptr:expr) => {
        unsafe { &mut *$ptr }
    };
}

macro_rules! deref {
    ($ptr:expr) => {
        unsafe { &*$ptr }
    };
}

struct RenderRequest {
    lat: f32,
    lon: f32,
    res_channel: futures_channel::oneshot::Sender<Vec<u8>>,
}

fn run_shader(args: Args, mut rx: futures_channel::mpsc::UnboundedReceiver<RenderRequest>) {
    info!(
        "Using a {}x{}px canvas for rendering",
        args.width as u32,
        args.width / 2
    );

    trace!("Loading textures...");
    let mut earth_day = Texture::new().unwrap();
    let mut earth_night = Texture::new().unwrap();

    //embed the images if in release mode
    #[cfg(not(debug_assertions))]
    {
        earth_day
            .load_from_memory(include_bytes!("../earth_day.png"), IntRect::default())
            .unwrap();
        earth_night
            .load_from_memory(include_bytes!("../earth_night.png"), IntRect::default())
            .unwrap();
    }

    //load the images from file if in debug mode
    #[cfg(debug_assertions)]
    {
        earth_day
            .load_from_file("./earth_day.png", IntRect::default())
            .unwrap();
        earth_night
            .load_from_file("./earth_night.png", IntRect::default())
            .unwrap();
    }

    trace!("Loading shaders...");

    //embed the shaders if in release mode
    #[cfg(not(debug_assertions))]
    let shader = &mut Shader::from_memory_vert_frag(
        include_str!("../vertex.vert"),
        include_str!("../day-night-shader.frag"),
    )
    .unwrap() as *mut Shader;

    //load the shaders from file if in debug mode
    #[cfg(debug_assertions)]
    let shader = &mut Shader::from_file_vert_frag("./vertex.vert", "./day-night-shader.frag")
        .unwrap() as *mut Shader;

    trace!("Finished loading!");

    let width = args.width;
    let height = args.width / 2;

    let mut render_texture =
        RenderTexture::new(width, height).expect("Failed to create SFML RenderTexture.");
    let render_state = &mut RenderStates::default() as *mut RenderStates;
    deref_mut!(render_state).set_shader(Some(deref!(shader)));
    let shader = deref_mut!(shader);
    shader.set_uniform_texture("u_map_day", &earth_day);
    shader.set_uniform_texture("u_map_night", &earth_night);
    shader.set_uniform_vec2("u_resolution", Vector2f::new(width as f32, height as f32));

    let mut shape = RectangleShape::new();
    shape.set_size(Vector2f::new(width as f32, height as f32));

    loop {
        if let Ok(Some(RenderRequest {
            lat,
            lon,
            res_channel,
        })) = rx.try_next()
        {
            render_texture.clear(Color::rgb(0, 0, 0));

            let slat = lat * std::f32::consts::PI / 180.;
            let slon = lon * std::f32::consts::PI / 180.;

            shader.set_uniform_vec3(
                "u_sun_dir",
                sfml::graphics::glsl::Vec3::new(
                    f32::cos(slat) * f32::cos(slon),
                    f32::cos(slat) * f32::sin(slon),
                    f32::sin(slat),
                ),
            );
            render_texture.draw_with_renderstates(&shape, deref!(render_state));

            let pixels = render_texture.texture().copy_to_image().unwrap();
            let pixels = pixels.pixel_data().to_vec();

            if let Err(_) = res_channel.send(pixels) {
                warn!("Failed to send rendered image to http handler task")
            } else {
                trace!("Rendered and sent image for (lat, lon): ({lat}, {lon})");
            }
        }
    }
}

use std::collections::HashMap;
use std::io::BufWriter;
use std::sync::Arc;

async fn service(
    tx: futures_channel::mpsc::UnboundedSender<RenderRequest>,
    args: Arc<Args>,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let mut process_start = None;
    debug!("{}", {
        process_start = Some(Instant::now());
        "Received http request"
    });

    let result = req.uri().query().map(|queries| {
        let params = url::form_urlencoded::parse(queries.as_bytes())
            .into_owned()
            .collect::<HashMap<String, String>>();

        let (lat, lon) = (
            params.get("lat").map(|v| v.parse::<f32>()),
            params.get("lon").map(|v| v.parse::<f32>()),
        );

        if let (Some(Ok(lat)), Some(Ok(lon))) = (lat, lon) {
            Some((lat, lon))
        } else {
            None
        }
    });

    let result = match result {
        Some(Some((lat, lon))) => {
            let (res_tx, res_rx) = futures_channel::oneshot::channel();

            if let Err(_) = tx.unbounded_send(RenderRequest {
                lat,
                lon,
                res_channel: res_tx,
            }) {
                error!("Failed to send render request to rendering thread");
                None
            } else {
                match res_rx.await {
                    Ok(pixels) => {
                        let mut buf =
                            Vec::with_capacity(args.width as usize * args.width as usize * 4);

                        {
                            let ref mut w = BufWriter::new(&mut buf);

                            let mut encoder = png::Encoder::new(
                                w,
                                args.width as u32,
                                (args.width as f32 * 0.5) as u32,
                            );
                            encoder.set_color(png::ColorType::Rgba);
                            encoder.set_depth(png::BitDepth::Eight);

                            let mut writer = encoder.write_header().unwrap();
                            writer.write_image_data(&pixels).unwrap();
                        }

                        debug!(
                            "Took {}ms to process request",
                            process_start.unwrap().elapsed().as_millis()
                        );
                        Some(buf)
                    }
                    _ => None,
                }
            }
        }
        _ => {
            debug!("Query parsing failed for an incoming request");
            None
        }
    };

    match result {
        Some(png_data) => Ok(Response::builder()
            .header("Content-Type", "image/png")
            .body(png_data.into())
            .unwrap()),

        _ => Ok(Response::builder()
            .status(StatusCode::IM_A_TEAPOT)
            .body(Body::from("<h1 style='font-size:90vh;'>L</h1>"))
            .unwrap()),
    }
}

use hyper::service::service_fn;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let args: Args = Args::parse();

    let (tx, rx) = futures_channel::mpsc::unbounded();

    let args_clone = args.clone();
    tokio::task::spawn_blocking(|| run_shader(args_clone, rx));

    let addr = args.addr;

    let listener = TcpListener::bind(addr).await?;
    info!("Listening on http://{}", addr);

    use hyper::server::conn::Http;

    let args = Arc::new(args);

    loop {
        let (stream, _) = listener.accept().await?;

        let args = Arc::clone(&args);
        let tx = tx.clone();

        tokio::spawn(async move {
            let service_built = service_fn(move |req| {
                let tx = tx.clone();
                let args = args.clone();
                service(tx, args, req)
            });
            if let Err(http_err) = Http::new()
                .http1_only(true)
                .http1_keep_alive(true)
                .serve_connection(stream, service_built)
                .await
            {
                debug!("Error while serving HTTP connection: {}", http_err);
            }
        });
    }
}
