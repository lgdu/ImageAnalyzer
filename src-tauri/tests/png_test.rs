use std::fs;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn parses_valid_png_file() {
    let png_path = project_root()
        .join("icons")
        .join("256x256.png")
        .to_str()
        .unwrap()
        .to_string();

    let result = image_analyzer::analyzer::png_parser::analyze_png(&png_path);
    let analysis = result.expect("Should parse valid PNG");

    assert_eq!(analysis.file_name, "256x256.png");
    assert!(analysis.width > 0, "Width should be positive");
    assert!(analysis.height > 0, "Height should be positive");
    assert!(
        !analysis.structure.is_empty(),
        "Should have chunk structure"
    );
    assert!(analysis.analysis_errors.is_empty(), "Should have no errors");
}

#[test]
fn png_has_ihdr_chunk() {
    let png_path = project_root()
        .join("icons")
        .join("128x128.png")
        .to_str()
        .unwrap()
        .to_string();

    let analysis = image_analyzer::analyzer::png_parser::analyze_png(&png_path).unwrap();

    let ihdr = analysis
        .structure
        .iter()
        .find(|b| b.name == "IHDR")
        .expect("PNG should have IHDR chunk");

    assert!(ihdr.decoded_info.is_some());
    let info = ihdr.decoded_info.as_ref().unwrap();
    assert!(info.contains("Width"));
    assert!(info.contains("Height"));
}

#[test]
fn png_has_idat_chunks() {
    let png_path = project_root()
        .join("icons")
        .join("icon.png")
        .to_str()
        .unwrap()
        .to_string();

    let analysis = image_analyzer::analyzer::png_parser::analyze_png(&png_path).unwrap();

    let idat_blocks: Vec<_> = analysis
        .structure
        .iter()
        .filter(|b| b.name == "IDAT")
        .collect();

    assert!(
        !idat_blocks.is_empty(),
        "PNG should have at least one IDAT chunk"
    );

    for block in &idat_blocks {
        assert!(
            block.data_preview.is_some(),
            "IDAT should have data preview"
        );
    }
}

#[test]
fn png_has_iend_chunk() {
    let png_path = project_root()
        .join("icons")
        .join("32x32.png")
        .to_str()
        .unwrap()
        .to_string();

    let analysis = image_analyzer::analyzer::png_parser::analyze_png(&png_path).unwrap();

    let iend = analysis
        .structure
        .iter()
        .find(|b| b.name == "IEND")
        .expect("PNG should have IEND chunk");

    assert!(iend.decoded_info.is_some());
}

#[test]
fn png_structure_blocks_have_offsets_and_lengths() {
    let png_path = project_root()
        .join("icons")
        .join("256x256.png")
        .to_str()
        .unwrap()
        .to_string();

    let analysis = image_analyzer::analyzer::png_parser::analyze_png(&png_path).unwrap();

    for block in &analysis.structure {
        assert!(
            block.offset >= 8,
            "Chunk offset should be >= 8 (after signature)"
        );
        assert!(block.length > 0, "Chunk length should be positive");
        assert!(!block.name.is_empty(), "Chunk name should not be empty");
        assert_eq!(block.name.len(), 4, "Chunk name should be 4 bytes");
    }
}

#[test]
fn first_chunk_is_ihdr() {
    let png_path = project_root()
        .join("icons")
        .join("icon.png")
        .to_str()
        .unwrap()
        .to_string();

    let analysis = image_analyzer::analyzer::png_parser::analyze_png(&png_path).unwrap();

    assert_eq!(
        analysis.structure[0].name, "IHDR",
        "First chunk should be IHDR"
    );
}

#[test]
fn last_chunk_is_iend() {
    let png_path = project_root()
        .join("icons")
        .join("icon.png")
        .to_str()
        .unwrap()
        .to_string();

    let analysis = image_analyzer::analyzer::png_parser::analyze_png(&png_path).unwrap();

    let last = analysis.structure.last().unwrap();
    assert_eq!(last.name, "IEND", "Last chunk should be IEND");
}

#[test]
fn rejects_file_with_invalid_signature() {
    // Create a temporary file with invalid PNG signature
    let tmp = std::env::temp_dir().join("test_invalid_sig.png");
    let mut data = vec![0u8; 16];
    data[0..8].copy_from_slice(b"\x00\x00\x00\x00\x00\x00\x00\x00");
    fs::write(&tmp, &data).unwrap();

    let result = image_analyzer::analyzer::png_parser::analyze_png(tmp.to_str().unwrap());
    assert!(result.is_err(), "Should reject invalid PNG signature");
    let err = result.unwrap_err();
    assert!(
        err.contains("signature") || err.contains("Invalid"),
        "Error should mention signature: {}",
        err
    );

    fs::remove_file(&tmp).ok();
}

#[test]
fn rejects_empty_file() {
    let tmp = std::env::temp_dir().join("test_empty.png");
    fs::write(&tmp, b"").unwrap();

    let result = image_analyzer::analyzer::png_parser::analyze_png(tmp.to_str().unwrap());
    assert!(result.is_err(), "Should reject empty file");

    fs::remove_file(&tmp).ok();
}

#[test]
fn rejects_nonexistent_file() {
    let result = image_analyzer::analyzer::png_parser::analyze_png("/nonexistent/path/file.png");
    assert!(result.is_err(), "Should reject nonexistent file");
}

