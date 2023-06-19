use std::io::prelude::*;

use sfml::graphics::{
    Color, RectangleShape, RenderStates, RenderTarget, RenderTexture, Shader, Texture,
};
use sfml::system::Vector2f;

use std::net::SocketAddr;

use std::io::BufReader;
use std::net::TcpListener;

use httparse::Request;
use url::Url;

use clap::Parser;

#[derive(Parser, Debug)]
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

fn main() -> std::io::Result<()> {
    let args: Args = Args::parse();

    println!("Loading textures...");
    let earth_day = Texture::from_file("./earth_day.png").unwrap();
    let earth_night = Texture::from_file("./earth_night.png").unwrap();

    println!("Loading shaders...");
    let mut shader = &mut Shader::from_file_vert_frag("./vertex.vert", "./day-night-shader.frag")
        .unwrap() as *mut Shader;

    println!("Done!");

    let addr = args.addr;
    let listener = TcpListener::bind(addr)?;

    let width = args.width;
    let height = args.width / 2;
    let mut render_texture = RenderTexture::new(width, height).unwrap();
    let render_state = &mut RenderStates::default() as *mut RenderStates;
    deref_mut!(render_state).set_shader(Some(deref!(shader)));

    let shader = deref_mut!(shader);
    shader.set_uniform_texture("u_map_day", &earth_day);
    shader.set_uniform_texture("u_map_night", &earth_night);
    shader.set_uniform_vec2("u_resolution", Vector2f::new(width as f32, height as f32));

    let mut shape = RectangleShape::new();
    shape.set_size(Vector2f::new(width as f32, height as f32));

    println!("Listening on {addr}");

    for stream in listener.incoming() {
        let processing_start = std::time::Instant::now();

        let Ok(mut stream) = stream else { continue };

        let buf_reader = BufReader::new(&mut stream);
        let http_request = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect::<Vec<String>>();

        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = Request::new(&mut headers);

        if let Ok(_) = req.parse(http_request[0].as_bytes()) {
            let Some(path) = req.path else { continue };
            let Ok(parsed_url) = Url::parse(&format!("http://localhost:3000/{path}")) else { continue };

            let mut query_pairs = parsed_url.query_pairs();

            let mut lat = None;
            let mut lon = None;
            while let Some(pair) = query_pairs.next() {
                match &*pair.0 {
                    "lat" => {
                        if let Ok(val) = pair.1.parse::<f32>() {
                            lat = Some(val)
                        }
                    }
                    "lon" => {
                        if let Ok(val) = pair.1.parse::<f32>() {
                            lon = Some(val)
                        }
                    }
                    _ => (),
                }
            }

            if let (Some(lat), Some(lon)) = (lat, lon) {
                render_texture.clear(Color::rgb(0, 0, 0));
                shader.set_uniform_float("u_lat", lat * std::f32::consts::PI / 180.0);
                shader.set_uniform_float("u_lon", lon * std::f32::consts::PI / 180.0);
                render_texture.draw_with_renderstates(&shape, deref!(render_state));
                // render_texture.display();

                let pixels = render_texture.texture().copy_to_image().unwrap();
                let pixels = pixels.pixel_data();


                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type:image/png\r\n\r\n");


                {
                    use std::io::BufWriter;
                    let ref mut w = BufWriter::new(stream);

                    let mut encoder = png::Encoder::new(w, width as u32, height as u32);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);

                    let mut writer = encoder.write_header().unwrap();
                    writer.write_image_data(pixels).unwrap();
                }

                println!(
                    "Took {}ms in total to process request for (lat,lon): ({lat}, {lon})",
                    processing_start.elapsed().as_millis()
                );
            }
        };
    }

    Ok(())
}
