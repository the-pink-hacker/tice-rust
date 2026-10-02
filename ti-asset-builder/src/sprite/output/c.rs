use std::path::Path;

use tokio::io::AsyncWriteExt;

use crate::sprite::{Color8, RawSprite};

fn rgb_line_to_string_c(rgb: &[Color8]) -> String {
    rgb.iter()
        .map(|pixel| format!("0x{:X}", u8::from(*pixel)))
        .collect::<Vec<_>>()
        .join(",")
}

fn generate_c_sprite_pixels_rgb(output: &mut String, sprite: &RawSprite) {
    output.push_str(
        &sprite
            .pixels
            .chunks_exact(sprite.width as usize)
            .map(rgb_line_to_string_c)
            .map(|line| format!("\n        {}", line))
            .collect::<Vec<String>>()
            .join(","),
    );
}

pub async fn generate_sprite_file_c(
    sprites: Vec<RawSprite>,
    name: &str,
    out_path: &Path,
) -> anyhow::Result<()> {
    let mut header_output = "#pragma once\n\n#include <graphx.h>\n".to_string();
    let header_name = format!("{}.h", name);
    let mut c_output = format!("#include \"{header_name}\"\n");

    for sprite in sprites {
        let name = &sprite.name;
        let width = sprite.width;
        let height = sprite.height;
        let sprite_suffix_upper = name.to_uppercase();

        header_output += &format!(
            "\n#define SPRITE_{sprite_suffix_upper}_WIDTH {width}\n\
            #define SPRITE_{sprite_suffix_upper}_HEIGHT {height}\n\
            extern const gfx_sprite_t SPRITE_{sprite_suffix_upper};\n"
        );

        c_output += &format!(
            "\nconst gfx_sprite_t sprite_{name} = {{\n    \
                .width = {width},\n    \
                .height = {height},\n    \
                .data = {{"
        );

        generate_c_sprite_pixels_rgb(&mut c_output, &sprite);

        c_output += "\n    }\n};\n";
    }

    let c_out = out_path.join(format!("{name}.c"));
    let mut file = tokio::fs::File::create(c_out).await?;
    file.write_all(c_output.as_bytes()).await?;

    let header_out = out_path.join(header_name);
    let mut file = tokio::fs::File::create(header_out).await?;
    file.write_all(header_output.as_bytes()).await?;

    Ok(())
}