#[test]
fn parses_png_with_text_metadata() {
    // Create a minimal PNG with tEXt chunk manually
    let png_data = build_png_with_text_chunk("Title", "Test Image", "Description", "A test PNG");
    let tmp = std::env::temp_dir().join("test_text_metadata.png");
    fs::write(&tmp, &png_data).unwrap();

    let analysis = image_analyzer::analyzer::png_parser::analyze_png(tmp.to_str().unwrap())
        .expect("Should parse PNG with text metadata");

    assert!(analysis.width > 0);
    assert!(analysis.height > 0);
    assert!(
        !analysis.metadata.is_empty(),
        "Should have metadata entries"
    );

    let titles: Vec<_> = analysis
        .metadata
        .iter()
        .filter(|m| m.tag_name == "tEXt:Title")
        .collect();
    assert!(!titles.is_empty(), "Should have Title metadata");
    assert_eq!(titles[0].tag_value, "Test Image");

    let descriptions: Vec<_> = analysis
        .metadata
        .iter()
        .filter(|m| m.tag_name == "tEXt:Description")
        .collect();
    assert!(!descriptions.is_empty(), "Should have Description metadata");
    assert_eq!(descriptions[0].tag_value, "A test PNG");

    fs::remove_file(&tmp).ok();
}

#[test]
fn parses_png_with_gamma_chunk() {
    let png_data = build_png_with_gamma(45000); // gamma = 0.45
    let tmp = std::env::temp_dir().join("test_gamma.png");
    fs::write(&tmp, &png_data).unwrap();

    let analysis = image_analyzer::analyzer::png_parser::analyze_png(tmp.to_str().unwrap())
        .expect("Should parse PNG with gamma");

    let gama = analysis
        .structure
        .iter()
        .find(|b| b.name == "gAMA")
        .expect("Should have gAMA chunk");

    assert!(gama.decoded_info.is_some());
    let info = gama.decoded_info.as_ref().unwrap();
    assert!(info.contains("Gamma"));
    assert!(info.contains("0.45"));

    fs::remove_file(&tmp).ok();
}

#[test]
fn parses_all_chunks_count_correctly() {
    let png_path = project_root()
        .join("icons")
        .join("icon.png")
        .to_str()
        .unwrap()
        .to_string();

    let analysis = image_analyzer::analyzer::png_parser::analyze_png(&png_path).unwrap();

    // Count chunk types
    let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for block in &analysis.structure {
        *counts.entry(block.name.clone()).or_insert(0) += 1;
    }

    assert_eq!(
        *counts.get("IHDR").unwrap_or(&0),
        1,
        "Should have exactly 1 IHDR"
    );
    assert_eq!(
        *counts.get("IEND").unwrap_or(&0),
        1,
        "Should have exactly 1 IEND"
    );
    assert!(
        *counts.get("IDAT").unwrap_or(&0) >= 1,
        "Should have at least 1 IDAT"
    );
}

/// Build a minimal valid PNG with tEXt chunks.
fn build_png_with_text_chunk(
    keyword1: &str,
    value1: &str,
    keyword2: &str,
    value2: &str,
) -> Vec<u8> {
    let mut data = Vec::new();

    // PNG signature
    data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR chunk (13 bytes data)
    let ihdr_data: Vec<u8> = vec![
        0, 0, 0, 1, // width = 1
        0, 0, 0, 1, // height = 1
        8, // bit depth = 8
        2, // color type = 2 (RGB)
        0, // compression method
        0, // filter method
        0, // interlace method
    ];
    write_chunk(&mut data, b"IHDR", &ihdr_data);

    // First tEXt chunk
    let mut text1_data = keyword1.as_bytes().to_vec();
    text1_data.push(0); // null separator
    text1_data.extend_from_slice(value1.as_bytes());
    write_chunk(&mut data, b"tEXt", &text1_data);

    // Second tEXt chunk
    let mut text2_data = keyword2.as_bytes().to_vec();
    text2_data.push(0);
    text2_data.extend_from_slice(value2.as_bytes());
    write_chunk(&mut data, b"tEXt", &text2_data);

    // Minimal IDAT chunk (1x1 RGB pixel, compressed)
    // Uncompressed: filter byte (0) + RGB (3 bytes) = [0, 255, 0, 0]
    use miniz_oxide::deflate::compress_to_vec;
    let raw = vec![0u8, 255, 0, 0]; // filter=0, R=255, G=0, B=0
    let compressed = compress_to_vec(&raw, 6);
    write_chunk(&mut data, b"IDAT", &compressed);

    // IEND chunk (no data)
    write_chunk(&mut data, b"IEND", &[]);

    data
}

/// Build a minimal valid PNG with gAMA chunk.
fn build_png_with_gamma(gamma_int: u32) -> Vec<u8> {
    let mut data = Vec::new();

    // PNG signature
    data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR chunk
    let ihdr_data: Vec<u8> = vec![0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0];
    write_chunk(&mut data, b"IHDR", &ihdr_data);

    // gAMA chunk (4 bytes, gamma * 100000)
    let gama_data = gamma_int.to_be_bytes();
    write_chunk(&mut data, b"gAMA", &gama_data);

    // IDAT chunk
    use miniz_oxide::deflate::compress_to_vec;
    let raw = vec![0u8, 255, 0, 0];
    let compressed = compress_to_vec(&raw, 6);
    write_chunk(&mut data, b"IDAT", &compressed);

    // IEND chunk
    write_chunk(&mut data, b"IEND", &[]);

    data
}

/// Write a PNG chunk: length(4) + name(4) + data + crc(4)
fn write_chunk(data: &mut Vec<u8>, name: &[u8; 4], chunk_data: &[u8]) {
    // Length (big-endian u32)
    data.extend_from_slice(&(chunk_data.len() as u32).to_be_bytes());
    // Name
    data.extend_from_slice(name);
    // Data
    data.extend_from_slice(chunk_data);
    // CRC (CRC32 of name + data)
    let crc = crc32fast::hash(&[name.as_slice(), chunk_data].concat());
    data.extend_from_slice(&crc.to_be_bytes());
}
