use std::path::Path;

use linked_hash_map::LinkedHashMap;
use tokio::io::AsyncWriteExt;

use crate::sprite::{Color8, RawImage, RawSprite, definition::Sprite};

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
    sprite_table_path: &Path,
    out_path: &Path,
    sprite_collection_name: &str,
    collection: LinkedHashMap<String, Sprite>,
) -> anyhow::Result<()> {
    let mut header_output = "#include <graphx.h>\n".to_string();
    let mut c_output = header_output.clone();

    for (sprite_suffix, sprite) in collection {
        let (width, height, pixels) = RawImage::load_sprite(sprite_table_path, sprite).await?;
        let sprite_suffix_upper = sprite_suffix.to_uppercase();

        header_output += &format!(
            "\n#define SPRITE_{sprite_suffix_upper}_WIDTH {width}\n\
            #define SPRITE_{sprite_suffix_upper}_HEIGHT {height}\n\
            extern const gfx_sprite_t sprite_{sprite_suffix};\n"
        );

        c_output += &format!(
            "\nconst gfx_sprite_t sprite_{sprite_suffix} = {{\n    \
                .width = {width},\n    \
                .height = {height},\n    \
                .data = {{"
        );

        let raw_sprite = RawSprite::new(pixels, width);
        generate_c_sprite_pixels_rgb(&mut c_output, &raw_sprite);

        c_output += "\n    }\n};\n";
    }

    let c_out = out_path.join(format!("{sprite_collection_name}.c"));
    let mut file = tokio::fs::File::create(c_out).await?;
    file.write_all(c_output.as_bytes()).await?;

    let header_out = out_path.join(format!("{sprite_collection_name}.h"));
    let mut file = tokio::fs::File::create(header_out).await?;
    file.write_all(header_output.as_bytes()).await?;

    Ok(())
}
