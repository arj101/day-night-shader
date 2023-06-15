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
}

fn main() -> std::io::Result<()> {
    let args: Args = Args::parse();

    println!("Loading textures...");
    let earth_day = Texture::from_file("./earth_day.png").unwrap();
    let earth_night = Texture::from_file("./earth_night.png").unwrap();

    println!("Loading shaders...");
    let mut shader =
        Shader::from_file_vert_frag("./vertex.vert", "./day-night-shader.frag").unwrap();

    println!("Done!");

    let addr = args.addr;
    let listener = TcpListener::bind(addr)?;

    println!("Listening on {addr}");

    for stream in listener.incoming() {
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
            let mut width = None;
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
                    "width" => {
                        if let Ok(val) = pair.1.parse::<f32>() {
                            width = Some(val)
                        }
                    }
                    _ => (),
                }
            }

            if let (Some(lat), Some(lon)) = (lat, lon) {
                let width = width.unwrap_or(800.0);
                let height = width * 0.5;

                

                println!("Canvas size: {}x{}, (lat,lon): ({lat}, {lon})", width as u32, height as u32);
                let mut render_texture = RenderTexture::new(width as u32, height as u32).unwrap();
                println!("Rendering...");
                render_texture.clear(Color::rgb(0, 0, 0));
                let mut state = RenderStates::default();

                shader
                    .set_uniform_vec2("u_resolution", Vector2f::new(width.floor(), height.floor()));
                shader.set_uniform_float("u_lat", lat * std::f32::consts::PI/180.0);
                shader.set_uniform_float("u_lon", lon * std::f32::consts::PI/180.0);
                shader.set_uniform_texture("u_map_day", &earth_day);
                shader.set_uniform_texture("u_map_night", &earth_night);
                state.set_shader(Some(&shader));

                let mut shape = RectangleShape::new();
                shape.set_size(Vector2f::new(width, height));
                render_texture.draw_with_renderstates(&shape, &state);
                render_texture.display();

                
                println!("Temp saving image...");
                let img = render_texture.texture().copy_to_image();
                assert!(img.unwrap().save_to_file(&args.tmp_file), "Failed to save image");
                println!("Loading it back...");
                let img = std::fs::read(&args.tmp_file).unwrap();

                println!("Sending image...");
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type:image/png\r\n\r\n",
                );
                let _ = stream.write_all(&img);
            }
        };
    }

    Ok(())
}
